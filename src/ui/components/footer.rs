use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the bottom footer bar with rounded borders:
/// - Left side: Navigation and command keybindings:
///   `[Tab] Panel │ [Enter] Run │ [m] View Mode │ [1/2/3] Toggle Panes │ [w] Wi-Fi ADB │ [v] Variant │ [?] Help │ [q] Quit`
/// - Right side: Active project name, detected `applicationId`, and active build variant:
///   `📦 MyApp (com.example.app) │ ⚡ Variant: debug`
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
            Constraint::Percentage(64), // Shortcuts
            Constraint::Percentage(36), // Project info & Variant
        ])
        .split(inner);

    let key_style = Style::default()
        .fg(Theme::WARNING)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::TEXT_PRIMARY);
    let sep_style = Style::default().fg(Theme::TEXT_MUTED);

    let shortcuts_line = Line::from(vec![
        Span::styled(" [Tab] ", key_style),
        Span::styled("Panel", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[Enter] ", key_style),
        Span::styled("Run", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[p] ", key_style),
        Span::styled("Project", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[y/Y] ", key_style),
        Span::styled("Copy", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[m] ", key_style),
        Span::styled("View", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[w] ", key_style),
        Span::styled("ADB", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[v] ", key_style),
        Span::styled("Variant", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[?] ", key_style),
        Span::styled("Help", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[q] ", key_style),
        Span::styled("Quit", desc_style),
    ]);

    let mut info_spans = vec![
        Span::styled("📦 ", Style::default().fg(Theme::ACCENT)),
        Span::styled(
            &app.project.name,
            Style::default()
                .fg(Theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if app.project_loading {
        info_spans.push(Span::styled(
            " loading…",
            Style::default().fg(Theme::WARNING),
        ));
    } else if let Some(ref app_id) = app.project.application_id {
        info_spans.push(Span::styled(
            format!(" ({app_id})"),
            Style::default().fg(Theme::TEXT_MUTED),
        ));
    }

    info_spans.push(Span::styled(" │ ⚡ Variant: ", sep_style));
    info_spans.push(Span::styled(
        format!("{} ", app.project.active_variant),
        Style::default()
            .fg(Theme::WARNING)
            .add_modifier(Modifier::BOLD),
    ));

    let project_info_line = Line::from(info_spans);

    frame.render_widget(Paragraph::new(shortcuts_line), chunks[0]);
    frame.render_widget(
        Paragraph::new(project_info_line).alignment(Alignment::Right),
        chunks[1],
    );
}
