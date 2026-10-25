use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::android::DeviceAction;
use crate::app::{AppState, Panel};
use crate::ui::theme::Theme;

fn device_action_icon(action: DeviceAction) -> &'static str {
    match action {
        DeviceAction::WirelessPairAndConnect => "📶",
        DeviceAction::EnableTcpip5555 => "🌐",
        DeviceAction::ToggleWifi => "📡",
        DeviceAction::ToggleMobileData => "📶",
        DeviceAction::ToggleStayAwake => "👁 ",
        DeviceAction::ToggleDarkMode => "🌓",
        DeviceAction::ToggleShowTaps => "👆",
        DeviceAction::OpenWirelessDebuggingSettings => "⚙ ",
        DeviceAction::TakeScreenshot => "📸",
        DeviceAction::RebootDevice => "🔄",
    }
}

/// Renders the `[ Device Controls (ADB) ]` panel with rounded borders,
/// presenting direct quick hardware toggles and Wi-Fi ADB pairing shortcuts.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::DeviceControls;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Device Controls (ADB) ")
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let show_hint = inner.height >= 12;
    let (list_area, hint_area) = if show_hint {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(1)])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner, None)
    };

    let actions = DeviceAction::all();
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let is_selected = idx == app.selected_device_control_index;
            let cursor = if is_selected && is_focused { "▶ " } else { "  " };

            let mut spans = vec![
                Span::styled(cursor, Style::default().fg(Theme::ACCENT)),
                Span::styled(
                    format!("{} ", device_action_icon(*action)),
                    Style::default().fg(Theme::ACCENT),
                ),
                Span::styled(
                    action.title(),
                    if is_selected {
                        Style::default()
                            .fg(Theme::ACCENT)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Theme::TEXT_PRIMARY)
                    },
                ),
            ];

            // Highlight shortcut for Wi-Fi pair/connect
            if *action == DeviceAction::WirelessPairAndConnect {
                spans.push(Span::styled(
                    " [w]",
                    Style::default()
                        .fg(Theme::WARNING)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            let item = ListItem::new(Line::from(spans));
            if is_selected && is_focused {
                item.style(Theme::selected_style())
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    if let Some(h_area) = hint_area {
        if let Some(action) = app.selected_device_control() {
            let desc_line = Line::from(vec![
                Span::styled("  ℹ ", Style::default().fg(Theme::ACCENT)),
                Span::styled(action.description(), Style::default().fg(Theme::TEXT_MUTED)),
            ]);
            frame.render_widget(Paragraph::new(desc_line), h_area);
        }
    }
}
