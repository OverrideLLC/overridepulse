use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, ClickTarget, SettingsFocus, SettingsSection};
use crate::config::{SdkConfig, UserConfig};
use crate::i18n::Language;
use crate::ui::theme::{ThemePalette, ThemePreset};

/// Renders the full-screen Settings view with left sidebar navigation and full content pane.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(27), Constraint::Percentage(73)])
        .split(area);

    render_sidebar(frame, app, cols[0]);
    render_content(frame, app, cols[1]);
}

fn render_sidebar(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;
    let is_focused = app.settings_focus == SettingsFocus::Sidebar;

    let title = match lang {
        Language::En => " ⚙ Settings ",
        Language::Es => " ⚙ Ajustes ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(theme.title_style(is_focused))
        .border_style(theme.border_style(is_focused))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 || inner.width < 12 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(5)])
        .split(inner);

    let mut items = Vec::new();
    for section in SettingsSection::all() {
        let active = *section == app.settings_section;
        let cursor = if active { "▶ " } else { "  " };

        let row_style = if active && is_focused {
            theme.selected_style()
        } else if active {
            Style::default()
                .bg(theme.bg_selected_inactive)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_primary)
        };

        let line1 = Line::from(vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(format!("{} ", section.icon()), row_style),
            Span::styled(section.label(lang), row_style),
        ]);
        let line2 = Line::from(vec![
            Span::raw("    "),
            Span::styled(
                section.description(lang),
                Style::default().fg(theme.text_muted),
            ),
        ]);
        items.push(ListItem::new(vec![line1, line2, Line::raw("")]));
    }

    let mut hit_map = app.hit_map.borrow_mut();
    for (idx, section) in SettingsSection::all().iter().enumerate() {
        let y = chunks[0].y + (idx as u16) * 3;
        if y < chunks[0].bottom() {
            let h = 2.min(chunks[0].bottom() - y);
            hit_map.push(
                Rect {
                    x: chunks[0].x,
                    y,
                    width: chunks[0].width,
                    height: h,
                },
                ClickTarget::SettingsSection(*section),
            );
        }
    }
    drop(hit_map);

    frame.render_widget(List::new(items), chunks[0]);

    let info_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_dark));
    let info_inner = info_block.inner(chunks[1]);
    frame.render_widget(info_block, chunks[1]);

    let version = env!("CARGO_PKG_VERSION");
    let nav_hint = match lang {
        Language::En => "[↑/↓] Section · [→/Tab] Edit",
        Language::Es => "[↑/↓] Sección · [→/Tab] Editar",
    };

    let info_lines = vec![
        Line::from(vec![
            Span::styled("⚡ OverridePulse ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled(format!("v{version}"), Style::default().fg(theme.success)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{} · {}", app.theme.name(), app.language.name()),
                Style::default().fg(theme.text_muted),
            ),
        ]),
        Line::from(vec![Span::styled(
            nav_hint,
            Style::default().fg(theme.warning),
        )]),
    ];
    frame.render_widget(Paragraph::new(info_lines), info_inner);
}

fn render_content(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;
    let is_focused = app.settings_focus == SettingsFocus::Content;

    let hint = if app.settings_editing_adb_path {
        match lang {
            Language::En => "[Enter] Save · [Esc] Cancel",
            Language::Es => "[Enter] Guardar · [Esc] Cancelar",
        }
    } else {
        match lang {
            Language::En => "[Enter] Apply · [←/Tab] Menu",
            Language::Es => "[Enter] Aplicar · [←/Tab] Menú",
        }
    };

    let title = format!(
        " {} {} │ {} ",
        app.settings_section.icon(),
        app.settings_section.label(lang),
        hint
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

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    match app.settings_section {
        SettingsSection::Appearance => render_appearance(frame, app, inner, theme),
        SettingsSection::AdbSdk => render_adb_sdk(frame, app, inner, theme),
        SettingsSection::ConsoleLogcat => render_console_logcat(frame, app, inner, theme),
        SettingsSection::About => render_about(frame, app, inner, theme),
    }
}

fn item_style(selected: bool, content_focused: bool, theme: ThemePalette) -> Style {
    if selected && content_focused {
        theme.selected_style()
    } else if selected {
        Style::default()
            .bg(theme.bg_selected_inactive)
            .fg(theme.text_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text_primary)
    }
}

fn render_appearance(frame: &mut Frame, app: &AppState, area: Rect, theme: ThemePalette) {
    let lang = app.language;
    let content_focused = app.settings_focus == SettingsFocus::Content;
    let themes = ThemePreset::all();
    let langs = Language::all();

    let mut lines: Vec<Line> = Vec::new();
    let theme_header = match lang {
        Language::En => "Color Theme",
        Language::Es => "Tema de Color",
    };
    lines.push(Line::from(vec![Span::styled(
        format!("🎨 {theme_header}"),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]));

    for (idx, preset) in themes.iter().enumerate() {
        let is_cursor = content_focused && app.settings_item_index == idx;
        let is_active = *preset == app.theme;
        let p = preset.palette();

        let cursor = if is_cursor { "▶ " } else { "  " };
        let radio = if is_active { "● " } else { "○ " };
        let row_style = item_style(is_cursor, content_focused, theme);
        let radio_style = if is_active {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted)
        };

        let mut spans = vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(radio, radio_style),
            Span::styled("█", Style::default().fg(p.accent)),
            Span::styled("█", Style::default().fg(p.success)),
            Span::styled("█ ", Style::default().fg(p.warning)),
            Span::styled(format!("{:<18}", preset.name()), row_style),
            Span::styled(preset.tagline(), Style::default().fg(theme.text_muted)),
        ];
        if is_active {
            spans.push(Span::styled(
                match lang {
                    Language::En => "  [Active]",
                    Language::Es => "  [Activo]",
                },
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        let y = area.y + lines.len() as u16;
        if y < area.bottom() {
            app.hit_map.borrow_mut().push(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                ClickTarget::SettingsItem(idx),
            );
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::raw(""));
    let lang_header = match lang {
        Language::En => "Interface Language",
        Language::Es => "Idioma de la Interfaz",
    };
    lines.push(Line::from(vec![Span::styled(
        format!("🌐 {lang_header}"),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]));

    for (offset, l) in langs.iter().enumerate() {
        let idx = themes.len() + offset;
        let is_cursor = content_focused && app.settings_item_index == idx;
        let is_active = *l == app.language;

        let cursor = if is_cursor { "▶ " } else { "  " };
        let radio = if is_active { "● " } else { "○ " };
        let row_style = item_style(is_cursor, content_focused, theme);
        let radio_style = if is_active {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted)
        };

        let mut spans = vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(radio, radio_style),
            Span::styled(format!("{:<18}", l.name()), row_style),
            Span::styled(l.native_tagline(), Style::default().fg(theme.text_muted)),
        ];
        if is_active {
            spans.push(Span::styled(
                match lang {
                    Language::En => "  [Active]",
                    Language::Es => "  [Activo]",
                },
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        let y = area.y + lines.len() as u16;
        if y < area.bottom() {
            app.hit_map.borrow_mut().push(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                ClickTarget::SettingsItem(idx),
            );
        }
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_adb_sdk(frame: &mut Frame, app: &AppState, area: Rect, theme: ThemePalette) {
    let lang = app.language;
    let content_focused = app.settings_focus == SettingsFocus::Content;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(6)])
        .split(area);

    // 1. Detected environment card
    let env_title = match lang {
        Language::En => " Detected Android SDK & Tools ",
        Language::Es => " Entorno Android SDK y Herramientas ",
    };
    let env_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(env_title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_card));
    let env_inner = env_block.inner(chunks[0]);
    frame.render_widget(env_block, chunks[0]);

    let adb_ok = SdkConfig::is_adb_path_reachable(&app.adb_path);
    let adb_badge = if adb_ok {
        match lang {
            Language::En => "✓ Ready",
            Language::Es => "✓ Listo",
        }
    } else {
        match lang {
            Language::En => "✗ Not found",
            Language::Es => "✗ No encontrado",
        }
    };
    let adb_source = if app.custom_adb_path.is_some() {
        match lang {
            Language::En => "(Custom)",
            Language::Es => "(Personalizado)",
        }
    } else {
        match lang {
            Language::En => "(Auto)",
            Language::Es => "(Auto)",
        }
    };

    let missing_label = match lang {
        Language::En => "Not detected",
        Language::Es => "No detectado",
    };

    let format_opt = |opt: &Option<std::path::PathBuf>| -> (String, Style) {
        match opt {
            Some(p) => (p.display().to_string(), Style::default().fg(theme.text_primary)),
            None => (
                missing_label.to_string(),
                Style::default().fg(theme.text_muted),
            ),
        }
    };
    let (sdk_str, sdk_style) = format_opt(&app.sdk_config.sdk_root);
    let (bt_str, bt_style) = format_opt(&app.sdk_config.bundletool_path);
    let (scrcpy_str, scrcpy_style) = format_opt(&app.sdk_config.scrcpy_path);

    let env_lines = vec![
        Line::from(vec![
            Span::styled("  ADB Effective : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                format!("{} ", app.adb_path.display()),
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{adb_source} "), Style::default().fg(theme.accent)),
            Span::styled(
                adb_badge,
                Style::default()
                    .fg(if adb_ok { theme.success } else { theme.error })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  SDK Root      : ", Style::default().fg(theme.text_muted)),
            Span::styled(sdk_str, sdk_style),
        ]),
        Line::from(vec![
            Span::styled("  bundletool    : ", Style::default().fg(theme.text_muted)),
            Span::styled(bt_str, bt_style),
        ]),
        Line::from(vec![
            Span::styled("  scrcpy        : ", Style::default().fg(theme.text_muted)),
            Span::styled(scrcpy_str, scrcpy_style),
        ]),
    ];
    frame.render_widget(Paragraph::new(env_lines), env_inner);

    // 2. Interactive ADB settings & actions
    let mut lines = Vec::new();
    lines.push(Line::raw(""));
    let sec_title = match lang {
        Language::En => "ADB Configuration & Actions",
        Language::Es => "Configuración y Acciones de ADB",
    };
    lines.push(Line::from(vec![Span::styled(
        format!("🔧 {sec_title}"),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::raw(""));

    // Item 0: Custom ADB path
    let c0 = content_focused && app.settings_item_index == 0;
    let cursor0 = if c0 { "▶ " } else { "  " };
    let path_label = match lang {
        Language::En => "Custom ADB Binary Path",
        Language::Es => "Ruta Personalizada de ADB",
    };
    let path_val = if app.settings_editing_adb_path {
        format!("{}█", app.settings_adb_input)
    } else {
        app.custom_adb_path
            .clone()
            .unwrap_or_else(|| match lang {
                Language::En => "<Auto-detect from SDK / PATH>".to_string(),
                Language::Es => "<Auto-detectar desde SDK / PATH>".to_string(),
            })
    };
    let path_hint = if app.settings_editing_adb_path {
        match lang {
            Language::En => "  [Enter] Save (empty = auto) · [Esc] Cancel",
            Language::Es => "  [Enter] Guardar (vacío = auto) · [Esc] Cancelar",
        }
    } else {
        match lang {
            Language::En => "  [Enter to edit]",
            Language::Es => "  [Enter para editar]",
        }
    };
    let y0 = chunks[1].y + lines.len() as u16;
    if y0 < chunks[1].bottom() {
        app.hit_map.borrow_mut().push(
            Rect {
                x: chunks[1].x,
                y: y0,
                width: chunks[1].width,
                height: 1,
            },
            ClickTarget::SettingsItem(0),
        );
    }
    lines.push(Line::from(vec![
        Span::styled(cursor0, Style::default().fg(theme.accent)),
        Span::styled(
            format!("{path_label}: "),
            item_style(c0, content_focused, theme),
        ),
        Span::styled(
            path_val,
            if app.settings_editing_adb_path {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent)
            },
        ),
        Span::styled(path_hint, Style::default().fg(theme.text_muted)),
    ]));
    lines.push(Line::raw(""));

    let connected_count = app.devices.len();
    let actions: Vec<(&str, String)> = match lang {
        Language::En => vec![
            ("🔍 Re-detect SDK & Tools", String::new()),
            ("🔄 Restart ADB Server", String::new()),
            ("📶 Wi-Fi ADB Pair & Connect", String::new()),
            ("📱 Devices & ADB Controls", format!("({connected_count})")),
        ],
        Language::Es => vec![
            ("🔍 Re-detectar SDK", String::new()),
            ("🔄 Reiniciar ADB", String::new()),
            ("📶 ADB Wi-Fi", String::new()),
            ("📱 Equipos y Controles", format!("({connected_count})")),
        ],
    };

    for (i, (title, desc)) in actions.into_iter().enumerate() {
        let idx = i + 1;
        let selected = content_focused && app.settings_item_index == idx;
        let cursor = if selected { "▶ " } else { "  " };
        let y = chunks[1].y + lines.len() as u16;
        if y < chunks[1].bottom() {
            app.hit_map.borrow_mut().push(
                Rect {
                    x: chunks[1].x,
                    y,
                    width: chunks[1].width,
                    height: 1,
                },
                ClickTarget::SettingsItem(idx),
            );
        }
        lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(
                format!("{title:<42}"),
                item_style(selected, content_focused, theme),
            ),
            Span::styled(desc, Style::default().fg(theme.text_muted)),
        ]));
        lines.push(Line::raw(""));
    }

    frame.render_widget(Paragraph::new(lines), chunks[1]);
}

fn console_preset_label(app: &AppState) -> &'static str {
    let lang = app.language;
    match (
        app.show_simplified_pane,
        app.show_build_pane,
        app.show_logcat_pane,
    ) {
        (true, false, false) => match lang {
            Language::En => "Status Only",
            Language::Es => "Solo Estado",
        },
        (false, true, false) => match lang {
            Language::En => "Build Output Only",
            Language::Es => "Solo Compilación",
        },
        (false, false, true) => match lang {
            Language::En => "Live Logcat Only",
            Language::Es => "Solo Logcat",
        },
        (true, false, true) => match lang {
            Language::En => "Status + Logcat",
            Language::Es => "Estado + Logcat",
        },
        (true, true, true) => match lang {
            Language::En => "Triple View",
            Language::Es => "Vista Triple",
        },
        _ => match lang {
            Language::En => "Custom Split",
            Language::Es => "Combinación Personalizada",
        },
    }
}

fn render_console_logcat(frame: &mut Frame, app: &AppState, area: Rect, theme: ThemePalette) {
    let lang = app.language;
    let content_focused = app.settings_focus == SettingsFocus::Content;

    let mut lines = Vec::new();
    let on_badge = "[ON] ";
    let off_badge = "[OFF]";
    let scope_badge = if app.logcat_filter_by_app {
        match lang {
            Language::En => "📦 [App]",
            Language::Es => "📦 [App]",
        }
    } else {
        match lang {
            Language::En => "🌐 [All]",
            Language::Es => "🌐 [Todo]",
        }
    };

    let rows: Vec<(&str, String, bool)> = match lang {
        Language::En => vec![
            ("◫ Console Preset", format!("[{}]", console_preset_label(app)), true),
            ("📊 Status Pane", if app.show_simplified_pane { on_badge } else { off_badge }.to_string(), app.show_simplified_pane),
            ("🔨 Build Pane", if app.show_build_pane { on_badge } else { off_badge }.to_string(), app.show_build_pane),
            ("📜 Logcat Pane", if app.show_logcat_pane { on_badge } else { off_badge }.to_string(), app.show_logcat_pane),
            ("🔍 Logcat Scope", scope_badge.to_string(), app.logcat_filter_by_app),
            ("✨ Visual Tour", "[▶]".to_string(), true),
        ],
        Language::Es => vec![
            ("◫ Preset Consola", format!("[{}]", console_preset_label(app)), true),
            ("📊 Panel Estado", if app.show_simplified_pane { on_badge } else { off_badge }.to_string(), app.show_simplified_pane),
            ("🔨 Panel Build", if app.show_build_pane { on_badge } else { off_badge }.to_string(), app.show_build_pane),
            ("📜 Panel Logcat", if app.show_logcat_pane { on_badge } else { off_badge }.to_string(), app.show_logcat_pane),
            ("🔍 Filtro Logcat", scope_badge.to_string(), app.logcat_filter_by_app),
            ("✨ Tour Visual", "[▶]".to_string(), true),
        ],
    };

    for (idx, (label, badge, active)) in rows.into_iter().enumerate() {
        let selected = content_focused && app.settings_item_index == idx;
        let cursor = if selected { "▶ " } else { "  " };
        let badge_color = if active { theme.success } else { theme.text_muted };
        let y = area.y + lines.len() as u16;
        if y < area.bottom() {
            app.hit_map.borrow_mut().push(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                ClickTarget::SettingsItem(idx),
            );
        }
        lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(
                format!("{label:<36}"),
                item_style(selected, content_focused, theme),
            ),
            Span::styled(
                badge,
                Style::default()
                    .fg(badge_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_about(frame: &mut Frame, app: &AppState, area: Rect, theme: ThemePalette) {
    let lang = app.language;
    let content_focused = app.settings_focus == SettingsFocus::Content;
    let version = env!("CARGO_PKG_VERSION");
    let config_path = UserConfig::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/overridepulse/config".to_string());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(7),
            Constraint::Min(4),
        ])
        .split(area);

    // 1. App Metadata card
    let app_card_title = match lang {
        Language::En => " OverridePulse Application Info ",
        Language::Es => " Información de OverridePulse ",
    };
    let app_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(app_card_title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_card));
    let app_inner = app_block.inner(chunks[0]);
    frame.render_widget(app_block, chunks[0]);

    let app_lines = vec![
        Line::from(vec![
            Span::styled("  Version    : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                format!("v{version}"),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Authors    : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                "Override LLC · ChristopherTwo",
                Style::default().fg(theme.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled("  License    : ", Style::default().fg(theme.text_muted)),
            Span::styled("MIT", Style::default().fg(theme.text_primary)),
        ]),
        Line::from(vec![
            Span::styled("  Website    : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                "https://www.override.com.mx/pulse",
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Repository : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                env!("CARGO_PKG_REPOSITORY"),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Config     : ", Style::default().fg(theme.text_muted)),
            Span::styled(config_path, Style::default().fg(theme.text_primary)),
        ]),
        Line::from(vec![
            Span::styled("  Summary    : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                env!("CARGO_PKG_DESCRIPTION"),
                Style::default().fg(theme.text_muted),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(app_lines), app_inner);

    // 2. Active Workspace card
    let ws_title = match lang {
        Language::En => " Active Workspace ",
        Language::Es => " Espacio de Trabajo Activo ",
    };
    let ws_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(ws_title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_card));
    let ws_inner = ws_block.inner(chunks[1]);
    frame.render_widget(ws_block, chunks[1]);

    let target_label = app
        .project
        .active_target()
        .map(|t| t.display_label())
        .unwrap_or_else(|| "Android".to_string());
    let app_id = app
        .project
        .application_id
        .as_deref()
        .or(app.project.namespace.as_deref())
        .unwrap_or("—");

    let ws_lines = vec![
        Line::from(vec![
            Span::styled("  Project    : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                &app.project.name,
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Root Path  : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                app.project.root_path.display().to_string(),
                Style::default().fg(theme.text_primary),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Target     : ", Style::default().fg(theme.text_muted)),
            Span::styled(
                format!("{target_label} ({})", app.project.active_variant),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  App ID     : ", Style::default().fg(theme.text_muted)),
            Span::styled(app_id, Style::default().fg(theme.text_primary)),
        ]),
    ];
    frame.render_widget(Paragraph::new(ws_lines), ws_inner);

    // 3. Actions
    let actions = match lang {
        Language::En => [
            "✨ Open Interactive Visual Tour (Onboarding)",
            "❓ Open Full Keybindings Reference (Help)",
        ],
        Language::Es => [
            "✨ Abrir Tour Visual Interactivo (Onboarding)",
            "❓ Abrir Referencia Completa de Atajos (Ayuda)",
        ],
    };

    let mut action_lines = vec![Line::raw("")];
    for (idx, label) in actions.iter().enumerate() {
        let selected = content_focused && app.settings_item_index == idx;
        let cursor = if selected { "▶ " } else { "  " };
        let y = chunks[2].y + action_lines.len() as u16;
        if y < chunks[2].bottom() {
            app.hit_map.borrow_mut().push(
                Rect {
                    x: chunks[2].x,
                    y,
                    width: chunks[2].width,
                    height: 1,
                },
                ClickTarget::SettingsItem(idx),
            );
        }
        action_lines.push(Line::from(vec![
            Span::styled(cursor, Style::default().fg(theme.accent)),
            Span::styled(*label, item_style(selected, content_focused, theme)),
        ]));
    }
    frame.render_widget(Paragraph::new(action_lines), chunks[2]);
}
