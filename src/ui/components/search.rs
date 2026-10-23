use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the search/filter modal bar with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let search_area = centered_search_bar(65, 3, area);

    frame.render_widget(Clear, search_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Filter / Search ")
        .title_style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Theme::ACCENT))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(search_area);
    frame.render_widget(block, search_area);

    let prompt = Line::from(vec![
        Span::styled(
            "/ ",
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.search_query, Style::default().fg(Theme::TEXT_PRIMARY)),
        Span::styled("█", Style::default().fg(Theme::ACCENT)),
        Span::styled(
            "  (Enter: Apply │ Esc: Cancel)",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);

    let paragraph = Paragraph::new(prompt);
    frame.render_widget(paragraph, inner);
}

fn centered_search_bar(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vertical_margin = (r.height.saturating_sub(height)) / 2;

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_margin),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v_chunks[1]);

    h_chunks[1]
}
