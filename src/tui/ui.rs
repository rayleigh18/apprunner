//! TUI rendering — sidebar, output pane, and status bar.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::App;
use crate::process::ProcessState;
use crate::tui::file_browser::render_file_browser;
use crate::tui::form;
use crate::tui::input::FocusMode;

/// Render the full application UI.
pub fn render(frame: &mut Frame, app: &App) {
    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(frame.area());

    let main_area = outer_layout[0];
    let status_area = outer_layout[1];

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(main_area);

    let sidebar_area = main_layout[0];
    let output_area = main_layout[1];

    render_sidebar(frame, app, sidebar_area);
    render_output(frame, app, output_area);
    render_status_bar(frame, app, status_area);

    // Render help overlay if in Help mode
    if app.focus == FocusMode::Help {
        render_help_overlay(frame);
    }

    // Render form overlay if form is active
    if let Some(ref form_state) = app.form {
        form::render_form(frame, form_state);
    }

    // Render file browser overlay if active
    if let Some(ref browser) = app.file_browser {
        render_file_browser(frame, browser);
    }
}

/// Render the left sidebar with app list.
fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == FocusMode::AppList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Apps ");

    let items: Vec<ListItem> = app
        .apps
        .iter()
        .enumerate()
        .map(|(i, app_config)| {
            let app_id = app_config.id;

            // Determine status icon
            let (icon, icon_style) = if let Some(proc) = app.processes.get(&app_id) {
                match &proc.state {
                    ProcessState::Running { .. } => ("●", Style::default().fg(Color::Green)),
                    ProcessState::Stopped => ("○", Style::default().fg(Color::DarkGray)),
                    ProcessState::Crashed { .. } => ("✗", Style::default().fg(Color::Red)),
                    ProcessState::Attached => ("⊙", Style::default().fg(Color::Yellow)),
                    ProcessState::Starting => ("◐", Style::default().fg(Color::Yellow)),
                }
            } else {
                ("○", Style::default().fg(Color::DarkGray))
            };

            // Get metrics display
            let (cpu_display, mem_display) = if let Some(proc) = app.processes.get(&app_id) {
                if let ProcessState::Running { pid, .. } = &proc.state {
                    if let Some(metrics) = app.metrics.get_metrics(*pid) {
                        (metrics.cpu_display.clone(), metrics.memory_display.clone())
                    } else {
                        ("—".to_string(), "—".to_string())
                    }
                } else {
                    ("—".to_string(), "—".to_string())
                }
            } else {
                ("—".to_string(), "—".to_string())
            };

            // Check if this app has a runtime alert
            let alert_icon = if app.runtime_alert_ids.contains(&app_id) {
                "\u{23f1} "
            } else {
                ""
            };

            let content = Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(" "),
                Span::styled(alert_icon, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{:<10}", app_config.name),
                    Style::default().fg(Color::White),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:>5}", cpu_display),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{:>4}", mem_display),
                    Style::default().fg(Color::Gray),
                ),
            ]);

            let item = ListItem::new(content);
            if i == app.selected_index {
                item.style(Style::default().bg(Color::DarkGray).fg(Color::White))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray));

    frame.render_widget(list, area);
}

/// Render the output pane showing scrollback for the selected app.
fn render_output(frame: &mut Frame, app: &App, area: Rect) {
    let selected_name = app
        .selected_app()
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "none".to_string());

    let border_style = if app.focus == FocusMode::OutputPane {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Output: {} ", selected_name));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Get available height for content
    let available_height = inner_area.height as usize;

    // Render scrollback content for selected app
    if let Some(app_id) = app.selected_app_id() {
        if let Some(scrollback) = app.scrollbacks.get(&app_id) {
            let total = scrollback.total_lines();
            let lines = scrollback.get_lines(app.scroll_offset, available_height);

            let mut ratatui_lines: Vec<Line> = lines
                .iter()
                .map(|terminal_line| {
                    let spans: Vec<Span> = terminal_line
                        .chars
                        .iter()
                        .map(|sc| Span::styled(sc.ch.to_string(), sc.style))
                        .collect();
                    Line::from(spans)
                })
                .collect();

            // Also show the current (incomplete) line if we're at the bottom
            if app.scroll_offset + available_height >= total {
                let current = scrollback.current_line();
                if !current.chars.is_empty() {
                    let spans: Vec<Span> = current
                        .chars
                        .iter()
                        .map(|sc| Span::styled(sc.ch.to_string(), sc.style))
                        .collect();
                    ratatui_lines.push(Line::from(spans));
                }
            }

            let paragraph = Paragraph::new(ratatui_lines);
            frame.render_widget(paragraph, inner_area);
        }
    }
}

/// Render the status bar at the bottom.
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Top line: keybindings
    let keybinds = match app.focus {
        FocusMode::AppList => {
            " [s]tart [x]top [r]estart [a]ttach [n]ew [e]dit [d]el [?]help [q]uit"
        }
        FocusMode::OutputPane => " [j/k]scroll [G]bottom [g]top [Esc]back [q]uit",
        FocusMode::Help => " [Esc/q/?] close help",
        FocusMode::Form => " [Tab]next [Shift+Tab]prev [Ctrl+B]browse [Esc]cancel [Enter]save",
    };

    let keybind_line =
        Paragraph::new(keybinds).style(Style::default().fg(Color::Cyan).bg(Color::Black));
    frame.render_widget(keybind_line, status_layout[0]);

    // Bottom line: alerts
    let alert_text = if let Some(alert) = app.alerts.last() {
        format!(" ⚠ {}", alert)
    } else {
        String::new()
    };

    let alert_line =
        Paragraph::new(alert_text).style(Style::default().fg(Color::Yellow).bg(Color::Black));
    frame.render_widget(alert_line, status_layout[1]);
}

/// Render a centered help overlay popup.
fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Create a centered rect for the help popup
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 18.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(Span::styled(
            " Keybindings",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" App List:"),
        Line::from("   j/↓      Navigate down"),
        Line::from("   k/↑      Navigate up"),
        Line::from("   s        Start app"),
        Line::from("   x        Stop app"),
        Line::from("   r        Restart app"),
        Line::from("   a        Attach (Ghostty)"),
        Line::from("   n        New app"),
        Line::from("   e        Edit app"),
        Line::from("   d        Delete app"),
        Line::from("   Enter    Focus output"),
        Line::from("   q        Quit"),
        Line::from(""),
        Line::from(" Output Pane:"),
        Line::from("   j/k      Scroll down/up"),
        Line::from("   G/g      Bottom/Top"),
        Line::from("   Esc      Back to app list"),
    ];

    let help_paragraph = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    );

    frame.render_widget(help_paragraph, popup_area);
}
