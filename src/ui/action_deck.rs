use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::app::{AppState, Panel, ViewMode};
use crate::ui::theme::Theme;

/// Renders the `[ Quick Actions ]` deck panel with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Actions;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Quick Actions ")
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused));

    let mut items: Vec<ListItem> = app
        .actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let is_selected = idx == app.selected_action_index;
            let cursor = if is_selected && is_focused { "▶ " } else { "  " };

            let shortcut = action.shortcut();
            let title = action.title();

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    format!("[{shortcut}] "),
                    Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    title,
                    if is_selected {
                        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Theme::TEXT_PRIMARY)
                    },
                ),
            ]);

            let item = ListItem::new(line);
            if is_selected && is_focused {
                item.style(Theme::selected_style())
            } else {
                item
            }
        })
        .collect();

    // Append "[l] Switch to Live Logcat" (or Build Output) as a quick action indicator
    let logcat_action_title = match app.view_mode {
        ViewMode::BuildOutput => "Switch to Live Logcat",
        ViewMode::LogcatView => "Switch to Build Output",
    };

    let logcat_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            "[l] ",
            Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            logcat_action_title,
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);
    items.push(ListItem::new(logcat_line));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
