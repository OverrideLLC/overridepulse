use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::ThemePreset;

/// Renders the first-run onboarding modal:
/// - Step 0: What OverridePulse is and how to use it
/// - Step 1: Live theme selector
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let popup_area = centered_rect(76, 76, area);
    frame.render_widget(Clear, popup_area);

    let title = if app.onboarding_step == 0 {
        " Welcome to OverridePulse ⚡ (1/2) "
    } else {
        " Choose Your Theme 🎨 (2/2) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if inner.height < 10 || inner.width < 30 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(2)])
        .split(inner);

    let content_area = chunks[0];
    let footer_area = chunks[1];

    if app.onboarding_step == 0 {
        render_welcome_step(frame, app, content_area);
    } else {
        render_theme_step(frame, app, content_area);
    }

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let sep_style = Style::default().fg(theme.text_muted);

    let footer_line = if app.onboarding_step == 0 {
        Line::from(vec![
            Span::styled("[Enter / →] ", key_style),
            Span::styled("Choose Theme", desc_style),
            Span::styled("   │   ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Skip", desc_style),
        ])
    } else {
        Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Preview", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled("Save & Start", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[←] ", key_style),
            Span::styled("Back", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Skip", desc_style),
        ])
    };

    frame.render_widget(
        Paragraph::new(footer_line).alignment(Alignment::Center),
        footer_area,
    );
}

fn render_welcome_step(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let heading = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let key = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let body = Style::default().fg(theme.text_primary);
    let muted = Style::default().fg(theme.text_muted);

    let lines = vec![
        Line::from(vec![Span::styled(
            "What is OverridePulse?",
            heading,
        )]),
        Line::from(vec![Span::styled(
            "  An ultralightweight TUI for Android, Kotlin Multiplatform (KMP), and Gradle.",
            body,
        )]),
        Line::from(vec![Span::styled(
            "  Compile, run dev servers, inspect logs, and manage devices from one terminal.",
            muted,
        )]),
        Line::raw(""),
        Line::from(vec![Span::styled("How to use it", heading)]),
        Line::from(vec![
            Span::styled("  1. Actions Grid     ", key),
            Span::styled(
                "[b] build/compile · [r] run/dev server · arrows + [Enter] for dist & tests",
                body,
            ),
        ]),
        Line::from(vec![
            Span::styled("  2. KMP Targets      ", key),
            Span::styled(
                "[t] switches Android, JVM/Desktop, WasmJS, and JS (no ADB needed for host)",
                body,
            ),
        ]),
        Line::from(vec![
            Span::styled("  3. Devices & Views  ", key),
            Span::styled(
                "[d] opens Devices & ADB Controls · [1/2/3] toggle Status / Build / Logcat",
                body,
            ),
        ]),
        Line::from(vec![
            Span::styled("  4. Projects & Help  ", key),
            Span::styled(
                "[p] switch project · [Shift+T] cycle theme anytime · [?] full shortcuts",
                body,
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_theme_step(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(area);

    let intro = Paragraph::new(Line::from(vec![Span::styled(
        "Select a color palette (live preview updates immediately; saved to ~/.config/overridepulse):",
        Style::default().fg(theme.text_muted),
    )]));
    frame.render_widget(intro, chunks[0]);

    let items: Vec<ListItem> = ThemePreset::all()
        .iter()
        .enumerate()
        .map(|(idx, preset)| {
            let is_selected = idx == app.selected_theme_index;
            let p = preset.palette();
            let cursor = if is_selected { "▶ ● " } else { "  ○ " };

            let name_style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };

            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled("██", Style::default().fg(p.accent)),
                Span::styled("██", Style::default().fg(p.success)),
                Span::styled("██", Style::default().fg(p.warning)),
                Span::styled("██ ", Style::default().fg(p.error)),
                Span::styled(format!("{:<18}", preset.name()), name_style),
                Span::styled(preset.tagline(), Style::default().fg(theme.text_muted)),
            ]);

            let item = ListItem::new(line);
            if is_selected {
                item.style(theme.selected_style())
            } else {
                item
            }
        })
        .collect();

    frame.render_widget(List::new(items), chunks[1]);
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
