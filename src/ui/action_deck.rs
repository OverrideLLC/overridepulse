use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::ui::theme::Theme;

/// Renders the `[ App & Gradle Actions ]` deck panel with rounded borders.
/// Displays single-letter shortcuts only for frequent repetitive actions:
/// `[b]`, `[r]`, `[k]`, `[u]`, and `[g]`.
/// Menu/utility actions (`Clear App Data`, `Build AAB & Deploy`, `Launch Scrcpy`)
/// are presented cleanly without a letter badge.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Actions;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" App & Gradle Actions ")
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused))
        .style(Style::default().bg(Theme::BG_DARK));

    let items: Vec<ListItem> = app
        .actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let is_selected = idx == app.selected_action_index;
            let cursor = if is_selected && is_focused { "▶ " } else { "  " };

            let title = action.title();

            let mut spans = Vec::new();
            spans.push(Span::styled(cursor, Style::default().fg(Theme::ACCENT)));

            if let Some(sc) = action.shortcut() {
                spans.push(Span::styled(
                    format!("[{sc}] "),
                    Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(
                    "• ",
                    Style::default().fg(Theme::TEXT_MUTED),
                ));
            }

            spans.push(Span::styled(
                title,
                if is_selected {
                    Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Theme::TEXT_PRIMARY)
                },
            ));

            let item = ListItem::new(Line::from(spans));
            if is_selected && is_focused {
                item.style(Theme::selected_style())
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
