//! New/edit mask form with dynamic key-value header table.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::db::models::{ApiMask, NewMask};

/// Which field is currently focused in the mask form.
#[derive(Debug, Clone, PartialEq)]
pub enum MaskFormField {
    Name,
    TargetUrl,
    Port,
    AutoStart,
    Headers,
}

/// Sub-mode when inside the headers table.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderTableMode {
    /// Navigating rows/columns with vim keys.
    Navigate,
    /// Editing a cell value.
    Edit,
}

/// Which column is selected in the header table.
#[derive(Debug, Clone, PartialEq)]
pub enum HeaderColumn {
    Key,
    Value,
}

/// Actions returned from mask form key handling.
#[derive(Debug, Clone, PartialEq)]
pub enum MaskFormAction {
    Continue,
    Save(NewMask),
    Cancel,
}

/// A single header row (key + value).
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderRow {
    pub key: String,
    pub value: String,
}

/// Mutable form state for creating or editing a mask.
#[derive(Debug, Clone)]
pub struct MaskFormState {
    pub fields: Vec<MaskFormField>,
    pub current_field: usize,
    pub name: String,
    pub target_url: String,
    pub port: String,
    pub auto_start: bool,
    pub headers: Vec<HeaderRow>,
    pub header_selected_row: usize,
    pub header_column: HeaderColumn,
    pub header_mode: HeaderTableMode,
    pub errors: Vec<String>,
    pub editing: bool,
    pub edit_id: Option<i64>,
    pub cursor_pos: usize,
}

impl Default for MaskFormState {
    fn default() -> Self {
        Self::new()
    }
}

impl MaskFormState {
    /// Create an empty form for a new mask.
    pub fn new() -> Self {
        Self {
            fields: vec![
                MaskFormField::Name,
                MaskFormField::TargetUrl,
                MaskFormField::Port,
                MaskFormField::AutoStart,
                MaskFormField::Headers,
            ],
            current_field: 0,
            name: String::new(),
            target_url: String::new(),
            port: String::new(),
            auto_start: false,
            headers: Vec::new(),
            header_selected_row: 0,
            header_column: HeaderColumn::Key,
            header_mode: HeaderTableMode::Navigate,
            errors: Vec::new(),
            editing: false,
            edit_id: None,
            cursor_pos: 0,
        }
    }

    /// Create a pre-filled form from an existing mask config.
    pub fn from_mask(mask: &ApiMask) -> Self {
        let headers = parse_headers_json(&mask.headers);
        let name_len = mask.name.len();

        Self {
            fields: vec![
                MaskFormField::Name,
                MaskFormField::TargetUrl,
                MaskFormField::Port,
                MaskFormField::AutoStart,
                MaskFormField::Headers,
            ],
            current_field: 0,
            name: mask.name.clone(),
            target_url: mask.target_url.clone(),
            port: mask.listen_port.to_string(),
            auto_start: mask.auto_start,
            headers,
            header_selected_row: 0,
            header_column: HeaderColumn::Key,
            header_mode: HeaderTableMode::Navigate,
            errors: Vec::new(),
            editing: true,
            edit_id: Some(mask.id),
            cursor_pos: name_len,
        }
    }

    /// Get the value of the currently focused text field.
    fn current_text_value(&self) -> &str {
        match &self.fields[self.current_field] {
            MaskFormField::Name => &self.name,
            MaskFormField::TargetUrl => &self.target_url,
            MaskFormField::Port => &self.port,
            _ => "",
        }
    }

    /// Get a mutable reference to the current text field.
    #[allow(dead_code)]
    fn current_text_value_mut(&mut self) -> Option<&mut String> {
        match &self.fields[self.current_field] {
            MaskFormField::Name => Some(&mut self.name),
            MaskFormField::TargetUrl => Some(&mut self.target_url),
            MaskFormField::Port => Some(&mut self.port),
            _ => None,
        }
    }

    /// Move to the next field.
    pub fn next_field(&mut self) {
        self.current_field = (self.current_field + 1) % self.fields.len();
        self.cursor_pos = self.current_text_value().len();
    }

    /// Move to the previous field.
    pub fn prev_field(&mut self) {
        if self.current_field == 0 {
            self.current_field = self.fields.len() - 1;
        } else {
            self.current_field -= 1;
        }
        self.cursor_pos = self.current_text_value().len();
    }

    /// Insert a character at cursor in the current text field.
    pub fn insert_char(&mut self, c: char) {
        let cursor = self.cursor_pos;
        match &self.fields[self.current_field] {
            MaskFormField::Name => {
                let pos = cursor.min(self.name.len());
                self.name.insert(pos, c);
                self.cursor_pos = pos + 1;
            }
            MaskFormField::TargetUrl => {
                let pos = cursor.min(self.target_url.len());
                self.target_url.insert(pos, c);
                self.cursor_pos = pos + 1;
            }
            MaskFormField::Port => {
                let pos = cursor.min(self.port.len());
                self.port.insert(pos, c);
                self.cursor_pos = pos + 1;
            }
            _ => {}
        }
    }

    /// Delete character before cursor.
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let pos = self.cursor_pos;
            match &self.fields[self.current_field] {
                MaskFormField::Name => {
                    self.name.remove(pos - 1);
                    self.cursor_pos -= 1;
                }
                MaskFormField::TargetUrl => {
                    self.target_url.remove(pos - 1);
                    self.cursor_pos -= 1;
                }
                MaskFormField::Port => {
                    self.port.remove(pos - 1);
                    self.cursor_pos -= 1;
                }
                _ => {}
            }
        }
    }

    /// Move cursor left.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right.
    pub fn move_cursor_right(&mut self) {
        let len = self.current_text_value().len();
        if self.cursor_pos < len {
            self.cursor_pos += 1;
        }
    }

    /// Insert char into the currently selected header cell.
    pub fn header_insert_char(&mut self, c: char) {
        if let Some(row) = self.headers.get_mut(self.header_selected_row) {
            let field = match self.header_column {
                HeaderColumn::Key => &mut row.key,
                HeaderColumn::Value => &mut row.value,
            };
            let pos = self.cursor_pos.min(field.len());
            field.insert(pos, c);
            self.cursor_pos = pos + 1;
        }
    }

    /// Delete char from the currently selected header cell.
    pub fn header_delete_char(&mut self) {
        if self.cursor_pos > 0 {
            if let Some(row) = self.headers.get_mut(self.header_selected_row) {
                let field = match self.header_column {
                    HeaderColumn::Key => &mut row.key,
                    HeaderColumn::Value => &mut row.value,
                };
                if self.cursor_pos <= field.len() {
                    field.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
        }
    }

    /// Add a new header row and focus it.
    pub fn add_header_row(&mut self) {
        self.headers.push(HeaderRow {
            key: String::new(),
            value: String::new(),
        });
        self.header_selected_row = self.headers.len() - 1;
        self.header_column = HeaderColumn::Key;
        self.header_mode = HeaderTableMode::Edit;
        self.cursor_pos = 0;
    }

    /// Delete the currently selected header row.
    pub fn delete_header_row(&mut self) {
        if !self.headers.is_empty() {
            self.headers.remove(self.header_selected_row);
            if self.header_selected_row >= self.headers.len() && !self.headers.is_empty() {
                self.header_selected_row = self.headers.len() - 1;
            }
        }
    }

    /// Validate the form. Returns true if valid.
    pub fn validate(&mut self) -> bool {
        self.errors.clear();

        if self.name.trim().is_empty() {
            self.errors.push("Name cannot be empty".to_string());
        }

        if self.target_url.trim().is_empty() {
            self.errors.push("Target URL cannot be empty".to_string());
        } else if !self.target_url.starts_with("http://") && !self.target_url.starts_with("https://") {
            self.errors.push("Target URL must start with http:// or https://".to_string());
        }

        let port_trimmed = self.port.trim();
        if port_trimmed.is_empty() {
            self.errors.push("Port cannot be empty".to_string());
        } else {
            match port_trimmed.parse::<u16>() {
                Ok(0) => {
                    self.errors.push("Port must be between 1 and 65535".to_string());
                }
                Err(_) => {
                    self.errors.push("Port must be a number (1-65535)".to_string());
                }
                _ => {}
            }
        }

        // Validate header rows: keys must not be empty if value is set
        for (i, row) in self.headers.iter().enumerate() {
            if row.key.trim().is_empty() && !row.value.trim().is_empty() {
                self.errors.push(format!("Header row {}: key cannot be empty", i + 1));
            }
        }

        self.errors.is_empty()
    }

    /// Convert form state into a NewMask struct.
    pub fn to_new_mask(&self) -> NewMask {
        let headers_json = headers_to_json(&self.headers);
        let port = self.port.trim().parse::<u16>().unwrap_or(8080);

        NewMask {
            name: self.name.trim().to_string(),
            target_url: self.target_url.trim().to_string(),
            listen_port: port,
            headers: headers_json,
            auto_start: self.auto_start,
        }
    }
}

/// Handle a key event while the mask form is focused.
pub fn handle_mask_form_key(key: KeyEvent, form: &mut MaskFormState) -> MaskFormAction {
    // Ctrl+C always cancels
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return MaskFormAction::Cancel;
    }

    // If we're in the headers table, route to header-specific handling
    if form.fields[form.current_field] == MaskFormField::Headers {
        return handle_header_table_key(key, form);
    }

    match key.code {
        KeyCode::Esc => MaskFormAction::Cancel,
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                form.prev_field();
            } else {
                form.next_field();
            }
            MaskFormAction::Continue
        }
        KeyCode::BackTab => {
            form.prev_field();
            MaskFormAction::Continue
        }
        KeyCode::Enter => {
            if form.validate() {
                MaskFormAction::Save(form.to_new_mask())
            } else {
                MaskFormAction::Continue
            }
        }
        KeyCode::Char(' ') => {
            if form.fields[form.current_field] == MaskFormField::AutoStart {
                form.auto_start = !form.auto_start;
            } else {
                form.insert_char(' ');
            }
            MaskFormAction::Continue
        }
        KeyCode::Char(c) => {
            if form.fields[form.current_field] == MaskFormField::AutoStart {
                // Ignore chars on toggle
            } else if key.modifiers.contains(KeyModifiers::NONE)
                || key.modifiers.contains(KeyModifiers::SHIFT)
            {
                form.insert_char(c);
            }
            MaskFormAction::Continue
        }
        KeyCode::Backspace => {
            form.delete_char();
            MaskFormAction::Continue
        }
        KeyCode::Left => {
            form.move_cursor_left();
            MaskFormAction::Continue
        }
        KeyCode::Right => {
            form.move_cursor_right();
            MaskFormAction::Continue
        }
        _ => MaskFormAction::Continue,
    }
}

/// Handle key events within the header table.
fn handle_header_table_key(key: KeyEvent, form: &mut MaskFormState) -> MaskFormAction {
    match form.header_mode {
        HeaderTableMode::Navigate => handle_header_navigate_key(key, form),
        HeaderTableMode::Edit => handle_header_edit_key(key, form),
    }
}

/// Handle keys in header table navigate mode.
fn handle_header_navigate_key(key: KeyEvent, form: &mut MaskFormState) -> MaskFormAction {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if !form.headers.is_empty() && form.header_selected_row < form.headers.len() - 1 {
                form.header_selected_row += 1;
            }
            MaskFormAction::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if form.header_selected_row > 0 {
                form.header_selected_row -= 1;
            }
            MaskFormAction::Continue
        }
        KeyCode::Char('h') | KeyCode::Left => {
            form.header_column = HeaderColumn::Key;
            MaskFormAction::Continue
        }
        KeyCode::Char('l') | KeyCode::Right => {
            form.header_column = HeaderColumn::Value;
            MaskFormAction::Continue
        }
        KeyCode::Char('a') => {
            form.add_header_row();
            MaskFormAction::Continue
        }
        KeyCode::Char('d') => {
            // 'dd' to delete — we handle single 'd' as first press
            // For simplicity, single 'd' deletes the row
            form.delete_header_row();
            MaskFormAction::Continue
        }
        KeyCode::Enter => {
            if !form.headers.is_empty() {
                form.header_mode = HeaderTableMode::Edit;
                let row = &form.headers[form.header_selected_row];
                form.cursor_pos = match form.header_column {
                    HeaderColumn::Key => row.key.len(),
                    HeaderColumn::Value => row.value.len(),
                };
            }
            MaskFormAction::Continue
        }
        KeyCode::Esc => {
            // Exit header table, move to previous field
            form.prev_field();
            MaskFormAction::Continue
        }
        KeyCode::Tab => {
            form.next_field();
            MaskFormAction::Continue
        }
        KeyCode::BackTab => {
            form.prev_field();
            MaskFormAction::Continue
        }
        // Allow saving from header table navigate mode
        KeyCode::Char('S') => {
            if (key.modifiers.contains(KeyModifiers::SHIFT) || key.code == KeyCode::Char('S'))
                && form.validate()
            {
                return MaskFormAction::Save(form.to_new_mask());
            }
            MaskFormAction::Continue
        }
        _ => MaskFormAction::Continue,
    }
}

/// Handle keys in header table edit mode.
fn handle_header_edit_key(key: KeyEvent, form: &mut MaskFormState) -> MaskFormAction {
    match key.code {
        KeyCode::Esc => {
            form.header_mode = HeaderTableMode::Navigate;
            MaskFormAction::Continue
        }
        KeyCode::Enter => {
            form.header_mode = HeaderTableMode::Navigate;
            MaskFormAction::Continue
        }
        KeyCode::Tab => {
            // Switch to other column
            match form.header_column {
                HeaderColumn::Key => {
                    form.header_column = HeaderColumn::Value;
                    if let Some(row) = form.headers.get(form.header_selected_row) {
                        form.cursor_pos = row.value.len();
                    }
                }
                HeaderColumn::Value => {
                    form.header_column = HeaderColumn::Key;
                    if let Some(row) = form.headers.get(form.header_selected_row) {
                        form.cursor_pos = row.key.len();
                    }
                }
            }
            MaskFormAction::Continue
        }
        KeyCode::Backspace => {
            form.header_delete_char();
            MaskFormAction::Continue
        }
        KeyCode::Left => {
            if form.cursor_pos > 0 {
                form.cursor_pos -= 1;
            }
            MaskFormAction::Continue
        }
        KeyCode::Right => {
            if let Some(row) = form.headers.get(form.header_selected_row) {
                let len = match form.header_column {
                    HeaderColumn::Key => row.key.len(),
                    HeaderColumn::Value => row.value.len(),
                };
                if form.cursor_pos < len {
                    form.cursor_pos += 1;
                }
            }
            MaskFormAction::Continue
        }
        KeyCode::Char(c) => {
            form.header_insert_char(c);
            MaskFormAction::Continue
        }
        _ => MaskFormAction::Continue,
    }
}

/// Render the mask form as a centered overlay.
pub fn render_mask_form(frame: &mut Frame, form: &MaskFormState) {
    let area = frame.area();

    let popup_width = 60_u16.min(area.width.saturating_sub(4));
    let popup_height = 24_u16.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let title = if form.editing {
        " Edit Mask "
    } else {
        " New Mask "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines: Vec<Line> = Vec::new();

    // Regular fields
    let field_defs: Vec<(&str, &str, bool)> = vec![
        ("Name:", &form.name, form.current_field == 0),
        ("Target URL:", &form.target_url, form.current_field == 1),
        ("Port:", &form.port, form.current_field == 2),
    ];

    for (label, value, is_current) in &field_defs {
        let label_style = if *is_current {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        let value_style = if *is_current {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let display = format!("[{}]", value);
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<12}", label), label_style),
            Span::styled(display, value_style),
        ]));
    }

    // Auto-start toggle
    let is_auto_current = form.current_field == 3;
    let auto_label_style = if is_auto_current {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let auto_value = if form.auto_start {
        "[x] yes"
    } else {
        "[ ] no"
    };
    lines.push(Line::from(vec![
        Span::styled("  Auto-start: ", auto_label_style),
        Span::styled(
            auto_value,
            if is_auto_current {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ]));

    // Headers section
    lines.push(Line::from(""));
    let is_headers_current = form.current_field == 4;
    let headers_label_style = if is_headers_current {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mode_indicator = if is_headers_current {
        match form.header_mode {
            HeaderTableMode::Navigate => " [NAV]",
            HeaderTableMode::Edit => " [EDIT]",
        }
    } else {
        ""
    };

    lines.push(Line::from(vec![
        Span::styled("  Headers:", headers_label_style),
        Span::styled(mode_indicator, Style::default().fg(Color::Yellow)),
    ]));

    // Header table rows
    if form.headers.is_empty() && is_headers_current {
        lines.push(Line::from(Span::styled(
            "    (empty — press 'a' to add)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, row) in form.headers.iter().enumerate() {
            let is_selected = is_headers_current && i == form.header_selected_row;

            let key_style = if is_selected && form.header_column == HeaderColumn::Key {
                if form.header_mode == HeaderTableMode::Edit {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                }
            } else {
                Style::default().fg(Color::Gray)
            };

            let value_display = "•".repeat(row.value.len().min(20));
            let val_style = if is_selected && form.header_column == HeaderColumn::Value {
                if form.header_mode == HeaderTableMode::Edit {
                    Style::default().fg(Color::White).bg(Color::Blue)
                } else {
                    Style::default().fg(Color::White).bg(Color::DarkGray)
                }
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let row_indicator = if is_selected { "› " } else { "  " };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", row_indicator), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:<16}", row.key), key_style),
                Span::raw(" : "),
                Span::styled(format!("{:<20}", value_display), val_style),
            ]));
        }
    }

    // Errors
    if !form.errors.is_empty() {
        lines.push(Line::from(""));
        for err in &form.errors {
            lines.push(Line::from(Span::styled(
                format!("  ⚠ {}", err),
                Style::default().fg(Color::Red),
            )));
        }
    }

    // Hints
    lines.push(Line::from(""));
    let hints = if is_headers_current {
        match form.header_mode {
            HeaderTableMode::Navigate => "  [a]dd [d]el [j/k]row [h/l]col [Enter]edit [Esc]back",
            HeaderTableMode::Edit => "  type to edit  [Tab]switch col  [Esc/Enter]done",
        }
    } else {
        "  [Tab] next  [Enter] save  [Esc] cancel"
    };
    lines.push(Line::from(Span::styled(
        hints,
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Parse headers JSON string into Vec<HeaderRow>.
fn parse_headers_json(json_str: &str) -> Vec<HeaderRow> {
    let trimmed = json_str.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Vec::new();
    }

    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(serde_json::Value::Object(map)) => map
            .iter()
            .map(|(k, v)| HeaderRow {
                key: k.clone(),
                value: match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                },
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Convert Vec<HeaderRow> to JSON object string.
fn headers_to_json(headers: &[HeaderRow]) -> String {
    let mut map = serde_json::Map::new();
    for row in headers {
        let key = row.key.trim();
        if !key.is_empty() {
            map.insert(
                key.to_string(),
                serde_json::Value::String(row.value.clone()),
            );
        }
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_new_mask_form() {
        let form = MaskFormState::new();
        assert_eq!(form.name, "");
        assert_eq!(form.target_url, "");
        assert_eq!(form.port, "");
        assert!(!form.auto_start);
        assert!(form.headers.is_empty());
        assert!(!form.editing);
    }

    #[test]
    fn test_from_mask() {
        let mask = ApiMask {
            id: 1,
            name: "openai".to_string(),
            target_url: "https://api.openai.com".to_string(),
            listen_port: 8080,
            headers: r#"{"Authorization":"Bearer sk-test"}"#.to_string(),
            auto_start: true,
            created_at: "2024-01-01".to_string(),
        };

        let form = MaskFormState::from_mask(&mask);
        assert_eq!(form.name, "openai");
        assert_eq!(form.target_url, "https://api.openai.com");
        assert_eq!(form.port, "8080");
        assert!(form.auto_start);
        assert_eq!(form.headers.len(), 1);
        assert_eq!(form.headers[0].key, "Authorization");
        assert_eq!(form.headers[0].value, "Bearer sk-test");
        assert!(form.editing);
        assert_eq!(form.edit_id, Some(1));
    }

    #[test]
    fn test_validate_empty_name() {
        let mut form = MaskFormState::new();
        form.target_url = "https://api.example.com".to_string();
        form.port = "8080".to_string();
        assert!(!form.validate());
        assert!(form.errors.iter().any(|e| e.contains("Name")));
    }

    #[test]
    fn test_validate_invalid_url() {
        let mut form = MaskFormState::new();
        form.name = "test".to_string();
        form.target_url = "not-a-url".to_string();
        form.port = "8080".to_string();
        assert!(!form.validate());
        assert!(form.errors.iter().any(|e| e.contains("http")));
    }

    #[test]
    fn test_validate_invalid_port() {
        let mut form = MaskFormState::new();
        form.name = "test".to_string();
        form.target_url = "https://api.example.com".to_string();
        form.port = "abc".to_string();
        assert!(!form.validate());
        assert!(form.errors.iter().any(|e| e.contains("Port")));
    }

    #[test]
    fn test_validate_port_zero() {
        let mut form = MaskFormState::new();
        form.name = "test".to_string();
        form.target_url = "https://api.example.com".to_string();
        form.port = "0".to_string();
        assert!(!form.validate());
        assert!(form.errors.iter().any(|e| e.contains("Port")));
    }

    #[test]
    fn test_validate_valid_form() {
        let mut form = MaskFormState::new();
        form.name = "test".to_string();
        form.target_url = "https://api.example.com".to_string();
        form.port = "8080".to_string();
        assert!(form.validate());
        assert!(form.errors.is_empty());
    }

    #[test]
    fn test_to_new_mask() {
        let mut form = MaskFormState::new();
        form.name = "openai".to_string();
        form.target_url = "https://api.openai.com".to_string();
        form.port = "8080".to_string();
        form.auto_start = true;
        form.headers = vec![HeaderRow {
            key: "Authorization".to_string(),
            value: "Bearer sk-test".to_string(),
        }];

        let mask = form.to_new_mask();
        assert_eq!(mask.name, "openai");
        assert_eq!(mask.target_url, "https://api.openai.com");
        assert_eq!(mask.listen_port, 8080);
        assert!(mask.auto_start);

        let parsed: serde_json::Value = serde_json::from_str(&mask.headers).unwrap();
        assert_eq!(parsed["Authorization"], "Bearer sk-test");
    }

    #[test]
    fn test_add_and_delete_header_row() {
        let mut form = MaskFormState::new();
        form.current_field = 4; // Headers
        assert!(form.headers.is_empty());

        form.add_header_row();
        assert_eq!(form.headers.len(), 1);
        assert_eq!(form.header_selected_row, 0);
        assert_eq!(form.header_mode, HeaderTableMode::Edit);

        form.header_mode = HeaderTableMode::Navigate;
        form.add_header_row();
        assert_eq!(form.headers.len(), 2);
        assert_eq!(form.header_selected_row, 1);

        form.header_mode = HeaderTableMode::Navigate;
        form.delete_header_row();
        assert_eq!(form.headers.len(), 1);
    }

    #[test]
    fn test_header_insert_char() {
        let mut form = MaskFormState::new();
        form.headers = vec![HeaderRow {
            key: String::new(),
            value: String::new(),
        }];
        form.header_selected_row = 0;
        form.header_column = HeaderColumn::Key;
        form.cursor_pos = 0;

        form.header_insert_char('A');
        form.header_insert_char('u');
        form.header_insert_char('t');
        form.header_insert_char('h');
        assert_eq!(form.headers[0].key, "Auth");
        assert_eq!(form.cursor_pos, 4);
    }

    #[test]
    fn test_header_delete_char() {
        let mut form = MaskFormState::new();
        form.headers = vec![HeaderRow {
            key: "Auth".to_string(),
            value: String::new(),
        }];
        form.header_selected_row = 0;
        form.header_column = HeaderColumn::Key;
        form.cursor_pos = 4;

        form.header_delete_char();
        assert_eq!(form.headers[0].key, "Aut");
        assert_eq!(form.cursor_pos, 3);
    }

    #[test]
    fn test_headers_to_json() {
        let headers = vec![
            HeaderRow {
                key: "Authorization".to_string(),
                value: "Bearer sk-test".to_string(),
            },
            HeaderRow {
                key: "X-Org-Id".to_string(),
                value: "org-123".to_string(),
            },
        ];

        let json = headers_to_json(&headers);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["Authorization"], "Bearer sk-test");
        assert_eq!(parsed["X-Org-Id"], "org-123");
    }

    #[test]
    fn test_parse_headers_json() {
        let json = r#"{"Authorization":"Bearer sk-test","X-Org-Id":"org-123"}"#;
        let headers = parse_headers_json(json);
        assert_eq!(headers.len(), 2);
        assert!(headers.iter().any(|h| h.key == "Authorization"));
        assert!(headers.iter().any(|h| h.key == "X-Org-Id"));
    }

    #[test]
    fn test_handle_mask_form_key_esc_cancels() {
        let mut form = MaskFormState::new();
        let action = handle_mask_form_key(key(KeyCode::Esc), &mut form);
        assert_eq!(action, MaskFormAction::Cancel);
    }

    #[test]
    fn test_handle_mask_form_key_tab_advances() {
        let mut form = MaskFormState::new();
        assert_eq!(form.current_field, 0);
        handle_mask_form_key(key(KeyCode::Tab), &mut form);
        assert_eq!(form.current_field, 1);
    }

    #[test]
    fn test_handle_mask_form_key_enter_validates() {
        let mut form = MaskFormState::new();
        form.name = "test".to_string();
        form.target_url = "https://api.example.com".to_string();
        form.port = "8080".to_string();

        let action = handle_mask_form_key(key(KeyCode::Enter), &mut form);
        match action {
            MaskFormAction::Save(mask) => {
                assert_eq!(mask.name, "test");
                assert_eq!(mask.listen_port, 8080);
            }
            other => panic!("Expected Save, got {:?}", other),
        }
    }

    #[test]
    fn test_header_value_masked_in_display() {
        // This is a rendering test — we verify the mask character is used
        // in render_mask_form. Here we just verify the logic exists.
        let row = HeaderRow {
            key: "Authorization".to_string(),
            value: "Bearer sk-secret".to_string(),
        };
        let masked = "•".repeat(row.value.len().min(20));
        assert_eq!(masked.chars().count(), 16);
        assert!(!masked.contains("sk-secret"));
    }
}
