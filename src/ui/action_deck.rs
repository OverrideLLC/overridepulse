use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::android::QuickAction;
use crate::app::{grid_cols_for_width, AppState, Panel};
use crate::ui::grid::{render_card_grid, GridCardItem};

fn concise_action_title(action: QuickAction, app: &AppState) -> String {
    let target = app.project.active_target();
    match action {
        QuickAction::BuildAndInstallDebug => "Build & Install".to_string(),
        QuickAction::BuildAabAndDeploy => "Build AAB".to_string(),
        QuickAction::ClearAppData => "Clear App Data".to_string(),
        QuickAction::ForceStopApp => "Force Stop".to_string(),
        QuickAction::RelaunchActivity => "Relaunch App".to_string(),
        QuickAction::UninstallApp => "Uninstall App".to_string(),
        QuickAction::LaunchScrcpy => "Scrcpy Mirror".to_string(),
        QuickAction::StopGradleDaemons => "Stop Gradle".to_string(),
        _ => action.title_for(target),
    }
}

/// Renders the Actions panel as a responsive card grid.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let is_focused = app.current_panel == Panel::Actions;
    let active_target = app.project.active_target();

    let panel_title = match active_target {
        Some(target)
            if app.project.is_kmp
                || !target.platform.is_android()
                || app.project.targets.len() > 1 =>
        {
            format!(" Actions · {} ", target.display_label())
        }
        _ => " Actions ".to_string(),
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
        .map(|action| GridCardItem {
            badge: action.shortcut().map(|c| format!("[{c}]")),
            title: concise_action_title(*action, app),
            subtitle: None,
        })
        .collect();

    let cols = grid_cols_for_width(inner.width, items.len(), 4);
    render_card_grid(
        frame,
        inner,
        &items,
        app.selected_action_index,
        is_focused,
        cols,
        theme,
    );
}
