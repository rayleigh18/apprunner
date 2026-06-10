//! TUI rendering — sidebar, output pane, and status bar.

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, MaskStatus, TemplateModal};
use crate::process::ProcessState;
use crate::tui::file_browser::render_file_browser;
use crate::tui::form;
use crate::tui::input::{ActiveTab, FocusMode};
use crate::tui::mask_form;

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

    match app.active_tab {
        ActiveTab::Apps => {
            render_sidebar(frame, app, sidebar_area);
            render_output(frame, app, output_area);
        }
        ActiveTab::Masks => {
            render_mask_sidebar(frame, app, sidebar_area);
            render_mask_log(frame, app, output_area);
        }
    }

    render_status_bar(frame, app, status_area);

    // Render help overlay if in Help mode
    if app.focus == FocusMode::Help {
        render_help_overlay(frame, app);
    }

    // Render form overlay if form is active
    if let Some(ref form_state) = app.form {
        form::render_form(frame, form_state);
    }

    // Render mask form overlay if active
    if let Some(ref mask_form_state) = app.mask_form {
        mask_form::render_mask_form(frame, mask_form_state);
    }

    // Render file browser overlay if active
    if let Some(ref browser) = app.file_browser {
        render_file_browser(frame, browser);
    }

    // Render template variable override modal if active
    if let Some(ref modal) = app.template_modal {
        render_template_modal(frame, modal);
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
            let (icon, icon_style) = if app_config.is_cron() {
                // Cron job status
                let is_active = app.cron_active.get(&app_id).copied().unwrap_or(false);
                if let Some(proc) = app.processes.get(&app_id) {
                    if matches!(proc.state, ProcessState::Running { .. }) {
                        ("⟳", Style::default().fg(Color::Green))
                    } else if is_active {
                        ("◷", Style::default().fg(Color::Cyan))
                    } else {
                        ("○", Style::default().fg(Color::DarkGray))
                    }
                } else if is_active {
                    ("◷", Style::default().fg(Color::Cyan))
                } else {
                    ("○", Style::default().fg(Color::DarkGray))
                }
            } else if let Some(proc) = app.processes.get(&app_id) {
                match &proc.state {
                    ProcessState::Running { .. } => ("●", Style::default().fg(Color::Green)),
                    ProcessState::Stopped => ("○", Style::default().fg(Color::DarkGray)),
                    ProcessState::Crashed { .. } => ("✗", Style::default().fg(Color::Red)),
                    ProcessState::Attached => ("⊙", Style::default().fg(Color::Yellow)),
                    ProcessState::Starting => ("◐", Style::default().fg(Color::Yellow)),
                }
            } else if app_config.has_required_template_vars()
                && !app.processes.contains_key(&app_id)
            {
                // Pending Input: has required vars, never started
                ("?", Style::default().fg(Color::Yellow))
            } else {
                ("○", Style::default().fg(Color::DarkGray))
            };

            // Get metrics display or cron countdown
            let (cpu_display, mem_display) = if app_config.is_cron() {
                let is_active = app.cron_active.get(&app_id).copied().unwrap_or(false);
                if let Some(proc) = app.processes.get(&app_id) {
                    if matches!(proc.state, ProcessState::Running { .. }) {
                        ("run".to_string(), "".to_string())
                    } else if is_active {
                        if let Some(next_fire) = app.cron_next_fire.get(&app_id) {
                            let now = std::time::Instant::now();
                            if *next_fire > now {
                                let remaining = next_fire.duration_since(now).as_secs();
                                let mins = remaining / 60;
                                let secs = remaining % 60;
                                if mins > 0 {
                                    (format!("{}m{}s", mins, secs), "".to_string())
                                } else {
                                    (format!("{}s", secs), "".to_string())
                                }
                            } else {
                                ("due".to_string(), "".to_string())
                            }
                        } else {
                            ("—".to_string(), "".to_string())
                        }
                    } else {
                        ("off".to_string(), "".to_string())
                    }
                } else if is_active {
                    if let Some(next_fire) = app.cron_next_fire.get(&app_id) {
                        let now = std::time::Instant::now();
                        if *next_fire > now {
                            let remaining = next_fire.duration_since(now).as_secs();
                            let mins = remaining / 60;
                            let secs = remaining % 60;
                            if mins > 0 {
                                (format!("{}m{}s", mins, secs), "".to_string())
                            } else {
                                (format!("{}s", secs), "".to_string())
                            }
                        } else {
                            ("due".to_string(), "".to_string())
                        }
                    } else {
                        ("—".to_string(), "".to_string())
                    }
                } else {
                    ("off".to_string(), "".to_string())
                }
            } else if let Some(proc) = app.processes.get(&app_id) {
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

            // Cron tag
            let cron_tag = if app_config.is_cron() { "[cron] " } else { "" };

            let content = Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(" "),
                Span::styled(alert_icon, Style::default().fg(Color::Yellow)),
                Span::styled(cron_tag, Style::default().fg(Color::Magenta)),
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

    // Tab indicator + keybindings
    let tab_indicator = match app.active_tab {
        ActiveTab::Apps => "[1:Apps] 2:Masks",
        ActiveTab::Masks => " 1:Apps [2:Masks]",
    };

    let keybinds = match app.focus {
        FocusMode::AppList => {
            " [s]tart [x]top [r]estart [a]ttach [n]ew [e]dit [d]el [?]help [q]uit"
        }
        FocusMode::OutputPane => " [j/k]scroll [G]bottom [g]top [Esc]back [q]uit",
        FocusMode::Help => " [Esc/q/?] close help",
        FocusMode::Form | FocusMode::MaskForm => {
            " [Tab]next [Shift+Tab]prev [Ctrl+B]browse [Esc]cancel [Enter]save"
        }
        FocusMode::MaskList => {
            " [s]activate [x]deactivate [n]ew [e]dit [d]el [?]help [q]uit"
        }
        FocusMode::MaskLog => " [j/k]scroll [G]bottom [g]top [Esc]back [q]uit",
    };

    let keybind_line = Paragraph::new(format!("{} |{}", tab_indicator, keybinds))
        .style(Style::default().fg(Color::Cyan).bg(Color::Black));
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
fn render_help_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create a centered rect for the help popup
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let help_text = match app.active_tab {
        ActiveTab::Apps => vec![
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
            Line::from("   S        Start with options"),
            Line::from("   x        Stop app"),
            Line::from("   r        Restart app"),
            Line::from("   R        Restart with options"),
            Line::from("   a        Attach (Ghostty)"),
            Line::from("   n        New app"),
            Line::from("   e        Edit app"),
            Line::from("   d        Delete app"),
            Line::from("   Enter    Focus output"),
            Line::from("   1/2      Switch tab"),
            Line::from("   q        Quit"),
            Line::from(""),
            Line::from(" Output Pane:"),
            Line::from("   j/k      Scroll down/up"),
            Line::from("   G/g      Bottom/Top"),
            Line::from("   Esc      Back to app list"),
        ],
        ActiveTab::Masks => vec![
            Line::from(Span::styled(
                " Keybindings",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(" Mask List:"),
            Line::from("   j/↓      Navigate down"),
            Line::from("   k/↑      Navigate up"),
            Line::from("   s        Activate mask"),
            Line::from("   x        Deactivate mask"),
            Line::from("   n        New mask"),
            Line::from("   e        Edit mask"),
            Line::from("   d        Delete mask"),
            Line::from("   Enter    View request log"),
            Line::from("   1/2      Switch tab"),
            Line::from("   q        Quit"),
            Line::from(""),
            Line::from(" Request Log:"),
            Line::from("   j/k      Scroll down/up"),
            Line::from("   G/g      Bottom/Top"),
            Line::from("   Esc      Back to mask list"),
        ],
    };

    let help_paragraph = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help "),
    );

    frame.render_widget(help_paragraph, popup_area);
}

/// Render the mask list sidebar.
fn render_mask_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == FocusMode::MaskList {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Masks ");

    let items: Vec<ListItem> = app
        .masks
        .iter()
        .enumerate()
        .map(|(i, mask)| {
            let status = app
                .mask_statuses
                .get(&mask.id)
                .unwrap_or(&MaskStatus::Inactive);

            let (icon, icon_style) = match status {
                MaskStatus::Active => ("●", Style::default().fg(Color::Green)),
                MaskStatus::Inactive => ("○", Style::default().fg(Color::DarkGray)),
                MaskStatus::Error(_) => ("✗", Style::default().fg(Color::Red)),
            };

            let status_text = match status {
                MaskStatus::Active => "Active".to_string(),
                MaskStatus::Inactive => "Inactive".to_string(),
                MaskStatus::Error(e) => format!("Err: {}", truncate_str(e, 12)),
            };

            let content = Line::from(vec![
                Span::styled(icon, icon_style),
                Span::raw(" "),
                Span::styled(
                    format!("{:<10}", mask.name),
                    Style::default().fg(Color::White),
                ),
                Span::raw(" "),
                Span::styled(
                    format!(":{}", mask.listen_port),
                    Style::default().fg(Color::Gray),
                ),
                Span::raw(" "),
                Span::styled(status_text, Style::default().fg(Color::Gray)),
            ]);

            let item = ListItem::new(content);
            if i == app.mask_selected_index {
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

/// Render the mask request log pane.
fn render_mask_log(frame: &mut Frame, app: &App, area: Rect) {
    let selected_name = app
        .selected_mask()
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "none".to_string());

    let border_style = if app.focus == FocusMode::MaskLog {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Log: {} ", selected_name));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    let available_height = inner_area.height as usize;

    if let Some(mask_id) = app.selected_mask_id() {
        if let Some(log_arc) = app.mask_logs.get(&mask_id) {
            let log = log_arc.blocking_lock();
            let entries = log.entries();
            let total = entries.len();

            let start = app.mask_scroll_offset.min(total.saturating_sub(available_height));
            let end = (start + available_height).min(total);

            let lines: Vec<Line> = entries
                .iter()
                .skip(start)
                .take(end - start)
                .map(|entry| {
                    let status_str = match entry.status_code {
                        Some(code) => {
                            let color = if code < 300 {
                                Color::Green
                            } else if code < 400 {
                                Color::Yellow
                            } else {
                                Color::Red
                            };
                            Span::styled(format!("{}", code), Style::default().fg(color))
                        }
                        None => Span::styled("ERR", Style::default().fg(Color::Red)),
                    };

                    let error_suffix = if let Some(err) = &entry.error {
                        format!(" {}", truncate_str(err, 30))
                    } else {
                        String::new()
                    };

                    Line::from(vec![
                        Span::styled(
                            &entry.timestamp,
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("{:<6}", entry.method),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::styled(
                            truncate_str(&entry.path, 30).to_string(),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw("  "),
                        status_str,
                        Span::raw(format!("  {}ms", entry.latency_ms)),
                        Span::styled(error_suffix, Style::default().fg(Color::Red)),
                    ])
                })
                .collect();

            let paragraph = Paragraph::new(lines);
            frame.render_widget(paragraph, inner_area);
        }
    }
}

/// Truncate a string to max_len, appending "…" if truncated.
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

/// Render the template variable override modal.
fn render_template_modal(frame: &mut Frame, modal: &TemplateModal) {
    let area = frame.area();

    // Calculate popup size based on number of variables
    let var_count = modal.vars.len() as u16;
    let popup_height = (4 + var_count * 3).min(area.height.saturating_sub(4));
    let popup_width = 50_u16.min(area.width.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear background
    frame.render_widget(Clear, popup_area);

    let title = if modal.is_restart {
        " Restart with Options "
    } else {
        " Start with Options "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    frame.render_widget(block, popup_area);

    let inner_area = Rect::new(
        popup_area.x + 2,
        popup_area.y + 1,
        popup_area.width.saturating_sub(4),
        popup_area.height.saturating_sub(2),
    );

    let mut y = inner_area.y;

    for (i, var) in modal.vars.iter().enumerate() {
        if y >= inner_area.y + inner_area.height {
            break;
        }

        // Variable label with description
        let label = if var.description.is_empty() {
            var.name.clone()
        } else {
            format!("{} ({})", var.name, var.description)
        };

        let label_style = if var.is_required() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let label_line = Line::from(Span::styled(label, label_style));
        frame.render_widget(Paragraph::new(label_line), Rect::new(inner_area.x, y, inner_area.width, 1));
        y += 1;

        if y >= inner_area.y + inner_area.height {
            break;
        }

        // Value input field
        let is_active = i == modal.current_field;
        let value = &modal.values[i];

        let field_style = if is_active {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };

        let display_value = if value.is_empty() && !is_active {
            "(empty)".to_string()
        } else {
            value.clone()
        };

        let max_width = inner_area.width.saturating_sub(4) as usize;
        let truncated = if display_value.len() > max_width {
            &display_value[..max_width]
        } else {
            &display_value
        };

        let field_text = format!("  {:<width$}", truncated, width = max_width);
        let field_line = Line::from(Span::styled(field_text, field_style));
        frame.render_widget(Paragraph::new(field_line), Rect::new(inner_area.x, y, inner_area.width, 1));

        // Show cursor for active field
        if is_active {
            let cursor_x = inner_area.x + 2 + modal.cursor_pos as u16;
            if cursor_x < inner_area.x + inner_area.width {
                frame.set_cursor_position((cursor_x, y));
            }
        }

        y += 2; // spacing between variables
    }
}
