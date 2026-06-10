//! Vim-like keybinding handling and input dispatch.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Which panel/mode currently has focus.
#[derive(Debug, Clone, PartialEq)]
pub enum FocusMode {
    AppList,
    OutputPane,
    Help,
    Form,
    MaskList,
    MaskLog,
    MaskForm,
}

/// Which tab is currently active.
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTab {
    Apps,
    Masks,
}

/// Actions that can be dispatched from key events.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    NavigateUp,
    NavigateDown,
    StartApp,
    StartAppWithOptions,
    StopApp,
    RestartApp,
    RestartAppWithOptions,
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
    // Tab switching
    SwitchToApps,
    SwitchToMasks,
    // Mask actions
    ActivateMask,
    DeactivateMask,
    NewMask,
    EditMask,
    DeleteMask,
    FocusMaskLog,
    FocusMaskList,
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
        FocusMode::MaskList => handle_mask_list_key(key),
        FocusMode::MaskLog => handle_mask_log_key(key),
        FocusMode::MaskForm => Action::None, // Mask form keys handled separately
    }
}

fn handle_app_list_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::NavigateDown,
        KeyCode::Char('k') | KeyCode::Up => Action::NavigateUp,
        KeyCode::Char('s') => Action::StartApp,
        KeyCode::Char('S') => Action::StartAppWithOptions,
        KeyCode::Char('x') => Action::StopApp,
        KeyCode::Char('r') => Action::RestartApp,
        KeyCode::Char('R') => Action::RestartAppWithOptions,
        KeyCode::Char('a') => Action::AttachApp,
        KeyCode::Char('n') => Action::NewApp,
        KeyCode::Char('e') => Action::EditApp,
        KeyCode::Char('d') => Action::DeleteApp,
        KeyCode::Enter => Action::FocusOutput,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('?') => Action::ShowHelp,
        KeyCode::Char('1') => Action::SwitchToApps,
        KeyCode::Char('2') => Action::SwitchToMasks,
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

fn handle_mask_list_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::NavigateDown,
        KeyCode::Char('k') | KeyCode::Up => Action::NavigateUp,
        KeyCode::Char('s') => Action::ActivateMask,
        KeyCode::Char('x') => Action::DeactivateMask,
        KeyCode::Char('n') => Action::NewMask,
        KeyCode::Char('e') => Action::EditMask,
        KeyCode::Char('d') => Action::DeleteMask,
        KeyCode::Enter => Action::FocusMaskLog,
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Char('?') => Action::ShowHelp,
        KeyCode::Char('1') => Action::SwitchToApps,
        KeyCode::Char('2') => Action::SwitchToMasks,
        _ => Action::None,
    }
}

fn handle_mask_log_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char('G') => Action::ScrollToBottom,
        KeyCode::Char('g') => Action::ScrollToTop,
        KeyCode::Esc => Action::FocusMaskList,
        KeyCode::Char('q') => Action::Quit,
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
        assert_eq!(handle_key_event(ev, &FocusMode::MaskList), Action::Quit);
        assert_eq!(handle_key_event(ev, &FocusMode::MaskLog), Action::Quit);
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
    fn test_app_list_tab_switching() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('1')), &FocusMode::AppList),
            Action::SwitchToApps
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('2')), &FocusMode::AppList),
            Action::SwitchToMasks
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
    fn test_mask_list_navigation() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('j')), &FocusMode::MaskList),
            Action::NavigateDown
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('k')), &FocusMode::MaskList),
            Action::NavigateUp
        );
    }

    #[test]
    fn test_mask_list_actions() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('s')), &FocusMode::MaskList),
            Action::ActivateMask
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('x')), &FocusMode::MaskList),
            Action::DeactivateMask
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('n')), &FocusMode::MaskList),
            Action::NewMask
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('e')), &FocusMode::MaskList),
            Action::EditMask
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('d')), &FocusMode::MaskList),
            Action::DeleteMask
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Enter), &FocusMode::MaskList),
            Action::FocusMaskLog
        );
    }

    #[test]
    fn test_mask_list_tab_switching() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('1')), &FocusMode::MaskList),
            Action::SwitchToApps
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('2')), &FocusMode::MaskList),
            Action::SwitchToMasks
        );
    }

    #[test]
    fn test_mask_log_scrolling() {
        assert_eq!(
            handle_key_event(key(KeyCode::Char('j')), &FocusMode::MaskLog),
            Action::ScrollDown
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('k')), &FocusMode::MaskLog),
            Action::ScrollUp
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('G')), &FocusMode::MaskLog),
            Action::ScrollToBottom
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('g')), &FocusMode::MaskLog),
            Action::ScrollToTop
        );
    }

    #[test]
    fn test_mask_log_escape() {
        assert_eq!(
            handle_key_event(key(KeyCode::Esc), &FocusMode::MaskLog),
            Action::FocusMaskList
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
        assert_eq!(
            handle_key_event(key(KeyCode::Char('z')), &FocusMode::MaskList),
            Action::None
        );
        assert_eq!(
            handle_key_event(key(KeyCode::Char('m')), &FocusMode::MaskLog),
            Action::None
        );
    }
}
