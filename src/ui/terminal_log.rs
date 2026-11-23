use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::ui::theme::ThemePalette;

/// Renders the `[ Build & Process Output ]` panel with rounded borders,
/// dynamic Gradle progress bar, and syntax highlighting.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let is_focused = app.current_panel == Panel::Output || app.current_panel == Panel::Console;

    let follow_tag = if app.auto_scroll_console {
        ""
    } else {
        " [Paused]"
    };

    let title = format!(
        " [2] Build ({}){} │ [Y] Copy ",
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

    let show_progress =
        app.process_state.is_running || app.process_state.progress_percent.is_some();

    let (progress_area, log_area) = if show_progress && inner.height >= 3 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, inner)
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
    let total_lines = app.console_output.len();

    let start_idx = if app.auto_scroll_console {
        total_lines.saturating_sub(visible_height)
    } else {
        app.console_scroll
            .min(total_lines.saturating_sub(visible_height))
    };

    let lines: Vec<Line> = app
        .console_output
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|line_text| highlight_line(line_text, theme))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, log_area);
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
