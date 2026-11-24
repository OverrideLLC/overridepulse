use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::android::QuickAction;
use crate::app::{grid_cols_for_width, AppState, ClickTarget, Panel};
use crate::i18n::{localized_action_title, ui_text};
use crate::ui::grid::{render_card_grid_with_hits, GridCardItem};

/// Returns a subtle 2-row Braille-dot watermark sprite tailored to each quick action.
pub fn action_watermark(action: QuickAction) -> &'static [&'static str] {
    match action {
        // APK box + install arrow
        QuickAction::BuildAndInstallDebug => &["⢰⣟⣛⣻⡆⠀", "⠸⣣⣿⣜⠇⠀"],
        // Bundle cube
        QuickAction::BuildAabAndDeploy => &["⢠⣖⣻⣟⣲⡄", "⠘⠧⣼⣧⠼⠃"],
        // Broom / sweep data
        QuickAction::ClearAppData => &["⠀⣀⣀⣴⠋⠀", "⠼⠋⠟⠹⠇⠀"],
        // Octagon stop sign
        QuickAction::ForceStopApp => &["⢠⡞⢍⡩⢳⡄", "⠘⢧⣊⣑⡼⠃"],
        // Play / launch arrow
        QuickAction::RelaunchActivity => &["⠀⢸⣷⣦⣄⠀", "⠀⠸⠟⠋⠁⠀"],
        // Trash bin
        QuickAction::UninstallApp => &["⠐⡶⣿⢿⡶⡂", "⠀⠧⠿⠼⠧⠃"],
        // Phone mirror display
        QuickAction::LaunchScrcpy => &["⠀⣾⢩⡍⣷⠀", "⠀⠻⠶⠶⠟⠀"],
        // Power-off daemon stop
        QuickAction::StopGradleDaemons => &["⣠⠆⠿⠰⣄⠀", "⠙⠦⠤⠴⠋⠀"],
        // Compile gear/anvil
        QuickAction::KmpCompileTarget => &["⠰⢟⣒⡻⠆⠀", "⠰⣮⠭⣵⠆⠀"],
        // Play circle
        QuickAction::KmpRunTarget => &["⢠⡞⣭⣉⢳⡄", "⠈⠳⠭⠤⠞⠁"],
        // Distribution package
        QuickAction::KmpBundleDist => &["⢸⡯⢽⡯⢽⡇", "⠸⠧⠼⠧⠼⠇"],
        // Test checkmark
        QuickAction::KmpTestTarget => &["⢀⡀⢀⣴⠟⠀", "⠈⠻⠟⠁⠀⠀"],
        // Multi-module layers
        QuickAction::GradleBuildAll => &["⠀⣒⣿⣿⣒⠀", "⠈⠓⠾⠷⠚⠁"],
    }
}

fn action_icon(action: QuickAction) -> &'static str {
    match action {
        QuickAction::BuildAndInstallDebug | QuickAction::KmpCompileTarget => "🔨",
        QuickAction::BuildAabAndDeploy | QuickAction::KmpBundleDist => "📦",
        QuickAction::ClearAppData => "🧹",
        QuickAction::ForceStopApp => "🛑",
        QuickAction::RelaunchActivity | QuickAction::KmpRunTarget => "🚀",
        QuickAction::UninstallApp => "🗑",
        QuickAction::LaunchScrcpy => "🪞",
        QuickAction::StopGradleDaemons => "⏻",
        QuickAction::KmpTestTarget => "🧪",
        QuickAction::GradleBuildAll => "🧱",
    }
}

/// Renders the Actions panel as a responsive card grid.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::Actions;
    let active_target = app.project.active_target();

    let panel_title = match active_target {
        Some(target)
            if app.project.is_kmp
                || !target.platform.is_android()
                || app.project.targets.len() > 1 =>
        {
            format!(" {} · {} ", txt.actions_title, target.display_label())
        }
        _ => format!(" {} ", txt.actions_title),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(panel_title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<GridCardItem> = app
        .actions
        .iter()
        .map(|action| {
            let icon = action_icon(*action);
            let badge = match action.shortcut() {
                Some(c) => format!("[{c}] {icon}"),
                None => icon.to_string(),
            };
            GridCardItem {
                badge: Some(badge),
                title: localized_action_title(*action, active_target, app.language),
                subtitle: None,
                watermark: Some(action_watermark(*action)),
            }
        })
        .collect();

    let cols = grid_cols_for_width(inner.width, items.len(), 4);
    let mut hit_map = app.hit_map.borrow_mut();
    hit_map.push(area, ClickTarget::FocusPanel(Panel::Actions));
    render_card_grid_with_hits(
        frame,
        inner,
        &items,
        app.selected_action_index,
        is_focused,
        cols,
        theme,
        |idx, rect| hit_map.push(rect, ClickTarget::ActionCard(idx)),
    );
}
