//! New/edit app form with inline validation.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::db::models::{AppConfig, NewApp};
use crate::process::health::check_app_health;
use crate::template::{self, TemplateVar};

/// Which field is currently focused in the form.
#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Name,
    WorkingDir,
    Command,
    EnvVars,
    AutoStart,
    MaxRuntime,
    Interval,
}

/// Actions returned from form key handling.
#[derive(Debug, Clone, PartialEq)]
pub enum FormAction {
    Continue,
    Save(NewApp),
    Cancel,
    OpenFileBrowser,
}

/// Mutable form state for creating or editing an app.
#[derive(Debug, Clone)]
pub struct FormState {
    pub fields: Vec<FormField>,
    pub current_field: usize,
    pub name: String,
    pub working_dir: String,
    pub command: String,
    pub env_vars: String,
    pub auto_start: bool,
    pub max_runtime: String,
    pub interval: String,
    pub errors: Vec<String>,
    pub editing: bool,
    pub edit_id: Option<i64>,
    pub cursor_pos: usize,
    pub template_vars: Vec<TemplateVar>,
}

impl Default for FormState {
    fn default() -> Self {
        Self::new()
    }
}

impl FormState {
    /// Create an empty form for a new app.
    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField::Name,
                FormField::WorkingDir,
                FormField::Command,
                FormField::EnvVars,
                FormField::AutoStart,
                FormField::MaxRuntime,
                FormField::Interval,
            ],
            current_field: 0,
            name: String::new(),
            working_dir: String::new(),
            command: String::new(),
            env_vars: String::new(),
            auto_start: false,
            max_runtime: String::new(),
            interval: String::new(),
            errors: Vec::new(),
            editing: false,
            edit_id: None,
            cursor_pos: 0,
            template_vars: Vec::new(),
        }
    }

    /// Create a pre-filled form from an existing app config.
    pub fn from_app(app: &AppConfig) -> Self {
        // Convert JSON env vars back to KEY=VALUE,KEY2=VALUE2 format
        let env_display = json_to_env_string(&app.env_vars);

        let max_runtime = match app.max_runtime_secs {
            Some(secs) => secs.to_string(),
            None => String::new(),
        };

        let interval = match app.interval_seconds {
            Some(secs) => secs.to_string(),
            None => String::new(),
        };

        let name_len = app.name.len();

        Self {
            fields: vec![
                FormField::Name,
                FormField::WorkingDir,
                FormField::Command,
                FormField::EnvVars,
                FormField::AutoStart,
                FormField::MaxRuntime,
                FormField::Interval,
            ],
            current_field: 0,
            name: app.name.clone(),
            working_dir: app.working_dir.clone(),
            command: app.command.clone(),
            env_vars: env_display,
            auto_start: app.auto_start,
            max_runtime,
            interval,
            errors: Vec::new(),
            editing: true,
            edit_id: Some(app.id),
            cursor_pos: name_len,
            template_vars: app.template_vars.clone(),
        }
    }

    /// Get the value of the currently focused field.
    pub fn current_field_value(&self) -> &str {
        match &self.fields[self.current_field] {
            FormField::Name => &self.name,
            FormField::WorkingDir => &self.working_dir,
            FormField::Command => &self.command,
            FormField::EnvVars => &self.env_vars,
            FormField::AutoStart => {
                if self.auto_start {
                    "true"
                } else {
                    "false"
                }
            }
            FormField::MaxRuntime => &self.max_runtime,
            FormField::Interval => &self.interval,
        }
    }

    /// Get a mutable reference to the current field's string value.
    /// For AutoStart, returns a dummy — use toggle_auto_start() instead.
    pub fn current_field_value_mut(&mut self) -> &mut String {
        match &self.fields[self.current_field] {
            FormField::Name => &mut self.name,
            FormField::WorkingDir => &mut self.working_dir,
            FormField::Command => &mut self.command,
            FormField::EnvVars => &mut self.env_vars,
            FormField::AutoStart => &mut self.name, // placeholder, use toggle instead
            FormField::MaxRuntime => &mut self.max_runtime,
            FormField::Interval => &mut self.interval,
        }
    }

    /// Move to the next field (wraps around).
    pub fn next_field(&mut self) {
        self.current_field = (self.current_field + 1) % self.fields.len();
        self.cursor_pos = self.current_field_value().len();
    }

    /// Move to the previous field (wraps around).
    pub fn prev_field(&mut self) {
        if self.current_field == 0 {
            self.current_field = self.fields.len() - 1;
        } else {
            self.current_field -= 1;
        }
        self.cursor_pos = self.current_field_value().len();
    }

    /// Toggle auto_start (only meaningful when on the AutoStart field).
    pub fn toggle_auto_start(&mut self) {
        self.auto_start = !self.auto_start;
    }

    /// Insert a character at the cursor position in the current field.
    pub fn insert_char(&mut self, c: char) {
        if self.fields[self.current_field] == FormField::AutoStart {
            return;
        }
        let len = self.current_field_value().len();
        if self.cursor_pos > len {
            self.cursor_pos = len;
        }
        let pos = self.cursor_pos;
        let value = self.current_field_value_mut();
        value.insert(pos, c);
        self.cursor_pos += 1;
    }

    /// Delete the character before the cursor (backspace).
    pub fn delete_char(&mut self) {
        if self.fields[self.current_field] == FormField::AutoStart {
            return;
        }
        if self.cursor_pos > 0 {
            let pos = self.cursor_pos;
            let value = self.current_field_value_mut();
            value.remove(pos - 1);
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor left within the current field.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right within the current field.
    pub fn move_cursor_right(&mut self) {
        let len = self.current_field_value().len();
        if self.cursor_pos < len {
            self.cursor_pos += 1;
        }
    }

    /// Validate the form, populating self.errors. Returns true if valid.
    pub fn validate(&mut self) -> bool {
        self.errors.clear();

        // 1. Name cannot be empty
        if self.name.trim().is_empty() {
            self.errors.push("Name cannot be empty".to_string());
        }

        // 2. Convert env vars and run health check
        let env_json = env_string_to_json(&self.env_vars);
        let health = check_app_health(&self.working_dir, &self.command, &env_json);
        for err in &health.errors {
            self.errors.push(err.to_string());
        }

        // 3. Validate max_runtime
        let trimmed = self.max_runtime.trim();
        if !trimmed.is_empty() {
            match trimmed.parse::<i64>() {
                Ok(v) if v <= 0 => {
                    self.errors
                        .push("Max runtime must be a positive number".to_string());
                }
                Err(_) => {
                    self.errors
                        .push("Max runtime must be a valid number".to_string());
                }
                _ => {}
            }
        }

        // 4. Validate interval
        let interval_trimmed = self.interval.trim();
        if !interval_trimmed.is_empty() {
            match interval_trimmed.parse::<i64>() {
                Ok(v) if v <= 0 => {
                    self.errors
                        .push("Interval must be a positive number (seconds)".to_string());
                }
                Err(_) => {
                    self.errors
                        .push("Interval must be a valid number (seconds)".to_string());
                }
                _ => {}
            }
        }

        self.errors.is_empty()
    }

    /// Convert form state into a NewApp struct ready for DB insertion.
    pub fn to_new_app(&self) -> NewApp {
        let env_json = env_string_to_json(&self.env_vars);
        let max_runtime_secs = self
            .max_runtime
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|v| *v > 0);

        let interval_seconds = self
            .interval
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|v| *v > 0);

        NewApp {
            name: self.name.trim().to_string(),
            working_dir: self.working_dir.trim().to_string(),
            command: self.command.trim().to_string(),
            env_vars: env_json,
            auto_start: self.auto_start,
            max_runtime_secs,
            interval_seconds,
            template_vars: self.template_vars.clone(),
        }
    }

    /// Detect template variables from command, working_dir, and env_vars fields.
    /// Syncs the detected variables with existing template_vars metadata.
    pub fn detect_template_vars(&mut self) {
        let env_json = env_string_to_json(&self.env_vars);
        let detected = template::extract_variables_from_fields(&[
            &self.command,
            &self.working_dir,
            &env_json,
        ]);
        self.template_vars = template::sync_template_vars(&self.template_vars, &detected);
    }
}

/// Handle a key event while the form is focused.
pub fn handle_form_key(key: KeyEvent, form: &mut FormState) -> FormAction {
    // Ctrl+C quits the form
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return FormAction::Cancel;
    }

    // Ctrl+B on working_dir field opens file browser
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('b')
        && form.fields[form.current_field] == FormField::WorkingDir
    {
        return FormAction::OpenFileBrowser;
    }

    match key.code {
        KeyCode::Esc => FormAction::Cancel,
        KeyCode::Tab => {
            // Detect template variables when leaving command, working_dir, or env_vars fields
            let current = &form.fields[form.current_field];
            if matches!(
                current,
                FormField::Command | FormField::WorkingDir | FormField::EnvVars
            ) {
                form.detect_template_vars();
            }
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                form.prev_field();
            } else {
                form.next_field();
            }
            FormAction::Continue
        }
        KeyCode::BackTab => {
            // Detect template variables when leaving command, working_dir, or env_vars fields
            let current = &form.fields[form.current_field];
            if matches!(
                current,
                FormField::Command | FormField::WorkingDir | FormField::EnvVars
            ) {
                form.detect_template_vars();
            }
            form.prev_field();
            FormAction::Continue
        }
        KeyCode::Enter => {
            if form.validate() {
                FormAction::Save(form.to_new_app())
            } else {
                FormAction::Continue
            }
        }
        KeyCode::Char(' ') => {
            if form.fields[form.current_field] == FormField::AutoStart {
                form.toggle_auto_start();
            } else {
                form.insert_char(' ');
            }
            FormAction::Continue
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::NONE)
                || key.modifiers.contains(KeyModifiers::SHIFT)
            {
                form.insert_char(c);
            }
            FormAction::Continue
        }
        KeyCode::Backspace => {
            form.delete_char();
            FormAction::Continue
        }
        KeyCode::Left => {
            form.move_cursor_left();
            FormAction::Continue
        }
        KeyCode::Right => {
            form.move_cursor_right();
            FormAction::Continue
        }
        _ => FormAction::Continue,
    }
}

/// Render the form as a centered overlay.
pub fn render_form(frame: &mut Frame, form: &FormState) {
    let area = frame.area();

    let popup_width = 50_u16.min(area.width.saturating_sub(4));
    let popup_height = 18_u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let title = if form.editing {
        " Edit App "
    } else {
        " New App "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Layout the form fields inside the inner area
    let field_labels = [
        "Name:",
        "Directory:",
        "Command:",
        "Env vars:",
        "Auto-start:",
        "Max runtime:",
        "Interval:",
    ];

    // Reserve lines: fields + blank + errors + blank + hints
    let mut lines: Vec<Line> = Vec::new();

    for (i, label) in field_labels.iter().enumerate() {
        let is_current = i == form.current_field;
        let label_style = if is_current {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let value_str = match &form.fields[i] {
            FormField::Name => form.name.clone(),
            FormField::WorkingDir => form.working_dir.clone(),
            FormField::Command => form.command.clone(),
            FormField::EnvVars => form.env_vars.clone(),
            FormField::AutoStart => {
                if form.auto_start {
                    "[x] yes".to_string()
                } else {
                    "[ ] no".to_string()
                }
            }
            FormField::MaxRuntime => {
                if form.max_runtime.is_empty() {
                    String::new()
                } else {
                    format!("{} secs", form.max_runtime)
                }
            }
            FormField::Interval => {
                if form.interval.is_empty() {
                    String::new()
                } else {
                    format!("{} secs", form.interval)
                }
            }
        };

        // For text fields, show cursor indicator
        let display_value = if is_current && form.fields[i] != FormField::AutoStart {
            let raw = match &form.fields[i] {
                FormField::Name => &form.name,
                FormField::WorkingDir => &form.working_dir,
                FormField::Command => &form.command,
                FormField::EnvVars => &form.env_vars,
                FormField::MaxRuntime => &form.max_runtime,
                FormField::Interval => &form.interval,
                _ => &form.name,
            };
            // Truncate for display
            let max_val_width = (inner.width as usize).saturating_sub(14);
            let display: String = if raw.len() > max_val_width {
                raw[..max_val_width].to_string()
            } else {
                raw.clone()
            };
            format!("[{}]", display)
        } else if form.fields[i] == FormField::AutoStart {
            value_str
        } else {
            let max_val_width = (inner.width as usize).saturating_sub(14);
            let display: String = if value_str.len() > max_val_width {
                value_str[..max_val_width].to_string()
            } else {
                value_str
            };
            format!("[{}]", display)
        };

        let value_style = if is_current {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Add suffix hint for working_dir
        let suffix = if form.fields[i] == FormField::WorkingDir && is_current {
            " ^B"
        } else {
            ""
        };

        let line = Line::from(vec![
            Span::styled(format!("  {:<12}", label), label_style),
            Span::styled(display_value, value_style),
            Span::styled(suffix, Style::default().fg(Color::DarkGray)),
        ]);
        lines.push(line);
    }

    // Blank line before errors
    lines.push(Line::from(""));

    // Show errors in red
    for err in &form.errors {
        let err_line = Line::from(Span::styled(
            format!("  ⚠ {}", err),
            Style::default().fg(Color::Red),
        ));
        lines.push(err_line);
    }

    // Add blank line if there were errors
    if !form.errors.is_empty() {
        lines.push(Line::from(""));
    }

    // Hints at the bottom
    lines.push(Line::from(Span::styled(
        "  [Tab] next  [Enter] save  [Esc] cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    // Set cursor position if on a text field
    if form.fields[form.current_field] != FormField::AutoStart {
        let cursor_y = popup_area.y + 1 + form.current_field as u16;
        // label takes "  " + 12 chars + "[" = 15 chars offset
        let field_x_offset = popup_area.x + 1 + 2 + 12 + 1; // border + padding + label + bracket
        let cursor_x = field_x_offset + form.cursor_pos as u16;
        if cursor_x < popup_area.x + popup_width - 1 && cursor_y < popup_area.y + popup_height - 1 {
            frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

/// Convert "KEY=VALUE,KEY2=VALUE2" format to JSON object string.
pub fn env_string_to_json(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "{}".to_string();
    }

    let mut map = serde_json::Map::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() {
                map.insert(
                    key.to_string(),
                    serde_json::Value::String(value.to_string()),
                );
            }
        }
    }

    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

/// Convert JSON object string back to "KEY=VALUE,KEY2=VALUE2" format for display.
fn json_to_env_string(json_str: &str) -> String {
    let trimmed = json_str.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return String::new();
    }

    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(serde_json::Value::Object(map)) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| {
                    let val = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    format!("{}={}", k, val)
                })
                .collect();
            pairs.join(",")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_form_state_new_creates_empty_form() {
        let form = FormState::new();
        assert_eq!(form.name, "");
        assert_eq!(form.working_dir, "");
        assert_eq!(form.command, "");
        assert_eq!(form.env_vars, "");
        assert!(!form.auto_start);
        assert_eq!(form.max_runtime, "");
        assert!(!form.editing);
        assert_eq!(form.edit_id, None);
        assert_eq!(form.current_field, 0);
        assert_eq!(form.cursor_pos, 0);
        assert!(form.errors.is_empty());
        assert_eq!(form.fields.len(), 7);
    }

    #[test]
    fn test_form_state_from_app_prefills_correctly() {
        let app = AppConfig {
            id: 42,
            name: "my-api".to_string(),
            working_dir: "/home/user/project".to_string(),
            command: "cargo run".to_string(),
            env_vars: r#"{"PORT":"3000","NODE_ENV":"dev"}"#.to_string(),
            auto_start: true,
            max_runtime_secs: Some(300),
            interval_seconds: None,
            template_vars: vec![],
            created_at: "2024-01-01".to_string(),
        };

        let form = FormState::from_app(&app);
        assert_eq!(form.name, "my-api");
        assert_eq!(form.working_dir, "/home/user/project");
        assert_eq!(form.command, "cargo run");
        // env_vars should be in KEY=VALUE format
        assert!(form.env_vars.contains("PORT=3000"));
        assert!(form.env_vars.contains("NODE_ENV=dev"));
        assert!(form.auto_start);
        assert_eq!(form.max_runtime, "300");
        assert!(form.editing);
        assert_eq!(form.edit_id, Some(42));
        assert_eq!(form.cursor_pos, 6); // len of "my-api"
    }

    #[test]
    fn test_next_field_cycles_through_fields() {
        let mut form = FormState::new();
        assert_eq!(form.current_field, 0);
        form.next_field();
        assert_eq!(form.current_field, 1);
        form.next_field();
        assert_eq!(form.current_field, 2);
        form.next_field();
        assert_eq!(form.current_field, 3);
        form.next_field();
        assert_eq!(form.current_field, 4);
        form.next_field();
        assert_eq!(form.current_field, 5);
        form.next_field();
        assert_eq!(form.current_field, 6);
        form.next_field();
        assert_eq!(form.current_field, 0); // wraps
    }

    #[test]
    fn test_prev_field_cycles_through_fields() {
        let mut form = FormState::new();
        assert_eq!(form.current_field, 0);
        form.prev_field();
        assert_eq!(form.current_field, 6); // wraps to end
        form.prev_field();
        assert_eq!(form.current_field, 5);
        form.prev_field();
        assert_eq!(form.current_field, 4);
    }

    #[test]
    fn test_insert_char() {
        let mut form = FormState::new();
        // current_field is Name (index 0)
        form.insert_char('h');
        form.insert_char('i');
        assert_eq!(form.name, "hi");
        assert_eq!(form.cursor_pos, 2);
    }

    #[test]
    fn test_delete_char() {
        let mut form = FormState::new();
        form.name = "hello".to_string();
        form.cursor_pos = 5;
        form.delete_char();
        assert_eq!(form.name, "hell");
        assert_eq!(form.cursor_pos, 4);
        form.delete_char();
        assert_eq!(form.name, "hel");
        assert_eq!(form.cursor_pos, 3);
    }

    #[test]
    fn test_delete_char_at_start_does_nothing() {
        let mut form = FormState::new();
        form.name = "hello".to_string();
        form.cursor_pos = 0;
        form.delete_char();
        assert_eq!(form.name, "hello");
        assert_eq!(form.cursor_pos, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut form = FormState::new();
        form.name = "hello".to_string();
        form.cursor_pos = 3;

        form.move_cursor_left();
        assert_eq!(form.cursor_pos, 2);

        form.move_cursor_right();
        assert_eq!(form.cursor_pos, 3);

        // Can't go past end
        form.cursor_pos = 5;
        form.move_cursor_right();
        assert_eq!(form.cursor_pos, 5);

        // Can't go below 0
        form.cursor_pos = 0;
        form.move_cursor_left();
        assert_eq!(form.cursor_pos, 0);
    }

    #[test]
    fn test_toggle_auto_start() {
        let mut form = FormState::new();
        assert!(!form.auto_start);
        form.toggle_auto_start();
        assert!(form.auto_start);
        form.toggle_auto_start();
        assert!(!form.auto_start);
    }

    #[test]
    fn test_validate_catches_empty_name() {
        let mut form = FormState::new();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        let valid = form.validate();
        assert!(!valid);
        assert!(form
            .errors
            .iter()
            .any(|e| e.contains("Name cannot be empty")));
    }

    #[test]
    fn test_validate_catches_invalid_working_dir() {
        let mut form = FormState::new();
        form.name = "test".to_string();
        form.working_dir = "/nonexistent/path/xyz123".to_string();
        form.command = "echo hi".to_string();
        let valid = form.validate();
        assert!(!valid);
        assert!(form.errors.iter().any(|e| e.contains("not found")));
    }

    #[test]
    fn test_validate_catches_bad_max_runtime() {
        let mut form = FormState::new();
        form.name = "test".to_string();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        form.max_runtime = "abc".to_string();
        let valid = form.validate();
        assert!(!valid);
        assert!(form.errors.iter().any(|e| e.contains("Max runtime")));
    }

    #[test]
    fn test_validate_catches_negative_max_runtime() {
        let mut form = FormState::new();
        form.name = "test".to_string();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        form.max_runtime = "-5".to_string();
        let valid = form.validate();
        assert!(!valid);
        assert!(form.errors.iter().any(|e| e.contains("positive number")));
    }

    #[test]
    fn test_to_new_app_produces_correct_output() {
        let mut form = FormState::new();
        form.name = "my-api".to_string();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        form.env_vars = "PORT=3000,NODE_ENV=dev".to_string();
        form.auto_start = true;
        form.max_runtime = "300".to_string();

        let new_app = form.to_new_app();
        assert_eq!(new_app.name, "my-api");
        assert_eq!(new_app.working_dir, "/tmp");
        assert_eq!(new_app.command, "echo hi");
        assert!(new_app.auto_start);
        assert_eq!(new_app.max_runtime_secs, Some(300));

        // Verify JSON env vars
        let parsed: serde_json::Value = serde_json::from_str(&new_app.env_vars).unwrap();
        assert_eq!(parsed["PORT"], "3000");
        assert_eq!(parsed["NODE_ENV"], "dev");
    }

    #[test]
    fn test_env_string_to_json_basic() {
        let json = env_string_to_json("PORT=3000,NODE_ENV=development");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["PORT"], "3000");
        assert_eq!(parsed["NODE_ENV"], "development");
    }

    #[test]
    fn test_env_string_to_json_empty() {
        let json = env_string_to_json("");
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_env_string_to_json_whitespace() {
        let json = env_string_to_json("  PORT = 3000 , HOST = localhost  ");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["PORT"], "3000");
        assert_eq!(parsed["HOST"], "localhost");
    }

    #[test]
    fn test_json_to_env_string_roundtrip() {
        let json = r#"{"PORT":"3000","NODE_ENV":"dev"}"#;
        let env_str = json_to_env_string(json);
        // Should contain both pairs
        assert!(env_str.contains("PORT=3000"));
        assert!(env_str.contains("NODE_ENV=dev"));
    }

    #[test]
    fn test_json_to_env_string_empty() {
        assert_eq!(json_to_env_string("{}"), "");
        assert_eq!(json_to_env_string(""), "");
    }

    #[test]
    fn test_handle_form_key_tab_advances_field() {
        let mut form = FormState::new();
        assert_eq!(form.current_field, 0);
        let action = handle_form_key(key(KeyCode::Tab), &mut form);
        assert_eq!(action, FormAction::Continue);
        assert_eq!(form.current_field, 1);
    }

    #[test]
    fn test_handle_form_key_backtab_goes_back() {
        let mut form = FormState::new();
        form.current_field = 2;
        let action = handle_form_key(key(KeyCode::BackTab), &mut form);
        assert_eq!(action, FormAction::Continue);
        assert_eq!(form.current_field, 1);
    }

    #[test]
    fn test_handle_form_key_esc_returns_cancel() {
        let mut form = FormState::new();
        let action = handle_form_key(key(KeyCode::Esc), &mut form);
        assert_eq!(action, FormAction::Cancel);
    }

    #[test]
    fn test_handle_form_key_enter_returns_save_on_valid() {
        let mut form = FormState::new();
        form.name = "test".to_string();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        let action = handle_form_key(key(KeyCode::Enter), &mut form);
        match action {
            FormAction::Save(new_app) => {
                assert_eq!(new_app.name, "test");
            }
            other => panic!("Expected Save, got {:?}", other),
        }
    }

    #[test]
    fn test_handle_form_key_enter_stays_on_invalid() {
        let mut form = FormState::new();
        // name is empty, should fail validation
        form.working_dir = "/tmp".to_string();
        form.command = "echo hi".to_string();
        let action = handle_form_key(key(KeyCode::Enter), &mut form);
        assert_eq!(action, FormAction::Continue);
        assert!(!form.errors.is_empty());
    }

    #[test]
    fn test_handle_form_key_space_toggles_autostart() {
        let mut form = FormState::new();
        form.current_field = 4; // AutoStart
        assert!(!form.auto_start);
        let action = handle_form_key(key(KeyCode::Char(' ')), &mut form);
        assert_eq!(action, FormAction::Continue);
        assert!(form.auto_start);
    }

    #[test]
    fn test_handle_form_key_ctrl_b_on_working_dir() {
        let mut form = FormState::new();
        form.current_field = 1; // WorkingDir
        let action = handle_form_key(
            key_with_mod(KeyCode::Char('b'), KeyModifiers::CONTROL),
            &mut form,
        );
        assert_eq!(action, FormAction::OpenFileBrowser);
    }

    #[test]
    fn test_handle_form_key_ctrl_b_on_other_field() {
        let mut form = FormState::new();
        form.current_field = 0; // Name
        let action = handle_form_key(
            key_with_mod(KeyCode::Char('b'), KeyModifiers::CONTROL),
            &mut form,
        );
        // Should not open file browser, just continue
        assert_eq!(action, FormAction::Continue);
    }

    #[test]
    fn test_handle_form_key_char_inserts() {
        let mut form = FormState::new();
        handle_form_key(key(KeyCode::Char('a')), &mut form);
        handle_form_key(key(KeyCode::Char('b')), &mut form);
        assert_eq!(form.name, "ab");
    }

    #[test]
    fn test_handle_form_key_backspace_deletes() {
        let mut form = FormState::new();
        form.name = "abc".to_string();
        form.cursor_pos = 3;
        handle_form_key(key(KeyCode::Backspace), &mut form);
        assert_eq!(form.name, "ab");
    }

    #[test]
    fn test_insert_char_on_autostart_field_does_nothing() {
        let mut form = FormState::new();
        form.current_field = 4; // AutoStart
        form.insert_char('x');
        // auto_start remains unchanged, no crash
        assert!(!form.auto_start);
    }

    #[test]
    fn test_valid_form_passes_validation() {
        let mut form = FormState::new();
        form.name = "test-app".to_string();
        form.working_dir = "/tmp".to_string();
        form.command = "echo hello".to_string();
        form.max_runtime = "60".to_string();
        assert!(form.validate());
        assert!(form.errors.is_empty());
    }
}
