use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::ThemePalette;

/// Single card entry rendered inside a responsive grid panel.
#[derive(Default)]
pub struct GridCardItem {
    pub badge: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub watermark: Option<&'static [&'static str]>,
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
    render_card_grid_with_hits(
        frame,
        area,
        items,
        selected_index,
        is_focused,
        cols,
        theme,
        |_, _| {},
    );
}

/// Renders `items` as a responsive grid and reports each rendered card's `Rect` via `on_card`.
#[allow(clippy::too_many_arguments)]
pub fn render_card_grid_with_hits(
    frame: &mut Frame,
    area: Rect,
    items: &[GridCardItem],
    selected_index: usize,
    is_focused: bool,
    cols: usize,
    theme: ThemePalette,
    mut on_card: impl FnMut(usize, Rect),
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

            on_card(item_idx, card_rect);
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

    let title_line = Line::from(title_spans);
    let title_width = title_line.width() as u16;

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

        let mut subtitle_width = 0u16;
        let mut lines = vec![title_line];
        if inner.height >= 2 {
            if let Some(ref sub) = item.subtitle {
                let sub_text = format!(" {sub}");
                subtitle_width = sub_text.chars().count() as u16;
                lines.push(Line::from(Span::styled(
                    sub_text,
                    Style::default().fg(theme.text_muted),
                )));
            }
        }
        frame.render_widget(Paragraph::new(lines), inner);

        if let Some(wm) = item.watermark {
            let wm_color = if is_active_selected {
                theme.accent
            } else if is_selected {
                theme.text_muted
            } else {
                theme.inactive_border
            };
            render_watermark(
                frame,
                inner,
                wm,
                title_width,
                subtitle_width,
                wm_color,
            );
        }
    } else {
        let bg = if is_active_selected {
            theme.bg_selected
        } else if is_selected {
            theme.bg_selected_inactive
        } else {
            theme.bg_card
        };
        let paragraph = Paragraph::new(title_line).style(Style::default().bg(bg));
        frame.render_widget(paragraph, area);

        if let Some(wm) = item.watermark {
            let wm_color = if is_active_selected {
                theme.accent
            } else if is_selected {
                theme.text_muted
            } else {
                theme.inactive_border
            };
            render_watermark(frame, area, wm, title_width, 0, wm_color);
        }
    }
}

/// Renders a subtle Braille-dot watermark in the bottom-right (or remaining free space)
/// of a card without overlapping its title or subtitle.
fn render_watermark(
    frame: &mut Frame,
    area: Rect,
    wm: &[&str],
    title_width: u16,
    subtitle_width: u16,
    color: ratatui::style::Color,
) {
    if wm.is_empty() || area.width < 6 || area.height == 0 {
        return;
    }

    let wm_width = wm.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u16;
    let wm_height = wm.len() as u16;
    if wm_width == 0 || area.width <= wm_width {
        return;
    }

    let right_pad = u16::from(area.width >= wm_width + 2);
    let draw_x = area.right().saturating_sub(wm_width).saturating_sub(right_pad);

    let (slice_start, start_y) = if area.height > wm_height {
        (0usize, area.bottom() - wm_height)
    } else if area.height == wm_height && area.x + title_width < draw_x {
        (0usize, area.y)
    } else if area.height >= 2 {
        (wm.len() - 1, area.bottom() - 1)
    } else if area.x + title_width + 2 <= draw_x {
        (0usize, area.y)
    } else {
        return;
    };

    let buf = frame.buffer_mut();
    for (row_offset, row_str) in wm[slice_start..].iter().enumerate() {
        let cy = start_y + row_offset as u16;
        if cy >= area.bottom() {
            break;
        }
        // Never overwrite title (row 0) or subtitle (row 1) if they reach into draw_x.
        if cy == area.y && area.x + title_width >= draw_x {
            continue;
        }
        if cy == area.y + 1 && subtitle_width > 0 && area.x + subtitle_width >= draw_x {
            continue;
        }
        for (col_offset, ch) in row_str.chars().enumerate() {
            if ch == ' ' || ch == '\u{2800}' {
                continue;
            }
            let cx = draw_x + col_offset as u16;
            if cx < area.right() {
                if let Some(cell) = buf.cell_mut((cx, cy)) {
                    cell.set_char(ch).set_fg(color);
                }
            }
        }
    }
}
