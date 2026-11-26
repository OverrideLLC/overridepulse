use ratatui::style::{Color, Modifier, Style};

/// Built-in color theme presets selectable during onboarding or via `[T]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    #[default]
    OverrideCyan,
    TokyoNight,
    CatppuccinMocha,
    Dracula,
    GruvboxDark,
    EmeraldMatrix,
    DeepBlack,
    NordFrost,
    RosePine,
    MonokaiPro,
}

impl ThemePreset {
    pub fn all() -> &'static [ThemePreset] {
        &[
            Self::OverrideCyan,
            Self::TokyoNight,
            Self::CatppuccinMocha,
            Self::Dracula,
            Self::GruvboxDark,
            Self::EmeraldMatrix,
            Self::DeepBlack,
            Self::NordFrost,
            Self::RosePine,
            Self::MonokaiPro,
        ]
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::OverrideCyan => "override-cyan",
            Self::TokyoNight => "tokyo-night",
            Self::CatppuccinMocha => "catppuccin",
            Self::Dracula => "dracula",
            Self::GruvboxDark => "gruvbox",
            Self::EmeraldMatrix => "emerald",
            Self::DeepBlack => "deep-black",
            Self::NordFrost => "nord",
            Self::RosePine => "rose-pine",
            Self::MonokaiPro => "monokai-pro",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::OverrideCyan => "Override Cyan",
            Self::TokyoNight => "Tokyo Night",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::Dracula => "Dracula",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::EmeraldMatrix => "Emerald Matrix",
            Self::DeepBlack => "Deep Black",
            Self::NordFrost => "Nord Frost",
            Self::RosePine => "Rosé Pine",
            Self::MonokaiPro => "Monokai Pro",
        }
    }

    pub fn tagline(&self) -> &'static str {
        match self {
            Self::OverrideCyan => "Crisp Android cyan & dark slate (Default)",
            Self::TokyoNight => "Deep indigo with neon sapphire & amber",
            Self::CatppuccinMocha => "Warm mocha surface with pastel lavender",
            Self::Dracula => "Classic purple, pink & vibrant cyan",
            Self::GruvboxDark => "Retro warm espresso with aqua & gold",
            Self::EmeraldMatrix => "Minimal terminal emerald & mint",
            Self::DeepBlack => "Pure OLED pitch black with crisp cyan",
            Self::NordFrost => "Arctic slate with glacier blue & frost",
            Self::RosePine => "Soho obsidian with rosewood, iris & gold",
            Self::MonokaiPro => "Charcoal studio with vivid lime & amber",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        let norm = id.trim().trim_matches('"').trim_matches('\'');
        Self::all().iter().copied().find(|p| p.id() == norm)
    }

    pub fn palette(&self) -> ThemePalette {
        match self {
            Self::OverrideCyan => ThemePalette {
                accent: Color::Rgb(79, 195, 247),
                success: Color::Rgb(102, 187, 106),
                warning: Color::Rgb(255, 183, 77),
                error: Color::Rgb(239, 83, 80),
                text_primary: Color::Rgb(240, 240, 240),
                text_muted: Color::Rgb(130, 130, 130),
                bg_dark: Color::Rgb(20, 24, 30),
                bg_card: Color::Rgb(26, 31, 40),
                bg_selected: Color::Rgb(38, 52, 74),
                bg_selected_inactive: Color::Rgb(30, 38, 52),
                active_border: Color::Rgb(79, 195, 247),
                inactive_border: Color::Rgb(60, 65, 75),
            },
            Self::TokyoNight => ThemePalette {
                accent: Color::Rgb(122, 162, 247),
                success: Color::Rgb(158, 206, 106),
                warning: Color::Rgb(224, 175, 104),
                error: Color::Rgb(247, 118, 142),
                text_primary: Color::Rgb(192, 202, 245),
                text_muted: Color::Rgb(86, 95, 137),
                bg_dark: Color::Rgb(26, 27, 38),
                bg_card: Color::Rgb(36, 40, 59),
                bg_selected: Color::Rgb(46, 60, 100),
                bg_selected_inactive: Color::Rgb(41, 46, 66),
                active_border: Color::Rgb(122, 162, 247),
                inactive_border: Color::Rgb(65, 72, 104),
            },
            Self::CatppuccinMocha => ThemePalette {
                accent: Color::Rgb(137, 180, 250),
                success: Color::Rgb(166, 227, 161),
                warning: Color::Rgb(249, 226, 175),
                error: Color::Rgb(243, 139, 168),
                text_primary: Color::Rgb(205, 214, 244),
                text_muted: Color::Rgb(147, 153, 178),
                bg_dark: Color::Rgb(30, 30, 46),
                bg_card: Color::Rgb(42, 43, 61),
                bg_selected: Color::Rgb(69, 71, 90),
                bg_selected_inactive: Color::Rgb(49, 50, 68),
                active_border: Color::Rgb(180, 190, 254),
                inactive_border: Color::Rgb(88, 91, 112),
            },
            Self::Dracula => ThemePalette {
                accent: Color::Rgb(189, 147, 249),
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(241, 250, 140),
                error: Color::Rgb(255, 85, 85),
                text_primary: Color::Rgb(248, 248, 242),
                text_muted: Color::Rgb(98, 114, 164),
                bg_dark: Color::Rgb(40, 42, 54),
                bg_card: Color::Rgb(52, 55, 70),
                bg_selected: Color::Rgb(68, 71, 90),
                bg_selected_inactive: Color::Rgb(56, 58, 76),
                active_border: Color::Rgb(139, 233, 253),
                inactive_border: Color::Rgb(68, 71, 90),
            },
            Self::GruvboxDark => ThemePalette {
                accent: Color::Rgb(142, 192, 124),
                success: Color::Rgb(184, 187, 38),
                warning: Color::Rgb(250, 189, 47),
                error: Color::Rgb(251, 73, 52),
                text_primary: Color::Rgb(235, 219, 178),
                text_muted: Color::Rgb(146, 131, 116),
                bg_dark: Color::Rgb(40, 40, 40),
                bg_card: Color::Rgb(50, 48, 47),
                bg_selected: Color::Rgb(80, 73, 69),
                bg_selected_inactive: Color::Rgb(60, 56, 54),
                active_border: Color::Rgb(250, 189, 47),
                inactive_border: Color::Rgb(102, 92, 84),
            },
            Self::EmeraldMatrix => ThemePalette {
                accent: Color::Rgb(52, 211, 153),
                success: Color::Rgb(110, 231, 183),
                warning: Color::Rgb(251, 191, 36),
                error: Color::Rgb(248, 113, 113),
                text_primary: Color::Rgb(236, 253, 245),
                text_muted: Color::Rgb(107, 140, 128),
                bg_dark: Color::Rgb(13, 22, 19),
                bg_card: Color::Rgb(20, 34, 29),
                bg_selected: Color::Rgb(27, 58, 48),
                bg_selected_inactive: Color::Rgb(22, 42, 36),
                active_border: Color::Rgb(52, 211, 153),
                inactive_border: Color::Rgb(45, 74, 64),
            },
            Self::DeepBlack => ThemePalette {
                accent: Color::Rgb(56, 189, 248),
                success: Color::Rgb(74, 222, 128),
                warning: Color::Rgb(251, 191, 36),
                error: Color::Rgb(248, 113, 113),
                text_primary: Color::Rgb(245, 245, 247),
                text_muted: Color::Rgb(110, 110, 122),
                bg_dark: Color::Rgb(0, 0, 0),
                bg_card: Color::Rgb(10, 10, 12),
                bg_selected: Color::Rgb(24, 34, 48),
                bg_selected_inactive: Color::Rgb(16, 20, 26),
                active_border: Color::Rgb(56, 189, 248),
                inactive_border: Color::Rgb(38, 38, 44),
            },
            Self::NordFrost => ThemePalette {
                accent: Color::Rgb(136, 192, 208),
                success: Color::Rgb(163, 190, 140),
                warning: Color::Rgb(235, 203, 139),
                error: Color::Rgb(191, 97, 106),
                text_primary: Color::Rgb(236, 239, 244),
                text_muted: Color::Rgb(143, 155, 179),
                bg_dark: Color::Rgb(46, 52, 64),
                bg_card: Color::Rgb(59, 66, 82),
                bg_selected: Color::Rgb(67, 76, 94),
                bg_selected_inactive: Color::Rgb(54, 61, 76),
                active_border: Color::Rgb(136, 192, 208),
                inactive_border: Color::Rgb(76, 86, 106),
            },
            Self::RosePine => ThemePalette {
                accent: Color::Rgb(235, 188, 186),
                success: Color::Rgb(156, 207, 216),
                warning: Color::Rgb(246, 193, 119),
                error: Color::Rgb(235, 111, 146),
                text_primary: Color::Rgb(224, 222, 244),
                text_muted: Color::Rgb(144, 140, 170),
                bg_dark: Color::Rgb(25, 23, 36),
                bg_card: Color::Rgb(31, 29, 46),
                bg_selected: Color::Rgb(48, 43, 72),
                bg_selected_inactive: Color::Rgb(38, 35, 58),
                active_border: Color::Rgb(235, 188, 186),
                inactive_border: Color::Rgb(64, 61, 82),
            },
            Self::MonokaiPro => ThemePalette {
                accent: Color::Rgb(120, 220, 232),
                success: Color::Rgb(169, 220, 118),
                warning: Color::Rgb(255, 216, 102),
                error: Color::Rgb(255, 97, 136),
                text_primary: Color::Rgb(252, 252, 250),
                text_muted: Color::Rgb(147, 146, 147),
                bg_dark: Color::Rgb(34, 31, 34),
                bg_card: Color::Rgb(45, 42, 46),
                bg_selected: Color::Rgb(64, 62, 65),
                bg_selected_inactive: Color::Rgb(52, 49, 54),
                active_border: Color::Rgb(255, 216, 102),
                inactive_border: Color::Rgb(82, 78, 84),
            },
        }
    }
}

/// Active color palette passed to all UI renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePalette {
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub bg_dark: Color,
    pub bg_card: Color,
    pub bg_selected: Color,
    pub bg_selected_inactive: Color,
    pub active_border: Color,
    pub inactive_border: Color,
}

impl ThemePalette {
    pub fn title_style(&self, active: bool) -> Style {
        if active {
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.text_muted)
        }
    }

    pub fn border_style(&self, active: bool) -> Style {
        if active {
            Style::default().fg(self.active_border)
        } else {
            Style::default().fg(self.inactive_border)
        }
    }

    pub fn selected_style(&self) -> Style {
        Style::default()
            .bg(self.bg_selected)
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }
}

pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Rgb(79, 195, 247); // Android cyan/blue
    pub const SUCCESS: Color = Color::Rgb(102, 187, 106); // Green
    pub const WARNING: Color = Color::Rgb(255, 183, 77); // Orange
    pub const ERROR: Color = Color::Rgb(239, 83, 80); // Red
    pub const TEXT_PRIMARY: Color = Color::Rgb(240, 240, 240);
    pub const TEXT_MUTED: Color = Color::Rgb(130, 130, 130);
    pub const BG_DARK: Color = Color::Rgb(20, 24, 30);
    pub const BG_CARD: Color = Color::Rgb(26, 31, 40);
    pub const ACTIVE_BORDER: Color = Color::Rgb(79, 195, 247);
    pub const INACTIVE_BORDER: Color = Color::Rgb(60, 65, 75);

    pub fn title_style(active: bool) -> Style {
        ThemePreset::OverrideCyan.palette().title_style(active)
    }

    pub fn border_style(active: bool) -> Style {
        ThemePreset::OverrideCyan.palette().border_style(active)
    }

    pub fn selected_style() -> Style {
        ThemePreset::OverrideCyan.palette().selected_style()
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
