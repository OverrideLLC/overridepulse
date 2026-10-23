use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    cursor::Show,
    event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::UnboundedSender;

use overridepulse::{
    android::{
        device::spawn_device_monitor, ActionContext, LogcatStream, ProcessRunner, QuickAction,
    },
    app::{AppState, Panel, ViewMode},
    config::SdkConfig,
    event::{AppEvent, EventHandler},
    ui,
};

#[derive(Parser, Debug)]
#[command(
    name = "overridepulse",
    author = "OverridePulse Contributors",
    version,
    about = "Ultralightweight TUI for Android & Gradle development (Lazygit for Android developers)"
)]
struct Cli {
    /// Path to Android project root (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    project: Option<PathBuf>,

    /// Target device serial number
    #[arg(short, long)]
    device: Option<String>,

    /// UI tick rate in milliseconds (default: 100)
    #[arg(long, default_value_t = 100)]
    tick_rate: u64,
}

/// RAII Guard ensuring terminal restoration even if exiting via early return or error.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
    }
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
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
    let adb_path = sdk_config
        .adb_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("adb"));

    // 3. Initialize terminal and panic hook
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 4. Initialize State and Event Handler
    let mut app = AppState::new(root_path);
    app.target_device_serial = cli.device.clone();

    app.append_console_line("⚡ OverridePulse initialized".to_string());
    app.append_console_line(format!("📋 {}", sdk_config.status_summary()));
    app.append_console_line(format!("📂 Project root: {}", app.project.root_path.display()));
    if let Some(ref target) = cli.device {
        app.append_console_line(format!("🎯 Target device specified: {target}"));
    }

    let mut event_handler = EventHandler::new(Duration::from_millis(cli.tick_rate));
    let sender = event_handler.sender();

    // 5. Spawn background ADB device monitor (periodic refresh every 3s)
    let _device_monitor_handle =
        spawn_device_monitor(adb_path.clone(), sender.clone(), Duration::from_secs(3));

    // 6. Asynchronous execution engines
    let mut runner = ProcessRunner::new();
    let mut logcat_stream = LogcatStream::new();

    // 7. Main event loop
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(event) = event_handler.next().await {
            match event {
                AppEvent::Key(key) => {
                    handle_key_event(
                        &mut app,
                        &mut runner,
                        &mut logcat_stream,
                        key,
                        &adb_path,
                        &sender,
                    );
                }
                AppEvent::Tick => {
                    app.tick();
                }
                AppEvent::ProcessOutput(line) => {
                    app.append_console_line(line);
                }
                AppEvent::ProcessFinished(success, message) => {
                    app.finish_task(success, message);
                }
                AppEvent::LogcatLine(entry) => {
                    app.append_logcat_entry(entry);
                }
                AppEvent::DevicesUpdated(devices) => {
                    let prev_serial = app.selected_device().map(|d| d.serial.clone());
                    app.set_devices(devices);
                    let new_serial = app.selected_device().map(|d| d.serial.clone());

                    // If device changed while in logcat view or newly discovered, sync stream
                    if app.view_mode == ViewMode::LogcatView && prev_serial != new_serial {
                        sync_logcat_stream(&app, &mut logcat_stream, &sender);
                    }
                }
                AppEvent::Resize(_, _) => {
                    // Redrawn automatically on next draw
                }
                AppEvent::Mouse(_) => {}
            }
        }
    }

    // Clean up background stream
    logcat_stream.stop();
    runner.abort();

    Ok(())
}

fn trigger_quick_action(
    app: &mut AppState,
    runner: &mut ProcessRunner,
    _logcat_stream: &mut LogcatStream,
    action: QuickAction,
    adb_path: &Path,
    sender: &UnboundedSender<AppEvent>,
) {
    let serial = match app.selected_device() {
        Some(dev) => dev.serial.clone(),
        None => {
            if action == QuickAction::StopGradleDaemons {
                "local".to_string()
            } else {
                let msg = format!(
                    "⚠ Cannot execute '{}': No Android device connected or selected.",
                    action.title()
                );
                app.append_console_line(msg);
                app.finish_task(false, "No device connected");
                return;
            }
        }
    };

    let title = action.title().to_string();
    app.start_task(title.clone());
    if action == QuickAction::StopGradleDaemons {
        app.append_console_line(format!(
            "> [{}] Starting {}...",
            action.shortcut(),
            title
        ));
    } else {
        app.append_console_line(format!(
            "> [{}] Starting {} on device '{}'...",
            action.shortcut(),
            title,
            serial
        ));
    }

    // Show build output terminal for build/process actions (except scrcpy which is a separate GUI)
    if action != QuickAction::LaunchScrcpy && app.view_mode != ViewMode::BuildOutput {
        app.view_mode = ViewMode::BuildOutput;
        app.current_panel = Panel::Output;
    }

    let effective_app_id = app
        .project
        .application_id
        .clone()
        .or_else(|| app.project.namespace.clone());

    let mut ctx = ActionContext::new(&app.project.root_path, &serial)
        .with_variant(&app.project.active_variant)
        .with_adb_path(adb_path);

    if let Some(ref app_id) = effective_app_id {
        ctx = ctx.with_app_id(app_id);
    }
    if let Some(ref activity) = app.project.launcher_activity {
        ctx = ctx.with_main_activity(activity);
    }

    runner.execute_action(action, ctx, sender.clone());
}

fn sync_logcat_stream(
    app: &AppState,
    logcat_stream: &mut LogcatStream,
    sender: &UnboundedSender<AppEvent>,
) {
    if app.view_mode == ViewMode::LogcatView {
        if let Some(dev) = app.selected_device() {
            let running_serial = logcat_stream.current_serial();
            let running_pkg = logcat_stream.package_name();

            let target_serial = &dev.serial;
            let target_pkg = app
                .project
                .application_id
                .as_deref()
                .or(app.project.namespace.as_deref());

            let needs_start = !logcat_stream.is_running()
                || running_serial != Some(target_serial)
                || running_pkg != target_pkg;

            if needs_start {
                let pkg_owned = target_pkg.map(|s| s.to_string());
                logcat_stream.start(
                    target_serial,
                    pkg_owned,
                    app.logcat_filter_by_app,
                    sender.clone(),
                );
            } else {
                logcat_stream.set_filter_by_app(app.logcat_filter_by_app);
            }
        } else {
            logcat_stream.stop();
        }
    } else if logcat_stream.is_running() {
        logcat_stream.stop();
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
                if let Some(path) = app.selected_project().cloned() {
                    app.switch_project(path);
                    sync_logcat_stream(app, logcat_stream, sender);
                }
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
        }
        // Project selector modal
        KeyCode::Char('p') => {
            app.toggle_project_selector();
        }
        // Toggle Logcat View
        KeyCode::Char('l') => {
            app.toggle_view_mode();
            sync_logcat_stream(app, logcat_stream, sender);
        }
        // Toggle Logcat app filter
        KeyCode::Char('f') => {
            app.toggle_logcat_filter();
            logcat_stream.set_filter_by_app(app.logcat_filter_by_app);
        }
        // Navigation / Scroll
        KeyCode::Up => {
            let prev_serial = app.selected_device().map(|d| d.serial.clone());
            app.scroll_up();
            let new_serial = app.selected_device().map(|d| d.serial.clone());
            if app.view_mode == ViewMode::LogcatView && prev_serial != new_serial {
                sync_logcat_stream(app, logcat_stream, sender);
            }
        }
        KeyCode::Down => {
            let prev_serial = app.selected_device().map(|d| d.serial.clone());
            app.scroll_down();
            let new_serial = app.selected_device().map(|d| d.serial.clone());
            if app.view_mode == ViewMode::LogcatView && prev_serial != new_serial {
                sync_logcat_stream(app, logcat_stream, sender);
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
            Panel::Devices => {
                if let Some(dev) = app.selected_device() {
                    app.append_console_line(format!("> Active device: {}", dev.display_name()));
                }
            }
            _ => {}
        },
        // Quick Action Shortcuts: [b], [a], [c], [k], [r], [u], [s], [g]
        KeyCode::Char(c) => {
            if let Some(action) = QuickAction::from_char(c) {
                trigger_quick_action(app, runner, logcat_stream, action, adb_path, sender);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use overridepulse::android::{Device, DeviceState};

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
        assert!(app.console_output.back().unwrap().contains("No Android device connected"));

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
        assert!(app.console_output.back().unwrap().contains("cancelled by user"));

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
        assert!(app.console_output.back().unwrap().contains("cancelled by user"));

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

        assert_eq!(app.current_panel, Panel::Devices);

        // Tab to Next Panel
        let key_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_tab, &adb_path, &tx);
        assert_eq!(app.current_panel, Panel::Actions);

        // Up / Down navigation
        let key_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_down, &adb_path, &tx);
        assert_eq!(app.selected_action_index, 1);

        let key_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_up, &adb_path, &tx);
        assert_eq!(app.selected_action_index, 0);

        // Toggle help modal
        let key_help = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_help, &adb_path, &tx);
        assert!(app.help_active);

        // Dismiss help with Esc
        let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key_event(&mut app, &mut runner, &mut logcat, key_esc, &adb_path, &tx);
        assert!(!app.help_active);
    }
}

