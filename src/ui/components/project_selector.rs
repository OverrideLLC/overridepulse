use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the interactive Android project selector modal with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let popup_area = centered_rect(75, 65, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Select Android Project ")
        .title_style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Theme::ACCENT))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.nearby_projects.is_empty() {
        let empty_msg = vec![
            Line::from(vec![
                Span::styled(
                    "⚠️  No Android projects found nearby",
                    Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    "OverridePulse scanned up to 3 directory levels from: ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {}", app.project.root_path.display()),
                    Style::default().fg(Theme::TEXT_PRIMARY),
                ),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    "Please navigate to an Android project folder and run overridepulse,",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "or specify the project root via: ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled("overridepulse --project <PATH>", Style::default().fg(Theme::ACCENT)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("[Esc] / [q] Dismiss", Style::default().fg(Theme::WARNING)),
            ]),
        ];
        let paragraph = Paragraph::new(empty_msg);
        frame.render_widget(paragraph, inner);
        return;
    }

    // Split inner into: Project list and Bottom navigation hint
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(2)])
        .split(inner);

    let list_area = chunks[0];
    let footer_area = chunks[1];

    let items: Vec<ListItem> = app
        .nearby_projects
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let is_selected = idx == app.selected_project_index;
            let cursor = if is_selected { "▶ " } else { "  " };

            let project_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("AndroidProject");

            let name_style = if is_selected {
                Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    format!("{idx}. "),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
                Span::styled(project_name, name_style),
                Span::styled(
                    format!("  ({})", path.display()),
                    Style::default().fg(Theme::TEXT_MUTED),
                ),
            ]);

            let item = ListItem::new(line);
            if is_selected {
                item.style(Theme::selected_style())
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    let nav_hint = Line::from(vec![
        Span::styled("[Enter]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::raw(" Open Project  "),
        Span::styled("[↑/↓ or k/j]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("[Esc/p]", Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD)),
        Span::raw(" Close"),
    ]);
    frame.render_widget(Paragraph::new(nav_hint), footer_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
