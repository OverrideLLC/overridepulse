use std::collections::VecDeque;
use std::path::PathBuf;

use crate::android::{AndroidProject, Device, LogEntry, QuickAction};

pub const DEFAULT_MAX_BUFFER_LINES: usize = 2000;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Devices,
    Actions,
    Output,
    Logcat,
}

impl Panel {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Devices => " Connected Devices ",
            Self::Actions => " Quick Actions ",
            Self::Output => " Build & Process Output ",
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
}

pub struct AppState {
    pub project: AndroidProject,
    pub nearby_projects: Vec<PathBuf>,
    pub selected_project_index: usize,
    pub project_selector_active: bool,

    pub devices: Vec<Device>,
    pub selected_device_index: usize,
    pub target_device_serial: Option<String>,

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
            current_panel: Panel::Devices,
            view_mode: ViewMode::BuildOutput,
            search_active: false,
            search_query: String::new(),
            help_active: false,
            console_output: VecDeque::with_capacity(DEFAULT_MAX_BUFFER_LINES),
            max_console_lines: DEFAULT_MAX_BUFFER_LINES,
            logcat_entries: VecDeque::with_capacity(DEFAULT_MAX_BUFFER_LINES),
            max_logcat_lines: DEFAULT_MAX_BUFFER_LINES,
            logcat_filter_by_app: false,
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
    }

    pub fn finish_task(&mut self, success: bool, message: impl Into<String>) {
        let msg = message.into();
        self.process_state.is_running = false;
        self.process_state.task_description = None;
        self.process_state.progress_percent = None;
        self.status_message = Some(if success {
            format!("Success: {msg}")
        } else {
            format!("Failed: {msg}")
        });
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
            Panel::Devices => Panel::Actions,
            Panel::Actions => {
                if self.view_mode == ViewMode::BuildOutput {
                    Panel::Output
                } else {
                    Panel::Logcat
                }
            }
            Panel::Output | Panel::Logcat => Panel::Devices,
        };
    }

    pub fn prev_panel(&mut self) {
        self.current_panel = match self.current_panel {
            Panel::Devices => {
                if self.view_mode == ViewMode::BuildOutput {
                    Panel::Output
                } else {
                    Panel::Logcat
                }
            }
            Panel::Actions => Panel::Devices,
            Panel::Output | Panel::Logcat => Panel::Actions,
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
            Panel::Output => {
                self.auto_scroll_console = false;
                self.console_scroll = self.console_scroll.saturating_sub(1);
            }
            Panel::Logcat => {
                self.auto_scroll_logcat = false;
                self.logcat_scroll = self.logcat_scroll.saturating_sub(1);
            }
            Panel::Devices => self.select_prev_device(),
            Panel::Actions => self.select_prev_action(),
        }
    }

    pub fn scroll_down(&mut self) {
        match self.current_panel {
            Panel::Output => {
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
        assert_eq!(app.current_panel, Panel::Actions);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::Output);

        app.next_panel();
        assert_eq!(app.current_panel, Panel::Devices);

        app.prev_panel();
        assert_eq!(app.current_panel, Panel::Output);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut app = AppState::new(PathBuf::from("."));
        assert!(!app.process_state.is_running);

        app.start_task("Gradle build");
        assert!(app.process_state.is_running);
        assert_eq!(
            app.process_state.task_description.as_deref(),
            Some("Gradle build")
        );

        app.finish_task(true, "Build complete");
        assert!(!app.process_state.is_running);
        assert!(app.status_message.unwrap().contains("Success"));
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
    fn test_logcat_filter_toggle() {
        let mut app = AppState::new(PathBuf::from("."));
        assert!(!app.logcat_filter_by_app);
        app.toggle_logcat_filter();
        assert!(app.logcat_filter_by_app);
        app.toggle_logcat_filter();
        assert!(!app.logcat_filter_by_app);
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
