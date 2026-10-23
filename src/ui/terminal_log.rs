use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::{AppState, Panel};
use crate::ui::theme::Theme;

/// Renders the `[ Build & Process Output ]` panel with rounded borders,
/// dynamic Gradle progress bar, and syntax highlighting.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Output;

    let auto_scroll_tag = if app.auto_scroll_console {
        " [Follow: ON]"
    } else {
        " [Follow: OFF]"
    };

    let title = format!(
        " Build & Process Output ({}){} ",
        app.console_output.len(),
        auto_scroll_tag
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let show_progress = app.process_state.is_running || app.process_state.progress_percent.is_some();

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
                    .fg(Theme::ACCENT)
                    .bg(Color::Rgb(30, 36, 48)),
            )
            .percent(pct as u16)
            .label(Span::styled(
                label,
                Style::default()
                    .fg(Theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ));
        frame.render_widget(gauge, p_area);
    }

    let visible_height = log_area.height as usize;
    let total_lines = app.console_output.len();

    let start_idx = if app.auto_scroll_console {
        total_lines.saturating_sub(visible_height)
    } else {
        app.console_scroll.min(total_lines.saturating_sub(visible_height))
    };

    let lines: Vec<Line> = app
        .console_output
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|line_text| highlight_line(line_text))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, log_area);
}

fn highlight_line(line: &str) -> Line<'_> {
    let trimmed = line.trim_start();

    let style = if line.contains("BUILD SUCCESSFUL")
        || line.contains("SUCCESS")
        || line.starts_with("Success:")
        || line.contains("Successfully installed")
    {
        Style::default().fg(Theme::SUCCESS).add_modifier(Modifier::BOLD)
    } else if line.contains("BUILD FAILED")
        || line.contains("FAILURE:")
        || line.contains("ERROR:")
        || line.starts_with("Failed:")
        || line.starts_with("Error:")
    {
        Style::default().fg(Theme::ERROR).add_modifier(Modifier::BOLD)
    } else if line.contains("WARNING:") || line.starts_with("Warning:") {
        Style::default().fg(Theme::WARNING)
    } else if trimmed.starts_with("> Task :") || trimmed.starts_with("Task :") {
        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
    } else if trimmed.starts_with('>') || trimmed.starts_with("Executing") {
        Style::default().fg(Theme::ACCENT)
    } else {
        Style::default().fg(Theme::TEXT_PRIMARY)
    };

    Line::from(vec![Span::styled(line, style)])
}
