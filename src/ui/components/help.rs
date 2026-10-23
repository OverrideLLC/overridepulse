use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

/// Renders the help overlay modal with rounded borders.
pub fn render(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(65, 70, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Help & Keybindings ")
        .title_style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Theme::ACCENT))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let key_style = Style::default().fg(Theme::WARNING);
    let title_style = Style::default()
        .fg(Theme::ACCENT)
        .add_modifier(Modifier::BOLD);

    let help_text = vec![
        Line::from(vec![Span::styled("Navigation & Focus", title_style)]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab   ", key_style),
            Span::raw("Cycle between focused panels"),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ or k / j    ", key_style),
            Span::raw("Navigate items or scroll logs"),
        ]),
        Line::from(vec![
            Span::styled("  v                 ", key_style),
            Span::raw("Cycle active build variant (debug/release/...)"),
        ]),
        Line::from(vec![
            Span::styled("  l                 ", key_style),
            Span::raw("Toggle between Build Output & Live Logcat"),
        ]),
        Line::from(vec![
            Span::styled("  p                 ", key_style),
            Span::raw("Open Android project selector"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled("Quick Actions", title_style)]),
        Line::from(vec![
            Span::styled("  Enter / Space     ", key_style),
            Span::raw("Execute highlighted action or select device"),
        ]),
        Line::from(vec![
            Span::styled("  b                 ", key_style),
            Span::raw("Build and install debug APK"),
        ]),
        Line::from(vec![
            Span::styled("  a                 ", key_style),
            Span::raw("Build AAB and deploy via Bundletool"),
        ]),
        Line::from(vec![
            Span::styled("  k                 ", key_style),
            Span::raw("Force stop application process"),
        ]),
        Line::from(vec![
            Span::styled("  r                 ", key_style),
            Span::raw("Relaunch launcher activity"),
        ]),
        Line::from(vec![
            Span::styled("  u                 ", key_style),
            Span::raw("Uninstall app package from device"),
        ]),
        Line::from(vec![
            Span::styled("  s                 ", key_style),
            Span::raw("Launch Scrcpy display mirror session"),
        ]),
        Line::from(vec![
            Span::styled("  g                 ", key_style),
            Span::raw("Stop all running Gradle daemons"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled("Filtering & Search", title_style)]),
        Line::from(vec![
            Span::styled("  /                 ", key_style),
            Span::raw("Filter Logcat or search logs"),
        ]),
        Line::from(vec![
            Span::styled("  c                 ", key_style),
            Span::raw("Clear console / logcat buffer"),
        ]),
        Line::from(vec![
            Span::styled("  Esc               ", key_style),
            Span::raw("Dismiss popup or clear filter"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled("Application", title_style)]),
        Line::from(vec![
            Span::styled("  ?                 ", key_style),
            Span::raw("Toggle this help popup"),
        ]),
        Line::from(vec![
            Span::styled("  q / Ctrl+C        ", key_style),
            Span::raw("Quit OverridePulse"),
        ]),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
