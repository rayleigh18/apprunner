//! Directory picker overlay widget.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::*;

/// State for the file browser overlay.
#[derive(Debug, Clone)]
pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<PathBuf>,
    pub selected_index: usize,
    pub visible: bool,
}

/// Result of handling a key event in the file browser.
#[derive(Debug, Clone, PartialEq)]
pub enum BrowserAction {
    /// Stay in the browser (no-op).
    Continue,
    /// A directory was selected; contains the chosen path.
    Select(PathBuf),
    /// User cancelled the browser.
    Cancel,
}

impl FileBrowser {
    /// Create a new file browser starting at the given path.
    /// If path is empty or invalid, start at home directory.
    pub fn new(start_path: &str) -> Self {
        let path = if start_path.is_empty() {
            Self::home_dir()
        } else {
            let p = PathBuf::from(start_path);
            if p.is_dir() {
                p
            } else {
                Self::home_dir()
            }
        };

        let mut browser = Self {
            current_dir: path,
            entries: Vec::new(),
            selected_index: 0,
            visible: true,
        };
        browser.refresh_entries();
        browser
    }

    /// Get the home directory, falling back to root.
    fn home_dir() -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    }

    /// Refresh the entries list by reading current_dir.
    /// Only directories are included. Sorted: regular dirs first (alpha), hidden dirs last (alpha).
    pub fn refresh_entries(&mut self) {
        let mut regular: Vec<PathBuf> = Vec::new();
        let mut hidden: Vec<PathBuf> = Vec::new();

        match std::fs::read_dir(&self.current_dir) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            hidden.push(path);
                        } else {
                            regular.push(path);
                        }
                    }
                }
            }
            Err(_) => {
                // Permission denied or other error — keep entries empty
            }
        }

        regular.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
        });
        hidden.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .cmp(&b.file_name().unwrap_or_default().to_ascii_lowercase())
        });

        self.entries = regular;
        self.entries.extend(hidden);

        // Clamp selected index
        if self.entries.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    /// Navigate selection down (wraps).
    pub fn move_down(&mut self) {
        if !self.entries.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.entries.len();
        }
    }

    /// Navigate selection up (wraps).
    pub fn move_up(&mut self) {
        if !self.entries.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.entries.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Enter the selected directory.
    pub fn enter_selected(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index).cloned() {
            self.current_dir = entry;
            self.selected_index = 0;
            self.refresh_entries();
        }
    }

    /// Go up to parent directory. At root `/`, stays at root.
    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let parent = parent.to_path_buf();
            // Only navigate if parent differs from current (i.e., not at root)
            if parent != self.current_dir {
                self.current_dir = parent;
                self.selected_index = 0;
                self.refresh_entries();
            }
        }
    }

    /// Select current directory (return its path).
    pub fn select_current(&self) -> PathBuf {
        self.current_dir.clone()
    }

    /// Get the currently highlighted entry.
    pub fn selected_entry(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected_index)
    }
}

/// Handle a key event within the file browser, returning the resulting action.
pub fn handle_browser_key(key: KeyEvent, browser: &mut FileBrowser) -> BrowserAction {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            browser.move_down();
            BrowserAction::Continue
        }
        KeyCode::Char('k') | KeyCode::Up => {
            browser.move_up();
            BrowserAction::Continue
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            browser.enter_selected();
            BrowserAction::Continue
        }
        KeyCode::Char('h') | KeyCode::Backspace => {
            browser.go_up();
            BrowserAction::Continue
        }
        KeyCode::Char('.') => BrowserAction::Select(browser.select_current()),
        KeyCode::Esc | KeyCode::Char('q') => BrowserAction::Cancel,
        _ => BrowserAction::Continue,
    }
}

/// Render the file browser as a centered overlay (60% width, 70% height).
pub fn render_file_browser(frame: &mut Frame, browser: &FileBrowser) {
    let area = frame.area();

    // Calculate centered overlay dimensions
    let popup_width = ((area.width as u32 * 60) / 100) as u16;
    let popup_height = ((area.height as u32 * 70) / 100) as u16;
    let popup_width = popup_width.max(30).min(area.width.saturating_sub(2));
    let popup_height = popup_height.max(10).min(area.height.saturating_sub(2));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Select Directory ");

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Layout: breadcrumb (1 line), separator (1 line), entries (remaining - 1 for hints)
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // breadcrumb
            Constraint::Length(1), // separator
            Constraint::Min(1),    // entries
            Constraint::Length(1), // hint line
        ])
        .split(inner);

    // Breadcrumb: current path
    let display_path = abbreviate_home(&browser.current_dir);
    let breadcrumb = Paragraph::new(Line::from(Span::styled(
        format!("  {}", display_path),
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(breadcrumb, layout[0]);

    // Separator
    let sep = Paragraph::new(Line::from(Span::styled(
        "  ".to_string() + &"─".repeat((inner.width.saturating_sub(4)) as usize),
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(sep, layout[1]);

    // Entries
    let entries_area = layout[2];
    if browser.entries.is_empty() {
        let empty_msg = Paragraph::new(Line::from(Span::styled(
            "    (empty)",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
        frame.render_widget(empty_msg, entries_area);
    } else {
        let visible_height = entries_area.height as usize;
        // Calculate scroll offset to keep selected item visible
        let scroll_offset = if browser.selected_index >= visible_height {
            browser.selected_index - visible_height + 1
        } else {
            0
        };

        let items: Vec<ListItem> = browser
            .entries
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(i, path)| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let is_hidden = name.starts_with('.');
                let is_selected = i == browser.selected_index;

                let prefix = if is_selected { "  > " } else { "    " };
                let display = format!("{}{}/", prefix, name);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_hidden {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(display, style)))
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, entries_area);
    }

    // Hint line
    let hints = Paragraph::new(Line::from(vec![
        Span::styled("  [.]", Style::default().fg(Color::Cyan)),
        Span::styled(" select  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[h]", Style::default().fg(Color::Cyan)),
        Span::styled(" up  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Esc]", Style::default().fg(Color::Cyan)),
        Span::styled(" cancel", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(hints, layout[3]);
}

/// Abbreviate the home directory prefix with ~.
fn abbreviate_home(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use std::fs;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_new_with_valid_path() {
        let tmp = std::env::temp_dir();
        let browser = FileBrowser::new(tmp.to_str().unwrap());
        assert_eq!(browser.current_dir, tmp);
        assert!(browser.visible);
    }

    #[test]
    fn test_new_with_empty_string_starts_at_home() {
        let browser = FileBrowser::new("");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        assert_eq!(browser.current_dir, home);
    }

    #[test]
    fn test_new_with_invalid_path_falls_back_to_home() {
        let browser = FileBrowser::new("/this/path/definitely/does/not/exist/xyz123");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        assert_eq!(browser.current_dir, home);
    }

    #[test]
    fn test_refresh_entries_only_shows_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();

        // Create directories and files
        fs::create_dir(tmp_path.join("dir_a")).unwrap();
        fs::create_dir(tmp_path.join("dir_b")).unwrap();
        fs::write(tmp_path.join("file.txt"), "hello").unwrap();

        let browser = FileBrowser::new(tmp_path.to_str().unwrap());
        // Should only contain directories
        assert_eq!(browser.entries.len(), 2);
        assert!(browser.entries.iter().all(|p| p.is_dir()));
    }

    #[test]
    fn test_move_down_wraps() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        fs::create_dir(tmp_path.join("a")).unwrap();
        fs::create_dir(tmp_path.join("b")).unwrap();

        let mut browser = FileBrowser::new(tmp_path.to_str().unwrap());
        assert_eq!(browser.entries.len(), 2);

        browser.selected_index = 1;
        browser.move_down();
        assert_eq!(browser.selected_index, 0); // Wrapped
    }

    #[test]
    fn test_move_up_wraps() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        fs::create_dir(tmp_path.join("a")).unwrap();
        fs::create_dir(tmp_path.join("b")).unwrap();

        let mut browser = FileBrowser::new(tmp_path.to_str().unwrap());
        assert_eq!(browser.entries.len(), 2);

        browser.selected_index = 0;
        browser.move_up();
        assert_eq!(browser.selected_index, 1); // Wrapped to end
    }

    #[test]
    fn test_enter_selected_changes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        fs::create_dir(tmp_path.join("subdir")).unwrap();

        let mut browser = FileBrowser::new(tmp_path.to_str().unwrap());
        assert!(!browser.entries.is_empty());

        // Find the subdir entry
        let subdir_idx = browser
            .entries
            .iter()
            .position(|p| p.file_name().unwrap().to_str().unwrap() == "subdir")
            .unwrap();
        browser.selected_index = subdir_idx;
        browser.enter_selected();

        assert_eq!(browser.current_dir, tmp_path.join("subdir"));
    }

    #[test]
    fn test_go_up_from_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        let sub = tmp_path.join("subdir");
        fs::create_dir(&sub).unwrap();

        let mut browser = FileBrowser::new(sub.to_str().unwrap());
        assert_eq!(browser.current_dir, sub);

        browser.go_up();
        assert_eq!(browser.current_dir, tmp_path.to_path_buf());
    }

    #[test]
    fn test_go_up_from_root_stays_at_root() {
        let mut browser = FileBrowser::new("/");
        assert_eq!(browser.current_dir, PathBuf::from("/"));

        browser.go_up();
        assert_eq!(browser.current_dir, PathBuf::from("/"));
    }

    #[test]
    fn test_select_current_returns_current_dir() {
        let tmp = std::env::temp_dir();
        let browser = FileBrowser::new(tmp.to_str().unwrap());
        assert_eq!(browser.select_current(), tmp);
    }

    #[test]
    fn test_hidden_directories_sorted_last() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();

        fs::create_dir(tmp_path.join("visible")).unwrap();
        fs::create_dir(tmp_path.join(".hidden")).unwrap();
        fs::create_dir(tmp_path.join("another")).unwrap();
        fs::create_dir(tmp_path.join(".secret")).unwrap();

        let browser = FileBrowser::new(tmp_path.to_str().unwrap());

        // Regular dirs first (alphabetical), then hidden (alphabetical)
        let names: Vec<String> = browser
            .entries
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["another", "visible", ".hidden", ".secret"]);
    }

    #[test]
    fn test_handle_browser_key_j_moves_down() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        fs::create_dir(tmp_path.join("a")).unwrap();
        fs::create_dir(tmp_path.join("b")).unwrap();

        let mut browser = FileBrowser::new(tmp_path.to_str().unwrap());
        assert_eq!(browser.selected_index, 0);

        let action = handle_browser_key(key(KeyCode::Char('j')), &mut browser);
        assert_eq!(action, BrowserAction::Continue);
        assert_eq!(browser.selected_index, 1);
    }

    #[test]
    fn test_handle_browser_key_esc_cancels() {
        let mut browser = FileBrowser::new("/tmp");
        let action = handle_browser_key(key(KeyCode::Esc), &mut browser);
        assert_eq!(action, BrowserAction::Cancel);
    }

    #[test]
    fn test_handle_browser_key_dot_selects() {
        let tmp = std::env::temp_dir();
        let mut browser = FileBrowser::new(tmp.to_str().unwrap());
        let action = handle_browser_key(key(KeyCode::Char('.')), &mut browser);
        assert_eq!(action, BrowserAction::Select(tmp));
    }
}
