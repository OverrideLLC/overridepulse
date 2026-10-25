use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
    Frame,
};

use crate::android::DeviceState;
use crate::app::{AppState, Panel};
use crate::ui::theme::Theme;

/// Renders the `[ Connected Devices ]` panel with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Devices;

    let title = format!(" Connected Devices ({}) │ [w] Wi-Fi ADB ", app.devices.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused));

    if app.devices.is_empty() {
        let empty_item = ListItem::new(Line::from(vec![
            Span::styled(
                "  No devices connected. Plug in a device or press [w] for Wi-Fi ADB.",
                Style::default().fg(Theme::TEXT_MUTED),
            ),
        ]));
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
                ("●", Theme::SUCCESS)
            } else if device.state == DeviceState::Device {
                ("○", Theme::TEXT_MUTED)
            } else {
                ("■", Theme::WARNING)
            };

            let mut spans = Vec::new();

            // Selection indicator
            let cursor = if is_selected && is_focused { "▶ " } else { "  " };
            spans.push(Span::styled(cursor, Style::default().fg(Theme::ACCENT)));

            // Bullet icon
            spans.push(Span::styled(
                format!("{status_bullet} "),
                Style::default().fg(bullet_color).add_modifier(Modifier::BOLD),
            ));

            // Device model name or serial
            let model_name = if device.model != "Unknown" && !device.model.is_empty() {
                &device.model
            } else {
                &device.serial
            };

            let name_style = if is_selected {
                Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT_PRIMARY)
            };
            spans.push(Span::styled(model_name, name_style));

            // Connection type
            spans.push(Span::styled(
                format!(" ({})", device.connection_type),
                Style::default().fg(Theme::TEXT_MUTED),
            ));

            // Active badge
            if is_selected {
                spans.push(Span::styled(
                    " [ACTIVE]",
                    Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD),
                ));
            }

            // Non-ready state badge if applicable
            if device.state != DeviceState::Device {
                spans.push(Span::styled(
                    format!(" [{}]", device.state),
                    Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
                ));
            }

            // Battery level
            if let Some(battery) = device.battery_level {
                let bat_color = if battery > 50 {
                    Theme::SUCCESS
                } else if battery > 20 {
                    Theme::WARNING
                } else {
                    Theme::ERROR
                };
                spans.push(Span::styled(
                    format!(" │ ⚡{battery}%"),
                    Style::default().fg(bat_color),
                ));
            }

            // Android OS / API Level
            match (&device.android_version, device.api_level) {
                (Some(ver), Some(api)) => {
                    spans.push(Span::styled(
                        format!(" │ Android {ver} (API {api})"),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
                (None, Some(api)) => {
                    spans.push(Span::styled(
                        format!(" │ API {api}"),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
                (Some(ver), None) => {
                    spans.push(Span::styled(
                        format!(" │ Android {ver}"),
                        Style::default().fg(Theme::TEXT_MUTED),
                    ));
                }
                (None, None) => {}
            }

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
