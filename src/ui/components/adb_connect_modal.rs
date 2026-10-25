use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::Theme;

/// Renders the ADB Wireless Pair & Connect modal dialog with rounded borders,
/// interactive text input fields, and keybinding navigation instructions:
/// `[Tab] Switch Field │ [Enter] Pair & Connect │ [Esc] Cancel`
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let popup_area = centered_rect(62, 55, area);

    // Clear background behind modal popup
    frame.render_widget(Clear, popup_area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 📶 ADB Wireless Pair & Connect ")
        .title_style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Theme::ACCENT))
        .style(Style::default().bg(Theme::BG_DARK));

    let inner = main_block.inner(popup_area);
    frame.render_widget(main_block, popup_area);

    if inner.height < 10 || inner.width < 20 {
        return;
    }

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Subtitle / explanation
            Constraint::Length(3), // Field 1: Connect Address
            Constraint::Length(3), // Field 2: Pairing Address
            Constraint::Length(3), // Field 3: Pairing Code
            Constraint::Min(1),    // Footer keybindings
        ])
        .split(inner);

    let intro_text = Line::from(vec![
        Span::styled(
            "Connect over Wi-Fi (Android 11+ Wireless Debugging or tcpip port):",
            Style::default().fg(Theme::TEXT_MUTED),
        ),
    ]);
    frame.render_widget(Paragraph::new(intro_text), vertical_chunks[0]);

    // Field 1: Connect IP:Port
    let f1_active = app.adb_modal_field_index == 0;
    let f1_border_style = if f1_active {
        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::INACTIVE_BORDER)
    };
    let f1_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 1. Connect Address (IP:Port) ")
        .title_style(f1_border_style)
        .border_style(f1_border_style);

    let f1_val = if app.adb_connect_ip_port.is_empty() {
        if f1_active { "█" } else { "192.168.1.50:5555" }
    } else if f1_active {
        &format!("{}█", app.adb_connect_ip_port)
    } else {
        &app.adb_connect_ip_port
    };
    let f1_para = Paragraph::new(Line::from(vec![
        Span::styled(f1_val, if f1_active { Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_PRIMARY) }),
    ])).block(f1_block);
    frame.render_widget(f1_para, vertical_chunks[1]);

    // Field 2: Pairing Address (Optional)
    let f2_active = app.adb_modal_field_index == 1;
    let f2_border_style = if f2_active {
        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::INACTIVE_BORDER)
    };
    let f2_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 2. Pairing Address (IP:Port) [Optional - Android 11+] ")
        .title_style(f2_border_style)
        .border_style(f2_border_style);

    let f2_val = if app.adb_pair_ip_port.is_empty() {
        if f2_active { "█" } else { "192.168.1.50:37123" }
    } else if f2_active {
        &format!("{}█", app.adb_pair_ip_port)
    } else {
        &app.adb_pair_ip_port
    };
    let f2_para = Paragraph::new(Line::from(vec![
        Span::styled(f2_val, if f2_active { Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_PRIMARY) }),
    ])).block(f2_block);
    frame.render_widget(f2_para, vertical_chunks[2]);

    // Field 3: Pairing Code (Optional)
    let f3_active = app.adb_modal_field_index == 2;
    let f3_border_style = if f3_active {
        Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Theme::INACTIVE_BORDER)
    };
    let f3_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" 3. 6-Digit Pairing Code [Optional] ")
        .title_style(f3_border_style)
        .border_style(f3_border_style);

    let f3_val = if app.adb_pair_code.is_empty() {
        if f3_active { "█" } else { "e.g. 123456" }
    } else if f3_active {
        &format!("{}█", app.adb_pair_code)
    } else {
        &app.adb_pair_code
    };
    let f3_para = Paragraph::new(Line::from(vec![
        Span::styled(f3_val, if f3_active { Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD) } else { Style::default().fg(Theme::TEXT_PRIMARY) }),
    ])).block(f3_block);
    frame.render_widget(f3_para, vertical_chunks[3]);

    // Instructions footer
    let key_style = Style::default()
        .fg(Theme::WARNING)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Theme::TEXT_PRIMARY);
    let sep_style = Style::default().fg(Theme::TEXT_MUTED);

    let instructions = Line::from(vec![
        Span::styled("[Tab] ", key_style),
        Span::styled("Switch Field", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[Enter] ", key_style),
        Span::styled("Pair & Connect", desc_style),
        Span::styled(" │ ", sep_style),
        Span::styled("[Esc] ", key_style),
        Span::styled("Cancel", desc_style),
    ]);
    frame.render_widget(
        Paragraph::new(instructions).alignment(Alignment::Center),
        vertical_chunks[4],
    );
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
