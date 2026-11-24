use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::{AppState, ClickTarget, Panel};
use crate::i18n::ui_text;
use crate::ui::theme::ThemePalette;
use crate::ui::{render_pane_toolbar, render_vertical_scrollbar, ToolbarButton};

/// Renders the `[ Build & Process Output ]` panel with rounded borders,
/// interactive clickable toolbar, vertical scrollbar, dynamic Gradle progress bar,
/// and syntax highlighting.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::Output
        || (app.current_panel == Panel::Console && !app.show_simplified_pane);

    let follow_tag = if app.auto_scroll_console { "" } else { " ⏸" };

    let title = format!(
        " 🔨 [2] {} ({}){} ",
        txt.build_tab_title,
        app.console_output.len(),
        follow_tag
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let show_toolbar = inner.height >= 2;
    let show_progress =
        app.process_state.is_running || app.process_state.progress_percent.is_some();

    let (toolbar_area, rest_area) = if show_toolbar {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, inner)
    };

    let (progress_area, log_area) = if show_progress && rest_area.height >= 2 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(rest_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, rest_area)
    };

    // Render progress bar if a task is running or reported percentage
    if let Some(p_area) = progress_area {
        let pct = app.process_state.progress_percent.unwrap_or(0);
        let task_name = app
            .process_state
            .task_description
            .as_deref()
            .unwrap_or("Executing task...");

        let label = if let Some(p) = app.process_state.progress_percent {
            format!("{task_name} ── {p}%")
        } else {
            format!("{} {}", app.process_state.spinner_char(), task_name)
        };

        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.bg_card),
            )
            .percent(pct as u16)
            .label(Span::styled(
                label,
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, p_area);
    }

    let visible_height = log_area.height as usize;
    app.console_visible_height.set(visible_height.max(1));
    let total_lines = app.console_output.len();

    let start_idx = if app.auto_scroll_console {
        total_lines.saturating_sub(visible_height)
    } else {
        app.console_scroll
            .min(total_lines.saturating_sub(visible_height))
    };

    if let Some(tb_area) = toolbar_area {
        let is_maximized =
            !app.show_simplified_pane && app.show_build_pane && !app.show_logcat_pane;
        let follow_lbl = if app.auto_scroll_console { "⬇" } else { "⏸" };
        let max_lbl = if is_maximized { "⤡" } else { "⤢" };
        let follow_color = if app.auto_scroll_console {
            theme.success
        } else {
            theme.warning
        };
        let end_line = (start_idx + visible_height).min(total_lines);
        let pos_info = (total_lines > 0).then(|| format!("{}-{}/{}", start_idx + 1, end_line, total_lines));

        let buttons = [
            ToolbarButton {
                label: "📋",
                fg: theme.accent,
                target: ClickTarget::CopyBuild,
            },
            ToolbarButton {
                label: follow_lbl,
                fg: follow_color,
                target: ClickTarget::ToggleBuildFollow,
            },
            ToolbarButton {
                label: "🗑",
                fg: theme.text_muted,
                target: ClickTarget::ClearBuild,
            },
            ToolbarButton {
                label: max_lbl,
                fg: theme.text_primary,
                target: ClickTarget::ToggleMaximizeBuild,
            },
        ];
        render_pane_toolbar(frame, app, tb_area, &buttons, pos_info.as_deref());
    }

    let text_area = if total_lines > visible_height && log_area.width > 2 {
        Rect {
            width: log_area.width - 1,
            ..log_area
        }
    } else {
        log_area
    };

    let lines: Vec<Line> = app
        .console_output
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|line_text| highlight_line(line_text, theme))
        .collect();

    frame.render_widget(Paragraph::new(lines), text_area);
    render_vertical_scrollbar(
        frame,
        log_area,
        total_lines,
        visible_height,
        start_idx,
        theme,
    );
}

fn highlight_line(line: &str, theme: ThemePalette) -> Line<'_> {
    let trimmed = line.trim_start();

    let style = if line.contains("BUILD SUCCESSFUL")
        || line.contains("SUCCESS")
        || line.starts_with("Success:")
        || line.contains("Successfully installed")
    {
        Style::default()
            .fg(theme.success)
            .add_modifier(Modifier::BOLD)
    } else if line.contains("BUILD FAILED")
        || line.contains("FAILURE:")
        || line.contains("ERROR:")
        || line.starts_with("Failed:")
        || line.starts_with("Error:")
    {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else if line.contains("WARNING:") || line.starts_with("Warning:") {
        Style::default().fg(theme.warning)
    } else if trimmed.starts_with("> Task :") || trimmed.starts_with("Task :") {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with('>') || trimmed.starts_with("Executing") {
        Style::default().fg(theme.accent)
    } else {
        Style::default().fg(theme.text_primary)
    };

    Line::from(vec![Span::styled(line, style)])
}
