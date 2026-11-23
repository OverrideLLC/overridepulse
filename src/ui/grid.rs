use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemePalette;

/// Single card entry rendered inside a responsive grid panel.
pub struct GridCardItem {
    pub badge: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
}

/// Renders `items` as a responsive `cols`-column grid of rounded cards (or compact tiles on short terminals).
pub fn render_card_grid(
    frame: &mut Frame,
    area: Rect,
    items: &[GridCardItem],
    selected_index: usize,
    is_focused: bool,
    cols: usize,
    theme: ThemePalette,
) {
    if area.width == 0 || area.height == 0 || items.is_empty() {
        return;
    }

    let cols = cols.max(1).min(items.len());
    let rows = items.len().div_ceil(cols);

    let row_constraints = vec![Constraint::Ratio(1, rows as u32); rows];
    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    let col_constraints = vec![Constraint::Ratio(1, cols as u32); cols];

    for (row_idx, row_area) in row_areas.iter().enumerate() {
        let cell_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints.clone())
            .split(*row_area);

        for (col_idx, cell_area) in cell_areas.iter().enumerate() {
            let item_idx = row_idx * cols + col_idx;
            let Some(item) = items.get(item_idx) else {
                continue;
            };

            let is_selected = item_idx == selected_index;
            let is_active_selected = is_selected && is_focused;

            // Leave a 1-column breathing gutter between side-by-side cards when space permits.
            let card_rect = if col_idx + 1 < cols && cell_area.width > 6 {
                Rect {
                    width: cell_area.width - 1,
                    ..*cell_area
                }
            } else {
                *cell_area
            };

            render_single_card(frame, card_rect, item, is_selected, is_active_selected, theme);
        }
    }
}

fn render_single_card(
    frame: &mut Frame,
    area: Rect,
    item: &GridCardItem,
    is_selected: bool,
    is_active_selected: bool,
    theme: ThemePalette,
) {
    let mut title_spans = Vec::new();
    if is_active_selected {
        title_spans.push(Span::styled("▶ ", Style::default().fg(theme.accent)));
    } else {
        title_spans.push(Span::raw(" "));
    }

    if let Some(ref badge) = item.badge {
        title_spans.push(Span::styled(
            format!("{badge} "),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    }

    title_spans.push(Span::styled(
        item.title.as_str(),
        if is_selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        },
    ));

    if area.height >= 3 {
        let (border_color, bg_color) = if is_active_selected {
            (theme.accent, theme.bg_selected)
        } else if is_selected {
            (theme.text_muted, theme.bg_selected_inactive)
        } else {
            (theme.inactive_border, theme.bg_card)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(bg_color));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let mut lines = vec![Line::from(title_spans)];
        if inner.height >= 2 {
            if let Some(ref sub) = item.subtitle {
                lines.push(Line::from(Span::styled(
                    format!(" {sub}"),
                    Style::default().fg(theme.text_muted),
                )));
            }
        }
        frame.render_widget(Paragraph::new(lines), inner);
    } else {
        let bg = if is_active_selected {
            theme.bg_selected
        } else if is_selected {
            theme.bg_selected_inactive
        } else {
            theme.bg_card
        };
        let paragraph = Paragraph::new(Line::from(title_spans)).style(Style::default().bg(bg));
        frame.render_widget(paragraph, area);
    }
}
