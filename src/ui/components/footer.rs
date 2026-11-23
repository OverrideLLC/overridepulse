use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::i18n::ui_text;

/// Renders the bottom footer bar with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(58), // Shortcuts
            Constraint::Percentage(42), // Project & Target
        ])
        .split(inner);

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let sep_style = Style::default().fg(theme.text_muted);

    let shortcuts_line = if app.devices_view_active {
        Line::from(vec![
            Span::styled(" [d/Esc] ", key_style),
            Span::styled(txt.footer_back, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[Tab] ", key_style),
            Span::styled(txt.footer_switch_pane, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[w] ", key_style),
            Span::styled("Wi-Fi ADB", desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled(txt.footer_select_run, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[q] ", key_style),
            Span::styled(txt.footer_quit, desc_style),
        ])
    } else {
        Line::from(vec![
            Span::styled(" [t] ", key_style),
            Span::styled(txt.footer_target, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[p] ", key_style),
            Span::styled(txt.footer_project, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[d] ", key_style),
            Span::styled(txt.footer_devices, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[1/2/3] ", key_style),
            Span::styled(txt.footer_views, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[?] ", key_style),
            Span::styled(txt.footer_help, desc_style),
            Span::styled("  ", sep_style),
            Span::styled("[q] ", key_style),
            Span::styled(txt.footer_quit, desc_style),
        ])
    };

    let mut info_spans = vec![
        Span::styled("📦 ", Style::default().fg(theme.accent)),
        Span::styled(
            &app.project.name,
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if app.project_loading {
        info_spans.push(Span::styled(
            " loading…",
            Style::default().fg(theme.warning),
        ));
    } else if let Some(target) = app.project.active_target() {
        info_spans.push(Span::styled(" │ ", sep_style));
        info_spans.push(Span::styled(
            format!("{} {}", target.platform.icon(), target.display_label()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.project.active_platform().is_android() {
        info_spans.push(Span::styled(
            format!(" ({}) ", app.project.active_variant),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        info_spans.push(Span::raw(" "));
    }

    let project_info_line = Line::from(info_spans);

    frame.render_widget(Paragraph::new(shortcuts_line), chunks[0]);
    frame.render_widget(
        Paragraph::new(project_info_line).alignment(Alignment::Right),
        chunks[1],
    );
}
