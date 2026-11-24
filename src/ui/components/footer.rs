use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::{AppState, ClickTarget, FooterAction};

/// Renders the bottom footer bar with rounded borders.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut info_spans = vec![
        Span::styled("📦 ", Style::default().fg(theme.accent)),
        Span::styled(
            &app.project.name,
            Style::default()
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if app.project_loading {
        info_spans.push(Span::styled(
            " loading…",
            Style::default().fg(theme.warning),
        ));
    } else if let Some(target) = app.project.active_target() {
        info_spans.push(Span::styled(" │ ", Style::default().fg(theme.text_muted)));
        info_spans.push(Span::styled(
            format!("{} {}", target.platform.icon(), target.display_label()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.project.active_platform().is_android() {
        info_spans.push(Span::styled(
            format!(" ({}) ", app.project.active_variant),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        info_spans.push(Span::raw(" "));
    }

    let project_info_line = Line::from(info_spans);
    let info_width = project_info_line.width() as u16;
    let right_width = info_width.min(inner.width / 2);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),              // Shortcuts (always gets remaining width)
            Constraint::Length(right_width), // Project & Target
        ])
        .split(inner);

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let sep_style = Style::default().fg(theme.text_muted);

    let build_items = || -> Vec<(&'static str, &'static str, Option<FooterAction>)> {
        if app.settings_view_active {
            vec![
                ("[s/Esc] ", "⬅", Some(FooterAction::Back)),
                ("[↑/↓] ", "↕", None),
                ("[←/→] ", "⇄", Some(FooterAction::SwitchPane)),
                ("[Enter] ", "✓", Some(FooterAction::SelectOrApply)),
                ("[q] ", "⏻", Some(FooterAction::Quit)),
            ]
        } else if app.devices_view_active {
            vec![
                ("[d/Esc] ", "⬅", Some(FooterAction::Back)),
                ("[s] ", "⚙", Some(FooterAction::Settings)),
                ("[Tab] ", "⇄", Some(FooterAction::SwitchPane)),
                ("[w] ", "📶", Some(FooterAction::WifiAdb)),
                ("[Enter] ", "▶", Some(FooterAction::SelectOrApply)),
                ("[q] ", "⏻", Some(FooterAction::Quit)),
            ]
        } else {
            vec![
                ("[s] ", "⚙", Some(FooterAction::Settings)),
                ("[d] ", "📱", Some(FooterAction::Devices)),
                ("[p] ", "📂", Some(FooterAction::Project)),
                ("[t] ", "🎯", Some(FooterAction::Target)),
                ("[1-3] ", "◫", Some(FooterAction::CycleViews)),
                ("[?] ", "❓", Some(FooterAction::Help)),
                ("[q] ", "⏻", Some(FooterAction::Quit)),
            ]
        }
    };

    let items = build_items();
    let render_line = |sep: &'static str| -> (Line<'static>, Vec<(u16, u16, FooterAction)>) {
        let mut spans = vec![Span::raw(" ")];
        let mut x_cursor = 1u16;
        let mut hits = Vec::new();
        let sep_w = sep.chars().count() as u16;

        for (i, (key, label, action)) in items.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(sep, sep_style));
                x_cursor += sep_w;
            }
            let start_x = x_cursor;
            let key_w = key.chars().count() as u16;
            let label_w = label.chars().count() as u16;
            spans.push(Span::styled(*key, key_style));
            spans.push(Span::styled(*label, desc_style));
            x_cursor += key_w + label_w;
            if let Some(act) = action {
                hits.push((start_x, x_cursor - start_x, *act));
            }
        }
        (Line::from(spans), hits)
    };

    let (wide_line, wide_hits) = render_line("  ");
    let (shortcuts_line, hits) = if wide_line.width() <= chunks[0].width as usize {
        (wide_line, wide_hits)
    } else {
        render_line(" ")
    };

    {
        let mut hit_map = app.hit_map.borrow_mut();
        for (rel_x, w, action) in hits {
            let abs_x = chunks[0].x.saturating_add(rel_x);
            if abs_x < chunks[0].right() {
                let clamped_w = w.min(chunks[0].right() - abs_x);
                hit_map.push(
                    Rect {
                        x: abs_x,
                        y: chunks[0].y,
                        width: clamped_w,
                        height: chunks[0].height,
                    },
                    ClickTarget::Footer(action),
                );
            }
        }
    }

    frame.render_widget(Paragraph::new(shortcuts_line), chunks[0]);
    frame.render_widget(
        Paragraph::new(project_info_line).alignment(Alignment::Right),
        chunks[1],
    );
}
