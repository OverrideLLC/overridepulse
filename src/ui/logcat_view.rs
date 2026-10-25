use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::android::{LogEntry, LogLevel};
use crate::app::{AppState, Panel};
use crate::ui::simplified_view::console_parity_bar;
use crate::ui::theme::Theme;

/// Renders the `[ Live Logcat ]` stream viewer with rounded borders,
/// PRD-defined severity color coding, application filtering, search highlights, and parity indicator.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.current_panel == Panel::Logcat || app.current_panel == Panel::Console;

    let app_filter_tag = if app.logcat_filter_by_app {
        if let Some(app_id) = &app.project.application_id {
            format!(" [App: {app_id} (ON)]")
        } else {
            " [App: ON]".to_string()
        }
    } else {
        " [App: ALL]".to_string()
    };

    let search_tag = if !app.search_query.is_empty() {
        format!(" [Filter: \"{}\"]", app.search_query)
    } else {
        String::new()
    };

    let follow_tag = if app.auto_scroll_logcat {
        " [Follow: ON]"
    } else {
        " [Follow: OFF]"
    };

    let parity = console_parity_bar(app);
    let title = format!(
        " [3] Logcat ({}){}{}{} │ {parity} ",
        app.logcat_entries.len(),
        app_filter_tag,
        search_tag,
        follow_tag
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(Theme::title_style(is_focused))
        .border_style(Theme::border_style(is_focused));

    let inner = block.inner(area);
    let visible_height = inner.height as usize;

    // Filter entries by app ID and search query if enabled
    let app_id_filter = if app.logcat_filter_by_app {
        app.project.application_id.as_deref()
    } else {
        None
    };

    let filtered_entries: Vec<&LogEntry> = app
        .logcat_entries
        .iter()
        .filter(|e| e.matches_filter(&app.search_query, None, None, app_id_filter))
        .collect();

    let total_lines = filtered_entries.len();
    let start_idx = if app.auto_scroll_logcat {
        total_lines.saturating_sub(visible_height)
    } else {
        app.logcat_scroll.min(total_lines.saturating_sub(visible_height))
    };

    let lines: Vec<Line> = filtered_entries
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|entry| format_logcat_line(entry))
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Formats a parsed `LogEntry` into styled TUI Spans according to PRD Section 5:
/// - `V` (Verbose): Gris
/// - `D` (Debug): Cian
/// - `I` (Info): Verde
/// - `W` (Warn): Amarillo
/// - `E`/`F` (Error/Fatal): Rojo
fn format_logcat_line(entry: &LogEntry) -> Line<'_> {
    let (level_color, is_bold) = match entry.level {
        LogLevel::Verbose => (Color::DarkGray, false),
        LogLevel::Debug => (Color::Cyan, false),
        LogLevel::Info => (Color::Green, false),
        LogLevel::Warn => (Color::Yellow, true),
        LogLevel::Error | LogLevel::Fatal => (Color::Red, true),
        LogLevel::Unknown => (Color::White, false),
    };

    let mut level_style = Style::default().fg(level_color);
    if is_bold {
        level_style = level_style.add_modifier(Modifier::BOLD);
    }

    let mut spans = Vec::new();

    // Timestamp
    if let Some(ts) = &entry.timestamp {
        spans.push(Span::styled(format!("{ts} "), Style::default().fg(Theme::TEXT_MUTED)));
    }

    // PID
    if let Some(pid) = entry.pid {
        spans.push(Span::styled(
            format!("{pid:>5} "),
            Style::default().fg(Color::Rgb(100, 110, 125)),
        ));
    }

    // Level badge
    let level_char = entry.level.as_char();
    spans.push(Span::styled(format!("[{level_char}] "), level_style));

    // Tag
    if !entry.tag.is_empty() {
        spans.push(Span::styled(
            format!("{}: ", entry.tag),
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Message
    spans.push(Span::styled(&entry.message, level_style));

    Line::from(spans)
}
