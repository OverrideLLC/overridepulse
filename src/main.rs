use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::Show,
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::UnboundedSender;

use overridepulse::{
    android::{
        device::spawn_device_monitor, ActionContext, AndroidProject, DeviceAction, LogcatStream,
        ProcessRunner, QuickAction,
    },
    app::{
        AppState, ClickTarget, FooterAction, Panel, SettingsFocus, SettingsSection, ViewMode,
    },
    clipboard::{self, CopyMethod},
    config::{SdkConfig, UserConfig},
    event::{AppEvent, EventHandler},
    i18n::Language,
    ui::{self, theme::ThemePreset},
};

#[derive(Parser, Debug)]
#[command(
    name = "overridepulse",
    author = "OverridePulse Contributors",
    version,
    about = "Ultralightweight TUI for Android, Kotlin Multiplatform (KMP) & Gradle development"
)]
struct Cli {
    /// Path to Android / KMP project root (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    project: Option<PathBuf>,

    /// Target device serial number
    #[arg(short, long)]
    device: Option<String>,

    /// UI tick rate in milliseconds (default: 100)
    #[arg(long, default_value_t = 100)]
    tick_rate: u64,

    /// Show the onboarding and theme setup wizard
    #[arg(long)]
    onboarding: bool,
}

/// RAII Guard ensuring terminal restoration even if exiting via early return or error.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
    }
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Resolve project root
    let root_path = match cli.project {
        Some(path) => path.canonicalize().unwrap_or(path),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    // 2. Detect Android SDK and tools
    let sdk_config = SdkConfig::detect();
    let user_config = UserConfig::load();
    let mut adb_path = sdk_config.effective_adb_path(user_config.custom_adb_path.as_deref());

    // 3. Initialize terminal and panic hook
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 4. Initialize State and Event Handler.
    // Project detection walks Gradle files and build outputs, so it runs off
    // the UI thread. The first frame paints immediately.
    let mut app = AppState::pending(root_path.clone());
    app.sdk_config = sdk_config.clone();
    app.custom_adb_path = user_config.custom_adb_path.clone();
    app.adb_path = adb_path.clone();
    app.target_device_serial = cli.device.clone();
    app.set_theme(user_config.theme);
    app.set_language(user_config.language);
    if cli.onboarding || !user_config.onboarding_completed {
        app.open_onboarding();
    }
    if let Ok(area) = terminal.size() {
        app.terminal_width = area.width;
    }

    app.append_console_line(format!("⚡ OverridePulse · {}", root_path.display()));
    if !SdkConfig::is_adb_path_reachable(&adb_path) {
        app.append_console_line("ℹ adb not found in PATH (host KMP targets still available)".to_string());
    }
    if let Some(ref target) = cli.device {
        app.append_console_line(format!("🎯 Target device: {target}"));
    }

    let mut event_handler = EventHandler::new(Duration::from_millis(cli.tick_rate));
    let sender = event_handler.sender();
    let load_generation = app.project_load_generation;
    spawn_project_load(root_path.clone(), load_generation, sender.clone());
    app.discovery_root = root_path.clone();
    spawn_project_discovery(root_path, sender.clone());

    // 5. Spawn background ADB device monitor (periodic refresh every 3s)
    let mut device_monitor_handle =
        spawn_device_monitor(adb_path.clone(), sender.clone(), Duration::from_secs(3));

    // 6. Asynchronous execution engines
    let mut runner = ProcessRunner::new();
    let mut logcat_stream = LogcatStream::new();

    // 7. Main event loop (dirty-flag + burst coalescing avoids idle 10Hz redraws)
    const MAX_EVENT_BATCH: usize = 256;
    let mut needs_redraw = true;
    while !app.should_quit {
        if needs_redraw {
            terminal.draw(|f| ui::render(f, &app))?;
            needs_redraw = false;
        }

        if let Some(event) = event_handler.next().await {
            if process_app_event(
                event,
                &mut app,
                &mut runner,
                &mut logcat_stream,
                &adb_path,
                &sender,
            ) {
                needs_redraw = true;
            }
            for _ in 1..MAX_EVENT_BATCH {
                if app.should_quit {
                    break;
                }
                let Some(extra) = event_handler.try_next() else {
                    break;
                };
                if process_app_event(
                    extra,
                    &mut app,
                    &mut runner,
                    &mut logcat_stream,
                    &adb_path,
                    &sender,
                ) {
                    needs_redraw = true;
                }
            }
            if app.adb_path != adb_path {
                adb_path = app.adb_path.clone();
                device_monitor_handle.abort();
                device_monitor_handle =
                    spawn_device_monitor(adb_path.clone(), sender.clone(), Duration::from_secs(3));
                sync_logcat_stream(&app, &mut logcat_stream, &adb_path, &sender);
                needs_redraw = true;
            }
        }
    }

    // Clean up background stream
    device_monitor_handle.abort();
    logcat_stream.stop();
    runner.abort();

    Ok(())
}

/// Processes one `AppEvent` and returns `true` if the UI should be redrawn.
fn process_app_event(
    event: AppEvent,
    app: &mut AppState,
    runner: &mut ProcessRunner,
    logcat_stream: &mut LogcatStream,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) -> bool {
    match event {
        AppEvent::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return false;
            }
            handle_key_event(app, runner, logcat_stream, key, adb_path, sender);
            true
        }
        AppEvent::Tick => {
            if app.process_state.is_running {
                app.tick();
                true
            } else {
                false
            }
        }
        AppEvent::ProcessOutput(line) => {
            app.append_console_line(line);
            true
        }
        AppEvent::ProcessFinished(success, message) => {
            app.finish_task(success, message);
            if app.project.application_id.is_none()
                && app.project.active_platform().is_android()
            {
                let project = app.project.clone();
                let generation = app.project_load_generation;
                let tx = sender.clone();
                tokio::task::spawn_blocking(move || {
                    if let Some(application_id) = project.resolve_effective_application_id() {
                        let _ = tx.send(AppEvent::ApplicationIdResolved {
                            generation,
                            application_id,
                        });
                    }
                });
            }
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
            true
        }
        AppEvent::LogcatLine(entry) => {
            app.append_logcat_entry(entry);
            true
        }
        AppEvent::DevicesUpdated(devices) => {
            let prev_serial = app.selected_device().map(|d| d.serial.clone());
            app.set_devices(devices);
            let new_serial = app.selected_device().map(|d| d.serial.clone());

            // If device changed while streaming logcat or newly discovered, sync stream
            if prev_serial != new_serial {
                sync_logcat_stream(app, logcat_stream, adb_path, sender);
            }
            true
        }
        AppEvent::ProjectLoaded {
            generation,
            project,
        } => {
            if generation != app.project_load_generation {
                return false;
            }
            let previous_root = app.project.root_path.clone();
            let resolved_root = project.root_path.clone();
            let valid = project.is_valid();
            app.apply_loaded_project(project);
            app.append_console_line(format!(
                "📂 Project: {} ({})",
                app.project.name,
                app.project.root_path.display()
            ));
            if let Some(id) = &app.project.application_id {
                app.append_console_line(format!("📦 applicationId: {id}"));
            }
            if app.project.is_kmp && !app.project.targets.is_empty() {
                let summary = app
                    .project
                    .targets
                    .iter()
                    .map(|t| t.display_label())
                    .collect::<Vec<_>>()
                    .join(", ");
                app.append_console_line(format!("🧩 KMP targets: {summary}"));
            }
            if !valid {
                app.project_selector_active = true;
                app.projects_discovering = true;
            }
            if resolved_root != previous_root {
                app.discovery_root = resolved_root.clone();
                spawn_project_discovery(resolved_root, sender.clone());
            }
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
            true
        }
        AppEvent::ProjectsDiscovered { root, projects } => {
            if root != app.discovery_root {
                return false;
            }
            app.projects_discovering = false;
            let previous = app.selected_project().cloned();
            app.nearby_projects = projects;
            if let Some(previous) = previous {
                if let Some(idx) = app
                    .nearby_projects
                    .iter()
                    .position(|path| path == &previous)
                {
                    app.selected_project_index = idx;
                }
            } else if let Some(idx) = app
                .nearby_projects
                .iter()
                .position(|path| path == &app.project.root_path)
            {
                app.selected_project_index = idx;
            }
            if !app.nearby_projects.is_empty()
                && app.selected_project_index >= app.nearby_projects.len()
            {
                app.selected_project_index = 0;
            }
            true
        }
        AppEvent::ApplicationIdResolved {
            generation,
            application_id,
        } => {
            if generation == app.project_load_generation && app.project.application_id.is_none() {
                app.append_console_line(format!("📦 applicationId: {application_id}"));
                app.project.application_id = Some(application_id);
                sync_logcat_stream(app, logcat_stream, adb_path, sender);
                true
            } else {
                false
            }
        }
        AppEvent::Resize(w, _) => {
            app.terminal_width = w;
            true
        }
        AppEvent::Mouse(mouse) => {
            handle_mouse_event(app, runner, logcat_stream, mouse, adb_path, sender)
        }
    }
}

fn build_action_context(
    app: &AppState,
    serial: &str,
    adb_path: &Path,
    include_app_id: bool,
) -> ActionContext {
    let mut ctx = ActionContext::new(&app.project.root_path, serial)
        .with_variant(&app.project.active_variant)
        .with_adb_path(adb_path);

    if include_app_id {
        // Avoid blocking the UI thread with disk walks; the async runner resolves fallback IDs off-thread.
        let cached_app_id = app
            .project
            .application_id
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                app.project
                    .namespace
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
            });
        if let Some(app_id) = cached_app_id {
            ctx = ctx.with_app_id(app_id);
        }
    }
    if let Some(ref activity) = app.project.launcher_activity {
        ctx = ctx.with_main_activity(activity);
    }
    if let Some(ref module) = app.project.active_module {
        ctx = ctx.with_module(module);
    }
    ctx
}

fn trigger_quick_action(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    _logcat_stream: &mut LogcatStream,
    action: QuickAction,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    let active_target = app.project.active_target().cloned();
    let title = action.title_for(active_target.as_ref());

    let serial = if !action.requires_adb_device() {
        "local".to_string()
    } else {
        match app.selected_device() {
            Some(dev) => dev.serial.clone(),
            None => {
                let msg = format!(
                    "⚠ Cannot execute '{}': No Android device connected or selected.",
                    title
                );
                app.append_console_line(msg);
                app.finish_task(false, "No device connected");
                return;
            }
        }
    };

    app.start_task(title.clone());

    let shortcut_str = action
        .shortcut()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "Enter".to_string());

    if !action.requires_adb_device() {
        if let Some(ref target) = active_target {
            if action != QuickAction::StopGradleDaemons && action != QuickAction::GradleBuildAll {
                app.append_console_line(format!(
                    "> [{shortcut_str}] Starting {title} ({})...",
                    target.display_label()
                ));
            } else {
                app.append_console_line(format!("> [{shortcut_str}] Starting {title}..."));
            }
        } else {
            app.append_console_line(format!("> [{shortcut_str}] Starting {title}..."));
        }
    } else {
        app.append_console_line(format!(
            "> [{shortcut_str}] Starting {title} on device '{serial}'..."
        ));
    }

    // Show build output terminal for build/process actions (except scrcpy which is a separate GUI)
    if action != QuickAction::LaunchScrcpy {
        if !app.show_build_pane && !app.show_simplified_pane {
            app.show_build_pane = true;
        }
        if app.view_mode != ViewMode::BuildOutput {
            app.view_mode = ViewMode::BuildOutput;
        }
    }

    let mut ctx = build_action_context(app, &serial, adb_path, action.requires_adb_device());
    if let Some(target) = active_target {
        ctx = ctx.with_target(target);
    }

    runner.execute_action(action, ctx, sender.clone());
}

fn trigger_device_action(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    action: DeviceAction,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    let serial = match app.selected_device() {
        Some(dev) => dev.serial.clone(),
        None => {
            let msg = format!(
                "⚠ Cannot execute '{}': No Android device connected or selected.",
                action.title()
            );
            app.append_console_line(msg);
            app.finish_task(false, "No device connected");
            return;
        }
    };

    let title = action.title().to_string();
    app.start_task(title.clone());
    app.append_console_line(format!(
        "> Starting device action '{title}' on device '{serial}'..."
    ));

    let ctx = build_action_context(app, &serial, adb_path, true);

    runner.execute_device_action(action, ctx, sender.clone());
}

fn spawn_project_load(path: PathBuf, generation: u64, sender: UnboundedSender<AppEvent>) {
    tokio::task::spawn_blocking(move || {
        let project = AndroidProject::detect(&path);
        let _ = sender.send(AppEvent::ProjectLoaded {
            generation,
            project,
        });
    });
}

fn spawn_project_discovery(root: PathBuf, sender: UnboundedSender<AppEvent>) {
    let scanned = root.clone();
    tokio::task::spawn_blocking(move || {
        let projects = AndroidProject::discover_switchable_projects(&scanned);
        let _ = sender.send(AppEvent::ProjectsDiscovered {
            root: scanned,
            projects,
        });
    });
}

fn copy_text_to_clipboard(app: &mut AppState, label: &str, text: &str) {
    match clipboard::copy_to_clipboard(text) {
        Ok(method) => {
            let lines = if text.is_empty() {
                0
            } else {
                text.lines().count()
            };
            let via = match method {
                CopyMethod::System => "clipboard",
                CopyMethod::Terminal => "terminal clipboard",
            };
            app.status_message = Some(format!("Copied {lines} {label} lines to the {via}"));
            app.append_simplified_event(format!("📋 Copied {lines} {label} lines"));
        }
        Err(err) => {
            app.status_message = Some(err);
        }
    }
}

fn copy_focused_console(app: &mut AppState, key: &KeyEvent) {
    let copy_build =
        matches!(key.code, KeyCode::Char('Y')) || key.modifiers.contains(KeyModifiers::SHIFT);
    let (label, text) = if copy_build || (app.current_panel == Panel::Output && app.show_build_pane) {
        ("build output", app.console_plaintext())
    } else if (app.current_panel == Panel::Logcat && app.show_logcat_pane) || app.show_logcat_pane {
        ("logcat", app.logcat_plaintext())
    } else if app.show_build_pane {
        ("build output", app.console_plaintext())
    } else {
        ("status", app.simplified_plaintext())
    };

    copy_text_to_clipboard(app, label, &text);
}

fn sync_logcat_stream(
    app: &AppState,
    logcat_stream: &mut LogcatStream,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    if app.project_loading {
        return;
    }

    let should_stream = app.show_logcat_pane || app.view_mode == ViewMode::LogcatView;

    if should_stream {
        if let Some(dev) = app.selected_device() {
            let target_serial = dev.serial.as_str();
            let target_pkg = app
                .project
                .application_id
                .clone()
                .or_else(|| app.project.namespace.clone());
            let target_variant = app.project.active_variant.clone();
            let target_filter = app.logcat_filter_by_app;

            let needs_start = !logcat_stream.is_running()
                || logcat_stream.current_serial() != Some(target_serial)
                || logcat_stream.package_name() != target_pkg.as_deref()
                || logcat_stream.build_variant() != Some(target_variant.as_str())
                || logcat_stream.is_filtering_by_app() != target_filter;

            if needs_start {
                logcat_stream.start(
                    adb_path,
                    target_serial,
                    target_pkg,
                    target_variant,
                    target_filter,
                    sender.clone(),
                );
            }
        } else {
            logcat_stream.stop();
        }
    } else if logcat_stream.is_running() {
        logcat_stream.stop();
    }
}

fn persist_user_config(app: &AppState) {
    #[cfg(not(test))]
    {
        let cfg = UserConfig {
            onboarding_completed: true,
            theme: app.theme,
            language: app.language,
            custom_adb_path: app.custom_adb_path.clone(),
        };
        let _ = cfg.save();
    }
    #[cfg(test)]
    let _ = app;
}

fn handle_settings_key_event(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    logcat_stream: &mut LogcatStream,
    key: KeyEvent,
    sender: &UnboundedSender<AppEvent>,
) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    if app.settings_editing_adb_path {
        match key.code {
            KeyCode::Esc => {
                app.settings_editing_adb_path = false;
                app.settings_adb_input = app.custom_adb_path.clone().unwrap_or_default();
            }
            KeyCode::Enter => {
                app.apply_custom_adb_path(Some(app.settings_adb_input.clone()));
                app.settings_editing_adb_path = false;
                persist_user_config(app);
                let msg = match app.language {
                    Language::En => format!("ADB path set to {}", app.adb_path.display()),
                    Language::Es => format!("Ruta ADB configurada: {}", app.adb_path.display()),
                };
                app.append_console_line(format!("🔌 {msg}"));
                app.status_message = Some(msg);
            }
            KeyCode::Backspace => {
                app.settings_adb_input.pop();
            }
            KeyCode::Char(c) => {
                app.settings_adb_input.push(c);
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc | KeyCode::Char('s') | KeyCode::Char(',') => {
            app.toggle_settings_view();
        }
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        KeyCode::Char('?') => {
            app.toggle_help();
        }
        KeyCode::Tab => {
            app.settings_focus = match app.settings_focus {
                SettingsFocus::Sidebar => SettingsFocus::Content,
                SettingsFocus::Content => SettingsFocus::Sidebar,
            };
        }
        KeyCode::BackTab => {
            app.settings_focus = match app.settings_focus {
                SettingsFocus::Sidebar => SettingsFocus::Content,
                SettingsFocus::Content => SettingsFocus::Sidebar,
            };
        }
        _ => match app.settings_focus {
            SettingsFocus::Sidebar => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.settings_prev_section();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.settings_next_section();
                }
                KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('l') => {
                    app.settings_focus = SettingsFocus::Content;
                }
                _ => {}
            },
            SettingsFocus::Content => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.settings_prev_item();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.settings_next_item();
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    app.settings_focus = SettingsFocus::Sidebar;
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    activate_settings_item(app, runner, logcat_stream, sender);
                }
                _ => {}
            },
        },
    }
}

fn activate_settings_item(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    logcat_stream: &mut LogcatStream,
    sender: &UnboundedSender<AppEvent>,
) {
    match app.settings_section {
        SettingsSection::Appearance => {
            let themes = ThemePreset::all();
            if app.settings_item_index < themes.len() {
                let preset = themes[app.settings_item_index];
                app.set_theme(preset);
                persist_user_config(app);
                app.status_message = Some(format!("Theme: {}", preset.name()));
            } else {
                let lang_idx = app.settings_item_index - themes.len();
                if let Some(&lang) = Language::all().get(lang_idx) {
                    app.set_language(lang);
                    persist_user_config(app);
                    app.status_message = Some(format!("Language: {}", lang.name()));
                }
            }
        }
        SettingsSection::AdbSdk => match app.settings_item_index {
            0 => {
                app.settings_adb_input = app.custom_adb_path.clone().unwrap_or_default();
                app.settings_editing_adb_path = true;
            }
            1 => {
                app.refresh_sdk_config();
                let msg = match app.language {
                    Language::En => format!("SDK & tools refreshed · {}", app.adb_path.display()),
                    Language::Es => format!("SDK y herramientas actualizados · {}", app.adb_path.display()),
                };
                app.append_console_line(format!("🔍 {msg}"));
                app.status_message = Some(msg);
            }
            2 => {
                let adb = app.adb_path.clone();
                app.start_task("Restart ADB Server");
                runner.execute_adb_restart_server(adb, sender.clone());
            }
            3 => {
                app.toggle_adb_connect_modal();
            }
            4 => {
                app.toggle_devices_view();
            }
            _ => {}
        },
        SettingsSection::ConsoleLogcat => match app.settings_item_index {
            0 => {
                app.cycle_console_preset();
                let adb = app.adb_path.clone();
                sync_logcat_stream(app, logcat_stream, &adb, sender);
            }
            1 => {
                app.toggle_simplified_pane();
            }
            2 => {
                app.toggle_build_pane();
            }
            3 => {
                app.toggle_logcat_pane();
                let adb = app.adb_path.clone();
                sync_logcat_stream(app, logcat_stream, &adb, sender);
            }
            4 => {
                app.toggle_logcat_filter();
                let adb = app.adb_path.clone();
                sync_logcat_stream(app, logcat_stream, &adb, sender);
            }
            5 => {
                app.open_onboarding();
            }
            _ => {}
        },
        SettingsSection::About => match app.settings_item_index {
            0 => {
                app.open_onboarding();
            }
            1 => {
                app.toggle_help();
            }
            _ => {}
        },
    }
}

fn confirm_selected_project(app: &mut AppState, sender: &UnboundedSender<AppEvent>) {
    if let Some(path) = app.selected_project().cloned() {
        if path == app.project.root_path && app.project.is_valid() && !app.project_loading {
            app.project_selector_active = false;
            app.status_message = Some(format!("Project: {}", app.project.name));
        } else {
            let generation = app.begin_project_switch(&path);
            spawn_project_load(path, generation, sender.clone());
        }
    }
}

fn submit_adb_connect_modal(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    let connect_addr = app.adb_connect_ip_port.trim().to_string();
    if connect_addr.is_empty() {
        app.append_console_line(
            "⚠ Please specify a valid target IP:Port to connect.".to_string(),
        );
        return;
    }

    let pair_addr = if app.adb_pair_ip_port.trim().is_empty() {
        None
    } else {
        Some(app.adb_pair_ip_port.trim().to_string())
    };
    let pair_code = if app.adb_pair_code.trim().is_empty() {
        None
    } else {
        Some(app.adb_pair_code.trim().to_string())
    };
    app.adb_connect_modal_active = false;
    app.start_task("ADB Wireless Pair & Connect");
    runner.execute_adb_pair_and_connect(
        adb_path.to_path_buf(),
        pair_addr,
        pair_code,
        connect_addr,
        sender.clone(),
    );
}

fn handle_mouse_event(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    logcat_stream: &mut LogcatStream,
    mouse: MouseEvent,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) -> bool {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let Some(target) = app.hit_map.borrow().hit_test(mouse.column, mouse.row) else {
                return false;
            };
            match target {
                ClickTarget::ActionCard(idx) => {
                    if let Some(action) = app.actions.get(idx).copied() {
                        app.current_panel = Panel::Actions;
                        app.selected_action_index = idx;
                        trigger_quick_action(app, runner, logcat_stream, action, adb_path, sender);
                        return true;
                    }
                    false
                }
                ClickTarget::DeviceControlCard(idx) => {
                    if let Some(action) = DeviceAction::all().get(idx).copied() {
                        app.current_panel = Panel::DeviceControls;
                        app.selected_device_control_index = idx;
                        if action == DeviceAction::WirelessPairAndConnect {
                            app.adb_connect_modal_active = true;
                        } else {
                            trigger_device_action(app, runner, action, adb_path, sender);
                        }
                        return true;
                    }
                    false
                }
                ClickTarget::DeviceRow(idx) => {
                    if idx < app.devices.len() {
                        let prev_serial = app.selected_device().map(|d| d.serial.clone());
                        app.current_panel = Panel::Devices;
                        app.selected_device_index = idx;
                        let new_serial = app.selected_device().map(|d| d.serial.clone());
                        if let Some(dev) = app.selected_device() {
                            app.append_console_line(format!(
                                "> Active device: {}",
                                dev.display_name()
                            ));
                        }
                        if prev_serial != new_serial {
                            sync_logcat_stream(app, logcat_stream, adb_path, sender);
                        }
                        return true;
                    }
                    false
                }
                ClickTarget::FocusPanel(panel) => {
                    app.current_panel = panel;
                    true
                }
                ClickTarget::ScrollArea(_) => false,
                ClickTarget::ToggleSimplifiedPane => {
                    app.toggle_simplified_pane();
                    true
                }
                ClickTarget::ToggleBuildPane => {
                    app.toggle_build_pane();
                    true
                }
                ClickTarget::ToggleLogcatPane => {
                    app.toggle_logcat_pane();
                    sync_logcat_stream(app, logcat_stream, adb_path, sender);
                    true
                }
                ClickTarget::CopyBuild => {
                    let text = app.console_plaintext();
                    copy_text_to_clipboard(app, "build output", &text);
                    true
                }
                ClickTarget::ToggleBuildFollow => {
                    app.toggle_console_follow();
                    true
                }
                ClickTarget::ClearBuild => {
                    app.clear_console();
                    app.status_message = Some("Build output cleared".to_string());
                    true
                }
                ClickTarget::ToggleMaximizeBuild => {
                    app.toggle_maximize_build();
                    sync_logcat_stream(app, logcat_stream, adb_path, sender);
                    true
                }
                ClickTarget::CopyLogcat => {
                    let text = app.logcat_plaintext();
                    copy_text_to_clipboard(app, "logcat", &text);
                    true
                }
                ClickTarget::ToggleLogcatScope => {
                    app.toggle_logcat_filter();
                    sync_logcat_stream(app, logcat_stream, adb_path, sender);
                    true
                }
                ClickTarget::OpenLogcatSearch => {
                    app.search_active = true;
                    true
                }
                ClickTarget::ClearLogcatSearch => {
                    app.search_query.clear();
                    true
                }
                ClickTarget::ToggleLogcatFollow => {
                    app.toggle_logcat_follow();
                    true
                }
                ClickTarget::ClearLogcat => {
                    app.clear_logcat();
                    app.status_message = Some("Logcat cleared".to_string());
                    true
                }
                ClickTarget::ToggleMaximizeLogcat => {
                    app.toggle_maximize_logcat();
                    sync_logcat_stream(app, logcat_stream, adb_path, sender);
                    true
                }
                ClickTarget::SettingsSection(section) => {
                    app.settings_section = section;
                    app.settings_item_index = 0;
                    app.settings_editing_adb_path = false;
                    app.settings_focus = SettingsFocus::Sidebar;
                    true
                }
                ClickTarget::SettingsItem(idx) => {
                    if idx < app.settings_item_count() {
                        app.settings_focus = SettingsFocus::Content;
                        app.settings_item_index = idx;
                        app.settings_editing_adb_path = false;
                        activate_settings_item(app, runner, logcat_stream, sender);
                        return true;
                    }
                    false
                }
                ClickTarget::Footer(action) => match action {
                    FooterAction::Back => {
                        if app.settings_view_active {
                            app.toggle_settings_view();
                        } else if app.devices_view_active {
                            app.toggle_devices_view();
                        }
                        true
                    }
                    FooterAction::Settings => {
                        app.toggle_settings_view();
                        true
                    }
                    FooterAction::Devices => {
                        app.toggle_devices_view();
                        true
                    }
                    FooterAction::Project => {
                        let opening = !app.project_selector_active;
                        app.toggle_project_selector();
                        if opening {
                            let root = app.project.root_path.clone();
                            app.discovery_root = root.clone();
                            spawn_project_discovery(root, sender.clone());
                        }
                        true
                    }
                    FooterAction::Target => {
                        app.next_target();
                        sync_logcat_stream(app, logcat_stream, adb_path, sender);
                        true
                    }
                    FooterAction::CycleViews => {
                        app.cycle_console_preset();
                        sync_logcat_stream(app, logcat_stream, adb_path, sender);
                        true
                    }
                    FooterAction::Help => {
                        app.toggle_help();
                        true
                    }
                    FooterAction::Quit => {
                        app.should_quit = true;
                        true
                    }
                    FooterAction::WifiAdb => {
                        app.toggle_adb_connect_modal();
                        true
                    }
                    FooterAction::SwitchPane => {
                        if app.settings_view_active {
                            app.settings_focus = match app.settings_focus {
                                SettingsFocus::Sidebar => SettingsFocus::Content,
                                SettingsFocus::Content => SettingsFocus::Sidebar,
                            };
                        } else {
                            app.next_panel();
                        }
                        true
                    }
                    FooterAction::SelectOrApply => {
                        if app.settings_view_active {
                            if app.settings_editing_adb_path {
                                app.apply_custom_adb_path(Some(app.settings_adb_input.clone()));
                                app.settings_editing_adb_path = false;
                                persist_user_config(app);
                            } else {
                                activate_settings_item(app, runner, logcat_stream, sender);
                            }
                            true
                        } else if app.devices_view_active {
                            if let Some(action) = app.selected_device_control() {
                                app.current_panel = Panel::DeviceControls;
                                if action == DeviceAction::WirelessPairAndConnect {
                                    app.adb_connect_modal_active = true;
                                } else {
                                    trigger_device_action(app, runner, action, adb_path, sender);
                                }
                                return true;
                            }
                            false
                        } else {
                            false
                        }
                    }
                },
                ClickTarget::ProjectRow(idx) => {
                    if idx < app.nearby_projects.len() {
                        app.selected_project_index = idx;
                        confirm_selected_project(app, sender);
                        return true;
                    }
                    false
                }
                ClickTarget::AdbModalField(idx) => {
                    if idx < 3 {
                        app.adb_modal_field_index = idx;
                        return true;
                    }
                    false
                }
                ClickTarget::AdbModalSubmit => {
                    submit_adb_connect_modal(app, runner, adb_path, sender);
                    true
                }
                ClickTarget::AdbModalCancel => {
                    app.adb_connect_modal_active = false;
                    true
                }
                ClickTarget::OnboardingLanguage(idx) => {
                    if let Some(&lang) = Language::all().get(idx) {
                        app.set_language(lang);
                        return true;
                    }
                    false
                }
                ClickTarget::OnboardingTourChapter(idx) => {
                    app.onboarding_tour_index = idx % 4;
                    app.onboarding_preview_card = 0;
                    true
                }
                ClickTarget::OnboardingPreview => {
                    app.cycle_onboarding_preview();
                    true
                }
                ClickTarget::OnboardingTheme(idx) => {
                    if let Some(&preset) = ThemePreset::all().get(idx) {
                        app.set_theme(preset);
                        return true;
                    }
                    false
                }
                ClickTarget::OnboardingNext => {
                    if app.onboarding_step < 2 {
                        app.onboarding_step += 1;
                    } else {
                        app.complete_onboarding();
                        persist_user_config(app);
                    }
                    true
                }
                ClickTarget::OnboardingBack => {
                    if app.onboarding_step > 0 {
                        app.onboarding_step -= 1;
                        return true;
                    }
                    false
                }
                ClickTarget::OnboardingSkip => {
                    app.complete_onboarding();
                    persist_user_config(app);
                    true
                }
                ClickTarget::ModalBody => false,
                ClickTarget::DismissModal => {
                    if app.onboarding_active {
                        app.complete_onboarding();
                        persist_user_config(app);
                        true
                    } else if app.project_selector_active {
                        app.project_selector_active = false;
                        true
                    } else if app.adb_connect_modal_active {
                        app.adb_connect_modal_active = false;
                        true
                    } else if app.help_active {
                        app.help_active = false;
                        true
                    } else if app.search_active {
                        app.search_active = false;
                        true
                    } else {
                        false
                    }
                }
            }
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            let up = matches!(mouse.kind, MouseEventKind::ScrollUp);
            if app.project_selector_active {
                if up {
                    app.select_prev_project();
                } else {
                    app.select_next_project();
                }
                return true;
            }
            if app.settings_view_active && !app.settings_editing_adb_path {
                if up {
                    app.settings_prev_item();
                } else {
                    app.settings_next_item();
                }
                return true;
            }
            let scroll_target = app.hit_map.borrow().scroll_test(mouse.column, mouse.row);
            match scroll_target {
                Some(ClickTarget::ScrollArea(Panel::Devices)) => {
                    let prev_serial = app.selected_device().map(|d| d.serial.clone());
                    app.current_panel = Panel::Devices;
                    if up {
                        app.scroll_up();
                    } else {
                        app.scroll_down();
                    }
                    let new_serial = app.selected_device().map(|d| d.serial.clone());
                    if prev_serial != new_serial {
                        sync_logcat_stream(app, logcat_stream, adb_path, sender);
                    }
                    true
                }
                Some(ClickTarget::ScrollArea(Panel::Output)) => {
                    if up {
                        app.scroll_console_up(3);
                    } else {
                        app.scroll_console_down(3);
                    }
                    true
                }
                Some(ClickTarget::ScrollArea(Panel::Logcat)) => {
                    if up {
                        app.scroll_logcat_up(3);
                    } else {
                        app.scroll_logcat_down(3);
                    }
                    true
                }
                Some(ClickTarget::ScrollArea(Panel::Console)) => {
                    if app.show_build_pane {
                        if up {
                            app.scroll_console_up(3);
                        } else {
                            app.scroll_console_down(3);
                        }
                        return true;
                    }
                    false
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn handle_key_event(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    logcat_stream: &mut LogcatStream,
    key: KeyEvent,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Modal: First-run Onboarding (Step 0: Language, Step 1: Visual Tour, Step 2: Theme)
    if app.onboarding_active {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            app.should_quit = true;
            return;
        }
        match app.onboarding_step {
            0 => match key.code {
                KeyCode::Up | KeyCode::Left | KeyCode::Char('k') => {
                    app.prev_language();
                }
                KeyCode::Down | KeyCode::Right | KeyCode::Char('j') => {
                    app.next_language();
                }
                KeyCode::Enter | KeyCode::Tab | KeyCode::Char(' ') => {
                    app.onboarding_step = 1;
                }
                KeyCode::Esc => {
                    app.complete_onboarding();
                    persist_user_config(app);
                }
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                _ => {}
            },
            1 => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.prev_onboarding_tour();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.next_onboarding_tour();
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                    app.cycle_onboarding_preview();
                }
                KeyCode::BackTab | KeyCode::Backspace => {
                    app.onboarding_step = 0;
                }
                KeyCode::Enter | KeyCode::Tab => {
                    app.onboarding_step = 2;
                }
                KeyCode::Esc => {
                    app.complete_onboarding();
                    persist_user_config(app);
                }
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                _ => {}
            },
            _ => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.prev_theme();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.next_theme();
                }
                KeyCode::Left | KeyCode::BackTab | KeyCode::Backspace => {
                    app.onboarding_step = 1;
                }
                KeyCode::Enter | KeyCode::Esc => {
                    app.complete_onboarding();
                    persist_user_config(app);
                }
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                _ => {}
            },
        }
        return;
    }

    // Modal: Help active
    if app.help_active {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                app.toggle_help();
            }
            _ => {}
        }
        return;
    }

    // Modal: Project Selector active
    if app.project_selector_active {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.project_selector_active = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_prev_project();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next_project();
            }
            KeyCode::Enter => {
                confirm_selected_project(app, sender);
            }
            _ => {}
        }
        return;
    }

    // Modal: ADB Wireless Pair & Connect active
    if app.adb_connect_modal_active {
        match key.code {
            KeyCode::Esc => {
                app.adb_connect_modal_active = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.next_adb_modal_field();
            }
            KeyCode::BackTab | KeyCode::Up => {
                app.prev_adb_modal_field();
            }
            KeyCode::Backspace => match app.adb_modal_field_index {
                0 => {
                    app.adb_connect_ip_port.pop();
                }
                1 => {
                    app.adb_pair_ip_port.pop();
                }
                2 => {
                    app.adb_pair_code.pop();
                }
                _ => {}
            },
            KeyCode::Char(c) => match app.adb_modal_field_index {
                0 => {
                    app.adb_connect_ip_port.push(c);
                }
                1 => {
                    app.adb_pair_ip_port.push(c);
                }
                2 => {
                    app.adb_pair_code.push(c);
                }
                _ => {}
            },
            KeyCode::Enter => {
                submit_adb_connect_modal(app, runner, adb_path, sender);
            }
            _ => {}
        }
        return;
    }

    // Modal: Search active
    if app.search_active {
        match key.code {
            KeyCode::Esc => {
                app.toggle_search();
            }
            KeyCode::Enter => {
                app.search_active = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        return;
    }

    // Global termination: Ctrl+C
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Global cancellation of active background process: Esc or Ctrl+X
    let is_cancel_key = (key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('x'))
        || key.code == KeyCode::Esc;

    if is_cancel_key && runner.is_running() {
        runner.abort();
        app.finish_task(false, "Process cancelled by user");
        app.append_console_line("🛑 Running process cancelled by user.".to_string());
        return;
    }

    if app.settings_view_active {
        handle_settings_key_event(app, runner, logcat_stream, key, sender);
        return;
    }

    if key.code == KeyCode::Esc && app.devices_view_active {
        app.toggle_devices_view();
        return;
    }

    match key.code {
        // App quit
        KeyCode::Char('q') => {
            app.should_quit = true;
        }
        // Help modal
        KeyCode::Char('?') => {
            app.toggle_help();
        }
        // Search modal
        KeyCode::Char('/') => {
            app.toggle_search();
        }
        // Panel navigation
        KeyCode::Tab => {
            app.next_panel();
        }
        KeyCode::BackTab => {
            app.prev_panel();
        }
        // Build variant cycle
        KeyCode::Char('v') => {
            app.next_variant();
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
        }
        // KMP target cycle
        KeyCode::Char('t') => {
            app.next_target();
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
        }
        // Theme cycle (Shift+T)
        KeyCode::Char('T') => {
            app.next_theme();
            persist_user_config(app);
        }
        // Language toggle (Shift+L)
        KeyCode::Char('L') => {
            app.toggle_language();
            persist_user_config(app);
        }
        // Reopen onboarding & theme wizard (Shift+O)
        KeyCode::Char('O') => {
            app.open_onboarding();
        }
        // Toggle separate Devices & ADB Controls view
        KeyCode::Char('d') => {
            app.toggle_devices_view();
        }
        // Toggle full Settings view
        KeyCode::Char('s') | KeyCode::Char(',') => {
            app.toggle_settings_view();
        }
        // Project selector modal
        KeyCode::Char('p') => {
            let opening = !app.project_selector_active;
            app.toggle_project_selector();
            if opening {
                let root = app.project.root_path.clone();
                app.discovery_root = root.clone();
                spawn_project_discovery(root, sender.clone());
            }
        }
        // Wireless ADB Pair & Connect modal shortcut
        KeyCode::Char('w') => {
            app.toggle_adb_connect_modal();
        }
        // Multi-view preset cycle (m or l)
        KeyCode::Char('m') | KeyCode::Char('l') => {
            app.cycle_console_preset();
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
        }
        // Pane toggles: 1 = Simplified, 2 = Build, 3 = Logcat
        KeyCode::Char('1') => {
            app.toggle_simplified_pane();
        }
        KeyCode::Char('2') => {
            app.toggle_build_pane();
        }
        KeyCode::Char('3') => {
            app.toggle_logcat_pane();
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
        }
        // Toggle Logcat app filter and restart the stream with the new scope.
        KeyCode::Char('f') => {
            app.toggle_logcat_filter();
            sync_logcat_stream(app, logcat_stream, adb_path, sender);
        }
        // Copy the visible console. `y` copies logcat; Shift+Y copies build output.
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            copy_focused_console(app, &key);
        }
        // Navigation / Scroll
        KeyCode::Left => {
            app.move_left();
        }
        KeyCode::Right => {
            app.move_right();
        }
        KeyCode::Up => {
            let prev_serial = app.selected_device().map(|d| d.serial.clone());
            app.scroll_up();
            let new_serial = app.selected_device().map(|d| d.serial.clone());
            if (app.show_logcat_pane || app.view_mode == ViewMode::LogcatView)
                && prev_serial != new_serial
            {
                sync_logcat_stream(app, logcat_stream, adb_path, sender);
            }
        }
        KeyCode::Down => {
            let prev_serial = app.selected_device().map(|d| d.serial.clone());
            app.scroll_down();
            let new_serial = app.selected_device().map(|d| d.serial.clone());
            if (app.show_logcat_pane || app.view_mode == ViewMode::LogcatView)
                && prev_serial != new_serial
            {
                sync_logcat_stream(app, logcat_stream, adb_path, sender);
            }
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }
        // Enter / Space: Execute selected action or focus device
        KeyCode::Enter | KeyCode::Char(' ') => match app.current_panel {
            Panel::Actions => {
                if let Some(action) = app.selected_action() {
                    trigger_quick_action(app, runner, logcat_stream, action, adb_path, sender);
                }
            }
            Panel::DeviceControls => {
                if let Some(action) = app.selected_device_control() {
                    if action == DeviceAction::WirelessPairAndConnect {
                        app.adb_connect_modal_active = true;
                    } else {
                        trigger_device_action(app, runner, action, adb_path, sender);
                    }
                }
            }
            Panel::Devices => {
                if let Some(dev) = app.selected_device() {
                    app.append_console_line(format!("> Active device: {}", dev.display_name()));
                }
            }
            _ => {}
        },
        // Quick Action Shortcuts: [b], [r], [k], [u], [g] (contextual to active target)
        KeyCode::Char(c) => {
            if let Some(action) =
                QuickAction::from_char_for_target(c, app.project.active_target())
            {
                trigger_quick_action(app, runner, logcat_stream, action, adb_path, sender);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use overridepulse::android::{Device, DeviceState};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_handle_key_event_quick_action_trigger() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Without devices, triggering a device-dependent action informs the user
        let key_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_b, &adb_path, &tx);
        assert!(app
            .console_output
            .back()
            .unwrap()
            .contains("No Android device connected"));

        // Without devices, triggering StopGradleDaemons ('g') succeeds because it's a host machine operation
        let key_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_g, &adb_path, &tx);
        assert!(app.process_state.is_running);
        assert_eq!(
            app.process_state.task_description.as_deref(),
            Some("Stop All Gradle Daemons")
        );

        // Cancel with Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.process_state.is_running);
        assert!(app
            .console_output
            .back()
            .unwrap()
            .contains("cancelled by user"));

        // With connected device
        let dev = Device::new("emulator-5554", DeviceState::Device);
        app.set_devices(vec![dev]);

        // Key 'k' -> QuickAction::ForceStopApp
        let key_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_k, &adb_path, &tx);
        assert!(app.process_state.is_running);
        assert_eq!(
            app.process_state.task_description.as_deref(),
            Some("Force Stop App")
        );

        // Cancel with Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.process_state.is_running);
        assert!(app
            .console_output
            .back()
            .unwrap()
            .contains("cancelled by user"));

        // Drain any messages
        while rx.try_recv().is_ok() {}
    }

    #[test]
    fn test_handle_key_event_navigation_and_panels() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        assert_eq!(app.current_panel, Panel::Actions);
        let expected_cols = app.actions_grid_cols();

        // Left / Right and Up / Down navigation in Actions grid
        let key_right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_right, &adb_path, &tx);
        assert_eq!(app.selected_action_index, 1);

        let key_left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_left, &adb_path, &tx);
        assert_eq!(app.selected_action_index, 0);

        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_down, &adb_path, &tx);
        assert_eq!(app.selected_action_index, expected_cols);

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_up, &adb_path, &tx);
        assert_eq!(app.selected_action_index, 0);

        // Open separate Devices & ADB Controls view with 'd'
        let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_d, &adb_path, &tx);
        assert!(app.devices_view_active);
        assert_eq!(app.current_panel, Panel::Devices);

        // Tab to Next Panel inside Devices view -> DeviceControls
        let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_tab, &adb_path, &tx);
        assert_eq!(app.current_panel, Panel::DeviceControls);

        // Esc closes Devices view and returns to Actions
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.devices_view_active);
        assert_eq!(app.current_panel, Panel::Actions);

        // Toggle help modal
        let key_help = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_help, &adb_path, &tx);
        assert!(app.help_active);

        // Dismiss help with Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.help_active);

        // Toggle ADB Wireless modal with 'w'
        let key_w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_w, &adb_path, &tx);
        assert!(app.adb_connect_modal_active);

        // Dismiss ADB modal with Esc
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.adb_connect_modal_active);
    }

    #[test]
    fn test_handle_key_event_device_controls_and_modal_navigation() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Navigate to DeviceControls panel
        app.current_panel = Panel::DeviceControls;
        app.selected_device_control_index = 0; // WirelessPairAndConnect
        assert_eq!(
            app.selected_device_control(),
            Some(DeviceAction::WirelessPairAndConnect)
        );

        // Press Enter on WirelessPairAndConnect -> triggers modal
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            key_enter,
            &adb_path,
            &tx,
        );
        assert!(app.adb_connect_modal_active);

        // Type characters into modal field 0 (connect_ip_port)
        let key_char = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_char, &adb_path, &tx);
        assert!(app.adb_connect_ip_port.ends_with('5'));

        // Switch field with Tab
        let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_tab, &adb_path, &tx);
        assert_eq!(app.adb_modal_field_index, 1);

        // Backspace removes character from current field
        let key_backspace = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_char, &adb_path, &tx);
        assert!(app.adb_pair_ip_port.ends_with('5'));
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            key_backspace,
            &adb_path,
            &tx,
        );
        assert!(!app.adb_pair_ip_port.ends_with('5'));

        // Dismiss with Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.adb_connect_modal_active);
    }

    #[test]
    fn test_handle_key_event_multi_pane_shortcuts() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Key '2' toggles build pane on so we have 2 visible panes
        let key_2 = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_2, &adb_path, &tx);
        assert!(app.show_build_pane);

        // Key '1' can now toggle simplified pane off
        let key_1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_1, &adb_path, &tx);
        assert!(!app.show_simplified_pane);

        // Key 'm' cycles presets
        let key_m = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_m, &adb_path, &tx);
        // Presets ensure at least one pane is always active
        assert!(app.show_simplified_pane || app.show_build_pane || app.show_logcat_pane);
    }

    #[tokio::test]
    async fn test_kmp_target_actions_run_without_adb_device() {
        use overridepulse::android::{KmpTarget, TargetPlatform};

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
        app.sync_actions_for_active_target();

        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Switch from Android to WasmJS using 't'
        let key_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_t, &adb_path, &tx);
        assert_eq!(
            app.project.active_target().map(|t| t.platform),
            Some(TargetPlatform::WasmJs)
        );

        // Press 'b' with zero ADB devices connected -> runs host KMP compile task
        let key_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_b, &adb_path, &tx);
        assert!(app.process_state.is_running);
        assert_eq!(
            app.process_state.task_description.as_deref(),
            Some("Compile WasmJS")
        );

        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        while rx.try_recv().is_ok() {}
    }

    #[test]
    fn test_onboarding_flow_and_theme_selection() {
        use overridepulse::i18n::Language;
        use overridepulse::ui::theme::ThemePreset;

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.open_onboarding();
        assert!(app.onboarding_active);
        assert_eq!(app.onboarding_step, 0);
        assert_eq!(app.language, Language::En);

        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Step 0 (Language): Down selects Español live
        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_down, &adb_path, &tx);
        assert_eq!(app.language, Language::Es);

        // Enter advances from Language (0) to Visual Tour (1)
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_enter, &adb_path, &tx);
        assert_eq!(app.onboarding_step, 1);

        // Down switches tour chapter, Space interacts with preview
        handle_key_event(&mut app, &mut runner, &mut logcat, key_down, &adb_path, &tx);
        assert_eq!(app.onboarding_tour_index, 1);
        let key_space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_space, &adb_path, &tx);
        assert_eq!(app.onboarding_preview_card, 1);

        // Enter advances from Visual Tour (1) to Theme Picker (2)
        handle_key_event(&mut app, &mut runner, &mut logcat, key_enter, &adb_path, &tx);
        assert_eq!(app.onboarding_step, 2);

        // Down previews the next theme live
        handle_key_event(&mut app, &mut runner, &mut logcat, key_down, &adb_path, &tx);
        assert_eq!(app.theme, ThemePreset::TokyoNight);

        // Enter completes onboarding while keeping selected language & theme
        handle_key_event(&mut app, &mut runner, &mut logcat, key_enter, &adb_path, &tx);
        assert!(!app.onboarding_active);
        assert_eq!(app.language, Language::Es);
        assert_eq!(app.theme, ThemePreset::TokyoNight);

        // Shift+T cycles theme directly; Shift+L toggles language directly
        let key_shift_t = KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_shift_t, &adb_path, &tx);
        assert_eq!(app.theme, ThemePreset::CatppuccinMocha);

        let key_shift_l = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_shift_l, &adb_path, &tx);
        assert_eq!(app.language, Language::En);
    }

    #[tokio::test]
    async fn test_settings_view_navigation_and_actions() {
        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        // Open Settings via 's'
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert!(app.settings_view_active);
        assert_eq!(app.settings_section, SettingsSection::Appearance);
        assert_eq!(app.settings_focus, SettingsFocus::Sidebar);

        // Move into Appearance content and apply TokyoNight (index 1) + Spanish (index 7)
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.settings_focus, SettingsFocus::Content);

        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.theme, ThemePreset::TokyoNight);

        // Jump to Español (last item in Appearance)
        for _ in 0..6 {
            handle_key_event(
                &mut app,
                &mut runner,
                &mut logcat,
                KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
                &adb_path,
                &tx,
            );
        }
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.language, Language::Es);

        // Back to Sidebar -> switch to AdbSdk -> edit Custom ADB Path
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.settings_focus, SettingsFocus::Sidebar);

        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.settings_section, SettingsSection::AdbSdk);

        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert_eq!(app.settings_focus, SettingsFocus::Content);

        // Press Enter on item 0 (Custom ADB Path) to enter edit mode
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert!(app.settings_editing_adb_path);

        for c in "/opt/adb".chars() {
            handle_key_event(
                &mut app,
                &mut runner,
                &mut logcat,
                KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
                &adb_path,
                &tx,
            );
        }
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert!(!app.settings_editing_adb_path);
        assert_eq!(app.custom_adb_path.as_deref(), Some("/opt/adb"));
        assert_eq!(app.adb_path, PathBuf::from("/opt/adb"));

        // Close Settings with Esc
        handle_key_event(
            &mut app,
            &mut runner,
            &mut logcat,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &adb_path,
            &tx,
        );
        assert!(!app.settings_view_active);
    }

    #[tokio::test]
    async fn test_mouse_click_actions_footer_and_settings() {
        use ratatui::backend::TestBackend;

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::render(f, &app)).unwrap();

        // Helper to grab center coordinate of the first matching target
        let find_center = |app: &AppState, target: ClickTarget| -> (u16, u16) {
            let hit_map = app.hit_map.borrow();
            let rect = hit_map
                .regions
                .iter()
                .rev()
                .find(|r| r.target == target)
                .map(|r| r.area)
                .expect("expected click region");
            (rect.x + rect.width / 2, rect.y + rect.height / 2)
        };

        // 1. Mouse move does not request redraw
        let moved = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            moved,
            &adb_path,
            &tx
        ));

        // 2. Click StopGradleDaemons action card
        let stop_idx = app
            .actions
            .iter()
            .position(|a| *a == QuickAction::StopGradleDaemons)
            .expect("StopGradleDaemons present");
        let (cx, cy) = find_center(&app, ClickTarget::ActionCard(stop_idx));
        let click_action = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cx,
            row: cy,
            modifiers: KeyModifiers::NONE,
        };
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            click_action,
            &adb_path,
            &tx
        ));
        assert_eq!(app.selected_action_index, stop_idx);
        assert!(app.process_state.is_running);
        runner.abort();
        app.finish_task(false, "cancelled");

        // 3. Click [s] Settings in the footer
        let (sx, sy) = find_center(&app, ClickTarget::Footer(FooterAction::Settings));
        let click_settings = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: sx,
            row: sy,
            modifiers: KeyModifiers::NONE,
        };
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            click_settings,
            &adb_path,
            &tx
        ));
        assert!(app.settings_view_active);

        // Re-render to populate Settings hit regions, then click Dracula theme (index 3)
        terminal.draw(|f| ui::render(f, &app)).unwrap();
        let (tx_pos, ty_pos) = find_center(&app, ClickTarget::SettingsItem(3));
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: tx_pos,
                row: ty_pos,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx
        ));
        assert_eq!(app.theme, ThemePreset::Dracula);

        // Click ADB & SDK sidebar section
        let (ax, ay) = find_center(&app, ClickTarget::SettingsSection(SettingsSection::AdbSdk));
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: ax,
                row: ay,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx
        ));
        assert_eq!(app.settings_section, SettingsSection::AdbSdk);

        // Click [s/Esc] Back in the Settings footer
        terminal.draw(|f| ui::render(f, &app)).unwrap();
        let (bx, by) = find_center(&app, ClickTarget::Footer(FooterAction::Back));
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: bx,
                row: by,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx
        ));
        assert!(!app.settings_view_active);

        // 4. Click [d] Devices in the footer, then click WirelessPairAndConnect card
        terminal.draw(|f| ui::render(f, &app)).unwrap();
        let (dx, dy) = find_center(&app, ClickTarget::Footer(FooterAction::Devices));
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: dx,
                row: dy,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx
        ));
        assert!(app.devices_view_active);

        terminal.draw(|f| ui::render(f, &app)).unwrap();
        let (wx, wy) = find_center(&app, ClickTarget::DeviceControlCard(0));
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: wx,
                row: wy,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx
        ));
        assert!(app.adb_connect_modal_active);
    }

    #[tokio::test]
    async fn test_console_and_logcat_scroll_and_toolbar_clicks_without_focus() {
        use overridepulse::android::{LogEntry, LogLevel};
        use ratatui::backend::TestBackend;

        let mut app = AppState::new(PathBuf::from("."));
        app.project_selector_active = false;
        app.show_simplified_pane = false;
        app.show_build_pane = true;
        app.show_logcat_pane = true;
        app.current_panel = Panel::Actions;

        for i in 0..80 {
            app.append_console_line(format!("Build line {i}"));
            app.append_logcat_entry(LogEntry {
                raw: format!("10-23 12:00:00.000 100 100 D Tag: Log {i}"),
                timestamp: Some("10-23 12:00:00.000".to_string()),
                pid: Some(100),
                tid: Some(100),
                level: LogLevel::Debug,
                tag: "Tag".to_string(),
                message: format!("Log {i}"),
            });
        }

        let mut runner = ProcessRunner::new();
        let mut logcat = LogcatStream::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let adb_path = PathBuf::from("adb");

        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui::render(f, &app)).unwrap();

        let vis_build = app.console_visible_height.get();
        let vis_logcat = app.logcat_visible_height.get();
        let expected_build_max = 80usize.saturating_sub(vis_build);
        let expected_logcat_max = 80usize.saturating_sub(vis_logcat);

        // Scroll wheel over Logcat while focus stays on Panel::Actions
        let logcat_rect = app
            .hit_map
            .borrow()
            .scroll_regions
            .iter()
            .find(|r| r.target == ClickTarget::ScrollArea(Panel::Logcat))
            .map(|r| r.area)
            .expect("logcat scroll region");
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: logcat_rect.x + 2,
                row: logcat_rect.y + 3,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx,
        ));
        assert_eq!(app.current_panel, Panel::Actions);
        assert!(!app.auto_scroll_logcat);
        assert_eq!(app.logcat_scroll, expected_logcat_max.saturating_sub(3));

        // Scroll wheel over Build while focus stays on Panel::Actions
        let build_rect = app
            .hit_map
            .borrow()
            .scroll_regions
            .iter()
            .find(|r| r.target == ClickTarget::ScrollArea(Panel::Output))
            .map(|r| r.area)
            .expect("build scroll region");
        assert!(handle_mouse_event(
            &mut app,
            &mut runner,
            &mut logcat,
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: build_rect.x + 2,
                row: build_rect.y + 3,
                modifiers: KeyModifiers::NONE,
            },
            &adb_path,
            &tx,
        ));
        assert_eq!(app.current_panel, Panel::Actions);
        assert!(!app.auto_scroll_console);
        assert_eq!(app.console_scroll, expected_build_max.saturating_sub(3));

        // Click toolbar buttons directly (ToggleLogcatScope, ToggleBuildFollow, ClearLogcat, ToggleMaximizeLogcat)
        let click_target = |app: &mut AppState,
                            runner: &mut ProcessRunner,
                            logcat: &mut LogcatStream,
                            target: ClickTarget| {
            let rect = app
                .hit_map
                .borrow()
                .regions
                .iter()
                .rev()
                .find(|r| r.target == target)
                .map(|r| r.area)
                .expect("expected toolbar button");
            handle_mouse_event(
                app,
                runner,
                logcat,
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: rect.x + 1,
                    row: rect.y,
                    modifiers: KeyModifiers::NONE,
                },
                &adb_path,
                &tx,
            )
        };

        assert!(app.logcat_filter_by_app);
        assert!(click_target(
            &mut app,
            &mut runner,
            &mut logcat,
            ClickTarget::ToggleLogcatScope
        ));
        assert!(!app.logcat_filter_by_app);

        assert!(click_target(
            &mut app,
            &mut runner,
            &mut logcat,
            ClickTarget::ToggleBuildFollow
        ));
        assert!(app.auto_scroll_console);

        assert!(click_target(
            &mut app,
            &mut runner,
            &mut logcat,
            ClickTarget::ToggleMaximizeLogcat
        ));
        assert!(!app.show_build_pane && app.show_logcat_pane);

        terminal.draw(|f| ui::render(f, &app)).unwrap();
        assert!(click_target(
            &mut app,
            &mut runner,
            &mut logcat,
            ClickTarget::ClearLogcat
        ));
        assert!(app.logcat_entries.is_empty());
    }
}
