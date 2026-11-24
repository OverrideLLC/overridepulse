use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::android::DeviceAction;
use crate::app::{grid_cols_for_width, AppState, ClickTarget, Panel};
use crate::i18n::{localized_device_action_title, ui_text};
use crate::ui::grid::{render_card_grid_with_hits, GridCardItem};

/// Returns a subtle 2-row Braille-dot watermark sprite tailored to each ADB device control.
pub fn device_action_watermark(action: DeviceAction) -> &'static [&'static str] {
    match action {
        DeviceAction::WirelessPairAndConnect => &["⠐⣫⠭⠭⣝⠂", "⠀⠀⠹⠏⠀⠀"],
        DeviceAction::EnableTcpip5555 => &["⢠⡼⠧⠼⢧⡄", "⠀⠉⢻⡟⠉⠀"],
        DeviceAction::ToggleWifi => &["⠀⠀⢠⡄⣿⠀", "⠀⠿⠸⠇⠿⠀"],
        DeviceAction::ToggleMobileData => &["⠠⢾⡷⠄⢰⡆", "⠀⠘⠃⠈⠻⠋"],
        DeviceAction::ToggleStayAwake => &["⣴⠚⣭⣭⠓⣦", "⠈⠙⠒⠒⠋⠁"],
        DeviceAction::ToggleDarkMode => &["⢠⣶⡟⠋⠁⠀", "⠈⠛⠷⠦⠄⠀"],
        DeviceAction::ToggleShowTaps => &["⢠⡞⢩⡍⢳⡄", "⠀⠉⠼⠧⠉⠀"],
        DeviceAction::OpenWirelessDebuggingSettings => &["⣐⡦⢿⡿⢴⣂", "⠐⠋⠿⠿⠙⠂"],
        DeviceAction::TakeScreenshot => &["⢰⡞⡻⢟⢳⡆", "⠘⠓⠚⠓⠚⠃"],
        DeviceAction::RebootDevice => &["⣴⠋⠉⠉⢙⣦", "⠙⠦⠤⠤⠭⠀"],
    }
}

fn device_action_icon(action: DeviceAction) -> &'static str {
    match action {
        DeviceAction::WirelessPairAndConnect => "📶",
        DeviceAction::EnableTcpip5555 => "🌐",
        DeviceAction::ToggleWifi => "📡",
        DeviceAction::ToggleMobileData => "📶",
        DeviceAction::ToggleStayAwake => "👁",
        DeviceAction::ToggleDarkMode => "🌓",
        DeviceAction::ToggleShowTaps => "👆",
        DeviceAction::OpenWirelessDebuggingSettings => "⚙",
        DeviceAction::TakeScreenshot => "📸",
        DeviceAction::RebootDevice => "🔄",
    }
}

/// Renders the `[ Device Controls (ADB) ]` panel as a compact card grid.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::DeviceControls;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(txt.devices_view_controls_title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let actions = DeviceAction::all();
    let items: Vec<GridCardItem> = actions
        .iter()
        .map(|action| GridCardItem {
            badge: Some(device_action_icon(*action).to_string()),
            title: localized_device_action_title(*action, app.language).to_string(),
            subtitle: (*action == DeviceAction::WirelessPairAndConnect)
                .then(|| "[w]".to_string()),
            watermark: Some(device_action_watermark(*action)),
        })
        .collect();

    let cols = grid_cols_for_width(inner.width, items.len(), 2);
    let mut hit_map = app.hit_map.borrow_mut();
    hit_map.push(area, ClickTarget::FocusPanel(Panel::DeviceControls));
    render_card_grid_with_hits(
        frame,
        inner,
        &items,
        app.selected_device_control_index,
        is_focused,
        cols,
        theme,
        |idx, rect| hit_map.push(rect, ClickTarget::DeviceControlCard(idx)),
    );
}
