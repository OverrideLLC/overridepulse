use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::android::{LogEntry, LogLevel};
use crate::app::{AppState, ClickTarget, Panel};
use crate::i18n::{ui_text, Language};
use crate::ui::theme::ThemePalette;
use crate::ui::{render_pane_toolbar, render_vertical_scrollbar, ToolbarButton};

/// Renders the `[ Live Logcat ]` stream viewer with rounded borders,
/// interactive clickable toolbar, vertical scrollbar, severity color coding,
/// application filtering, and search highlights.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let txt = ui_text(app.language);
    let is_focused = app.current_panel == Panel::Logcat
        || (app.current_panel == Panel::Console && !app.show_simplified_pane && !app.show_build_pane);

    let app_filter_tag = if app.logcat_filter_by_app {
        " 📦"
    } else {
        " 🌐"
    };

    let search_tag = if !app.search_query.is_empty() {
        format!(" 🔍{}", app.search_query)
    } else {
        String::new()
    };

    let follow_tag = if app.auto_scroll_logcat { "" } else { " ⏸" };

    let title = format!(
        " 📜 [3] {} ({}){}{}{} ",
        txt.logcat_tab_title,
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
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let show_toolbar = inner.height >= 2;
    let (toolbar_area, log_area) = if show_toolbar {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, inner)
    };

    let visible_height = log_area.height as usize;
    app.logcat_visible_height.set(visible_height.max(1));

    // The stream is already scoped to the app UID/PID. A second filter that
    // requires the package name inside each line hides ordinary app logs
    // (`D MyApp: onCreate` does not contain `com.example.app`).
    let query = app.search_query.trim();
    let (total_lines, start_idx, lines): (usize, usize, Vec<Line>) = if query.is_empty() {
        let total = app.logcat_entries.len();
        let start = if app.auto_scroll_logcat {
            total.saturating_sub(visible_height)
        } else {
            app.logcat_scroll.min(total.saturating_sub(visible_height))
        };

        let visible_lines = app
            .logcat_entries
            .iter()
            .skip(start)
            .take(visible_height)
            .map(|entry| format_logcat_line(entry, theme))
            .collect();
        (total, start, visible_lines)
    } else {
        let filtered_entries: Vec<&LogEntry> = app
            .logcat_entries
            .iter()
            .filter(|e| e.matches_filter(query, None, None, None))
            .collect();

        let total = filtered_entries.len();
        let start = if app.auto_scroll_logcat {
            total.saturating_sub(visible_height)
        } else {
            app.logcat_scroll.min(total.saturating_sub(visible_height))
        };

        let visible_lines = filtered_entries
            .into_iter()
            .skip(start)
            .take(visible_height)
            .map(|entry| format_logcat_line(entry, theme))
            .collect();
        (total, start, visible_lines)
    };

    if let Some(tb_area) = toolbar_area {
        let compact = tb_area.width < 44;
        let is_maximized =
            !app.show_simplified_pane && !app.show_build_pane && app.show_logcat_pane;
        let scope_lbl = match (app.language, compact, app.logcat_filter_by_app) {
            (_, true, true) => "📦",
            (_, true, false) => "🌐",
            (_, false, true) => "📦 App",
            (Language::En, false, false) => "🌐 All",
            (Language::Es, false, false) => "🌐 Todo",
        };
        let follow_lbl = if app.auto_scroll_logcat { "⬇" } else { "⏸" };
        let max_lbl = if is_maximized { "⤡" } else { "⤢" };
        let scope_color = if app.logcat_filter_by_app {
            theme.accent
        } else {
            theme.warning
        };
        let follow_color = if app.auto_scroll_logcat {
            theme.success
        } else {
            theme.warning
        };
        let end_line = (start_idx + visible_height).min(total_lines);
        let pos_info =
            (total_lines > 0).then(|| format!("{}-{}/{}", start_idx + 1, end_line, total_lines));

        let mut buttons = vec![
            ToolbarButton {
                label: "📋",
                fg: theme.accent,
                target: ClickTarget::CopyLogcat,
            },
            ToolbarButton {
                label: scope_lbl,
                fg: scope_color,
                target: ClickTarget::ToggleLogcatScope,
            },
            ToolbarButton {
                label: "🔍",
                fg: if query.is_empty() {
                    theme.text_primary
                } else {
                    theme.warning
                },
                target: ClickTarget::OpenLogcatSearch,
            },
        ];
        if !query.is_empty() {
            buttons.push(ToolbarButton {
                label: "✕",
                fg: theme.error,
                target: ClickTarget::ClearLogcatSearch,
            });
        }
        buttons.push(ToolbarButton {
            label: follow_lbl,
            fg: follow_color,
            target: ClickTarget::ToggleLogcatFollow,
        });
        buttons.push(ToolbarButton {
            label: "🗑",
            fg: theme.text_muted,
            target: ClickTarget::ClearLogcat,
        });
        buttons.push(ToolbarButton {
            label: max_lbl,
            fg: theme.text_primary,
            target: ClickTarget::ToggleMaximizeLogcat,
        });

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
