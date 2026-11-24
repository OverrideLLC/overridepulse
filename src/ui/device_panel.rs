use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::android::DeviceState;
use crate::app::{AppState, ClickTarget, Panel};
use crate::i18n::ui_text;

/// Renders the `[ Connected Devices ]` panel with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::Devices;

    let title = format!(
        " {} ({}) │ [w] Wi-Fi ADB │ [d/Esc] {} ",
        txt.footer_devices,
        app.devices.len(),
        txt.footer_back
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    if app.devices.is_empty() {
        let empty_item = ListItem::new(Line::from(vec![Span::styled(
            txt.no_devices,
            Style::default().fg(theme.text_muted),
        )]));
        let list = List::new(vec![empty_item]).block(block);
        frame.render_widget(list, area);
        return;
    }

    let items: Vec<ListItem> = app
        .devices
        .iter()
        .enumerate()
        .map(|(idx, device)| {
            let is_selected = idx == app.selected_device_index;

            // Dot and badge: ● for active/selected device, ○ for others
            let (status_bullet, bullet_color) = if is_selected {
                ("●", theme.success)
            } else if device.state == DeviceState::Device {
                ("○", theme.text_muted)
            } else {
                ("■", theme.warning)
            };

            let mut spans = Vec::new();

            // Selection indicator
            let cursor = if is_selected && is_focused {
                "▶ "
            } else {
                "  "
            };
            spans.push(Span::styled(cursor, Style::default().fg(theme.accent)));

            // Bullet icon
            spans.push(Span::styled(
                format!("{status_bullet} "),
                Style::default()
                    .fg(bullet_color)
                    .add_modifier(Modifier::BOLD),
            ));

            // Device model name or serial
            let model_name = if device.model != "Unknown" && !device.model.is_empty() {
                &device.model
            } else {
                &device.serial
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            spans.push(Span::styled(model_name, name_style));

            // Connection type
            spans.push(Span::styled(
                format!(" ({})", device.connection_type),
                Style::default().fg(theme.text_muted),
            ));

            // Non-ready state badge if applicable
            if device.state != DeviceState::Device {
                spans.push(Span::styled(
                    format!(" [{}]", device.state),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            // Battery level
            if let Some(battery) = device.battery_level {
                let bat_color = if battery > 50 {
                    theme.success
                } else if battery > 20 {
                    theme.warning
                } else {
                    theme.error
                };
                spans.push(Span::styled(
                    format!(" · {battery}%"),
                    Style::default().fg(bat_color),
                ));
            }

            // Android OS / API Level
            match (&device.android_version, device.api_level) {
                (Some(ver), Some(_)) => {
                    spans.push(Span::styled(
                        format!(" · Android {ver}"),
                        Style::default().fg(theme.text_muted),
                    ));
                }
                (None, Some(api)) => {
                    spans.push(Span::styled(
                        format!(" · API {api}"),
                        Style::default().fg(theme.text_muted),
                    ));
                }
                (Some(ver), None) => {
                    spans.push(Span::styled(
                        format!(" · Android {ver}"),
                        Style::default().fg(theme.text_muted),
                    ));
                }
                (None, None) => {}
            }

            let item = ListItem::new(Line::from(spans));
            if is_selected && is_focused {
                item.style(theme.selected_style())
            } else {
                item
            }
        })
        .collect();

    let inner = block.inner(area);
    {
        let mut hit_map = app.hit_map.borrow_mut();
        hit_map.push(area, ClickTarget::FocusPanel(Panel::Devices));
        hit_map.push_scroll(area, ClickTarget::ScrollArea(Panel::Devices));
        for idx in 0..app.devices.len() {
            let y = inner.y + idx as u16;
            if y < inner.bottom() {
                hit_map.push(
                    Rect {
                        x: inner.x,
                        y,
                        width: inner.width,
                        height: 1,
                    },
                    ClickTarget::DeviceRow(idx),
                );
            }
        }
    }

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
