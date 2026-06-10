//! Application state and event loop orchestration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event};
use rusqlite::Connection;
use tokio::sync::Mutex as TokioMutex;

use crate::alerts;
use crate::db;
use crate::db::models::{ApiMask, AppConfig};
use crate::ghostty::{self, GhosttySession};
use crate::mask::log::RequestLog;
use crate::mask::proxy::{self, MaskConfig, MaskProxy};
use crate::metrics::MetricsCollector;
use crate::process::{ManagedProcess, ProcessState};
use crate::template::{self, TemplateVar};
use crate::tui;
use crate::tui::file_browser::{handle_browser_key, BrowserAction, FileBrowser};
use crate::tui::form::{handle_form_key, FormAction, FormState};
use crate::tui::input::{handle_key_event, Action, ActiveTab, FocusMode};
use crate::tui::mask_form::{handle_mask_form_key, MaskFormAction, MaskFormState};
use crate::tui::ui;
use crate::vt::Scrollback;

/// Status of a running mask proxy.
#[derive(Debug, Clone, PartialEq)]
pub enum MaskStatus {
    Inactive,
    Active,
    Error(String),
}

/// State for the template variable override modal shown at start time.
#[derive(Debug, Clone)]
pub struct TemplateModal {
    pub app_id: i64,
    pub vars: Vec<TemplateVar>,
    pub values: Vec<String>,
    pub current_field: usize,
    pub cursor_pos: usize,
    pub is_restart: bool,
}

impl TemplateModal {
    /// Create a new template modal pre-filled with defaults.
    pub fn new(app_id: i64, vars: Vec<TemplateVar>, is_restart: bool) -> Self {
        let values: Vec<String> = vars
            .iter()
            .map(|v| v.default.clone().unwrap_or_default())
            .collect();
        let cursor_pos = values.first().map(|v| v.len()).unwrap_or(0);
        Self {
            app_id,
            vars,
            values,
            current_field: 0,
            cursor_pos,
            is_restart,
        }
    }

    /// Get the resolved values as a HashMap for substitution.
    pub fn resolved_values(&self) -> HashMap<String, String> {
        self.vars
            .iter()
            .zip(self.values.iter())
            .map(|(var, val)| (var.name.clone(), val.clone()))
            .collect()
    }

    /// Returns true if all required fields have a non-empty value.
    pub fn is_valid(&self) -> bool {
        self.vars.iter().zip(self.values.iter()).all(|(var, val)| {
            // Required vars (no default) must have a non-empty value
            if var.is_required() {
                !val.trim().is_empty()
            } else {
                true
            }
        })
    }
}

/// Main application state.
pub struct App {
    pub running: bool,
    pub focus: FocusMode,
    pub active_tab: ActiveTab,
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
    // Mask state
    pub masks: Vec<ApiMask>,
    pub mask_selected_index: usize,
    pub mask_scroll_offset: usize,
    pub mask_statuses: HashMap<i64, MaskStatus>,
    pub mask_proxies: HashMap<i64, MaskProxy>,
    pub mask_logs: HashMap<i64, Arc<TokioMutex<RequestLog>>>,
    pub mask_form: Option<MaskFormState>,
    // Template modal state
    pub template_modal: Option<TemplateModal>,
    // Cron scheduler state
    pub cron_next_fire: HashMap<i64, Instant>,
    pub cron_run_counts: HashMap<i64, u32>,
    pub cron_active: HashMap<i64, bool>,
    db: Connection,
}

impl App {
    /// Initialize the app: open DB, load apps, prepare metrics collector.
    pub fn new() -> Result<Self> {
        let db = db::init()?;
        let apps = db::operations::get_all_apps(&db)?;
        let masks = db::operations::get_all_masks(&db)?;

        let mut scrollbacks = HashMap::new();
        for app_config in &apps {
            scrollbacks.insert(app_config.id, Scrollback::new(10_000));
        }

        Ok(Self {
            running: true,
            focus: FocusMode::AppList,
            active_tab: ActiveTab::Apps,
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
            masks,
            mask_selected_index: 0,
            mask_scroll_offset: 0,
            mask_statuses: HashMap::new(),
            mask_proxies: HashMap::new(),
            mask_logs: HashMap::new(),
            mask_form: None,
            template_modal: None,
            cron_next_fire: HashMap::new(),
            cron_run_counts: HashMap::new(),
            cron_active: HashMap::new(),
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

    /// Get the currently selected mask config.
    pub fn selected_mask(&self) -> Option<&ApiMask> {
        self.masks.get(self.mask_selected_index)
    }

    /// Get the ID of the currently selected mask.
    pub fn selected_mask_id(&self) -> Option<i64> {
        self.selected_mask().map(|m| m.id)
    }

    /// Parse env_vars JSON string into a HashMap.
    fn parse_env_vars(env_vars: &str) -> HashMap<String, String> {
        serde_json::from_str(env_vars).unwrap_or_default()
    }

    /// Auto-start apps that have auto_start=true.
    fn auto_start_apps(&mut self) {
        let apps_to_start: Vec<(i64, String, String, String, bool, Option<i64>, Vec<TemplateVar>)> = self
            .apps
            .iter()
            .filter(|a| a.auto_start)
            .map(|a| {
                (
                    a.id,
                    a.working_dir.clone(),
                    a.command.clone(),
                    a.env_vars.clone(),
                    a.is_cron(),
                    a.interval_seconds,
                    a.template_vars.clone(),
                )
            })
            .collect();

        for (id, working_dir, command, env_vars, is_cron, interval_seconds, tmpl_vars) in apps_to_start {
            // Check if this app has required template vars without defaults
            if template::has_required_variables(&tmpl_vars) {
                self.alerts.push(format!(
                    "App '{}' requires input before starting",
                    self.apps.iter().find(|a| a.id == id).map(|a| a.name.as_str()).unwrap_or("unknown")
                ));
                continue;
            }

            if is_cron {
                // For cron jobs, activate the scheduler instead of starting immediately
                let interval = interval_seconds.unwrap_or(60) as u64;
                self.cron_active.insert(id, true);
                self.cron_next_fire
                    .insert(id, Instant::now() + Duration::from_secs(interval));
            } else {
                // Resolve template variables with defaults
                let resolved_command;
                let resolved_working_dir;
                let resolved_env_vars;

                if !tmpl_vars.is_empty() {
                    let values = match template::resolve_values(&tmpl_vars, &HashMap::new()) {
                        Ok(v) => v,
                        Err(e) => {
                            self.alerts.push(format!("Failed to resolve template vars for app {}: {}", id, e));
                            continue;
                        }
                    };
                    resolved_command = template::substitute(&command, &values).unwrap_or(command.clone());
                    resolved_working_dir = template::substitute(&working_dir, &values).unwrap_or(working_dir.clone());
                    resolved_env_vars = template::substitute(&env_vars, &values).unwrap_or(env_vars.clone());
                } else {
                    resolved_command = command;
                    resolved_working_dir = working_dir;
                    resolved_env_vars = env_vars;
                }

                let mut proc = ManagedProcess::new(id);
                let env = Self::parse_env_vars(&resolved_env_vars);
                if let Err(e) = proc.start(&resolved_working_dir, &resolved_command, &env) {
                    self.alerts
                        .push(format!("Failed to auto-start app {}: {}", id, e));
                }
                self.processes.insert(id, proc);
            }
        }
    }

    /// Show the template variable override modal for the given app.
    pub fn show_template_modal(&mut self, app_id: i64, is_restart: bool) {
        if let Some(app_config) = self.apps.iter().find(|a| a.id == app_id) {
            let modal = TemplateModal::new(app_id, app_config.template_vars.clone(), is_restart);
            self.template_modal = Some(modal);
        }
    }
}

/// Entry point for the TUI application.
pub fn run() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let mut terminal = tui::init()?;
    let mut app = App::new()?;

    // Auto-start apps marked with auto_start=true
    app.auto_start_apps();

    // Auto-start masks marked with auto_start=true
    auto_start_masks(&mut app, &rt);

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
                } else if app.template_modal.is_some() {
                    // Template variable override modal is active
                    handle_template_modal_key(&mut app, key)?;
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
                } else if app.focus == FocusMode::MaskForm {
                    // If mask form is active, route keys to the mask form handler
                    if let Some(ref mut mask_form) = app.mask_form {
                        match handle_mask_form_key(key, mask_form) {
                            MaskFormAction::Save(new_mask) => {
                                let edit_id = mask_form.edit_id;
                                if let Some(id) = edit_id {
                                    if let Err(e) =
                                        db::operations::update_mask(&app.db, id, &new_mask)
                                    {
                                        app.alerts.push(format!("Failed to update mask: {}", e));
                                    }
                                } else {
                                    match db::operations::create_mask(&app.db, &new_mask) {
                                        Ok(_id) => {}
                                        Err(e) => {
                                            app.alerts
                                                .push(format!("Failed to create mask: {}", e));
                                        }
                                    }
                                }
                                // Reload mask list
                                app.masks = db::operations::get_all_masks(&app.db)?;
                                app.mask_form = None;
                                app.focus = FocusMode::MaskList;
                            }
                            MaskFormAction::Cancel => {
                                app.mask_form = None;
                                app.focus = FocusMode::MaskList;
                            }
                            MaskFormAction::Continue => {}
                        }
                    }
                } else {
                    let action = handle_key_event(key, &app.focus);
                    handle_action(&mut app, action, Some(&rt))?;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            handle_action(&mut app, Action::Tick, Some(&rt))?;
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
    // Shut down all mask proxies
    for (_id, proxy) in app.mask_proxies.drain() {
        proxy.handle.shutdown();
    }

    tui::restore()?;
    Ok(())
}

/// Activate a single mask proxy.
fn activate_mask(app: &mut App, mask: &ApiMask, rt: &tokio::runtime::Runtime) {
    // Parse headers from JSON
    let headers: HashMap<String, String> = serde_json::from_str(&mask.headers).unwrap_or_default();

    let config = MaskConfig {
        name: mask.name.clone(),
        target_url: mask.target_url.clone(),
        listen_port: mask.listen_port,
        headers,
    };

    match rt.block_on(proxy::start_proxy(config)) {
        Ok(mask_proxy) => {
            let log = Arc::clone(&mask_proxy.log);
            app.mask_proxies.insert(mask.id, mask_proxy);
            app.mask_logs.insert(mask.id, log);
            app.mask_statuses.insert(mask.id, MaskStatus::Active);
        }
        Err(e) => {
            let error_msg = e.to_string();
            app.mask_statuses.insert(mask.id, MaskStatus::Error(error_msg.clone()));
            app.alerts.push(format!("Mask '{}': {}", mask.name, error_msg));
        }
    }
}

/// Auto-start masks marked with auto_start=true.
fn auto_start_masks(app: &mut App, rt: &tokio::runtime::Runtime) {
    let auto_masks: Vec<ApiMask> = app
        .masks
        .iter()
        .filter(|m| m.auto_start)
        .cloned()
        .collect();

    for mask in &auto_masks {
        activate_mask(app, mask, rt);
    }
}

/// Handle key events when the template variable override modal is active.
fn handle_template_modal_key(app: &mut App, key: crossterm::event::KeyEvent) -> Result<()> {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Ctrl+C cancels
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.template_modal = None;
        return Ok(());
    }

    let modal = match app.template_modal.as_mut() {
        Some(m) => m,
        None => return Ok(()),
    };

    match key.code {
        KeyCode::Esc => {
            app.template_modal = None;
        }
        KeyCode::Tab => {
            // Move to next field
            if !modal.vars.is_empty() {
                modal.current_field = (modal.current_field + 1) % modal.vars.len();
                modal.cursor_pos = modal.values[modal.current_field].len();
            }
        }
        KeyCode::BackTab => {
            // Move to previous field
            if !modal.vars.is_empty() {
                if modal.current_field == 0 {
                    modal.current_field = modal.vars.len() - 1;
                } else {
                    modal.current_field -= 1;
                }
                modal.cursor_pos = modal.values[modal.current_field].len();
            }
        }
        KeyCode::Enter => {
            // Submit — validate and start the app
            if modal.is_valid() {
                let app_id = modal.app_id;
                let is_restart = modal.is_restart;
                let values = modal.resolved_values();
                app.template_modal = None;

                // Find the app config
                if let Some(app_config) = app.apps.iter().find(|a| a.id == app_id).cloned() {
                    let command = template::substitute(&app_config.command, &values)
                        .unwrap_or(app_config.command.clone());
                    let working_dir = template::substitute(&app_config.working_dir, &values)
                        .unwrap_or(app_config.working_dir.clone());
                    let env_vars_str = template::substitute(&app_config.env_vars, &values)
                        .unwrap_or(app_config.env_vars.clone());
                    let env = App::parse_env_vars(&env_vars_str);

                    // If restart, stop existing process first
                    if is_restart {
                        if let Some(proc) = app.processes.get_mut(&app_id) {
                            if matches!(proc.state, ProcessState::Running { .. }) {
                                let _ = proc.stop();
                            }
                        }
                    }

                    let proc = app
                        .processes
                        .entry(app_id)
                        .or_insert_with(|| ManagedProcess::new(app_id));

                    if matches!(
                        proc.state,
                        ProcessState::Stopped | ProcessState::Crashed { .. }
                    ) {
                        if let Err(e) = proc.start(&working_dir, &command, &env) {
                            app.alerts
                                .push(format!("Failed to start {}: {}", app_config.name, e));
                        }
                        app.scrollbacks
                            .entry(app_id)
                            .or_insert_with(|| Scrollback::new(10_000));
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::NONE)
                || key.modifiers.contains(KeyModifiers::SHIFT)
            {
                let field_idx = modal.current_field;
                let pos = modal.cursor_pos;
                modal.values[field_idx].insert(pos, c);
                modal.cursor_pos += 1;
            }
        }
        KeyCode::Backspace => {
            let field_idx = modal.current_field;
            if modal.cursor_pos > 0 {
                modal.cursor_pos -= 1;
                modal.values[field_idx].remove(modal.cursor_pos);
            }
        }
        KeyCode::Left => {
            if modal.cursor_pos > 0 {
                modal.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            let field_idx = modal.current_field;
            if modal.cursor_pos < modal.values[field_idx].len() {
                modal.cursor_pos += 1;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Dispatch an action and mutate application state accordingly.
fn handle_action(app: &mut App, action: Action, rt: Option<&tokio::runtime::Runtime>) -> Result<()> {
    match action {
        Action::Quit => {
            app.running = false;
        }
        Action::NavigateUp => {
            match app.active_tab {
                ActiveTab::Apps => {
                    if !app.apps.is_empty() {
                        if app.selected_index == 0 {
                            app.selected_index = app.apps.len() - 1;
                        } else {
                            app.selected_index -= 1;
                        }
                        app.scroll_offset = 0;
                    }
                }
                ActiveTab::Masks => {
                    if !app.masks.is_empty() {
                        if app.mask_selected_index == 0 {
                            app.mask_selected_index = app.masks.len() - 1;
                        } else {
                            app.mask_selected_index -= 1;
                        }
                        app.mask_scroll_offset = 0;
                    }
                }
            }
        }
        Action::NavigateDown => {
            match app.active_tab {
                ActiveTab::Apps => {
                    if !app.apps.is_empty() {
                        app.selected_index = (app.selected_index + 1) % app.apps.len();
                        app.scroll_offset = 0;
                    }
                }
                ActiveTab::Masks => {
                    if !app.masks.is_empty() {
                        app.mask_selected_index =
                            (app.mask_selected_index + 1) % app.masks.len();
                        app.mask_scroll_offset = 0;
                    }
                }
            }
        }
        Action::StartApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                if app_config.is_cron() {
                    // For cron jobs, "start" activates the scheduler
                    let interval = app_config.interval_seconds.unwrap_or(60) as u64;
                    app.cron_active.insert(app_config.id, true);
                    app.cron_next_fire
                        .insert(app_config.id, Instant::now() + Duration::from_secs(interval));
                    // Ensure scrollback exists
                    app.scrollbacks
                        .entry(app_config.id)
                        .or_insert_with(|| Scrollback::new(10_000));
                } else if app_config.has_required_template_vars() {
                    // Has required vars with no defaults — must show modal
                    app.show_template_modal(app_config.id, false);
                } else {
                    // Resolve template vars with defaults (if any) and start
                    let resolved = if app_config.has_template_vars() {
                        match template::resolve_values(&app_config.template_vars, &HashMap::new()) {
                            Ok(values) => Some(values),
                            Err(e) => {
                                app.alerts.push(format!("Template error for {}: {}", app_config.name, e));
                                None
                            }
                        }
                    } else {
                        None
                    };

                    if let Some(ref values) = resolved {
                        let command = template::substitute(&app_config.command, values).unwrap_or(app_config.command.clone());
                        let working_dir = template::substitute(&app_config.working_dir, values).unwrap_or(app_config.working_dir.clone());
                        let env_vars_str = template::substitute(&app_config.env_vars, values).unwrap_or(app_config.env_vars.clone());
                        let env = App::parse_env_vars(&env_vars_str);
                        let proc = app
                            .processes
                            .entry(app_config.id)
                            .or_insert_with(|| ManagedProcess::new(app_config.id));
                        if matches!(
                            proc.state,
                            ProcessState::Stopped | ProcessState::Crashed { .. }
                        ) {
                            if let Err(e) = proc.start(&working_dir, &command, &env) {
                                app.alerts.push(format!("Failed to start {}: {}", app_config.name, e));
                            }
                            app.scrollbacks
                                .entry(app_config.id)
                                .or_insert_with(|| Scrollback::new(10_000));
                        }
                    } else if resolved.is_none() && !app_config.has_template_vars() {
                        // No template vars at all — plain start
                        let env = App::parse_env_vars(&app_config.env_vars);
                        let proc = app
                            .processes
                            .entry(app_config.id)
                            .or_insert_with(|| ManagedProcess::new(app_config.id));
                        if matches!(
                            proc.state,
                            ProcessState::Stopped | ProcessState::Crashed { .. }
                        ) {
                            if let Err(e) =
                                proc.start(&app_config.working_dir, &app_config.command, &env)
                            {
                                app.alerts
                                    .push(format!("Failed to start {}: {}", app_config.name, e));
                            }
                            app.scrollbacks
                                .entry(app_config.id)
                                .or_insert_with(|| Scrollback::new(10_000));
                        }
                    }
                }
            }
        }
        Action::StopApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                if app_config.is_cron() {
                    // For cron jobs, "stop" deactivates the scheduler and kills running instance
                    app.cron_active.insert(app_config.id, false);
                    app.cron_next_fire.remove(&app_config.id);
                    if let Some(proc) = app.processes.get_mut(&app_config.id) {
                        if matches!(proc.state, ProcessState::Running { .. }) {
                            let _ = proc.stop();
                        }
                    }
                } else {
                    if let Some(proc) = app.processes.get_mut(&app_config.id) {
                        if let Err(e) = proc.stop() {
                            app.alerts
                                .push(format!("Failed to stop {}: {}", app_config.name, e));
                        }
                    }
                }
            }
        }
        Action::RestartApp => {
            if let Some(app_config) = app.selected_app().cloned() {
                if app_config.is_cron() {
                    // For cron jobs, "restart" means "run now" and reset timer
                    let interval = app_config.interval_seconds.unwrap_or(60) as u64;
                    let is_running = app
                        .processes
                        .get(&app_config.id)
                        .map(|p| matches!(p.state, ProcessState::Running { .. }))
                        .unwrap_or(false);

                    if !is_running {
                        // Insert run separator (cap at 10 runs)
                        let run_count = app.cron_run_counts.entry(app_config.id).or_insert(0);
                        *run_count += 1;
                        if *run_count > 10 {
                            if let Some(scrollback) = app.scrollbacks.get_mut(&app_config.id) {
                                scrollback.clear();
                            }
                            *run_count = 1;
                        }
                        let separator = format!(
                            "\r\n\x1b[90m─── run #{} ───\x1b[0m\r\n",
                            run_count
                        );
                        if let Some(scrollback) = app.scrollbacks.get_mut(&app_config.id) {
                            scrollback.feed(separator.as_bytes());
                        }

                        let env = App::parse_env_vars(&app_config.env_vars);
                        let proc = app
                            .processes
                            .entry(app_config.id)
                            .or_insert_with(|| ManagedProcess::new(app_config.id));
                        if let Err(e) =
                            proc.start(&app_config.working_dir, &app_config.command, &env)
                        {
                            app.alerts
                                .push(format!("Failed to run {}: {}", app_config.name, e));
                        }
                    }

                    // Reset timer from now
                    app.cron_active.insert(app_config.id, true);
                    app.cron_next_fire
                        .insert(app_config.id, Instant::now() + Duration::from_secs(interval));
                } else {
                    let env = App::parse_env_vars(&app_config.env_vars);
                    let proc = app
                        .processes
                        .entry(app_config.id)
                        .or_insert_with(|| ManagedProcess::new(app_config.id));

                    if let Err(e) =
                        proc.restart(&app_config.working_dir, &app_config.command, &env)
                    {
                        app.alerts
                            .push(format!("Failed to restart {}: {}", app_config.name, e));
                    }
                    // Clear scrollback on restart
                    if let Some(sb) = app.scrollbacks.get_mut(&app_config.id) {
                        sb.clear();
                    }
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
            match app.focus {
                FocusMode::OutputPane if app.scroll_offset > 0 => {
                    app.scroll_offset -= 1;
                }
                FocusMode::MaskLog if app.mask_scroll_offset > 0 => {
                    app.mask_scroll_offset -= 1;
                }
                _ => {}
            }
        }
        Action::ScrollDown => {
            match app.focus {
                FocusMode::OutputPane => {
                    if let Some(app_id) = app.selected_app_id() {
                        if let Some(scrollback) = app.scrollbacks.get(&app_id) {
                            let total = scrollback.total_lines();
                            if app.scroll_offset < total.saturating_sub(1) {
                                app.scroll_offset += 1;
                            }
                        }
                    }
                }
                FocusMode::MaskLog => {
                    if let Some(mask_id) = app.selected_mask_id() {
                        if let Some(log) = app.mask_logs.get(&mask_id) {
                            let total = log.blocking_lock().len();
                            if app.mask_scroll_offset < total.saturating_sub(1) {
                                app.mask_scroll_offset += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Action::ScrollToTop => {
            match app.focus {
                FocusMode::OutputPane => {
                    app.scroll_offset = 0;
                }
                FocusMode::MaskLog => {
                    app.mask_scroll_offset = 0;
                }
                _ => {}
            }
        }
        Action::ScrollToBottom => {
            match app.focus {
                FocusMode::OutputPane => {
                    if let Some(app_id) = app.selected_app_id() {
                        if let Some(scrollback) = app.scrollbacks.get(&app_id) {
                            let total = scrollback.total_lines();
                            app.scroll_offset = total.saturating_sub(1);
                        }
                    }
                }
                FocusMode::MaskLog => {
                    if let Some(mask_id) = app.selected_mask_id() {
                        if let Some(log) = app.mask_logs.get(&mask_id) {
                            let total = log.blocking_lock().len();
                            app.mask_scroll_offset = total.saturating_sub(1);
                        }
                    }
                }
                _ => {}
            }
        }
        Action::ShowHelp => {
            app.focus = FocusMode::Help;
        }
        Action::HideHelp => {
            match app.active_tab {
                ActiveTab::Apps => app.focus = FocusMode::AppList,
                ActiveTab::Masks => app.focus = FocusMode::MaskList,
            }
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
        // Tab switching
        Action::SwitchToApps => {
            app.active_tab = ActiveTab::Apps;
            app.focus = FocusMode::AppList;
        }
        Action::SwitchToMasks => {
            app.active_tab = ActiveTab::Masks;
            app.focus = FocusMode::MaskList;
        }
        // Mask navigation (reuse NavigateUp/Down based on active tab)
        Action::FocusMaskLog => {
            if !app.masks.is_empty() {
                app.focus = FocusMode::MaskLog;
                app.mask_scroll_offset = 0;
            }
        }
        Action::FocusMaskList => {
            app.focus = FocusMode::MaskList;
        }
        // Mask actions
        Action::ActivateMask => {
            if let Some(mask) = app.selected_mask().cloned() {
                if let Some(runtime) = rt {
                    activate_mask(app, &mask, runtime);
                }
            }
        }
        Action::DeactivateMask => {
            if let Some(mask) = app.selected_mask().cloned() {
                if let Some(proxy) = app.mask_proxies.remove(&mask.id) {
                    proxy.handle.shutdown();
                }
                app.mask_statuses.insert(mask.id, MaskStatus::Inactive);
                app.mask_logs.remove(&mask.id);
            }
        }
        Action::NewMask => {
            app.mask_form = Some(MaskFormState::new());
            app.focus = FocusMode::MaskForm;
        }
        Action::EditMask => {
            if let Some(mask) = app.selected_mask().cloned() {
                app.mask_form = Some(MaskFormState::from_mask(&mask));
                app.focus = FocusMode::MaskForm;
            }
        }
        Action::DeleteMask => {
            if let Some(mask) = app.selected_mask().cloned() {
                // Stop proxy if running
                if let Some(proxy) = app.mask_proxies.remove(&mask.id) {
                    proxy.handle.shutdown();
                }
                app.mask_statuses.remove(&mask.id);
                app.mask_logs.remove(&mask.id);

                // Delete from DB
                if let Err(e) = db::operations::delete_mask(&app.db, mask.id) {
                    app.alerts
                        .push(format!("Failed to delete mask {}: {}", mask.name, e));
                } else {
                    app.masks.retain(|m| m.id != mask.id);
                    if app.mask_selected_index >= app.masks.len() && !app.masks.is_empty() {
                        app.mask_selected_index = app.masks.len() - 1;
                    }
                }
            }
        }
        Action::StartAppWithOptions => {
            // Always show the template variable override modal before starting
            if let Some(app_config) = app.selected_app().cloned() {
                if app_config.has_template_vars() {
                    app.show_template_modal(app_config.id, false);
                } else {
                    // No template vars — behave like regular start
                    let env = App::parse_env_vars(&app_config.env_vars);
                    let proc = app
                        .processes
                        .entry(app_config.id)
                        .or_insert_with(|| ManagedProcess::new(app_config.id));
                    if matches!(
                        proc.state,
                        ProcessState::Stopped | ProcessState::Crashed { .. }
                    ) {
                        if let Err(e) =
                            proc.start(&app_config.working_dir, &app_config.command, &env)
                        {
                            app.alerts
                                .push(format!("Failed to start {}: {}", app_config.name, e));
                        }
                        app.scrollbacks
                            .entry(app_config.id)
                            .or_insert_with(|| Scrollback::new(10_000));
                    }
                }
            }
        }
        Action::RestartAppWithOptions => {
            // Show the template variable override modal before (re)starting
            if let Some(app_config) = app.selected_app().cloned() {
                if app_config.has_template_vars() {
                    app.show_template_modal(app_config.id, true);
                } else {
                    // No template vars — behave like regular restart
                    if let Some(proc) = app.processes.get_mut(&app_config.id) {
                        if matches!(proc.state, ProcessState::Running { .. }) {
                            let _ = proc.stop();
                        }
                    }
                    let env = App::parse_env_vars(&app_config.env_vars);
                    let proc = app
                        .processes
                        .entry(app_config.id)
                        .or_insert_with(|| ManagedProcess::new(app_config.id));
                    if let Err(e) =
                        proc.start(&app_config.working_dir, &app_config.command, &env)
                    {
                        app.alerts
                            .push(format!("Failed to start {}: {}", app_config.name, e));
                    }
                    app.scrollbacks
                        .entry(app_config.id)
                        .or_insert_with(|| Scrollback::new(10_000));
                }
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
    let app_configs: Vec<(i64, String, String, String, bool)> = app
        .apps
        .iter()
        .map(|a| {
            (
                a.id,
                a.working_dir.clone(),
                a.command.clone(),
                a.env_vars.clone(),
                a.is_cron(),
            )
        })
        .collect();

    for (id, working_dir, command, env_vars, is_cron) in &app_configs {
        if let Some(proc) = app.processes.get_mut(id) {
            if *is_cron {
                // For cron jobs, don't use the normal restart logic.
                // Just check if the process exited and mark it stopped.
                proc.tick_cron();
            } else {
                let env = App::parse_env_vars(env_vars);
                proc.tick(&working_dir, &command, &env);
            }

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

    // 5. Cron scheduler: fire due cron jobs
    let now = Instant::now();
    for (id, _working_dir, command, env_vars, is_cron) in &app_configs {
        if !is_cron {
            continue;
        }
        // Only fire if this cron job is active (scheduled)
        if !app.cron_active.get(id).copied().unwrap_or(false) {
            continue;
        }
        // Check if it's time to fire
        let should_fire = app
            .cron_next_fire
            .get(id)
            .map(|next| now >= *next)
            .unwrap_or(false);
        if !should_fire {
            continue;
        }
        // Skip if still running
        let is_running = app
            .processes
            .get(id)
            .map(|p| matches!(p.state, ProcessState::Running { .. }))
            .unwrap_or(false);
        if is_running {
            continue;
        }

        // Find the interval for this app
        let interval_secs = app
            .apps
            .iter()
            .find(|a| a.id == *id)
            .and_then(|a| a.interval_seconds)
            .unwrap_or(60);

        // Insert run separator into scrollback (cap at 10 runs)
        let run_count = app.cron_run_counts.entry(*id).or_insert(0);
        *run_count += 1;
        if *run_count > 10 {
            // Clear scrollback and reset counter to keep only last 10 runs
            if let Some(scrollback) = app.scrollbacks.get_mut(id) {
                scrollback.clear();
            }
            *run_count = 1;
        }
        let separator = format!(
            "\r\n\x1b[90m─── run #{} ───\x1b[0m\r\n",
            run_count
        );
        if let Some(scrollback) = app.scrollbacks.get_mut(id) {
            scrollback.feed(separator.as_bytes());
        }

        // Start the process
        let env = App::parse_env_vars(env_vars);
        let working_dir_for_cron = app
            .apps
            .iter()
            .find(|a| a.id == *id)
            .map(|a| a.working_dir.clone())
            .unwrap_or_default();
        let proc = app
            .processes
            .entry(*id)
            .or_insert_with(|| ManagedProcess::new(*id));
        if let Err(e) = proc.start(&working_dir_for_cron, command, &env) {
            app.alerts
                .push(format!("Cron job {} failed to start: {}", id, e));
        }

        // Reset timer: next fire at now + interval
        app.cron_next_fire
            .insert(*id, now + Duration::from_secs(interval_secs as u64));
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
                interval_seconds: None,
                template_vars: vec![],
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
                interval_seconds: None,
                template_vars: vec![],
                created_at: "2024-01-01".to_string(),
            },
        ];

        app.selected_index = 1;
        handle_action(&mut app, Action::NavigateDown, None).unwrap();
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
                interval_seconds: None,
                template_vars: vec![],
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
                interval_seconds: None,
                template_vars: vec![],
                created_at: "2024-01-01".to_string(),
            },
        ];

        app.selected_index = 0;
        handle_action(&mut app, Action::NavigateUp, None).unwrap();
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
            interval_seconds: None,
            template_vars: vec![],
            created_at: "2024-01-01".to_string(),
        }];

        assert_eq!(app.focus, FocusMode::AppList);

        handle_action(&mut app, Action::FocusOutput, None).unwrap();
        assert_eq!(app.focus, FocusMode::OutputPane);

        handle_action(&mut app, Action::FocusAppList, None).unwrap();
        assert_eq!(app.focus, FocusMode::AppList);

        handle_action(&mut app, Action::ShowHelp, None).unwrap();
        assert_eq!(app.focus, FocusMode::Help);

        handle_action(&mut app, Action::HideHelp, None).unwrap();
        assert_eq!(app.focus, FocusMode::AppList);
    }

    #[test]
    fn test_quit_action() {
        let mut app = App::new().unwrap();
        assert!(app.running);
        handle_action(&mut app, Action::Quit, None).unwrap();
        assert!(!app.running);
    }

    #[test]
    fn test_navigate_empty_list() {
        let mut app = App::new().unwrap();
        app.apps.clear();
        // Should not panic
        handle_action(&mut app, Action::NavigateDown, None).unwrap();
        handle_action(&mut app, Action::NavigateUp, None).unwrap();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_focus_output_with_empty_list() {
        let mut app = App::new().unwrap();
        app.apps.clear();
        handle_action(&mut app, Action::FocusOutput, None).unwrap();
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
