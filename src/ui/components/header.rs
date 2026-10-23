use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the top application header bar with rounded borders according to PRD Section 6:
/// `OVERRIDEPULSE v0.1.0 ── Project: <Name> [<applicationId>] ─ Variant: <variant>` + Animated Spinner.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::INACTIVE_BORDER))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Title, Project, Variant
            Constraint::Percentage(30), // Process status / Spinner
        ])
        .split(inner);

    // Left block: OVERRIDEPULSE v0.1.0 ── Project: <Name> [<applicationId>] ─ Variant: <variant>
    let mut header_spans = vec![
        Span::styled(
            "⚡ OVERRIDEPULSE v0.1.0 ",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("── Project: ", Style::default().fg(Theme::TEXT_MUTED)),
        Span::styled(
            &app.project.name,
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some(app_id) = &app.project.application_id {
        header_spans.push(Span::styled(
            format!(" [{app_id}]"),
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    }

    header_spans.push(Span::styled(" ─ Variant: ", Style::default().fg(Theme::TEXT_MUTED)));
    header_spans.push(Span::styled(
        &app.project.active_variant,
        Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD),
    ));

    frame.render_widget(Paragraph::new(Line::from(header_spans)), chunks[0]);

    // Right block: Spinner + active task or idle status
    let status_line = if app.process_state.is_running {
        let spinner = app.process_state.spinner_char();
        let desc = app
            .process_state
            .task_description
            .as_deref()
            .unwrap_or("Running...");

        let pct_str = app
            .process_state
            .progress_percent
            .map(|p| format!(" [{p}%]"))
            .unwrap_or_default();

        Line::from(vec![
            Span::styled(
                format!("{spinner} "),
                Style::default()
                    .fg(Theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc, Style::default().fg(Theme::ACCENT)),
            Span::styled(pct_str, Style::default().fg(Theme::WARNING)),
        ])
    } else {
        Line::from(vec![
            Span::styled("● ", Style::default().fg(Theme::SUCCESS)),
            Span::styled("Idle", Style::default().fg(Theme::TEXT_MUTED)),
        ])
    };

    frame.render_widget(
        Paragraph::new(status_line).alignment(Alignment::Right),
        chunks[1],
    );
}
