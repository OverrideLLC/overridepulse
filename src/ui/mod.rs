pub mod action_deck;
pub mod components;
pub mod device_panel;
pub mod logcat_view;
pub mod terminal_log;
pub mod theme;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use crate::app::{AppState, ViewMode};

/// Renders the complete application user interface on the given Ratatui frame,
/// following the PRD Sections 2, 5, and 6 layout specifications:
/// - Top header
/// - Body top row (Horizontal split 50%/50%: Devices panel / Quick Actions deck)
/// - Body bottom row (Build Output terminal / Live Logcat stream)
/// - Bottom footer
/// - Modals / Overlays: Project selector, Help, and Search
pub fn render(frame: &mut Frame, app: &AppState) {
    let size = frame.area();

    // Main vertical layout: Header, Body, Footer
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header superior
            Constraint::Min(5),    // Cuerpo
            Constraint::Length(3), // Footer inferior
        ])
        .split(size);

    let header_area = vertical_chunks[0];
    let body_area = vertical_chunks[1];
    let footer_area = vertical_chunks[2];

    // Split body vertically into:
    // - Fila superior: Devices panel (left 50%) and Action deck (right 50%)
    // - Fila inferior: Terminal log or Live Logcat
    let body_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(42), // Fila superior
            Constraint::Percentage(58), // Fila inferior
        ])
        .split(body_area);

    let top_row_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Izquierda: device_panel
            Constraint::Percentage(50), // Derecha: action_deck
        ])
        .split(body_rows[0]);

    let devices_area = top_row_cols[0];
    let actions_area = top_row_cols[1];
    let bottom_area = body_rows[1];

    // 1. Render Header
    components::header::render(frame, app, header_area);

    // 2. Render Fila superior del cuerpo
    device_panel::render(frame, app, devices_area);
    action_deck::render(frame, app, actions_area);

    // 3. Render Fila inferior del cuerpo
    match app.view_mode {
        ViewMode::BuildOutput => terminal_log::render(frame, app, bottom_area),
        ViewMode::LogcatView => logcat_view::render(frame, app, bottom_area),
    }

    // 4. Render Footer
    components::footer::render(frame, app, footer_area);

    // 5. Overlays (Priority: Project Selector > Help > Search)
    if app.project_selector_active {
        components::project_selector::render(frame, app, size);
    } else if app.help_active {
        components::help::render(frame, size);
    } else if app.search_active {
        components::search::render(frame, app, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;
    use crate::android::{Device, DeviceState, LogEntry, LogLevel};

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
    fn test_render_logcat_view() {
        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.view_mode = ViewMode::LogcatView;
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

        // 2. Help overlay
        app.project_selector_active = false;
        app.help_active = true;
        assert!(terminal.draw(|f| render(f, &app)).is_ok());

        // 3. Search overlay
        app.help_active = false;
        app.search_active = true;
        app.search_query = "MainActivity".to_string();
        assert!(terminal.draw(|f| render(f, &app)).is_ok());
    }
}
