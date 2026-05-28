//! Application state and event loop orchestration.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event};
use rusqlite::Connection;

use crate::alerts;
use crate::db;
use crate::db::models::AppConfig;
use crate::ghostty::{self, GhosttySession};
use crate::metrics::MetricsCollector;
use crate::process::{ManagedProcess, ProcessState};
use crate::tui;
use crate::tui::file_browser::{handle_browser_key, BrowserAction, FileBrowser};
use crate::tui::form::{handle_form_key, FormAction, FormState};
use crate::tui::input::{handle_key_event, Action, FocusMode};
use crate::tui::ui;
use crate::vt::Scrollback;

/// Main application state.
pub struct App {
    pub running: bool,
    pub focus: FocusMode,
    pub apps: Vec<AppConfig>,
    pub selected_index: usize,
    pub processes: HashMap<i64, ManagedProcess>,
    pub scrollbacks: HashMap<i64, Scrollback>,
    pub ghostty_sessions: HashMap<i64, GhosttySession>,
    pub metrics: MetricsCollector,
    pub scroll_offset: usize,
    pub alerts: Vec<String>,
    pub runtime_alert_ids: Vec<i64>,
    pub show_delete_confirm: bool,
    pub form: Option<FormState>,
    pub file_browser: Option<FileBrowser>,
    db: Connection,
}

impl App {
    /// Initialize the app: open DB, load apps, prepare metrics collector.
    pub fn new() -> Result<Self> {
        let db = db::init()?;
        let apps = db::operations::get_all_apps(&db)?;

        let mut scrollbacks = HashMap::new();
        for app_config in &apps {
            scrollbacks.insert(app_config.id, Scrollback::new(10_000));
        }

        Ok(Self {
            running: true,
            focus: FocusMode::AppList,
            apps,
            selected_index: 0,
            processes: HashMap::new(),
            scrollbacks,
            ghostty_sessions: HashMap::new(),
            metrics: MetricsCollector::new(),
            scroll_offset: 0,
            alerts: Vec::new(),
            runtime_alert_ids: Vec::new(),
            show_delete_confirm: false,
            file_browser: None,
            form: None,
            db,
        })
    }

    /// Get the currently selected app config.
    pub fn selected_app(&self) -> Option<&AppConfig> {
        self.apps.get(self.selected_index)
    }

    /// Get the ID of the currently selected app.
    pub fn selected_app_id(&self) -> Option<i64> {
        self.selected_app().map(|a| a.id)
    }

    /// Parse env_vars JSON string into a HashMap.
    fn parse_env_vars(env_vars: &str) -> HashMap<String, String> {
        serde_json::from_str(env_vars).unwrap_or_default()
    }

    /// Auto-start apps that have auto_start=true.
    fn auto_start_apps(&mut self) {
        let apps_to_start: Vec<(i64, String, String, String)> = self
            .apps
            .iter()
            .filter(|a| a.auto_start)
            .map(|a| {
                (
                    a.id,
                    a.working_dir.clone(),
                    a.command.clone(),
                    a.env_vars.clone(),
                )
            })
            .collect();

        for (id, working_dir, command, env_vars) in apps_to_start {
            let mut proc = ManagedProcess::new(id);
            let env = Self::parse_env_vars(&env_vars);
            if let Err(e) = proc.start(&working_dir, &command, &env) {
                self.alerts
                    .push(format!("Failed to auto-start app {}: {}", id, e));
            }
            self.processes.insert(id, proc);
        }
    }
}

/// Entry point for the TUI application.
pub fn run() -> Result<()> {
    let mut terminal = tui::init()?;
    let mut app = App::new()?;

    // Auto-start apps marked with auto_start=true
    app.auto_start_apps();

    let tick_rate = Duration::from_secs(2);
    let mut last_tick = Instant::now();

    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // If file browser is active, route input there first
                if let Some(ref mut browser) = app.file_browser {
                    match handle_browser_key(key, browser) {
                        BrowserAction::Select(path) => {
                            // Update form working_dir if form is active
                            if let Some(ref mut form) = app.form {
                                form.working_dir = path.display().to_string();
                            }
                            app.file_browser = None;
                        }
                        BrowserAction::Cancel => {
                            app.file_browser = None;
                        }
                        BrowserAction::Continue => {}
                    }
                } else if app.focus == FocusMode::Form {
                    // If form is active, route keys to the form handler
                    if let Some(ref mut form) = app.form {
                        match handle_form_key(key, form) {
                            FormAction::Save(new_app) => {
                                let edit_id = form.edit_id;
                                if let Some(id) = edit_id {
                                    // Update existing app
                                    if let Err(e) =
                                        db::operations::update_app(&app.db, id, &new_app)
                                    {
                                        app.alerts.push(format!("Failed to update app: {}", e));
                                    }
                                } else {
                                    // Create new app
                                    match db::operations::create_app(&app.db, &new_app) {
                                        Ok(_id) => {}
                                        Err(e) => {
                                            app.alerts.push(format!("Failed to create app: {}", e));
                                        }
                                    }
                                }
                                // Reload app list
                                app.apps = db::operations::get_all_apps(&app.db)?;
                                // Ensure scrollbacks exist for new apps
                                for app_config in &app.apps {
                                    app.scrollbacks
                                        .entry(app_config.id)
                                        .or_insert_with(|| Scrollback::new(10_000));
                                }
                                app.form = None;
                                app.focus = FocusMode::AppList;
                            }
                            FormAction::Cancel => {
                                app.form = None;
                                app.focus = FocusMode::AppList;
                            }
                            FormAction::OpenFileBrowser => {
                                // Open file browser starting at the form's current working_dir
                                if let Some(ref form) = app.form {
                                    let start = form.working_dir.clone();
                                    app.file_browser = Some(FileBrowser::new(&start));
                                }
                            }
                            FormAction::Continue => {}
                        }
                    }
                } else {
                    let action = handle_key_event(key, &app.focus);
                    handle_action(&mut app, action)?;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            handle_action(&mut app, Action::Tick)?;
            last_tick = Instant::now();
        }
    }

    // Stop all running processes before exit
    for (_id, proc) in app.processes.iter_mut() {
        let _ = proc.stop();
    }
    // Kill any active ghostty sessions
    for (_id, session) in app.ghostty_sessions.iter_mut() {
        let _ = session.kill();
    }

    tui::restore()?;
    Ok(())
}

/// Dispatch an action and mutate application state accordingly.
fn handle_action(app: &mut App, action: Action) -> Result<()> {
    match action {
        Action::Quit => {
            app.running = false;
        }
        Action::NavigateUp => {
            if !app.apps.is_empty() {
                if app.selected_index == 0 {
                    app.selected_index = app.apps.len() - 1;
                } else {
                    app.selected_index -= 1;
                }
                app.scroll_offset = 0;
            }
        }
        Action::NavigateDown => {
            if !app.apps.is_empty() {
                app.selected_index = (app.selected_index + 1) % app.apps.len();
                app.scroll_offset = 0;
            }
        }
        Action::StartApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                let env = App::parse_env_vars(&app_config.env_vars);
                let proc = app
                    .processes
                    .entry(app_config.id)
                    .or_insert_with(|| ManagedProcess::new(app_config.id));

                if matches!(
                    proc.state,
                    ProcessState::Stopped | ProcessState::Crashed { .. }
                ) {
                    if let Err(e) = proc.start(&app_config.working_dir, &app_config.command, &env) {
                        app.alerts
                            .push(format!("Failed to start {}: {}", app_config.name, e));
                    }
                    // Ensure scrollback exists
                    app.scrollbacks
                        .entry(app_config.id)
                        .or_insert_with(|| Scrollback::new(10_000));
                }
            }
        }
        Action::StopApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                if let Some(proc) = app.processes.get_mut(&app_config.id) {
                    if let Err(e) = proc.stop() {
                        app.alerts
                            .push(format!("Failed to stop {}: {}", app_config.name, e));
                    }
                }
            }
        }
        Action::RestartApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                let env = App::parse_env_vars(&app_config.env_vars);
                let proc = app
                    .processes
                    .entry(app_config.id)
                    .or_insert_with(|| ManagedProcess::new(app_config.id));

                if let Err(e) = proc.restart(&app_config.working_dir, &app_config.command, &env) {
                    app.alerts
                        .push(format!("Failed to restart {}: {}", app_config.name, e));
                }
                // Clear scrollback on restart
                if let Some(sb) = app.scrollbacks.get_mut(&app_config.id) {
                    sb.clear();
                }
            }
        }
        Action::FocusOutput => {
            if !app.apps.is_empty() {
                app.focus = FocusMode::OutputPane;
            }
        }
        Action::FocusAppList => {
            app.focus = FocusMode::AppList;
        }
        Action::ScrollUp => {
            if app.scroll_offset > 0 {
                app.scroll_offset -= 1;
            }
        }
        Action::ScrollDown => {
            if let Some(app_id) = app.selected_app_id() {
                if let Some(scrollback) = app.scrollbacks.get(&app_id) {
                    let total = scrollback.total_lines();
                    if app.scroll_offset < total.saturating_sub(1) {
                        app.scroll_offset += 1;
                    }
                }
            }
        }
        Action::ScrollToTop => {
            app.scroll_offset = 0;
        }
        Action::ScrollToBottom => {
            if let Some(app_id) = app.selected_app_id() {
                if let Some(scrollback) = app.scrollbacks.get(&app_id) {
                    let total = scrollback.total_lines();
                    app.scroll_offset = total.saturating_sub(1);
                }
            }
        }
        Action::ShowHelp => {
            app.focus = FocusMode::Help;
        }
        Action::HideHelp => {
            app.focus = FocusMode::AppList;
        }
        Action::Tick => {
            handle_tick(app);
        }
        Action::DeleteApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                // Stop process if running
                if let Some(proc) = app.processes.get_mut(&app_config.id) {
                    let _ = proc.stop();
                }
                app.processes.remove(&app_config.id);
                app.scrollbacks.remove(&app_config.id);

                // Delete from DB
                if let Err(e) = db::operations::delete_app(&app.db, app_config.id) {
                    app.alerts
                        .push(format!("Failed to delete {}: {}", app_config.name, e));
                } else {
                    // Remove from local list
                    app.apps.retain(|a| a.id != app_config.id);
                    if app.selected_index >= app.apps.len() && !app.apps.is_empty() {
                        app.selected_index = app.apps.len() - 1;
                    }
                }
            }
        }
        Action::AttachApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                // 1. Check if ghostty is available
                if !ghostty::is_ghostty_available() {
                    app.alerts.push("ghostty not found in PATH".to_string());
                } else {
                    // 2. Stop the managed process if running
                    if let Some(proc) = app.processes.get_mut(&app_config.id) {
                        if matches!(proc.state, ProcessState::Running { .. }) {
                            let _ = proc.stop();
                        }
                    }

                    // 3. Spawn Ghostty session
                    match GhosttySession::spawn(
                        &app_config.working_dir,
                        &app_config.command,
                        app_config.id,
                    ) {
                        Ok(session) => {
                            app.ghostty_sessions.insert(app_config.id, session);

                            // 4. Set process state to Attached
                            let proc = app
                                .processes
                                .entry(app_config.id)
                                .or_insert_with(|| ManagedProcess::new(app_config.id));
                            proc.state = ProcessState::Attached;
                        }
                        Err(e) => {
                            app.alerts
                                .push(format!("Failed to attach {}: {}", app_config.name, e));
                        }
                    }
                }
            }
        }
        Action::NewApp => {
            app.form = Some(FormState::new());
            app.focus = FocusMode::Form;
        }
        Action::EditApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                app.form = Some(FormState::from_app(&app_config));
                app.focus = FocusMode::Form;
            }
        }
        Action::Confirm | Action::Cancel | Action::None => {}
    }
    Ok(())
}

/// Handle periodic tick: refresh metrics, check processes, feed output to scrollbacks.
fn handle_tick(app: &mut App) {
    // 1. Refresh system metrics
    app.metrics.refresh();

    // 1.5. Check ghostty sessions
    let mut exited_sessions: Vec<i64> = Vec::new();
    for (app_id, session) in app.ghostty_sessions.iter_mut() {
        if !session.is_running() {
            exited_sessions.push(*app_id);
        }
    }
    for app_id in exited_sessions {
        app.ghostty_sessions.remove(&app_id);

        // Set state to Stopped and reset crash counter
        if let Some(proc) = app.processes.get_mut(&app_id) {
            proc.state = ProcessState::Stopped;
            proc.restart_policy.reset();
        }

        // If the app has auto_start, restart in managed mode
        let should_restart = app.apps.iter().find(|a| a.id == app_id).map(|a| {
            (
                a.auto_start,
                a.working_dir.clone(),
                a.command.clone(),
                a.env_vars.clone(),
            )
        });

        if let Some((true, working_dir, command, env_vars)) = should_restart {
            let env = App::parse_env_vars(&env_vars);
            let proc = app
                .processes
                .entry(app_id)
                .or_insert_with(|| ManagedProcess::new(app_id));
            if let Err(e) = proc.start(&working_dir, &command, &env) {
                app.alerts
                    .push(format!("Failed to restart app {}: {}", app_id, e));
            }
        }
    }

    // 2. Tick each process (handles auto-restart) and feed output to scrollbacks
    let app_configs: Vec<(i64, String, String, String)> = app
        .apps
        .iter()
        .map(|a| {
            (
                a.id,
                a.working_dir.clone(),
                a.command.clone(),
                a.env_vars.clone(),
            )
        })
        .collect();

    for (id, working_dir, command, env_vars) in &app_configs {
        if let Some(proc) = app.processes.get_mut(id) {
            let env = App::parse_env_vars(env_vars);
            proc.tick(working_dir, command, &env);

            // 3. Feed new output into scrollback
            let output = proc.get_output();
            if !output.is_empty() {
                proc.clear_output();
                if let Some(scrollback) = app.scrollbacks.get_mut(id) {
                    scrollback.feed(&output);
                }
            }
        }
    }

    // 4. Check runtime alerts (max_runtime_secs)
    let global_max_secs: u64 = db::operations::get_config(&app.db, "global_max_runtime_secs")
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(18000);

    let process_states: Vec<(i64, &ProcessState)> = app
        .processes
        .iter()
        .map(|(id, proc)| (*id, &proc.state))
        .collect();

    let runtime_alerts = alerts::check_runtime_alerts(&app.apps, &process_states, global_max_secs);

    // Replace alerts with fresh runtime alerts each tick
    app.alerts.retain(|a| !a.starts_with('\u{23f1}'));
    app.runtime_alert_ids.clear();
    for alert in &runtime_alerts {
        let msg = alerts::format_alert(alert);
        if !app.alerts.contains(&msg) {
            app.alerts.push(msg);
        }
        app.runtime_alert_ids.push(alert.app_id);
    }

    // Keep alerts list manageable
    if app.alerts.len() > 10 {
        app.alerts.drain(0..app.alerts.len() - 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new_creates_valid_state() {
        let app = App::new().unwrap();
        assert!(app.running);
        assert_eq!(app.focus, FocusMode::AppList);
        assert_eq!(app.selected_index, 0);
        assert_eq!(app.scroll_offset, 0);
        assert!(app.alerts.is_empty());
        assert!(!app.show_delete_confirm);
    }

    #[test]
    fn test_navigate_down_wraps() {
        let mut app = App::new().unwrap();
        // Add some fake apps to the list
        app.apps = vec![
            AppConfig {
                id: 1,
                name: "app1".to_string(),
                working_dir: "/tmp".to_string(),
                command: "echo 1".to_string(),
                env_vars: "{}".to_string(),
                auto_start: false,
                max_runtime_secs: None,
                created_at: "2024-01-01".to_string(),
            },
            AppConfig {
                id: 2,
                name: "app2".to_string(),
                working_dir: "/tmp".to_string(),
                command: "echo 2".to_string(),
                env_vars: "{}".to_string(),
                auto_start: false,
                max_runtime_secs: None,
                created_at: "2024-01-01".to_string(),
            },
        ];

        app.selected_index = 1;
        handle_action(&mut app, Action::NavigateDown).unwrap();
        assert_eq!(app.selected_index, 0); // Wrapped
    }

    #[test]
    fn test_navigate_up_wraps() {
        let mut app = App::new().unwrap();
        app.apps = vec![
            AppConfig {
                id: 1,
                name: "app1".to_string(),
                working_dir: "/tmp".to_string(),
                command: "echo 1".to_string(),
                env_vars: "{}".to_string(),
                auto_start: false,
                max_runtime_secs: None,
                created_at: "2024-01-01".to_string(),
            },
            AppConfig {
                id: 2,
                name: "app2".to_string(),
                working_dir: "/tmp".to_string(),
                command: "echo 2".to_string(),
                env_vars: "{}".to_string(),
                auto_start: false,
                max_runtime_secs: None,
                created_at: "2024-01-01".to_string(),
            },
        ];

        app.selected_index = 0;
        handle_action(&mut app, Action::NavigateUp).unwrap();
        assert_eq!(app.selected_index, 1); // Wrapped to end
    }

    #[test]
    fn test_focus_mode_transitions() {
        let mut app = App::new().unwrap();
        app.apps = vec![AppConfig {
            id: 1,
            name: "test".to_string(),
            working_dir: "/tmp".to_string(),
            command: "echo hi".to_string(),
            env_vars: "{}".to_string(),
            auto_start: false,
            max_runtime_secs: None,
            created_at: "2024-01-01".to_string(),
        }];

        assert_eq!(app.focus, FocusMode::AppList);

        handle_action(&mut app, Action::FocusOutput).unwrap();
        assert_eq!(app.focus, FocusMode::OutputPane);

        handle_action(&mut app, Action::FocusAppList).unwrap();
        assert_eq!(app.focus, FocusMode::AppList);

        handle_action(&mut app, Action::ShowHelp).unwrap();
        assert_eq!(app.focus, FocusMode::Help);

        handle_action(&mut app, Action::HideHelp).unwrap();
        assert_eq!(app.focus, FocusMode::AppList);
    }

    #[test]
    fn test_quit_action() {
        let mut app = App::new().unwrap();
        assert!(app.running);
        handle_action(&mut app, Action::Quit).unwrap();
        assert!(!app.running);
    }

    #[test]
    fn test_navigate_empty_list() {
        let mut app = App::new().unwrap();
        app.apps.clear();
        // Should not panic
        handle_action(&mut app, Action::NavigateDown).unwrap();
        handle_action(&mut app, Action::NavigateUp).unwrap();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_focus_output_with_empty_list() {
        let mut app = App::new().unwrap();
        app.apps.clear();
        handle_action(&mut app, Action::FocusOutput).unwrap();
        // Should not switch to output if no apps
        assert_eq!(app.focus, FocusMode::AppList);
    }

    #[test]
    fn test_parse_env_vars() {
        let env = App::parse_env_vars(r#"{"PORT":"3000","HOST":"localhost"}"#);
        assert_eq!(env.get("PORT"), Some(&"3000".to_string()));
        assert_eq!(env.get("HOST"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_parse_env_vars_empty() {
        let env = App::parse_env_vars("{}");
        assert!(env.is_empty());
    }

    #[test]
    fn test_parse_env_vars_invalid() {
        let env = App::parse_env_vars("not json");
        assert!(env.is_empty());
    }
}
