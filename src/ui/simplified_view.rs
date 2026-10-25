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
    let s_status = if app.show_simplified_pane { "ON" } else { "OFF" };
    let b_status = if app.show_build_pane { "ON" } else { "OFF" };
    let l_status = if app.show_logcat_pane { "ON" } else { "OFF" };
    format!("[m] View │ [1] Simplified ({s_status}) │ [2] Build ({b_status}) │ [3] Logcat ({l_status})")
}

/// Renders the Simplified Status pane with status card, live timer & ETA,
/// progress gauge, and history of completed operations.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Console || app.current_panel == Panel::Output;

    let parity = console_parity_bar(app);
    let title = format!(" [1] Simplified Status │ {parity} ");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    // Split inner vertically into status card / progress (top) and history list (bottom)
    let card_height = if inner.height >= 8 { 4 } else { 3 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(card_height), // Status & Gauge
            Constraint::Min(2),             // Events history
        ])
        .split(inner);

    let status_area = chunks[0];
    let events_area = chunks[1];

    // 1. Status Card & Progress Gauge
    let status_subchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // State line
            Constraint::Length(1), // Timer & ETA line
            Constraint::Length(1), // Gauge
            Constraint::Min(0),
        ])
        .split(status_area);

    // Line 1: State line
    let state_line = if app.process_state.is_running {
        let spinner = app.process_state.spinner_char();
        let desc = app
            .process_state
            .task_description
            .as_deref()
            .unwrap_or("Running task...");
        Line::from(vec![
            Span::styled(
                format!("{spinner} ⚡ Running: "),
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
        ])
    } else if let Some(ref msg) = app.status_message {
        if msg.starts_with("Failed") || msg.starts_with("❌") {
            Line::from(vec![
                Span::styled(
                    "❌ Failed: ",
                    Style::default()
                        .fg(Theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg, Style::default().fg(Theme::TEXT_PRIMARY)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "🟢 Ready / Idle: ",
                    Style::default()
                        .fg(Theme::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(msg, Style::default().fg(Theme::TEXT_MUTED)),
            ])
        }
    } else {
        Line::from(vec![
            Span::styled(
                "🟢 Ready / Idle",
                Style::default()
                    .fg(Theme::SUCCESS)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    frame.render_widget(Paragraph::new(state_line), status_subchunks[0]);

    // Line 2: Timer & ETA
    let timer_eta = app.process_state.elapsed_and_eta_display();
    let timer_line = Line::from(vec![
        Span::styled(
            "⏱  ",
            Style::default().fg(Theme::WARNING),
        ),
        Span::styled(
            timer_eta,
            Style::default()
                .fg(Theme::WARNING)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(timer_line), status_subchunks[1]);

    // Line 3: Progress Gauge
    let pct = app.process_state.progress_percent.unwrap_or(0);
    let gauge_pct = if app.process_state.is_running {
        pct as u16
    } else {
        0
    };

    let gauge_label = if app.process_state.is_running {
        if let Some(p) = app.process_state.progress_percent {
            format!("{p}%")
        } else {
            "Progress: In Progress".to_string()
        }
    } else {
        "Idle".to_string()
    };

    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(Theme::ACCENT)
                .bg(Color::Rgb(30, 36, 48)),
        )
        .percent(gauge_pct)
        .label(Span::styled(
            gauge_label,
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(gauge, status_subchunks[2]);

    // 2. Events & Operations History
    let events_inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header label
            Constraint::Min(1),    // Items
        ])
        .split(events_area);

    let header_line = Line::from(vec![
        Span::styled(
            "📋 Recent Operations:",
            Style::default()
                .fg(Theme::TEXT_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header_line), events_inner_chunks[0]);

    let list_area = events_inner_chunks[1];
    let visible_items = list_area.height as usize;

    if app.simplified_events.is_empty() {
        let empty_item = ListItem::new(Line::from(vec![
            Span::styled(
                "  No completed operations yet. Press [b], [r], [k], etc.",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
        frame.render_widget(List::new(vec![empty_item]), list_area);
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
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {event_str}"), style),
                ]))
            })
            .collect();

        frame.render_widget(List::new(items), list_area);
    }
}
