use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

/// Renders the help overlay modal with rounded borders and comprehensive keybindings for v0.2.0.
pub fn render(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(68, 80, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Help & Keybindings (OverridePulse v0.2.0) ")
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
        Line::from(vec![Span::styled("Navigation & Layout", title_style)]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab   ", key_style),
            Span::raw("Cycle panels (Devices → Device Controls → Actions → Console)"),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / ↓ or k / j    ", key_style),
            Span::raw("Navigate active list or scroll console logs"),
        ]),
        Line::from(vec![
            Span::styled("  1 / 2 / 3         ", key_style),
            Span::raw("Toggle Simplified (1), Raw Build (2), or Live Logcat (3) pane"),
        ]),
        Line::from(vec![
            Span::styled("  m / l             ", key_style),
            Span::raw("Cycle console presets (Simplified / Build / Logcat / Parity / Triple)"),
        ]),
        Line::from(vec![
            Span::styled("  w                 ", key_style),
            Span::raw("Open ADB Wireless Pair & Connect modal"),
        ]),
        Line::from(vec![
            Span::styled("  v                 ", key_style),
            Span::raw("Cycle active build variant (debug/release/...)"),
        ]),
        Line::from(vec![
            Span::styled("  p                 ", key_style),
            Span::raw("Open Android project selector modal"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled("Actions & ADB Device Controls", title_style)]),
        Line::from(vec![
            Span::styled("  Enter / Space     ", key_style),
            Span::raw("Execute highlighted Action or Device Control / Select device"),
        ]),
        Line::from(vec![
            Span::styled("  b                 ", key_style),
            Span::raw("Build and install debug APK (auto-launches main activity)"),
        ]),
        Line::from(vec![
            Span::styled("  r                 ", key_style),
            Span::raw("Relaunch main launcher activity"),
        ]),
        Line::from(vec![
            Span::styled("  k                 ", key_style),
            Span::raw("Force stop app process"),
        ]),
        Line::from(vec![
            Span::styled("  u                 ", key_style),
            Span::raw("Uninstall app package from target device"),
        ]),
        Line::from(vec![
            Span::styled("  g                 ", key_style),
            Span::raw("Stop all running Gradle daemons (`./gradlew --stop`)"),
        ]),
        Line::from(vec![
            Span::styled("  Device Controls   ", Style::default().fg(Theme::TEXT_MUTED)),
            Span::raw("Wi-Fi, Mobile Data, Stay Awake, Dark Theme, Taps, Settings, Screenshot"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled("Search & Process Control", title_style)]),
        Line::from(vec![
            Span::styled("  /                 ", key_style),
            Span::raw("Search / filter logs in console"),
        ]),
        Line::from(vec![
            Span::styled("  Esc / Ctrl+X      ", key_style),
            Span::raw("Dismiss active modal OR abort running task / process"),
        ]),
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
