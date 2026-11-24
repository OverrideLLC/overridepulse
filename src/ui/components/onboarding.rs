use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};

use crate::android::{DeviceAction, QuickAction};
use crate::app::{AppState, ClickTarget};
use crate::i18n::Language;
use crate::ui::action_deck::action_watermark;
use crate::ui::device_controls::device_action_watermark;
use crate::ui::grid::{render_card_grid, render_card_grid_with_hits, GridCardItem};
use crate::ui::theme::ThemePreset;

/// Renders the 3-step visual & interactive first-run onboarding modal:
/// - Step 0: Language selection (applies to the entire app) with live preview
/// - Step 1: Interactive component tour with real Ratatui mini-previews
/// - Step 2: Live color theme selector
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;
    let popup_area = centered_rect(82, 82, area);
    {
        let mut hit_map = app.hit_map.borrow_mut();
        hit_map.push(area, ClickTarget::DismissModal);
        hit_map.push(popup_area, ClickTarget::ModalBody);
    }
    frame.render_widget(Clear, popup_area);

    let title = match (lang, app.onboarding_step) {
        (Language::En, 0) => " Welcome to OverridePulse ⚡ · Language (1/3) ",
        (Language::Es, 0) => " Bienvenido a OverridePulse ⚡ · Idioma (1/3) ",
        (Language::En, 1) => " Interactive Visual Tour ✨ (2/3) ",
        (Language::Es, 1) => " Tour Visual Interactivo ✨ (2/3) ",
        (Language::En, _) => " Choose Your Theme 🎨 (3/3) ",
        (Language::Es, _) => " Elige tu Tema 🎨 (3/3) ",
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

    if inner.height < 12 || inner.width < 36 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(2)])
        .split(inner);

    let content_area = chunks[0];
    let footer_area = chunks[1];

    match app.onboarding_step {
        0 => render_language_step(frame, app, content_area),
        1 => render_tour_step(frame, app, content_area),
        _ => render_theme_step(frame, app, content_area),
    }

    let key_style = Style::default()
        .fg(theme.warning)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme.text_primary);
    let sep_style = Style::default().fg(theme.text_muted);

    let footer_line = match (lang, app.onboarding_step) {
        (Language::En, 0) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Switch Language", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter / →] ", key_style),
            Span::styled("Next: Visual Tour", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Skip", desc_style),
        ]),
        (Language::Es, 0) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Cambiar Idioma", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter / →] ", key_style),
            Span::styled("Siguiente: Tour Visual", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Omitir", desc_style),
        ]),
        (Language::En, 1) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Section", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Space/←/→] ", key_style),
            Span::styled("Interact Preview", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled("Themes", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Backspace] ", key_style),
            Span::styled("Back", desc_style),
        ]),
        (Language::Es, 1) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Sección", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Space/←/→] ", key_style),
            Span::styled("Probar Preview", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled("Temas", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Backspace] ", key_style),
            Span::styled("Atrás", desc_style),
        ]),
        (Language::En, _) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Preview Theme", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled("Save & Start", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[←] ", key_style),
            Span::styled("Back", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Skip", desc_style),
        ]),
        (Language::Es, _) => Line::from(vec![
            Span::styled("[↑/↓] ", key_style),
            Span::styled("Probar Tema", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Enter] ", key_style),
            Span::styled("Guardar e Iniciar", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[←] ", key_style),
            Span::styled("Atrás", desc_style),
            Span::styled("  │  ", sep_style),
            Span::styled("[Esc] ", key_style),
            Span::styled("Omitir", desc_style),
        ]),
    };

    {
        let mut hit_map = app.hit_map.borrow_mut();
        let third = footer_area.width / 3;
        if app.onboarding_step > 0 {
            hit_map.push(
                Rect {
                    x: footer_area.x,
                    y: footer_area.y,
                    width: third,
                    height: footer_area.height,
                },
                ClickTarget::OnboardingBack,
            );
            hit_map.push(
                Rect {
                    x: footer_area.x + third,
                    y: footer_area.y,
                    width: third,
                    height: footer_area.height,
                },
                ClickTarget::OnboardingNext,
            );
            hit_map.push(
                Rect {
                    x: footer_area.x + third * 2,
                    y: footer_area.y,
                    width: footer_area.width.saturating_sub(third * 2),
                    height: footer_area.height,
                },
                ClickTarget::OnboardingSkip,
            );
        } else {
            let half = footer_area.width / 2;
            hit_map.push(
                Rect {
                    x: footer_area.x,
                    y: footer_area.y,
                    width: half,
                    height: footer_area.height,
                },
                ClickTarget::OnboardingNext,
            );
            hit_map.push(
                Rect {
                    x: footer_area.x + half,
                    y: footer_area.y,
                    width: footer_area.width.saturating_sub(half),
                    height: footer_area.height,
                },
                ClickTarget::OnboardingSkip,
            );
        }
    }

    frame.render_widget(
        Paragraph::new(footer_line).alignment(Alignment::Center),
        footer_area,
    );
}

fn render_language_step(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Intro
            Constraint::Length(5), // Language selector cards
            Constraint::Min(4),    // Live UI language preview
        ])
        .split(area);

    let intro_lines = match lang {
        Language::En => vec![
            Line::from(Span::styled(
                "Choose the language for the entire OverridePulse interface:",
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "You can change this anytime with [Shift+L].",
                Style::default().fg(theme.text_muted),
            )),
        ],
        Language::Es => vec![
            Line::from(Span::styled(
                "Elige el idioma en el que se configurará todo OverridePulse:",
                Style::default()
                    .fg(theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Puedes cambiarlo en cualquier momento con [Shift+L].",
                Style::default().fg(theme.text_muted),
            )),
        ],
    };
    frame.render_widget(Paragraph::new(intro_lines), chunks[0]);

    let lang_items: Vec<GridCardItem> = Language::all()
        .iter()
        .map(|l| GridCardItem {
            badge: Some(l.code().to_uppercase()),
            title: l.name().to_string(),
            subtitle: Some(l.native_tagline().to_string()),
            ..Default::default()
        })
        .collect();
    let mut hit_map = app.hit_map.borrow_mut();
    render_card_grid_with_hits(
        frame,
        chunks[1],
        &lang_items,
        app.selected_language_index,
        true,
        2,
        theme,
        |idx, rect| hit_map.push(rect, ClickTarget::OnboardingLanguage(idx)),
    );
    drop(hit_map);

    let preview_title = match lang {
        Language::En => " Live Workspace Preview ",
        Language::Es => " Vista Previa de la Interfaz ",
    };
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(preview_title)
        .title_style(Style::default().fg(theme.text_muted))
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_card));

    let preview_inner = preview_block.inner(chunks[2]);
    frame.render_widget(preview_block, chunks[2]);

    let sample_lines = match lang {
        Language::En => vec![
            Line::from(vec![
                Span::styled("  Actions · ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Build & Install  ·  Compile WasmJS  ·  Run Desktop", Style::default().fg(theme.text_primary)),
            ]),
            Line::from(vec![
                Span::styled("  [1] Status: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                Span::styled("Ready  ·  ", Style::default().fg(theme.text_primary)),
                Span::styled("[d] Devices  [t] Target  [?] Help", Style::default().fg(theme.text_muted)),
            ]),
        ],
        Language::Es => vec![
            Line::from(vec![
                Span::styled("  Acciones · ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                Span::styled("Compilar e Instalar  ·  Compilar WasmJS  ·  Ejecutar Desktop", Style::default().fg(theme.text_primary)),
            ]),
            Line::from(vec![
                Span::styled("  [1] Estado: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
                Span::styled("Listo  ·  ", Style::default().fg(theme.text_primary)),
                Span::styled("[d] Equipos  [t] Target  [?] Ayuda", Style::default().fg(theme.text_muted)),
            ]),
        ],
    };
    frame.render_widget(Paragraph::new(sample_lines), preview_inner);
}

fn render_tour_step(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(area);

    let chapters_title = match lang {
        Language::En => " Components ",
        Language::Es => " Componentes ",
    };
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(chapters_title)
        .title_style(theme.title_style(true))
        .border_style(theme.border_style(true))
        .style(Style::default().bg(theme.bg_dark));
    let left_inner = left_block.inner(cols[0]);
    frame.render_widget(left_block, cols[0]);

    let chapters: [(&str, &str); 4] = match lang {
        Language::En => [
            ("🗂 Actions Grid", "[b] [r] [←/→/↑/↓]"),
            ("🧩 KMP Targets", "[t] Android/JVM/Wasm/JS"),
            ("📊 Status & Logs", "[1] [2] [3] Panes"),
            ("📱 Devices & ADB", "[d] Separate view"),
        ],
        Language::Es => [
            ("🗂 Cuadrícula", "[b] [r] [←/→/↑/↓]"),
            ("🧩 Targets KMP", "[t] Android/JVM/Wasm/JS"),
            ("📊 Estado y Logs", "[1] [2] [3] Vistas"),
            ("📱 Equipos y ADB", "[d] Vista aparte"),
        ],
    };

    let items: Vec<ListItem> = chapters
        .iter()
        .enumerate()
        .map(|(idx, (name, hint))| {
            let is_sel = idx == app.onboarding_tour_index;
            let cursor = if is_sel { "▶ " } else { "  " };
            let name_style = if is_sel {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_primary)
            };
            let line = Line::from(vec![
                Span::styled(cursor, Style::default().fg(theme.accent)),
                Span::styled(*name, name_style),
            ]);
            let sub = Line::from(Span::styled(
                format!("    {hint}"),
                Style::default().fg(theme.text_muted),
            ));
            let item = ListItem::new(vec![line, sub]);
            if is_sel {
                item.style(theme.selected_style())
            } else {
                item
            }
        })
        .collect();
    {
        let mut hit_map = app.hit_map.borrow_mut();
        for idx in 0..chapters.len() {
            let y = left_inner.y + (idx as u16) * 2;
            if y < left_inner.bottom() {
                hit_map.push(
                    Rect {
                        x: left_inner.x,
                        y,
                        width: left_inner.width,
                        height: 2.min(left_inner.bottom() - y),
                    },
                    ClickTarget::OnboardingTourChapter(idx),
                );
            }
        }
        hit_map.push(cols[1], ClickTarget::OnboardingPreview);
    }
    frame.render_widget(List::new(items), left_inner);

    // Right: Live visual component preview for the selected chapter
    render_tour_preview(frame, app, cols[1]);
}

fn render_tour_preview(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;

    let (title, caption) = match (lang, app.onboarding_tour_index) {
        (Language::En, 0) => (
            " Live Preview: Actions Card Grid ",
            "Press [Space] or [←/→] to move the card highlight, [Enter] executes.",
        ),
        (Language::Es, 0) => (
            " Vista en Vivo: Cuadrícula de Acciones ",
            "Usa [Space] o [←/→] para mover la tarjeta activa; [Enter] ejecuta.",
        ),
        (Language::En, 1) => (
            " Live Preview: KMP Target Switcher [t] ",
            "Host targets (JVM, WasmJS, JS) build and run without any ADB device.",
        ),
        (Language::Es, 1) => (
            " Vista en Vivo: Selector KMP [t] ",
            "Los targets de host (JVM, WasmJS, JS) compilan y corren sin ADB.",
        ),
        (Language::En, 2) => (
            " Live Preview: [1] Status & [2/3] Logs ",
            "Starts calm with [1] Status; toggle [2] Build or [3] Logcat on demand.",
        ),
        (Language::Es, 2) => (
            " Vista en Vivo: [1] Estado y [2/3] Logs ",
            "Inicia limpio con [1] Estado; abre [2] Compilación o [3] Logcat al gusto.",
        ),
        (Language::En, _) => (
            " Live Preview: Separate [d] Devices & ADB View ",
            "Devices and 10 ADB toggles stay tucked away until you press [d].",
        ),
        (Language::Es, _) => (
            " Vista en Vivo: Vista Aparte [d] Equipos y ADB ",
            "Dispositivos y controles ADB se abren aparte solo cuando presionas [d].",
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .title_style(theme.title_style(false))
        .border_style(Style::default().fg(theme.inactive_border))
        .style(Style::default().bg(theme.bg_dark));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(2)])
        .split(inner);

    let preview_area = chunks[0];
    let caption_area = chunks[1];

    match app.onboarding_tour_index {
        0 => {
            let cards = match lang {
                Language::En => vec![
                    GridCardItem {
                        badge: Some("[b]".to_string()),
                        title: "Compile Target".to_string(),
                        subtitle: Some("Gradle build".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpCompileTarget)),
                    },
                    GridCardItem {
                        badge: Some("[r]".to_string()),
                        title: "Run / Dev Server".to_string(),
                        subtitle: Some("Launch app".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpRunTarget)),
                    },
                    GridCardItem {
                        badge: None,
                        title: "Bundle Dist".to_string(),
                        subtitle: Some("Release output".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpBundleDist)),
                    },
                    GridCardItem {
                        badge: Some("[g]".to_string()),
                        title: "Stop Gradle".to_string(),
                        subtitle: Some("Kill daemons".to_string()),
                        watermark: Some(action_watermark(QuickAction::StopGradleDaemons)),
                    },
                ],
                Language::Es => vec![
                    GridCardItem {
                        badge: Some("[b]".to_string()),
                        title: "Compilar Target".to_string(),
                        subtitle: Some("Build Gradle".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpCompileTarget)),
                    },
                    GridCardItem {
                        badge: Some("[r]".to_string()),
                        title: "Ejecutar / Server".to_string(),
                        subtitle: Some("Lanzar app".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpRunTarget)),
                    },
                    GridCardItem {
                        badge: None,
                        title: "Empaquetar Dist".to_string(),
                        subtitle: Some("Build final".to_string()),
                        watermark: Some(action_watermark(QuickAction::KmpBundleDist)),
                    },
                    GridCardItem {
                        badge: Some("[g]".to_string()),
                        title: "Detener Gradle".to_string(),
                        subtitle: Some("Limpiar daemons".to_string()),
                        watermark: Some(action_watermark(QuickAction::StopGradleDaemons)),
                    },
                ],
            };
            render_card_grid(
                frame,
                preview_area,
                &cards,
                app.onboarding_preview_card % cards.len(),
                true,
                2,
                theme,
            );
        }
        1 => {
            let no_adb = match lang {
                Language::En => "No ADB needed",
                Language::Es => "Sin ADB",
            };
            let adb_req = match lang {
                Language::En => "USB / Wi-Fi ADB",
                Language::Es => "ADB USB / Wi-Fi",
            };
            let cards = vec![
                GridCardItem {
                    badge: Some("🤖".to_string()),
                    title: "Android (:composeApp)".to_string(),
                    subtitle: Some(adb_req.to_string()),
                    ..Default::default()
                },
                GridCardItem {
                    badge: Some("☕".to_string()),
                    title: "JVM [desktop]".to_string(),
                    subtitle: Some(no_adb.to_string()),
                    ..Default::default()
                },
                GridCardItem {
                    badge: Some("🕸".to_string()),
                    title: "WasmJS (:composeApp)".to_string(),
                    subtitle: Some(no_adb.to_string()),
                    ..Default::default()
                },
                GridCardItem {
                    badge: Some("🟨".to_string()),
                    title: "JS (:webApp)".to_string(),
                    subtitle: Some(no_adb.to_string()),
                    ..Default::default()
                },
            ];
            render_card_grid(
                frame,
                preview_area,
                &cards,
                app.onboarding_preview_card % cards.len(),
                true,
                2,
                theme,
            );
        }
        2 => {
            let sub = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(2),
                ])
                .split(preview_area);
            let running = match lang {
                Language::En => "⠋ Running: :composeApp:wasmJsBrowserDistribution",
                Language::Es => "⠋ Ejecutando: :composeApp:wasmJsBrowserDistribution",
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    running,
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ))),
                sub[0],
            );
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(theme.accent).bg(theme.bg_card))
                .percent(68)
                .label(Span::styled(
                    "68%",
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(Modifier::BOLD),
                ));
            frame.render_widget(gauge, sub[1]);
            let logs = vec![
                Line::from(Span::styled(
                    "  ✅ :composeApp:compileKotlinWasmJs (1.8s)",
                    Style::default().fg(theme.success),
                )),
                Line::from(Span::styled(
                    "  [I] MainActivity: App started (UID scoped)",
                    Style::default().fg(theme.text_primary),
                )),
                Line::from(Span::styled(
                    "  [W] Network: Retrying socket...",
                    Style::default().fg(theme.warning),
                )),
            ];
            frame.render_widget(Paragraph::new(logs), sub[2]);
        }
        _ => {
            let split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(preview_area);

            let dev_lines = vec![
                Line::from(Span::styled(
                    "▶ ● Pixel_8 (USB)",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "    94% · Android 15",
                    Style::default().fg(theme.text_muted),
                )),
                Line::from(Span::styled(
                    "  ○ emulator-5554",
                    Style::default().fg(theme.text_primary),
                )),
            ];
            frame.render_widget(Paragraph::new(dev_lines), split[0]);

            let adb_cards = match lang {
                Language::En => vec![
                    GridCardItem {
                        badge: Some("📡".to_string()),
                        title: "Toggle Wi-Fi".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::ToggleWifi)),
                    },
                    GridCardItem {
                        badge: Some("🌓".to_string()),
                        title: "Dark Theme".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::ToggleDarkMode)),
                    },
                    GridCardItem {
                        badge: Some("📸".to_string()),
                        title: "Screenshot".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::TakeScreenshot)),
                    },
                    GridCardItem {
                        badge: Some("📶".to_string()),
                        title: "Wi-Fi ADB [w]".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(
                            DeviceAction::WirelessPairAndConnect,
                        )),
                    },
                ],
                Language::Es => vec![
                    GridCardItem {
                        badge: Some("📡".to_string()),
                        title: "Alternar Wi-Fi".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::ToggleWifi)),
                    },
                    GridCardItem {
                        badge: Some("🌓".to_string()),
                        title: "Tema Oscuro".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::ToggleDarkMode)),
                    },
                    GridCardItem {
                        badge: Some("📸".to_string()),
                        title: "Captura".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(DeviceAction::TakeScreenshot)),
                    },
                    GridCardItem {
                        badge: Some("📶".to_string()),
                        title: "ADB Wi-Fi [w]".to_string(),
                        subtitle: None,
                        watermark: Some(device_action_watermark(
                            DeviceAction::WirelessPairAndConnect,
                        )),
                    },
                ],
            };
            render_card_grid(
                frame,
                split[1],
                &adb_cards,
                app.onboarding_preview_card % adb_cards.len(),
                true,
                2,
                theme,
            );
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            caption,
            Style::default().fg(theme.text_muted),
        ))),
        caption_area,
    );
}

fn render_theme_step(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(4),
        ])
        .split(area);

    let intro_msg = match lang {
        Language::En => {
            "Select a color palette (live preview updates immediately; saved to ~/.config/overridepulse):"
        }
        Language::Es => {
            "Elige una paleta (vista previa en vivo; se guarda en ~/.config/overridepulse):"
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            intro_msg,
            Style::default().fg(theme.text_muted),
        ))),
        chunks[0],
    );

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

    {
        let mut hit_map = app.hit_map.borrow_mut();
        for idx in 0..ThemePreset::all().len() {
            let y = chunks[1].y + idx as u16;
            if y < chunks[1].bottom() {
                hit_map.push(
                    Rect {
                        x: chunks[1].x,
                        y,
                        width: chunks[1].width,
                        height: 1,
                    },
                    ClickTarget::OnboardingTheme(idx),
                );
            }
        }
    }

    frame.render_widget(List::new(items), chunks[1]);

    // Mini themed card preview strip at bottom of step 3
    let sample_cards = match lang {
        Language::En => vec![
            GridCardItem {
                badge: Some("[b]".to_string()),
                title: "Build & Install".to_string(),
                subtitle: None,
                watermark: Some(action_watermark(QuickAction::BuildAndInstallDebug)),
            },
            GridCardItem {
                badge: Some("[r]".to_string()),
                title: "Run Dev Server".to_string(),
                subtitle: None,
                watermark: Some(action_watermark(QuickAction::KmpRunTarget)),
            },
        ],
        Language::Es => vec![
            GridCardItem {
                badge: Some("[b]".to_string()),
                title: "Compilar Target".to_string(),
                subtitle: None,
                watermark: Some(action_watermark(QuickAction::KmpCompileTarget)),
            },
            GridCardItem {
                badge: Some("[r]".to_string()),
                title: "Ejecutar Server".to_string(),
                subtitle: None,
                watermark: Some(action_watermark(QuickAction::KmpRunTarget)),
            },
        ],
    };
    render_card_grid(frame, chunks[2], &sample_cards, 0, true, 2, theme);
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
