use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::ui::theme::Theme;

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
    let is_focused = app.current_panel == Panel::Console || app.current_panel == Panel::Output;

    let title = " [1] Status │ [2] Build  [3] Logcat ";

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(area);
    frame.render_widget(block, area);

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
                format!("{spinner} Running: "),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                desc,
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(state_line), status_subchunks[0]);

        let timer_eta = app.process_state.elapsed_and_eta_display();
        let timer_line = Line::from(vec![
            Span::styled("⏱ ", Style::default().fg(Theme::WARNING)),
            Span::styled(timer_eta, Style::default().fg(Theme::WARNING)),
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
                    .fg(Theme::ACCENT)
                    .bg(Color::Rgb(30, 36, 48)),
            )
            .percent(pct as u16)
            .label(Span::styled(
                gauge_label,
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, status_subchunks[2]);
    } else {
        let state_line = if app.project_loading {
            Line::from(vec![Span::styled(
                "⏳ Loading project…",
                Style::default()
                    .fg(Theme::WARNING)
                    .add_modifier(Modifier::BOLD),
            )])
        } else if let Some(ref msg) = app.status_message {
            if msg.starts_with("Failed") || msg.starts_with('❌') {
                Line::from(vec![
                    Span::styled(
                        "❌ ",
                        Style::default()
                            .fg(Theme::ERROR)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(msg, Style::default().fg(Theme::TEXT_PRIMARY)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("● ", Style::default().fg(Theme::SUCCESS)),
                    Span::styled(msg, Style::default().fg(Theme::TEXT_PRIMARY)),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled("● ", Style::default().fg(Theme::SUCCESS)),
                Span::styled("Ready", Style::default().fg(Theme::TEXT_PRIMARY)),
            ])
        };
        frame.render_widget(Paragraph::new(state_line), status_area);
    }

    // Recent operations list
    let visible_items = events_area.height as usize;
    if app.simplified_events.is_empty() {
        let empty_item = ListItem::new(Line::from(vec![Span::styled(
            "  No operations yet · [b] build  [r] run  [t] target",
            Style::default().fg(Theme::TEXT_MUTED),
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
                    Style::default().fg(Theme::SUCCESS)
                } else if event_str.starts_with('❌') || event_str.contains("Failed") {
                    Style::default().fg(Theme::ERROR)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
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
