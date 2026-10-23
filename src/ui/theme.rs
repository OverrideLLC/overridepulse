use ratatui::style::{Color, Modifier, Style};

pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Rgb(79, 195, 247); // Android cyan/blue
    pub const SUCCESS: Color = Color::Rgb(102, 187, 106); // Green
    pub const WARNING: Color = Color::Rgb(255, 183, 77); // Orange
    pub const ERROR: Color = Color::Rgb(239, 83, 80); // Red
    pub const TEXT_PRIMARY: Color = Color::Rgb(240, 240, 240);
    pub const TEXT_MUTED: Color = Color::Rgb(130, 130, 130);
    pub const BG_DARK: Color = Color::Rgb(20, 24, 30);
    pub const ACTIVE_BORDER: Color = Color::Rgb(79, 195, 247);
    pub const INACTIVE_BORDER: Color = Color::Rgb(60, 65, 75);

    pub fn title_style(active: bool) -> Style {
        if active {
            Style::default()
                .fg(Self::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Self::TEXT_MUTED)
        }
    }

    pub fn border_style(active: bool) -> Style {
        if active {
            Style::default().fg(Self::ACTIVE_BORDER)
        } else {
            Style::default().fg(Self::INACTIVE_BORDER)
        }
    }

    pub fn selected_style() -> Style {
        Style::default()
            .bg(Color::Rgb(40, 50, 70))
            .fg(Self::ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    pub fn log_level_style(level: crate::android::LogLevel) -> Style {
        use crate::android::LogLevel;
        match level {
            LogLevel::Verbose => Style::default().fg(Color::DarkGray),
            LogLevel::Debug => Style::default().fg(Color::Cyan),
            LogLevel::Info => Style::default().fg(Color::Green),
            LogLevel::Warn => Style::default().fg(Color::Yellow),
            LogLevel::Error | LogLevel::Fatal => {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            }
            LogLevel::Unknown => Style::default().fg(Color::White),
        }
    }
}
