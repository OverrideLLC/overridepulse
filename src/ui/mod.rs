pub mod action_deck;
pub mod components;
pub mod device_controls;
pub mod device_panel;
pub mod grid;
pub mod logcat_view;
pub mod settings_view;
pub mod simplified_view;
pub mod terminal_log;
pub mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{AppState, ClickTarget, Panel};
use crate::ui::theme::ThemePalette;

pub struct ToolbarButton<'a> {
    pub label: &'a str,
    pub fg: Color,
    pub target: ClickTarget,
}

/// Renders a 1-row interactive toolbar with clickable pill buttons and optional right status text.
pub fn render_pane_toolbar(
    frame: &mut Frame,
    app: &AppState,
    area: Rect,
    buttons: &[ToolbarButton<'_>],
    right_info: Option<&str>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let theme = app.palette();
    let mut spans = Vec::new();
    let mut x_cursor = area.x;
    let mut hit_map = app.hit_map.borrow_mut();

    for (idx, btn) in buttons.iter().enumerate() {
        if idx > 0 {
            spans.push(Span::styled(" ", Style::default().bg(theme.bg_card)));
            x_cursor = x_cursor.saturating_add(1);
        }
        let text = format!("[{}]", btn.label);
        let w = text.chars().count() as u16;
        if x_cursor < area.right() {
            let clamped = w.min(area.right() - x_cursor);
            hit_map.push(
                Rect {
                    x: x_cursor,
                    y: area.y,
                    width: clamped,
                    height: 1,
                },
                btn.target,
            );
        }
        spans.push(Span::styled(
            text,
            Style::default()
                .fg(btn.fg)
                .bg(theme.bg_card)
                .add_modifier(Modifier::BOLD),
        ));
        x_cursor = x_cursor.saturating_add(w);
    }

    if let Some(info) = right_info {
        let info_w = info.chars().count() as u16;
        if area.right() > x_cursor.saturating_add(info_w + 1) {
            let pad = (area.right() - x_cursor - info_w - 1) as usize;
            spans.push(Span::styled(
                " ".repeat(pad),
                Style::default().bg(theme.bg_card),
            ));
            spans.push(Span::styled(
                info.to_string(),
                Style::default().fg(theme.text_muted).bg(theme.bg_card),
            ));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg_card)),
        area,
    );
}

/// Renders a vertical scrollbar on the right edge when content overflows the viewport.
pub fn render_vertical_scrollbar(
    frame: &mut Frame,
    area: Rect,
    total_lines: usize,
    visible_height: usize,
    scroll_pos: usize,
    theme: ThemePalette,
) {
    if total_lines <= visible_height || area.width < 3 || area.height < 2 {
        return;
    }
    let max_scroll = total_lines.saturating_sub(visible_height);
    let mut state = ScrollbarState::new(max_scroll).position(scroll_pos.min(max_scroll));
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(theme.accent))
        .track_style(Style::default().fg(theme.inactive_border));
    frame.render_stateful_widget(scrollbar, area, &mut state);
}

/// Renders the complete application user interface on the given Ratatui frame,
/// according to the v0.2.0-B UI/UX specifications:
/// - No top header bar (project metadata and variant integrated into bottom footer)
/// - Main body top row (3 columns: Devices 30% / Device Controls 35% / Actions Deck 35%)
/// - Main body bottom row (Configurable multi-pane console with parity: Simplified / Build / Logcat)
/// - Bottom footer (left shortcuts + right project & variant badge)
/// - Modal overlays: Project selector, ADB Pair & Connect modal, Help, and Search
pub fn render(frame: &mut Frame, app: &AppState) {
    app.hit_map.borrow_mut().clear();
    let size = frame.area();

    // Main vertical layout: Body and Footer (No top header)
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // Cuerpo principal
            Constraint::Length(3), // Footer inferior
        ])
        .split(size);

    let body_area = vertical_chunks[0];
    let footer_area = vertical_chunks[1];

    if app.settings_view_active {
        settings_view::render(frame, app, body_area);
    } else if app.devices_view_active {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(body_area);
        device_panel::render(frame, app, cols[0]);
        device_controls::render(frame, app, cols[1]);
    } else {
        // Main workspace: Actions grid (40%) + Console (60%)
        let body_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ])
            .split(body_area);

        action_deck::render(frame, app, body_rows[0]);
        let bottom_area = body_rows[1];

        let active_simplified = app.show_simplified_pane;
        let active_build = app.show_build_pane;
        let active_logcat = app.show_logcat_pane;

        let register_pane = |rect: Rect, panel: Panel| {
            let mut hit_map = app.hit_map.borrow_mut();
            hit_map.push(rect, ClickTarget::FocusPanel(panel));
            hit_map.push_scroll(rect, ClickTarget::ScrollArea(panel));
        };

        match (active_simplified, active_build, active_logcat) {
            // Triple view: All 3 active
            (true, true, true) => {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(28), // Simplified Status
                        Constraint::Percentage(36), // Build Output
                        Constraint::Percentage(36), // Live Logcat
                    ])
                    .split(bottom_area);
                register_pane(cols[0], Panel::Console);
                register_pane(cols[1], Panel::Output);
                register_pane(cols[2], Panel::Logcat);
                simplified_view::render(frame, app, cols[0]);
                terminal_log::render(frame, app, cols[1]);
                logcat_view::render(frame, app, cols[2]);
            }
            // Dual view: Simplified + Logcat
            (true, false, true) => {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(35), // Simplified Status
                        Constraint::Percentage(65), // Live Logcat
                    ])
                    .split(bottom_area);
                register_pane(cols[0], Panel::Console);
                register_pane(cols[1], Panel::Logcat);
                simplified_view::render(frame, app, cols[0]);
                logcat_view::render(frame, app, cols[1]);
            }
            // Dual view: Simplified + Build
            (true, true, false) => {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(35), // Simplified Status
                        Constraint::Percentage(65), // Build Output
                    ])
                    .split(bottom_area);
                register_pane(cols[0], Panel::Console);
                register_pane(cols[1], Panel::Output);
                simplified_view::render(frame, app, cols[0]);
                terminal_log::render(frame, app, cols[1]);
            }
            // Dual view: Build + Logcat
            (false, true, true) => {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(50), // Build Output
                        Constraint::Percentage(50), // Live Logcat
                    ])
                    .split(bottom_area);
                register_pane(cols[0], Panel::Output);
                register_pane(cols[1], Panel::Logcat);
                terminal_log::render(frame, app, cols[0]);
                logcat_view::render(frame, app, cols[1]);
            }
            // Single view: Simplified only
            (true, false, false) => {
                register_pane(bottom_area, Panel::Console);
                simplified_view::render(frame, app, bottom_area);
            }
            // Single view: Build only
            (false, true, false) => {
                register_pane(bottom_area, Panel::Output);
                terminal_log::render(frame, app, bottom_area);
            }
            // Single view: Logcat only
            (false, false, true) => {
                register_pane(bottom_area, Panel::Logcat);
                logcat_view::render(frame, app, bottom_area);
            }
            // Fallback: Default to Simplified
            (false, false, false) => {
                register_pane(bottom_area, Panel::Console);
                simplified_view::render(frame, app, bottom_area);
            }
        }
    }

    // 3. Render Footer (Shortcuts + Project & Variant status)
    components::footer::render(frame, app, footer_area);

    // 4. Overlays (Priority: Onboarding > Project Selector > ADB Connect Modal > Help > Search)
    if app.onboarding_active {
        components::onboarding::render(frame, app, size);
    } else if app.project_selector_active {
        components::project_selector::render(frame, app, size);
    } else if app.adb_connect_modal_active {
        components::adb_connect_modal::render(frame, app, size);
    } else if app.help_active {
        components::help::render(frame, app, size);
    } else if app.search_active {
        components::search::render(frame, app, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::android::{Device, DeviceState, LogEntry, LogLevel};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    #[test]
    fn test_render_default_layout() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.append_console_line("Initializing UI test".to_string());

        let res = terminal.draw(|f| render(f, &app));
        assert!(res.is_ok());
    }

    #[test]
    fn test_render_parity_multi_pane() {
        let backend = TestBackend::new(140, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;

        // 1. Triple view
        app.show_simplified_pane = true;
        app.show_build_pane = true;
        app.show_logcat_pane = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 2. Simplified + Build
        app.show_simplified_pane = true;
        app.show_build_pane = true;
        app.show_logcat_pane = false;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 3. Simplified + Logcat
        app.show_simplified_pane = true;
        app.show_build_pane = false;
        app.show_logcat_pane = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 4. Build + Logcat
        app.show_simplified_pane = false;
        app.show_build_pane = true;
        app.show_logcat_pane = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 5. Simplified only
        app.show_simplified_pane = true;
        app.show_build_pane = false;
        app.show_logcat_pane = false;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
    }

    #[test]
    fn test_render_logcat_view() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.show_logcat_pane = true;
        app.append_logcat_entry(LogEntry {
            raw: "10-23 12:00:00.000 1234 5678 D TestTag: Debug message".to_string(),
            timestamp: Some("10-23 12:00:00.000".to_string()),
            pid: Some(1234),
            tid: Some(5678),
            level: LogLevel::Debug,
            tag: "TestTag".to_string(),
            message: "Debug message".to_string(),
        });
        app.append_logcat_entry(LogEntry {
            raw: "10-23 12:00:01.000 1234 5678 E TestTag: Error message".to_string(),
            timestamp: Some("10-23 12:00:01.000".to_string()),
            pid: Some(1234),
            tid: Some(5678),
            level: LogLevel::Error,
            tag: "TestTag".to_string(),
            message: "Error message".to_string(),
        });

        let res = terminal.draw(|f| render(f, &app));
        assert!(res.is_ok());
    }

    #[test]
    fn test_render_with_devices_and_progress() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut dev1 = Device::new("emulator-5554", DeviceState::Device);
        dev1.model = "Pixel_7_Pro".to_string();
        dev1.battery_level = Some(92);
        dev1.android_version = Some("14".to_string());
        dev1.api_level = Some(34);

        let dev2 = Device::new("offline-device", DeviceState::Offline);

        app.set_devices(vec![dev1, dev2]);
        app.start_task("Gradle assembleDebug");
        app.process_state.progress_percent = Some(85);

        let res = terminal.draw(|f| render(f, &app));
        assert!(res.is_ok());
    }

    #[test]
    fn test_render_overlays() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));

        // 1. Project selector overlay
        app.project_selector_active = true;
        app.nearby_projects = vec![
            PathBuf::from("/home/user/AndroidApp1"),
            PathBuf::from("/home/user/AndroidApp2"),
        ];
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 2. ADB Connect modal overlay
        app.project_selector_active = false;
        app.adb_connect_modal_active = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 3. Help overlay
        app.adb_connect_modal_active = false;
        app.help_active = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 4. Search overlay
        app.help_active = false;
        app.search_active = true;
        app.search_query = "MainActivity".to_string();
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
    }

    #[test]
    fn test_render_kmp_target_ui() {
        use crate::android::{KmpTarget, TargetPlatform};

        let backend = TestBackend::new(140, 38);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.project.is_kmp = true;
        app.project.targets = vec![
            KmpTarget::android(":composeApp", false),
            KmpTarget {
                platform: TargetPlatform::WasmJs,
                module: ":composeApp".to_string(),
                gradle_name: "wasmJs".to_string(),
                is_kmp_target: true,
                has_compose_desktop: false,
                has_application_plugin: false,
                has_browser: true,
                has_nodejs: false,
                is_library_only: false,
            },
        ];
        app.next_target();

        let res = terminal.draw(|f| render(f, &app));
        assert!(res.is_ok());
    }

    #[test]
    fn test_render_devices_view() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.toggle_devices_view();

        let res = terminal.draw(|f| render(f, &app));
        assert!(res.is_ok());
    }

    #[test]
    fn test_render_onboarding_steps() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.open_onboarding();
        // Step 0: Language
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
        app.next_language();
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // Step 1: Visual Tour (all 4 component previews)
        app.onboarding_step = 1;
        for idx in 0..4 {
            app.onboarding_tour_index = idx;
            assert!(terminal.draw(|f| render(f, &app)).is_ok());
        }

        // Step 2: Theme Picker
        app.onboarding_step = 2;
        app.next_theme();
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
    }

    #[test]
    fn test_render_settings_view_all_sections() {
        use crate::app::{SettingsFocus, SettingsSection};
        use crate::i18n::Language;

        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.toggle_settings_view();
        app.settings_focus = SettingsFocus::Content;

        for section in SettingsSection::all() {
            app.settings_section = *section;
            app.set_language(Language::En);
            assert!(terminal.draw(|f| render(f, &app)).is_ok());

            app.set_language(Language::Es);
            assert!(terminal.draw(|f| render(f, &app)).is_ok());
        }

        // Inline ADB path editing state
        app.settings_section = SettingsSection::AdbSdk;
        app.settings_editing_adb_path = true;
        app.settings_adb_input = "/custom/adb".to_string();
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
    }

    #[test]
    fn test_footer_displays_settings_option_on_narrow_terminal() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        terminal.draw(|f| render(f, &app)).unwrap();

        let buf = terminal.backend().buffer();
        let bottom_row: String = (0..80)
            .map(|x| buf[(x, 22)].symbol())
            .collect();
        assert!(
            bottom_row.contains("[s] ⚙"),
            "Expected footer to display [s] ⚙, got: {bottom_row}"
        );
    }

    #[test]
    fn test_action_cards_render_braille_watermarks_without_clipping_titles() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        terminal.draw(|f| render(f, &app)).unwrap();

        let buf = terminal.backend().buffer();
        let all_text: String = (0..36)
            .map(|y| (0..120).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        // Title remains intact and Braille dot watermark chars are present in the action deck.
        assert!(all_text.contains("Build & Install"));
        assert!(all_text
            .chars()
            .any(|c| ('\u{2801}'..='\u{28FF}').contains(&c)));

        // Also ensure compact terminal sizes render safely.
        let small_backend = TestBackend::new(80, 22);
        let mut small_term = Terminal::new(small_backend).unwrap();
        assert!(small_term.draw(|f| render(f, &app)).is_ok());
    }
}
