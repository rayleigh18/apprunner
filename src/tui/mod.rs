//! TUI module — terminal setup, rendering, and input handling.

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, Stdout};

pub mod file_browser;
pub mod form;
pub mod input;
pub mod ui;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal: enable raw mode, enter alternate screen, create backend.
pub fn init() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal: leave alternate screen, disable raw mode.
pub fn restore() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
