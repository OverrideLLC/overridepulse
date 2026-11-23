use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::ThemePalette;

/// Renders the help overlay modal with rounded borders and comprehensive keybindings.
pub fn render(frame: &mut Frame, area: Rect, theme: ThemePalette) {
    let popup_area = centered_rect(68, 82, area);

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Help & Keybindings ")
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let key_style = Style::default().fg(theme.warning);
    let title_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let help_text = vec![
        Line::from(vec![Span::styled("Navigation & Layout", title_style)]),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab   ", key_style),
            Span::raw("Cycle panels (Actions ↔ Console, or Devices ↔ ADB Controls)"),
        ]),
        Line::from(vec![
            Span::styled("  ←/→/↑/↓           ", key_style),
            Span::raw("Navigate action/control grid cards, devices, or scroll logs"),
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
            Span::styled("  d                 ", key_style),
            Span::raw("Open / close separate Devices & ADB Controls view"),
        ]),
        Line::from(vec![
            Span::styled("  w                 ", key_style),
            Span::raw("Open ADB Wireless Pair & Connect modal"),
        ]),
        Line::from(vec![
            Span::styled("  v                 ", key_style),
            Span::raw("Cycle active Android build variant (debug/release/...)"),
        ]),
        Line::from(vec![
            Span::styled("  t                 ", key_style),
            Span::raw("Cycle KMP target (Android / JVM / WasmJS / JS)"),
        ]),
        Line::from(vec![
            Span::styled("  p                 ", key_style),
            Span::raw("Switch project (siblings and nearby Gradle projects)"),
        ]),
        Line::from(vec![
            Span::styled("  T / O             ", key_style),
            Span::raw("Cycle color theme (Shift+T) or reopen onboarding (Shift+O)"),
        ]),
        Line::from(vec![
            Span::styled("  y / Y             ", key_style),
            Span::raw("Copy logcat (y) or build output (Y) to the clipboard"),
        ]),
        Line::from(vec![
            Span::styled("  f                 ", key_style),
            Span::raw("Toggle logcat between this app and all device logs"),
        ]),
        Line::raw(""),
        Line::from(vec![Span::styled(
            "Actions & ADB Device Controls",
            title_style,
        )]),
        Line::from(vec![
            Span::styled("  Enter / Space     ", key_style),
            Span::raw("Execute highlighted Action or Device Control / Select device"),
        ]),
        Line::from(vec![
            Span::styled("  b                 ", key_style),
            Span::raw("Build/install debug APK (Android) or compile active KMP target"),
        ]),
        Line::from(vec![
            Span::styled("  r                 ", key_style),
            Span::raw("Relaunch activity (Android) or run / dev server for KMP target"),
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
            Span::styled(
                "  Device Controls   ",
                Style::default().fg(theme.text_muted),
            ),
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
