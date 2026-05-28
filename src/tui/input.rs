//! Vim-like keybinding handling and input dispatch.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Which panel/mode currently has focus.
#[derive(Debug, Clone, PartialEq)]
pub enum FocusMode {
    AppList,
    OutputPane,
    Help,
    Form,
}

/// Actions that can be dispatched from key events.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    NavigateUp,
    NavigateDown,
    StartApp,
    StopApp,
    RestartApp,
    AttachApp,
    NewApp,
    EditApp,
    DeleteApp,
    FocusOutput,
    FocusAppList,
    ScrollUp,
    ScrollDown,
    ScrollToTop,
    ScrollToBottom,
    ShowHelp,
    HideHelp,
    Confirm,
    Cancel,
    Tick,
    None,
}

/// Map a key event to an action based on the current focus mode.
pub fn handle_key_event(key: KeyEvent, mode: &FocusMode) -> Action {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    match mode {
        FocusMode::AppList => handle_app_list_key(key),
        FocusMode::OutputPane => handle_output_pane_key(key),
        FocusMode::Help => handle_help_key(key),
        FocusMode::Form => Action::None, // Form keys handled separately via handle_form_key
    }
}

fn handle_app_list_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::NavigateDown,
        KeyCode::Char('k') | KeyCode::Up => Action::NavigateUp,
        KeyCode::Char('s') => Action::StartApp,
        KeyCode::Char('x') => Action::StopApp,
        KeyCode::Char('r') => Action::RestartApp,
        KeyCode::Char('a') => Action::AttachApp,
        KeyCode::Char('n') => Action::NewApp,
        KeyCode::Char('e') => Action::EditApp,
        KeyCode::Char('d') => Action::DeleteApp,
        KeyCode::Enter => Action::FocusOutput,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('?') => Action::ShowHelp,
        _ => Action::None,
    }
}

fn handle_output_pane_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('G') => Action::ScrollToBottom,
        KeyCode::Char('g') => Action::ScrollToTop,
        KeyCode::Esc => Action::FocusAppList,
        KeyCode::Char('q') => Action::Quit,
        _ => Action::None,
    }
}

fn handle_help_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Action::HideHelp,
        _ => Action::None,
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
    fn test_ctrl_c_always_quits() {
        let ev = key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(handle_key_event(ev, &FocusMode::AppList), Action::Quit);
        assert_eq!(handle_key_event(ev, &FocusMode::OutputPane), Action::Quit);
        assert_eq!(handle_key_event(ev, &FocusMode::Help), Action::Quit);
    }

    #[test]
    fn test_app_list_navigation() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('j')), &FocusMode::AppList),
            Action::NavigateDown
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('k')), &FocusMode::AppList),
            Action::NavigateUp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Down), &FocusMode::AppList),
            Action::NavigateDown
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Up), &FocusMode::AppList),
            Action::NavigateUp
        );
    }

    #[test]
    fn test_app_list_actions() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('s')), &FocusMode::AppList),
            Action::StartApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('x')), &FocusMode::AppList),
            Action::StopApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('r')), &FocusMode::AppList),
            Action::RestartApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('a')), &FocusMode::AppList),
            Action::AttachApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('n')), &FocusMode::AppList),
            Action::NewApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('e')), &FocusMode::AppList),
            Action::EditApp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('d')), &FocusMode::AppList),
            Action::DeleteApp
        );
    }

    #[test]
    fn test_app_list_focus_and_quit() {
        assert_eq!(
            handle_key_event(key(KeyCode::Enter), &FocusMode::AppList),
            Action::FocusOutput
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('q')), &FocusMode::AppList),
            Action::Quit
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('?')), &FocusMode::AppList),
            Action::ShowHelp
        );
    }

    #[test]
    fn test_output_pane_scrolling() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('j')), &FocusMode::OutputPane),
            Action::ScrollDown
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('k')), &FocusMode::OutputPane),
            Action::ScrollUp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('G')), &FocusMode::OutputPane),
            Action::ScrollToBottom
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('g')), &FocusMode::OutputPane),
            Action::ScrollToTop
        );
    }

    #[test]
    fn test_output_pane_escape_and_quit() {
        assert_eq!(
            handle_key_event(key(KeyCode::Esc), &FocusMode::OutputPane),
            Action::FocusAppList
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('q')), &FocusMode::OutputPane),
            Action::Quit
        );
    }

    #[test]
    fn test_help_mode_dismiss() {
        assert_eq!(
            handle_key_event(key(KeyCode::Esc), &FocusMode::Help),
            Action::HideHelp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('q')), &FocusMode::Help),
            Action::HideHelp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('?')), &FocusMode::Help),
            Action::HideHelp
        );
    }

    #[test]
    fn test_unknown_keys_return_none() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('z')), &FocusMode::AppList),
            Action::None
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('m')), &FocusMode::OutputPane),
            Action::None
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('a')), &FocusMode::Help),
            Action::None
        );
    }
}
