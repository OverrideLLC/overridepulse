use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, ClickTarget, Panel};
use crate::i18n::ui_text;

/// Formats the parity navigation bar showing which console panes are active:
/// `[m] View │ [1] Simplified (ON/OFF) │ [2] Build (ON/OFF) │ [3] Logcat (ON/OFF)`
pub fn console_parity_bar(app: &AppState) -> String {
    let s_status = if app.show_simplified_pane {
        "ON"
    } else {
        "OFF"
    };
    let b_status = if app.show_build_pane { "ON" } else { "OFF" };
    let l_status = if app.show_logcat_pane { "ON" } else { "OFF" };
    format!(
        "[m] View │ [1] Simplified ({s_status}) │ [2] Build ({b_status}) │ [3] Logcat ({l_status})"
    )
}

/// Renders the Simplified Status pane with a clean status header,
/// live timer/progress only while active, and recent operation history.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::Console;

    let title = txt.status_tab_title;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Make [2] Build and [3] Logcat tabs in the top border clickable to toggle panes with mouse.
    if area.height >= 1 && area.width > 12 {
        let mut hit_map = app.hit_map.borrow_mut();
        if let Some(byte_idx2) = title.find("[2]") {
            let char_x2 = title[..byte_idx2].chars().count() as u16 + 1;
            let byte_idx3 = title.find("[3]");
            let end_x2 = byte_idx3
                .map(|b| title[..b].chars().count() as u16 + 1)
                .unwrap_or(title.chars().count() as u16 + 1);
            if area.x + char_x2 < area.right() {
                hit_map.push(
                    Rect {
                        x: area.x + char_x2,
                        y: area.y,
                        width: end_x2.saturating_sub(char_x2).min(area.right() - (area.x + char_x2)),
                        height: 1,
                    },
                    ClickTarget::ToggleBuildPane,
                );
            }
        }
        if let Some(byte_idx3) = title.find("[3]") {
            let char_x3 = title[..byte_idx3].chars().count() as u16 + 1;
            let total_chars = title.chars().count() as u16 + 1;
            if area.x + char_x3 < area.right() {
                hit_map.push(
                    Rect {
                        x: area.x + char_x3,
                        y: area.y,
                        width: total_chars
                            .saturating_sub(char_x3)
                            .min(area.right() - (area.x + char_x3)),
                        height: 1,
                    },
                    ClickTarget::ToggleLogcatPane,
                );
            }
        }
    }

    if inner.height < 3 || inner.width < 10 {
        return;
    }

    // Show timer + progress gauge only while a task is actively running.
    let card_height = if app.process_state.is_running && inner.height >= 7 {
        4
    } else if app.process_state.is_running {
        2
    } else {
        1
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(card_height),
            Constraint::Min(1),
        ])
        .split(inner);

    let status_area = chunks[0];
    let events_area = chunks[1];

    if app.process_state.is_running {
        let status_subchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(status_area);

        let spinner = app.process_state.spinner_char();
        let desc = app
            .process_state
            .task_description
            .as_deref()
            .unwrap_or("Running task...");
        let state_line = Line::from(vec![
            Span::styled(
                format!("{spinner} {}", txt.running_prefix),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                desc,
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(state_line), status_subchunks[0]);

        let timer_eta = app.process_state.elapsed_and_eta_display();
        let timer_line = Line::from(vec![
            Span::styled("⏱ ", Style::default().fg(theme.warning)),
            Span::styled(timer_eta, Style::default().fg(theme.warning)),
        ]);
        frame.render_widget(Paragraph::new(timer_line), status_subchunks[1]);

        let pct = app.process_state.progress_percent.unwrap_or(0);
        let gauge_label = match app.process_state.progress_percent {
            Some(p) => format!("{p}%"),
            None => "In progress…".to_string(),
        };
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.bg_card),
            )
            .percent(pct as u16)
            .label(Span::styled(
                gauge_label,
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, status_subchunks[2]);
    } else {
        let state_line = if app.project_loading {
            Line::from(vec![Span::styled(
                txt.loading_project,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )])
        } else if let Some(ref msg) = app.status_message {
            if msg.starts_with("Failed") || msg.starts_with('❌') {
                Line::from(vec![
                    Span::styled(
                        "❌ ",
                        Style::default()
                            .fg(theme.error)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(msg, Style::default().fg(theme.text_primary)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("● ", Style::default().fg(theme.success)),
                    Span::styled(msg, Style::default().fg(theme.text_primary)),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled("● ", Style::default().fg(theme.success)),
                Span::styled(txt.ready, Style::default().fg(theme.text_primary)),
            ])
        };
        frame.render_widget(Paragraph::new(state_line), status_area);
    }

    // Recent operations list
    let visible_items = events_area.height as usize;
    if app.simplified_events.is_empty() {
        let empty_item = ListItem::new(Line::from(vec![Span::styled(
            txt.no_operations,
            Style::default().fg(theme.text_muted),
        )]));
        frame.render_widget(List::new(vec![empty_item]), events_area);
    } else {
        let items: Vec<ListItem> = app
            .simplified_events
            .iter()
            .rev()
            .take(visible_items)
            .map(|event_str| {
                let style = if event_str.starts_with('✅') || event_str.contains("Success") {
                    Style::default().fg(theme.success)
                } else if event_str.starts_with('❌') || event_str.contains("Failed") {
                    Style::default().fg(theme.error)
                } else {
                    Style::default().fg(theme.text_primary)
                };
                ListItem::new(Line::from(vec![Span::styled(
                    format!("  {event_str}"),
                    style,
                )]))
            })
            .collect();

        frame.render_widget(List::new(items), events_area);
    }
}
