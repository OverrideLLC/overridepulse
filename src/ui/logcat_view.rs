use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::android::{LogEntry, LogLevel};
use crate::app::{AppState, Panel};
use crate::ui::theme::ThemePalette;

/// Renders the `[ Live Logcat ]` stream viewer with rounded borders,
/// severity color coding, application filtering, and search highlights.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let is_focused = app.current_panel == Panel::Logcat || app.current_panel == Panel::Console;

    let app_filter_tag = if app.logcat_filter_by_app {
        " [App]"
    } else {
        " [All]"
    };

    let search_tag = if !app.search_query.is_empty() {
        format!(" [/{}]", app.search_query)
    } else {
        String::new()
    };

    let follow_tag = if app.auto_scroll_logcat {
        ""
    } else {
        " [Paused]"
    };

    let title = format!(
        " [3] Logcat ({}){}{}{} │ [f] Scope [y] Copy ",
        app.logcat_entries.len(),
        app_filter_tag,
        search_tag,
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
    let visible_height = inner.height as usize;

    // The stream is already scoped to the app UID/PID. A second filter that
    // requires the package name inside each line hides ordinary app logs
    // (`D MyApp: onCreate` does not contain `com.example.app`).
    let filtered_entries: Vec<&LogEntry> = app
        .logcat_entries
        .iter()
        .filter(|e| e.matches_filter(&app.search_query, None, None, None))
        .collect();

    let total_lines = filtered_entries.len();
    let start_idx = if app.auto_scroll_logcat {
        total_lines.saturating_sub(visible_height)
    } else {
        app.logcat_scroll
            .min(total_lines.saturating_sub(visible_height))
    };

    let lines: Vec<Line> = filtered_entries
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|entry| format_logcat_line(entry, theme))
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
fn format_logcat_line(entry: &LogEntry, theme: ThemePalette) -> Line<'_> {
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
        spans.push(Span::styled(
            format!("{ts} "),
            Style::default().fg(theme.text_muted),
        ));
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
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Message
    spans.push(Span::styled(&entry.message, level_style));

    Line::from(spans)
}
