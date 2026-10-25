use std::collections::VecDeque;
use std::path::PathBuf;

use crate::android::{AndroidProject, Device, DeviceAction, LogEntry, QuickAction};

pub const DEFAULT_MAX_BUFFER_LINES: usize = 2000;
pub const DEFAULT_MAX_SIMPLIFIED_EVENTS: usize = 100;

pub type DeviceControlAction = DeviceAction;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Devices,
    DeviceControls,
    Actions,
    Console,
    Output,
    Logcat,
}

impl Panel {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Devices => " Connected Devices ",
            Self::DeviceControls => " Device Controls (ADB) ",
            Self::Actions => " App & Gradle Actions ",
            Self::Console | Self::Output => " Build & Process Output ",
            Self::Logcat => " Live Logcat ",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    BuildOutput,
    LogcatView,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessState {
    pub is_running: bool,
    pub spinner_index: usize,
    pub task_description: Option<String>,
    pub progress_percent: Option<u8>,
    pub started_at: Option<std::time::Instant>,
    pub last_duration_secs: Option<f64>,
}

impl ProcessState {
    pub fn spinner_char(&self) -> &'static str {
        if self.is_running {
            SPINNER_FRAMES[self.spinner_index % SPINNER_FRAMES.len()]
        } else {
            " "
        }
    }

    pub fn tick(&mut self) {
        if self.is_running {
            self.spinner_index = (self.spinner_index + 1) % SPINNER_FRAMES.len();
        }
    }

    pub fn elapsed_and_eta_display(&self) -> String {
        if self.is_running {
            let elapsed = self.started_at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
            if let Some(pct) = self.progress_percent {
                if pct > 0 && pct < 100 {
                    let total_estimated = elapsed / (pct as f64 / 100.0);
                    let eta = (total_estimated - elapsed).max(0.0);
                    return format!("Elapsed: {:.1}s │ ETA: ~{:.1}s", elapsed, eta);
                }
            }
            if let Some(last_dur) = self.last_duration_secs {
                let eta = (last_dur - elapsed).max(0.0);
                format!("Elapsed: {:.1}s │ ETA: ~{:.1}s", elapsed, eta)
            } else {
                format!("Elapsed: {:.1}s │ ETA: calculating...", elapsed)
            }
        } else if let Some(last_dur) = self.last_duration_secs {
            format!("Last duration: {:.1}s", last_dur)
        } else {
            "Ready / Idle".to_string()
        }
    }
}

pub struct AppState {
    pub project: AndroidProject,
    pub nearby_projects: Vec<PathBuf>,
    pub selected_project_index: usize,
    pub project_selector_active: bool,

    pub devices: Vec<Device>,
    pub selected_device_index: usize,
    pub target_device_serial: Option<String>,

    pub selected_device_control_index: usize,

    pub adb_connect_modal_active: bool,
    pub adb_pair_ip_port: String,
    pub adb_pair_code: String,
    pub adb_connect_ip_port: String,
    pub adb_modal_field_index: usize,

    pub show_simplified_pane: bool,
    pub show_build_pane: bool,
    pub show_logcat_pane: bool,
    pub simplified_events: VecDeque<String>,
    pub max_simplified_events: usize,

    pub current_panel: Panel,
    pub view_mode: ViewMode,

    pub search_active: bool,
    pub search_query: String,
    pub help_active: bool,

    pub console_output: VecDeque<String>,
    pub max_console_lines: usize,

    pub logcat_entries: VecDeque<LogEntry>,
    pub max_logcat_lines: usize,
    pub logcat_filter_by_app: bool,

    pub process_state: ProcessState,
    pub status_message: Option<String>,

    pub actions: Vec<QuickAction>,
    pub selected_action_index: usize,

    pub console_scroll: usize,
    pub logcat_scroll: usize,
    pub auto_scroll_console: bool,
    pub auto_scroll_logcat: bool,

    pub should_quit: bool,
}

impl AppState {
    pub fn new(root_path: PathBuf) -> Self {
        let project = AndroidProject::detect(&root_path);
        let is_valid = project.is_valid();
        let nearby_projects = if !is_valid {
            AndroidProject::discover_nearby_projects(&root_path, 3)
        } else {
            Vec::new()
        };
        let project_selector_active = !is_valid;

        Self {
            project,
            nearby_projects,
            selected_project_index: 0,
            project_selector_active,
            devices: Vec::new(),
            selected_device_index: 0,
            target_device_serial: None,
            selected_device_control_index: 0,
            adb_connect_modal_active: false,
            adb_pair_ip_port: "192.168.1.50:37123".to_string(),
            adb_pair_code: String::new(),
            adb_connect_ip_port: "192.168.1.50:5555".to_string(),
            adb_modal_field_index: 0,
            show_simplified_pane: true,
            show_build_pane: false,
            show_logcat_pane: true,
            simplified_events: VecDeque::with_capacity(DEFAULT_MAX_SIMPLIFIED_EVENTS),
            max_simplified_events: DEFAULT_MAX_SIMPLIFIED_EVENTS,
            current_panel: Panel::Devices,
            view_mode: ViewMode::BuildOutput,
            search_active: false,
            search_query: String::new(),
            help_active: false,
            console_output: VecDeque::with_capacity(DEFAULT_MAX_BUFFER_LINES),
            max_console_lines: DEFAULT_MAX_BUFFER_LINES,
            logcat_entries: VecDeque::with_capacity(DEFAULT_MAX_BUFFER_LINES),
            max_logcat_lines: DEFAULT_MAX_BUFFER_LINES,
            logcat_filter_by_app: true,
            process_state: ProcessState::default(),
            status_message: Some("Ready".to_string()),
            actions: QuickAction::all().to_vec(),
            selected_action_index: 0,
            console_scroll: 0,
            logcat_scroll: 0,
            auto_scroll_console: true,
            auto_scroll_logcat: true,
            should_quit: false,
        }
    }

    pub fn selected_device(&self) -> Option<&Device> {
        self.devices.get(self.selected_device_index)
    }

    pub fn selected_action(&self) -> Option<QuickAction> {
        self.actions.get(self.selected_action_index).copied()
    }

    pub fn selected_device_control(&self) -> Option<DeviceControlAction> {
        DeviceControlAction::all().get(self.selected_device_control_index).copied()
    }

    pub fn select_next_device_control(&mut self) {
        let len = DeviceControlAction::all().len();
        if len > 0 {
            self.selected_device_control_index = (self.selected_device_control_index + 1) % len;
        }
    }

    pub fn select_prev_device_control(&mut self) {
        let len = DeviceControlAction::all().len();
        if len > 0 {
            if self.selected_device_control_index == 0 {
                self.selected_device_control_index = len - 1;
            } else {
                self.selected_device_control_index -= 1;
            }
        }
    }

    pub fn toggle_adb_connect_modal(&mut self) {
        self.adb_connect_modal_active = !self.adb_connect_modal_active;
    }

    pub fn next_adb_modal_field(&mut self) {
        self.adb_modal_field_index = (self.adb_modal_field_index + 1) % 3;
    }

    pub fn prev_adb_modal_field(&mut self) {
        if self.adb_modal_field_index == 0 {
            self.adb_modal_field_index = 2;
        } else {
            self.adb_modal_field_index -= 1;
        }
    }

    pub fn toggle_simplified_pane(&mut self) {
        if self.show_simplified_pane && !self.show_build_pane && !self.show_logcat_pane {
            return;
        }
        self.show_simplified_pane = !self.show_simplified_pane;
        self.sync_view_mode_from_panes();
    }

    pub fn toggle_build_pane(&mut self) {
        if self.show_build_pane && !self.show_simplified_pane && !self.show_logcat_pane {
            return;
        }
        self.show_build_pane = !self.show_build_pane;
        self.sync_view_mode_from_panes();
    }

    pub fn toggle_logcat_pane(&mut self) {
        if self.show_logcat_pane && !self.show_simplified_pane && !self.show_build_pane {
            return;
        }
        self.show_logcat_pane = !self.show_logcat_pane;
        self.sync_view_mode_from_panes();
    }

    pub fn cycle_console_preset(&mut self) {
        match (self.show_simplified_pane, self.show_build_pane, self.show_logcat_pane) {
            (true, false, false) => {
                // Preset 2: Solo Build
                self.show_simplified_pane = false;
                self.show_build_pane = true;
                self.show_logcat_pane = false;
            }
            (false, true, false) => {
                // Preset 3: Solo Logcat
                self.show_simplified_pane = false;
                self.show_build_pane = false;
                self.show_logcat_pane = true;
            }
            (false, false, true) => {
                // Preset 4: Paridad Simplificada + Logcat
                self.show_simplified_pane = true;
                self.show_build_pane = false;
                self.show_logcat_pane = true;
            }
            (true, false, true) => {
                // Preset 5: Triple Vista
                self.show_simplified_pane = true;
                self.show_build_pane = true;
                self.show_logcat_pane = true;
            }
            _ => {
                // Preset 1: Solo Simplificada
                self.show_simplified_pane = true;
                self.show_build_pane = false;
                self.show_logcat_pane = false;
            }
        }
        self.sync_view_mode_from_panes();
    }

    fn sync_view_mode_from_panes(&mut self) {
        if self.show_build_pane && !self.show_logcat_pane {
            self.view_mode = ViewMode::BuildOutput;
        } else if self.show_logcat_pane && !self.show_build_pane {
            self.view_mode = ViewMode::LogcatView;
        }
    }

    pub fn append_simplified_event(&mut self, event: impl Into<String>) {
        let msg = event.into();
        while self.simplified_events.len() >= self.max_simplified_events {
            self.simplified_events.pop_front();
        }
        self.simplified_events.push_back(msg);
    }

    pub fn selected_project(&self) -> Option<&PathBuf> {
        self.nearby_projects.get(self.selected_project_index)
    }

    pub fn select_next_project(&mut self) {
        if !self.nearby_projects.is_empty() {
            self.selected_project_index =
                (self.selected_project_index + 1) % self.nearby_projects.len();
        }
    }

    pub fn select_prev_project(&mut self) {
        if !self.nearby_projects.is_empty() {
            if self.selected_project_index == 0 {
                self.selected_project_index = self.nearby_projects.len() - 1;
            } else {
                self.selected_project_index -= 1;
            }
        }
    }

    pub fn switch_project(&mut self, path: PathBuf) {
        self.project = AndroidProject::detect(&path);
        self.project_selector_active = false;
        self.status_message = Some(format!("Project: {}", self.project.name));
    }

    pub fn toggle_project_selector(&mut self) {
        self.project_selector_active = !self.project_selector_active;
        if self.project_selector_active && self.nearby_projects.is_empty() {
            self.nearby_projects =
                AndroidProject::discover_nearby_projects(&self.project.root_path, 3);
            self.selected_project_index = 0;
        }
    }

    pub fn next_variant(&mut self) {
        if self.project.variants.is_empty() {
            return;
        }
        let current_idx = self
            .project
            .variants
            .iter()
            .position(|v| v == &self.project.active_variant)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % self.project.variants.len();
        self.project.active_variant = self.project.variants[next_idx].clone();
        self.status_message = Some(format!("Variant: {}", self.project.active_variant));
    }

    pub fn toggle_logcat_filter(&mut self) {
        self.logcat_filter_by_app = !self.logcat_filter_by_app;
        let state = if self.logcat_filter_by_app { "ON" } else { "OFF" };
        let app_id = self.project.application_id.as_deref().unwrap_or("app");
        self.status_message = Some(format!("Logcat filter ({app_id}): {state}"));
    }

    pub fn append_console_line(&mut self, line: String) {
        if let Some(pct) = crate::android::parse_gradle_progress(&line) {
            self.process_state.progress_percent = Some(pct);
        }

        while self.console_output.len() >= self.max_console_lines {
            self.console_output.pop_front();
        }
        self.console_output.push_back(line);
        if self.auto_scroll_console {
            self.console_scroll = self.console_output.len().saturating_sub(1);
        }
    }

    pub fn append_logcat_entry(&mut self, entry: LogEntry) {
        while self.logcat_entries.len() >= self.max_logcat_lines {
            self.logcat_entries.pop_front();
        }
        self.logcat_entries.push_back(entry);
        if self.auto_scroll_logcat {
            self.logcat_scroll = self.logcat_entries.len().saturating_sub(1);
        }
    }

    pub fn clear_console(&mut self) {
        self.console_output.clear();
        self.console_scroll = 0;
    }

    pub fn clear_logcat(&mut self) {
        self.logcat_entries.clear();
        self.logcat_scroll = 0;
    }

    pub fn start_task(&mut self, task: impl Into<String>) {
        let task_desc = task.into();
        self.status_message = Some(format!("Running: {task_desc}"));
        self.process_state.is_running = true;
        self.process_state.task_description = Some(task_desc);
        self.process_state.progress_percent = None;
        self.process_state.started_at = Some(std::time::Instant::now());
    }

    pub fn finish_task(&mut self, success: bool, message: impl Into<String>) {
        let msg = message.into();
        let dur = self.process_state.started_at.map(|t| t.elapsed().as_secs_f64());
        self.process_state.last_duration_secs = dur;
        self.process_state.started_at = None;
        self.process_state.is_running = false;
        self.process_state.task_description = None;
        self.process_state.progress_percent = None;

        let dur_tag = dur.map(|d| format!(" ({:.1}s)", d)).unwrap_or_default();
        if success {
            self.append_simplified_event(format!("✅ {msg}{dur_tag}"));
            self.status_message = Some(format!("Success: {msg}"));
        } else {
            self.append_simplified_event(format!("❌ {msg}{dur_tag}"));
            self.status_message = Some(format!("Failed: {msg}"));
        }
    }

    pub fn select_device_by_serial(&mut self, serial: &str) -> bool {
        if let Some(idx) = self.devices.iter().position(|d| d.serial == serial) {
            self.selected_device_index = idx;
            true
        } else {
            false
        }
    }

    pub fn set_devices(&mut self, devices: Vec<Device>) {
        self.devices = devices;
        if let Some(ref target) = self.target_device_serial {
            if let Some(idx) = self.devices.iter().position(|d| &d.serial == target) {
                self.selected_device_index = idx;
                return;
            }
        }
        if self.selected_device_index >= self.devices.len() && !self.devices.is_empty() {
            self.selected_device_index = self.devices.len() - 1;
        }
    }

    pub fn tick(&mut self) {
        self.process_state.tick();
    }

    pub fn next_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Devices => Panel::DeviceControls,
            Panel::DeviceControls => Panel::Actions,
            Panel::Actions => Panel::Console,
            Panel::Console | Panel::Output | Panel::Logcat => Panel::Devices,
        };
    }

    pub fn prev_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Devices => Panel::Console,
            Panel::DeviceControls => Panel::Devices,
            Panel::Actions => Panel::DeviceControls,
            Panel::Console | Panel::Output | Panel::Logcat => Panel::Actions,
        };
    }

    pub fn select_next_device(&mut self) {
        if !self.devices.is_empty() {
            self.selected_device_index = (self.selected_device_index + 1) % self.devices.len();
        }
    }

    pub fn select_prev_device(&mut self) {
        if !self.devices.is_empty() {
            if self.selected_device_index == 0 {
                self.selected_device_index = self.devices.len() - 1;
            } else {
                self.selected_device_index -= 1;
            }
        }
    }

    pub fn select_next_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action_index = (self.selected_action_index + 1) % self.actions.len();
        }
    }

    pub fn select_prev_action(&mut self) {
        if !self.actions.is_empty() {
            if self.selected_action_index == 0 {
                self.selected_action_index = self.actions.len() - 1;
            } else {
                self.selected_action_index -= 1;
            }
        }
    }

    pub fn toggle_help(&mut self) {
        self.help_active = !self.help_active;
    }

    pub fn toggle_search(&mut self) {
        self.search_active = !self.search_active;
        if !self.search_active {
            self.search_query.clear();
        }
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::BuildOutput => {
                if self.current_panel == Panel::Output {
                    self.current_panel = Panel::Logcat;
                }
                ViewMode::LogcatView
            }
            ViewMode::LogcatView => {
                if self.current_panel == Panel::Logcat {
                    self.current_panel = Panel::Output;
                }
                ViewMode::BuildOutput
            }
        };
    }

    pub fn scroll_up(&mut self) {
        match self.current_panel {
            Panel::Output | Panel::Console => {
                self.auto_scroll_console = false;
                self.console_scroll = self.console_scroll.saturating_sub(1);
            }
            Panel::Logcat => {
                self.auto_scroll_logcat = false;
                self.logcat_scroll = self.logcat_scroll.saturating_sub(1);
            }
            Panel::Devices => self.select_prev_device(),
            Panel::DeviceControls => self.select_prev_device_control(),
            Panel::Actions => self.select_prev_action(),
        }
    }

    pub fn scroll_down(&mut self) {
        match self.current_panel {
            Panel::Output | Panel::Console => {
                let max_scroll = self.console_output.len().saturating_sub(1);
                if self.console_scroll < max_scroll {
                    self.console_scroll += 1;
                }
                if self.console_scroll >= max_scroll {
                    self.auto_scroll_console = true;
                }
            }
            Panel::Logcat => {
                let max_scroll = self.logcat_entries.len().saturating_sub(1);
                if self.logcat_scroll < max_scroll {
                    self.logcat_scroll += 1;
                }
                if self.logcat_scroll >= max_scroll {
                    self.auto_scroll_logcat = true;
                }
            }
            Panel::Devices => self.select_next_device(),
            Panel::DeviceControls => self.select_next_device_control(),
            Panel::Actions => self.select_next_action(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_buffer_capacity() {
        let mut app = AppState::new(PathBuf::from("."));
        app.max_console_lines = 5;

        for i in 0..10 {
            app.append_console_line(format!("Line {i}"));
        }

        assert_eq!(app.console_output.len(), 5);
        assert_eq!(app.console_output.front().unwrap(), "Line 5");
        assert_eq!(app.console_output.back().unwrap(), "Line 9");
    }

    #[test]
    fn test_panel_cycling() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.current_panel, Panel::Devices);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::DeviceControls);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::Actions);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::Console);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::Devices);

        app.prev_panel();
        assert_eq!(app.current_panel, Panel::Console);

        app.prev_panel();
        assert_eq!(app.current_panel, Panel::Actions);

        app.prev_panel();
        assert_eq!(app.current_panel, Panel::DeviceControls);

        app.prev_panel();
        assert_eq!(app.current_panel, Panel::Devices);
    }

    #[test]
    fn test_task_lifecycle_and_eta() {
        let mut app = AppState::new(PathBuf::from("."));
        assert!(!app.process_state.is_running);
        assert_eq!(app.process_state.elapsed_and_eta_display(), "Ready / Idle");

        app.start_task("Gradle build");
        assert!(app.process_state.is_running);
        assert!(app.process_state.started_at.is_some());
        assert_eq!(
            app.process_state.task_description.as_deref(),
            Some("Gradle build")
        );

        app.process_state.progress_percent = Some(50);
        let display = app.process_state.elapsed_and_eta_display();
        assert!(display.contains("Elapsed:"));
        assert!(display.contains("ETA:"));

        app.finish_task(true, "Build complete");
        assert!(!app.process_state.is_running);
        assert!(app.status_message.unwrap().contains("Success"));
        assert!(app.process_state.last_duration_secs.is_some());
        assert_eq!(app.simplified_events.len(), 1);
        assert!(app.simplified_events[0].contains("✅ Build complete"));
    }

    #[test]
    fn test_append_console_line_progress() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.process_state.progress_percent, None);

        app.append_console_line("<==========---> 75% EXECUTING [10s]".to_string());
        assert_eq!(app.process_state.progress_percent, Some(75));

        app.append_console_line("Just normal output".to_string());
        assert_eq!(app.process_state.progress_percent, Some(75));
    }

    #[test]
    fn test_next_variant_cycling() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project.variants = vec![
            "debug".to_string(),
            "release".to_string(),
            "staging".to_string(),
        ];
        app.project.active_variant = "debug".to_string();

        app.next_variant();
        assert_eq!(app.project.active_variant, "release");

        app.next_variant();
        assert_eq!(app.project.active_variant, "staging");

        app.next_variant();
        assert_eq!(app.project.active_variant, "debug");
    }

    #[test]
    fn test_quick_actions_initialized() {
        let app = AppState::new(PathBuf::from("."));
        assert_eq!(app.actions.len(), QuickAction::all().len());
        assert_eq!(app.selected_action(), Some(QuickAction::BuildAndInstallDebug));
    }

    #[test]
    fn test_device_controls_cycling() {
        let mut app = AppState::new(PathBuf::from("."));
        assert_eq!(app.selected_device_control_index, 0);
        assert_eq!(
            app.selected_device_control(),
            Some(DeviceAction::WirelessPairAndConnect)
        );

        app.select_next_device_control();
        assert_eq!(app.selected_device_control_index, 1);
        assert_eq!(
            app.selected_device_control(),
            Some(DeviceAction::EnableTcpip5555)
        );

        app.select_prev_device_control();
        assert_eq!(app.selected_device_control_index, 0);

        app.select_prev_device_control();
        assert_eq!(
            app.selected_device_control_index,
            DeviceAction::all().len() - 1
        );
    }

    #[test]
    fn test_logcat_filter_toggle() {
        let mut app = AppState::new(PathBuf::from("."));
        assert!(app.logcat_filter_by_app); // Defaults to true
        app.toggle_logcat_filter();
        assert!(!app.logcat_filter_by_app);
        app.toggle_logcat_filter();
        assert!(app.logcat_filter_by_app);
    }

    #[test]
    fn test_parity_toggles_and_presets() {
        let mut app = AppState::new(PathBuf::from("."));
        // Default: Simplified (true), Build (false), Logcat (true)
        assert!(app.show_simplified_pane);
        assert!(!app.show_build_pane);
        assert!(app.show_logcat_pane);

        // Cycle preset -> Triple Vista
        app.cycle_console_preset();
        assert!(app.show_simplified_pane);
        assert!(app.show_build_pane);
        assert!(app.show_logcat_pane);

        // Cycle preset -> Solo Simplificada
        app.cycle_console_preset();
        assert!(app.show_simplified_pane);
        assert!(!app.show_build_pane);
        assert!(!app.show_logcat_pane);

        // Cannot toggle off the only active pane
        app.toggle_simplified_pane();
        assert!(app.show_simplified_pane);

        // Toggle on build
        app.toggle_build_pane();
        assert!(app.show_build_pane);
        // Now can toggle off simplified
        app.toggle_simplified_pane();
        assert!(!app.show_simplified_pane);
        assert!(app.show_build_pane);
    }

    #[test]
    fn test_adb_connect_modal_fields() {
        let mut app = AppState::new(PathBuf::from("."));
        assert!(!app.adb_connect_modal_active);
        app.toggle_adb_connect_modal();
        assert!(app.adb_connect_modal_active);

        assert_eq!(app.adb_modal_field_index, 0);
        app.next_adb_modal_field();
        assert_eq!(app.adb_modal_field_index, 1);
        app.next_adb_modal_field();
        assert_eq!(app.adb_modal_field_index, 2);
        app.next_adb_modal_field();
        assert_eq!(app.adb_modal_field_index, 0);

        app.prev_adb_modal_field();
        assert_eq!(app.adb_modal_field_index, 2);
    }

    #[test]
    fn test_device_selection_and_target_serial() {
        let mut app = AppState::new(PathBuf::from("."));
        app.target_device_serial = Some("target-device-123".to_string());

        let dev1 = Device::new("other-device-456", crate::android::DeviceState::Device);
        let dev2 = Device::new("target-device-123", crate::android::DeviceState::Device);

        app.set_devices(vec![dev1, dev2]);
        assert_eq!(app.selected_device_index, 1);
        assert_eq!(app.selected_device().map(|d| d.serial.as_str()), Some("target-device-123"));

        assert!(app.select_device_by_serial("other-device-456"));
        assert_eq!(app.selected_device_index, 0);
        assert!(!app.select_device_by_serial("non-existent"));
    }
}
