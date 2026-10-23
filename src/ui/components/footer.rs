use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the bottom footer bar with rounded borders and keybinding hints:
/// `[Tab] Switch Panel │ [g] Stop Gradle │ [/] Filter │ [v] Variant │ [l] Logcat │ [?] Help │ [q] Quit`
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
            Constraint::Percentage(78), // Shortcuts
            Constraint::Percentage(22), // Status message
        ])
        .split(inner);

    let key_style = Style::default()
        .fg(Theme::WARNING)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::TEXT_PRIMARY);
    let sep_style = Style::default().fg(Theme::TEXT_MUTED);

    let shortcuts_line = Line::from(vec![
        Span::styled(" [Tab] ", key_style),
        Span::styled("Switch Panel", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[g] ", key_style),
        Span::styled("Stop Gradle", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[/] ", key_style),
        Span::styled("Filter", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[v] ", key_style),
        Span::styled("Variant", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[l] ", key_style),
        Span::styled("Logcat", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[?] ", key_style),
        Span::styled("Help", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[q] ", key_style),
        Span::styled("Quit", desc_style),
    ]);

    let status_text = app.status_message.as_deref().unwrap_or("Ready");
    let status_line = Line::from(vec![
        Span::styled(status_text, Style::default().fg(Theme::TEXT_MUTED)),
        Span::raw(" "),
    ]);

    frame.render_widget(Paragraph::new(shortcuts_line), chunks[0]);
    frame.render_widget(
        Paragraph::new(status_line).alignment(Alignment::Right),
        chunks[1],
    );
}
