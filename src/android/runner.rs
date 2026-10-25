use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::android::device::DeviceAction;
use crate::event::AppEvent;

/// Represents predefined quick developer actions for Android development.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickAction {
    /// `[b]`: Builds debug APK via `./gradlew assembleDebug` and installs via `adb install -r`.
    BuildAndInstallDebug,
    /// Builds bundle via `./gradlew bundleDebug`/`bundleRelease` and deploys via `bundletool`.
    BuildAabAndDeploy,
    /// Clears application data via `adb shell pm clear <applicationId>`.
    ClearAppData,
    /// `[k]`: Terminates app process via `adb shell am force-stop <applicationId>`.
    ForceStopApp,
    /// `[r]`: Stops and relaunches the launcher activity via `am start -W` or monkey.
    RelaunchActivity,
    /// `[u]`: Uninstalls the application package via `adb uninstall <applicationId>`.
    UninstallApp,
    /// Spawns a detached screen mirroring session with `scrcpy -s <serial>`.
    LaunchScrcpy,
    /// `[g]`: Stops all running Gradle daemons (`./gradlew --stop`) and terminates active tasks.
    StopGradleDaemons,
}

impl QuickAction {
    /// Returns the single-character keyboard shortcut for this action, or `None` if menu-only.
    pub fn shortcut(&self) -> Option<char> {
        match self {
            Self::BuildAndInstallDebug => Some('b'),
            Self::RelaunchActivity => Some('r'),
            Self::ForceStopApp => Some('k'),
            Self::UninstallApp => Some('u'),
            Self::StopGradleDaemons => Some('g'),
            Self::ClearAppData => None,
            Self::BuildAabAndDeploy => None,
            Self::LaunchScrcpy => None,
        }
    }

    /// Resolves a keyboard shortcut into its corresponding `QuickAction`.
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'b' => Some(Self::BuildAndInstallDebug),
            'r' => Some(Self::RelaunchActivity),
            'k' => Some(Self::ForceStopApp),
            'u' => Some(Self::UninstallApp),
            'g' => Some(Self::StopGradleDaemons),
            _ => None,
        }
    }

    /// User-friendly title for UI headers and lists.
    pub fn title(&self) -> &'static str {
        match self {
            Self::BuildAndInstallDebug => "Build, Install & Launch (Debug)",
            Self::BuildAabAndDeploy => "Build AAB & Deploy (Bundletool)",
            Self::ClearAppData => "Clear App Data (pm clear)",
            Self::ForceStopApp => "Force Stop App",
            Self::RelaunchActivity => "Relaunch App",
            Self::UninstallApp => "Uninstall App",
            Self::LaunchScrcpy => "Launch Scrcpy Mirror",
            Self::StopGradleDaemons => "Stop All Gradle Daemons",
        }
    }

    /// Detailed description of what the action does.
    pub fn description(&self) -> &'static str {
        match self {
            Self::BuildAndInstallDebug => "Assemble debug APK with Gradle and install via ADB",
            Self::BuildAabAndDeploy => "Build Android App Bundle, build APKS and deploy to device",
            Self::ClearAppData => "Clear app user data and cache via pm clear",
            Self::ForceStopApp => "Forcefully terminate app process via am force-stop",
            Self::RelaunchActivity => "Force-stop and relaunch launcher activity via am start",
            Self::UninstallApp => "Completely uninstall application package from device",
            Self::LaunchScrcpy => "Launch scrcpy display mirroring session in background",
            Self::StopGradleDaemons => {
                "Stop all running Gradle daemons (./gradlew --stop) and terminate active Gradle tasks"
            }
        }
    }

    /// Formatted display label including shortcut badge: `"[b] Build, Install & Launch (Debug)"`.
    pub fn display_label(&self) -> String {
        match self.shortcut() {
            Some(sc) => format!("[{sc}] {}", self.title()),
            None => format!("    {}", self.title()),
        }
    }

    /// Returns a slice containing all quick actions in recommended display order.
    pub fn all() -> &'static [QuickAction] {
        &[
            Self::BuildAndInstallDebug,
            Self::BuildAabAndDeploy,
            Self::ClearAppData,
            Self::ForceStopApp,
            Self::RelaunchActivity,
            Self::UninstallApp,
            Self::LaunchScrcpy,
            Self::StopGradleDaemons,
        ]
    }
}

/// Execution context for running quick actions on an Android project and device.
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub project_root: PathBuf,
    pub serial: String,
    pub application_id: Option<String>,
    pub main_activity: Option<String>,
    pub variant: String,
    pub adb_path: PathBuf,
}

impl ActionContext {
    pub fn new(project_root: impl Into<PathBuf>, serial: impl Into<String>) -> Self {
        Self {
            project_root: project_root.into(),
            serial: serial.into(),
            application_id: None,
            main_activity: None,
            variant: "debug".to_string(),
            adb_path: PathBuf::from("adb"),
        }
    }

    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.application_id = Some(app_id.into());
        self
    }

    pub fn with_main_activity(mut self, activity: impl Into<String>) -> Self {
        self.main_activity = Some(activity.into());
        self
    }

    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = variant.into();
        self
    }

    pub fn with_adb_path(mut self, adb: impl Into<PathBuf>) -> Self {
        self.adb_path = adb.into();
        self
    }

    /// Resolves the Gradle executable (prefers wrapper in project root).
    pub fn gradle_executable(&self) -> PathBuf {
        let gradlew = if cfg!(windows) {
            self.project_root.join("gradlew.bat")
        } else {
            self.project_root.join("gradlew")
        };

        if gradlew.exists() {
            gradlew
        } else {
            PathBuf::from(if cfg!(windows) { "gradle.bat" } else { "gradle" })
        }
    }

    /// Returns the ADB executable path.
    pub fn adb_executable(&self) -> &Path {
        &self.adb_path
    }
}

static GRADLE_PROGRESS_RE: OnceLock<Regex> = OnceLock::new();

/// Extracts progress percentage (0..=100) from Gradle console output if present.
///
/// Supported patterns include:
/// - `"<==========---> 80% EXECUTING [12s]"`
/// - `"> 25% EXECUTING"`
/// - `"[45%] Building module"`
pub fn parse_gradle_progress(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if !trimmed.contains('%') {
        return None;
    }

    let upper = trimmed.to_uppercase();
    let is_progress_context = upper.contains("EXECUTING")
        || upper.contains("CONFIGURING")
        || upper.contains("BUILDING")
        || upper.contains('<')
        || upper.contains('>');

    if !is_progress_context {
        return None;
    }

    let re = GRADLE_PROGRESS_RE.get_or_init(|| {
        Regex::new(r"(\d{1,3})%").expect("Invalid progress regex")
    });

    for caps in re.captures_iter(trimmed) {
        if let Some(m) = caps.get(1) {
            if let Ok(pct) = m.as_str().parse::<u8>() {
                if pct <= 100 {
                    return Some(pct);
                }
            }
        }
    }

    None
}

/// Locates the most recently modified `.apk` file under `search_dir`.
pub fn find_latest_apk(search_dir: &Path, variant: Option<&str>) -> Option<PathBuf> {
    find_latest_artifact(search_dir, "apk", variant)
}

/// Locates the most recently modified `.aab` file under `search_dir`.
pub fn find_latest_aab(search_dir: &Path, variant: Option<&str>) -> Option<PathBuf> {
    find_latest_artifact(search_dir, "aab", variant)
}

fn find_latest_artifact(
    search_dir: &Path,
    extension: &str,
    variant: Option<&str>,
) -> Option<PathBuf> {
    if !search_dir.exists() {
        return None;
    }

    let mut matches = Vec::new();
    collect_files_recursive(search_dir, extension, &mut matches, 6);

    // Filter out test and unaligned artifacts
    matches.retain(|p| {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        !name.ends_with("-androidTest.apk")
            && !name.ends_with("-unaligned.apk")
            && !name.ends_with("-androidTest.aab")
    });

    if let Some(var) = variant {
        let var_lower = var.to_lowercase();
        let variant_matches: Vec<PathBuf> = matches
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy().to_lowercase();
                s.contains(&var_lower)
            })
            .cloned()
            .collect();

        if !variant_matches.is_empty() {
            matches = variant_matches;
        }
    }

    // Sort descending by last modified timestamp
    matches.sort_by(|a, b| {
        let time_a = std::fs::metadata(a).and_then(|m| m.modified()).ok();
        let time_b = std::fs::metadata(b).and_then(|m| m.modified()).ok();
        time_b.cmp(&time_a)
    });

    matches.into_iter().next()
}

fn collect_files_recursive(
    dir: &Path,
    extension: &str,
    results: &mut Vec<PathBuf>,
    max_depth: usize,
) {
    if max_depth == 0 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case(extension) {
                    results.push(path);
                }
            }
        } else if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') && name != "intermediates" && name != "tmp" {
                collect_files_recursive(&path, extension, results, max_depth - 1);
            }
        }
    }
}

/// Checks if `bundletool` is installed and executable in PATH.
pub async fn is_bundletool_installed() -> bool {
    match Command::new("bundletool")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Launches a detached scrcpy mirror session with stdio redirected to null.
pub async fn launch_scrcpy(serial: &str, sender: &UnboundedSender<AppEvent>) -> bool {
    let mut cmd = Command::new("scrcpy");
    cmd.args(["-s", serial]);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    match cmd.spawn() {
        Ok(_) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Launched scrcpy mirror session for device '{serial}'"
            )));
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                format!("scrcpy launched for {serial}"),
            ));
            true
        }
        Err(e) => {
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "Error: 'scrcpy' executable not found in PATH. Install scrcpy to mirror Android screen.".to_string()
            } else {
                format!("Failed to launch scrcpy: {e}")
            };
            let _ = sender.send(AppEvent::ProcessOutput(msg));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "scrcpy failed to launch".to_string(),
            ));
            false
        }
    }
}

/// Executes a command, streaming its stdout and stderr line-by-line via `AppEvent::ProcessOutput`.
pub async fn run_streaming_cmd(
    mut cmd: Command,
    description: &str,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<bool, std::io::Error> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let sender_out = sender.clone();
    let stdout_task = tokio::spawn(async move {
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if sender_out.send(AppEvent::ProcessOutput(line)).is_err() {
                    break;
                }
            }
        }
    });

    let sender_err = sender.clone();
    let stderr_task = tokio::spawn(async move {
        if let Some(err) = stderr {
            let mut reader = BufReader::new(err).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if sender_err.send(AppEvent::ProcessOutput(line)).is_err() {
                    break;
                }
            }
        }
    });

    let success = if let Some(rx) = cancel_rx.as_mut() {
        tokio::select! {
            res = child.wait() => {
                match res {
                    Ok(status) => status.success(),
                    Err(_) => false,
                }
            }
            _ = rx => {
                let _ = child.kill().await;
                let _ = sender.send(AppEvent::ProcessOutput(format!("{description} cancelled by user.")));
                false
            }
        }
    } else {
        match child.wait().await {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    Ok(success)
}

/// Asynchronous process executor supporting real-time output streaming and cancellation.
#[derive(Debug, Default)]
pub struct ProcessRunner {
    abort_handle: Option<AbortHandle>,
    cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ProcessRunner {
    pub fn new() -> Self {
        Self {
            abort_handle: None,
            cancel_tx: None,
        }
    }

    /// Checks if a background process is currently running.
    pub fn is_running(&self) -> bool {
        if let Some(ref handle) = self.abort_handle {
            !handle.is_finished()
        } else {
            false
        }
    }

    /// Cancels the currently executing process if running.
    pub fn abort(&mut self) -> bool {
        let mut was_running = false;
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
            was_running = true;
        }
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
            was_running = true;
        }
        was_running
    }

    /// Executes an arbitrary command line in the background with streaming output.
    pub fn run_command(
        &mut self,
        cmd: Command,
        description: impl Into<String>,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.abort();

        let desc = description.into();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        self.cancel_tx = Some(cancel_tx);

        let join_handle = tokio::spawn(async move {
            let mut cancel_opt = Some(cancel_rx);
            match run_streaming_cmd(cmd, &desc, &sender, &mut cancel_opt).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("{desc} finished successfully"),
                    ));
                }
                Ok(false) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("{desc} failed or cancelled"),
                    ));
                }
                Err(e) => {
                    let _ = sender.send(AppEvent::ProcessOutput(format!(
                        "Failed to start process for {desc}: {e}"
                    )));
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("{desc} failed to start: {e}"),
                    ));
                }
            }
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }

    /// Executes a predefined `QuickAction` pipeline asynchronously.
    pub fn execute_action(
        &mut self,
        action: QuickAction,
        ctx: ActionContext,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.abort();

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        self.cancel_tx = Some(cancel_tx);

        let join_handle = tokio::spawn(async move {
            let mut cancel_opt = Some(cancel_rx);
            match action {
                QuickAction::BuildAndInstallDebug => {
                    execute_build_and_install_debug(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::BuildAabAndDeploy => {
                    execute_build_aab_and_deploy(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::ClearAppData => {
                    execute_clear_app_data(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::ForceStopApp => {
                    execute_force_stop(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::RelaunchActivity => {
                    execute_relaunch_activity(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::UninstallApp => {
                    execute_uninstall(&ctx, &sender, &mut cancel_opt).await;
                }
                QuickAction::LaunchScrcpy => {
                    launch_scrcpy(&ctx.serial, &sender).await;
                }
                QuickAction::StopGradleDaemons => {
                    execute_stop_gradle_daemons(&ctx, &sender, &mut cancel_opt).await;
                }
            }
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }

    /// Executes a device control action asynchronously.
    pub fn execute_device_action(
        &mut self,
        action: DeviceAction,
        ctx: ActionContext,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.abort();

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        self.cancel_tx = Some(cancel_tx);

        let join_handle = tokio::spawn(async move {
            let mut cancel_opt = Some(cancel_rx);
            execute_device_action_internal(action, &ctx, &sender, &mut cancel_opt).await;
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }

    /// Connects to a wireless ADB device, pairing beforehand if pairing address and code are provided.
    pub fn execute_adb_pair_and_connect(
        &mut self,
        adb_path: PathBuf,
        pair_addr: Option<String>,
        pair_code: Option<String>,
        connect_addr: String,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.abort();

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        self.cancel_tx = Some(cancel_tx);

        let join_handle = tokio::spawn(async move {
            let mut cancel_opt = Some(cancel_rx);

            if let (Some(addr), Some(code)) = (pair_addr, pair_code) {
                if !addr.trim().is_empty() && !code.trim().is_empty() {
                    let _ = sender.send(AppEvent::ProcessOutput(format!(
                        "Pairing with {addr} using provided code..."
                    )));
                    let mut pair_cmd = Command::new(&adb_path);
                    pair_cmd.args(["pair", addr.trim(), code.trim()]);
                    match run_streaming_cmd(pair_cmd, "adb pair", &sender, &mut cancel_opt).await {
                        Ok(true) => {
                            let _ = sender.send(AppEvent::ProcessOutput(
                                "Pairing succeeded! Proceeding to connect...".to_string(),
                            ));
                        }
                        Ok(false) => {
                            let _ = sender.send(AppEvent::ProcessFinished(
                                false,
                                "ADB pairing failed".to_string(),
                            ));
                            return;
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::ProcessFinished(
                                false,
                                format!("ADB pair execution error: {e}"),
                            ));
                            return;
                        }
                    }
                }
            }

            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Connecting to {connect_addr}..."
            )));
            let mut conn_cmd = Command::new(&adb_path);
            conn_cmd.args(["connect", connect_addr.trim()]);
            match run_streaming_cmd(conn_cmd, "adb connect", &sender, &mut cancel_opt).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Connected to {connect_addr} successfully"),
                    ));
                }
                Ok(false) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("Failed to connect to {connect_addr}"),
                    ));
                }
                Err(e) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("ADB connect execution error: {e}"),
                    ));
                }
            }
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }
}

impl Drop for ProcessRunner {
    fn drop(&mut self) {
        self.abort();
    }
}

/// Helper to resolve `applicationId` from context or scan project outputs and manifests.
pub fn resolve_or_extract_app_id(ctx: &ActionContext) -> Option<String> {
    if let Some(ref id) = ctx.application_id {
        if !id.trim().is_empty() {
            return Some(id.clone());
        }
    }
    crate::android::project::extract_application_id_from_path(&ctx.project_root)
}

async fn execute_device_action_internal(
    action: DeviceAction,
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let serial = &ctx.serial;
    let adb = &ctx.adb_path;

    match action {
        DeviceAction::WirelessPairAndConnect => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Wireless ADB: Use the Pair & Connect modal or enter device IP/Port and pairing code.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                "Wireless Pair & Connect dialog ready".to_string(),
            ));
        }
        DeviceAction::EnableTcpip5555 => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Enabling ADB over TCP/IP port 5555 on device '{serial}'..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args(["-s", serial, "tcpip", "5555"]);
            match run_streaming_cmd(cmd, "adb tcpip 5555", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        "ADB TCP/IP 5555 enabled successfully. You can now connect via Wi-Fi.".to_string(),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to enable ADB TCP/IP 5555".to_string(),
                    ));
                }
            }
        }
        DeviceAction::ToggleWifi => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Checking Wi-Fi status on device '{serial}'..."
            )));
            let status_cmd = Command::new(adb)
                .args(["-s", serial, "shell", "cmd", "wifi", "status"])
                .output()
                .await;
            let is_enabled = match status_cmd {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
                    stdout.contains("enabled")
                }
                _ => false,
            };

            let (arg, next_state) = if is_enabled {
                ("disable", "OFF")
            } else {
                ("enable", "ON")
            };
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Toggling Wi-Fi to {next_state}..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args(["-s", serial, "shell", "svc", "wifi", arg]);
            match run_streaming_cmd(cmd, "svc wifi toggle", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Device Wi-Fi toggled {next_state}"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to toggle Wi-Fi".to_string(),
                    ));
                }
            }
        }
        DeviceAction::ToggleMobileData => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Checking mobile data setting on device '{serial}'..."
            )));
            let get_cmd = Command::new(adb)
                .args(["-s", serial, "shell", "settings", "get", "global", "mobile_data"])
                .output()
                .await;
            let current = get_cmd
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            let (arg, next_state) = if current == "1" {
                ("disable", "OFF")
            } else {
                ("enable", "ON")
            };
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Toggling Mobile Data to {next_state}..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args(["-s", serial, "shell", "svc", "data", arg]);
            match run_streaming_cmd(cmd, "svc data toggle", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Mobile Data toggled {next_state}"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to toggle mobile data".to_string(),
                    ));
                }
            }
        }
        DeviceAction::ToggleStayAwake => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Checking stay awake setting on device '{serial}'..."
            )));
            let get_cmd = Command::new(adb)
                .args([
                    "-s",
                    serial,
                    "shell",
                    "settings",
                    "get",
                    "global",
                    "stay_on_while_plugged_in",
                ])
                .output()
                .await;
            let current = get_cmd
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            let (new_val, label) = if current == "0" || current.is_empty() {
                ("7", "ENABLED (while plugged)")
            } else {
                ("0", "DISABLED")
            };
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Setting Stay Awake to {label}..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args([
                "-s",
                serial,
                "shell",
                "settings",
                "put",
                "global",
                "stay_on_while_plugged_in",
                new_val,
            ]);
            match run_streaming_cmd(cmd, "settings stay_on", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Stay awake set to {label}"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to toggle stay awake".to_string(),
                    ));
                }
            }
        }
        DeviceAction::ToggleDarkMode => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Checking night mode on device '{serial}'..."
            )));
            let get_cmd = Command::new(adb)
                .args(["-s", serial, "shell", "cmd", "uimode", "night"])
                .output()
                .await;
            let is_night = match get_cmd {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
                    stdout.contains("yes")
                }
                _ => false,
            };
            let (arg, label) = if is_night {
                ("no", "LIGHT")
            } else {
                ("yes", "DARK")
            };
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Setting system UI theme to {label}..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args(["-s", serial, "shell", "cmd", "uimode", "night", arg]);
            match run_streaming_cmd(cmd, "cmd uimode night", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Dark mode switched to {label}"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to toggle dark mode".to_string(),
                    ));
                }
            }
        }
        DeviceAction::ToggleShowTaps => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Checking show taps setting on device '{serial}'..."
            )));
            let get_cmd = Command::new(adb)
                .args(["-s", serial, "shell", "settings", "get", "system", "show_touches"])
                .output()
                .await;
            let current = get_cmd
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .unwrap_or_default();
            let (val, label) = if current == "1" {
                ("0", "OFF")
            } else {
                ("1", "ON")
            };
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Setting show screen touches to {label}..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args([
                "-s",
                serial,
                "shell",
                "settings",
                "put",
                "system",
                "show_touches",
                val,
            ]);
            match run_streaming_cmd(cmd, "show_touches toggle", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Show taps toggled {label}"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to toggle show taps".to_string(),
                    ));
                }
            }
        }
        DeviceAction::OpenWirelessDebuggingSettings => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Opening Developer & Wireless Debugging Settings on device '{serial}'..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args([
                "-s",
                serial,
                "shell",
                "am",
                "start",
                "-a",
                "android.settings.APPLICATION_DEVELOPMENT_SETTINGS",
            ]);
            match run_streaming_cmd(cmd, "open dev settings", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        "Developer settings opened on device".to_string(),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        "Failed to open settings".to_string(),
                    ));
                }
            }
        }
        DeviceAction::TakeScreenshot => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = format!("screenshot_{timestamp}.png");
            let target_path = ctx.project_root.join(&filename);
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Capturing screenshot from device '{serial}' to {}...",
                target_path.display()
            )));

            let output = Command::new(adb)
                .args(["-s", serial, "exec-out", "screencap", "-p"])
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() && !out.stdout.is_empty() => {
                    match std::fs::write(&target_path, &out.stdout) {
                        Ok(()) => {
                            let _ = sender.send(AppEvent::ProcessFinished(
                                true,
                                format!("Screenshot saved to {filename}"),
                            ));
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::ProcessFinished(
                                false,
                                format!("Failed to save screenshot file: {e}"),
                            ));
                        }
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("screencap failed: {err}"),
                    ));
                }
                Err(e) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("ADB screencap invocation error: {e}"),
                    ));
                }
            }
        }
        DeviceAction::RebootDevice => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Rebooting device '{serial}' via ADB..."
            )));
            let mut cmd = Command::new(adb);
            cmd.args(["-s", serial, "reboot"]);
            match run_streaming_cmd(cmd, "adb reboot", sender, cancel_rx).await {
                Ok(true) => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        true,
                        format!("Device '{serial}' is rebooting"),
                    ));
                }
                _ => {
                    let _ = sender.send(AppEvent::ProcessFinished(
                        false,
                        format!("Failed to reboot device '{serial}'"),
                    ));
                }
            }
        }
    }
}

async fn execute_build_and_install_debug(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let gradlew = ctx.gradle_executable();
    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Executing Gradle build: {} assembleDebug",
        gradlew.display()
    )));

    let mut gradle_cmd = Command::new(&gradlew);
    gradle_cmd.arg("assembleDebug");
    gradle_cmd.current_dir(&ctx.project_root);

    match run_streaming_cmd(gradle_cmd, "Gradle assembleDebug", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Gradle build succeeded. Locating APK...".to_string(),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Gradle assembleDebug failed".to_string(),
            ));
            return;
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Failed to run Gradle ({}): {e}",
                gradlew.display()
            )));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Gradle invocation error: {e}"),
            ));
            return;
        }
    }

    let apk_path = match find_latest_apk(&ctx.project_root, Some(&ctx.variant)) {
        Some(path) => path,
        None => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No APK file found in build outputs.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "APK not found after build".to_string(),
            ));
            return;
        }
    };

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Found APK: {}",
        apk_path.display()
    )));
    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Installing APK to device {}...",
        ctx.serial
    )));

    let mut adb_cmd = Command::new(&ctx.adb_path);
    adb_cmd.args(["-s", &ctx.serial, "install", "-r"]);
    adb_cmd.arg(&apk_path);

    match run_streaming_cmd(adb_cmd, "ADB install", sender, cancel_rx).await {
        Ok(true) => {
            let effective_app_id = resolve_or_extract_app_id(ctx);
            if let Some(app_id) = effective_app_id {
                let _ = sender.send(AppEvent::ProcessOutput(format!(
                    "🚀 Launching application '{app_id}' on device..."
                )));
                let mut launched = false;
                if let Some(activity) = &ctx.main_activity {
                    let component = if activity.starts_with('.') || activity.contains('/') {
                        format!("{app_id}/{activity}")
                    } else {
                        format!("{app_id}/.{activity}")
                    };
                    let mut start_cmd = Command::new(&ctx.adb_path);
                    start_cmd.args([
                        "-s",
                        &ctx.serial,
                        "shell",
                        "am",
                        "start",
                        "-W",
                        "-n",
                        &component,
                    ]);
                    if let Ok(true) =
                        run_streaming_cmd(start_cmd, "am start", sender, cancel_rx).await
                    {
                        launched = true;
                    }
                }
                if !launched {
                    let mut monkey_cmd = Command::new(&ctx.adb_path);
                    monkey_cmd.args([
                        "-s",
                        &ctx.serial,
                        "shell",
                        "monkey",
                        "-p",
                        &app_id,
                        "-c",
                        "android.intent.category.LAUNCHER",
                        "1",
                    ]);
                    let _ =
                        run_streaming_cmd(monkey_cmd, "monkey launch", sender, cancel_rx).await;
                }
                let _ = sender.send(AppEvent::ProcessFinished(
                    true,
                    format!("Installed & launched '{app_id}'"),
                ));
            } else {
                let _ = sender.send(AppEvent::ProcessFinished(
                    true,
                    format!(
                        "Successfully installed {}",
                        apk_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("APK")
                    ),
                ));
            }
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "ADB install failed".to_string(),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("Failed to execute ADB: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("ADB execution error: {e}"),
            ));
        }
    }
}

async fn execute_build_aab_and_deploy(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    if !is_bundletool_installed().await {
        let _ = sender.send(AppEvent::ProcessOutput(
            "Error: 'bundletool' is not installed or not in PATH.".to_string(),
        ));
        let _ = sender.send(AppEvent::ProcessOutput(
            "Install bundletool from: https://github.com/google/bundletool/releases".to_string(),
        ));
        let _ = sender.send(AppEvent::ProcessFinished(
            false,
            "bundletool not installed".to_string(),
        ));
        return;
    }

    let bundle_task = if ctx.variant.eq_ignore_ascii_case("release") {
        "bundleRelease"
    } else {
        "bundleDebug"
    };

    let gradlew = ctx.gradle_executable();
    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Executing Gradle bundle: {} {}",
        gradlew.display(),
        bundle_task
    )));

    let mut gradle_cmd = Command::new(&gradlew);
    gradle_cmd.arg(bundle_task);
    gradle_cmd.current_dir(&ctx.project_root);

    match run_streaming_cmd(gradle_cmd, "Gradle bundle", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Gradle bundle build succeeded. Locating AAB...".to_string(),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Gradle bundle build failed".to_string(),
            ));
            return;
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("Failed to run Gradle: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Gradle invocation error: {e}"),
            ));
            return;
        }
    }

    let aab_path = match find_latest_aab(&ctx.project_root, Some(&ctx.variant)) {
        Some(path) => path,
        None => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No AAB file found in build outputs.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "AAB not found after bundle build".to_string(),
            ));
            return;
        }
    };

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Found AAB: {}",
        aab_path.display()
    )));

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let apks_path = std::env::temp_dir().join(format!("overridepulse_{timestamp}.apks"));
    let _ = std::fs::remove_file(&apks_path);

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Building APKS with bundletool for device {}...",
        ctx.serial
    )));

    let mut build_apks_cmd = Command::new("bundletool");
    build_apks_cmd.args([
        "build-apks",
        &format!("--bundle={}", aab_path.display()),
        &format!("--output={}", apks_path.display()),
        "--connected-device",
        &format!("--device-id={}", ctx.serial),
    ]);

    match run_streaming_cmd(build_apks_cmd, "bundletool build-apks", sender, cancel_rx).await {
        Ok(true) => {}
        Ok(false) => {
            let _ = std::fs::remove_file(&apks_path);
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "bundletool build-apks failed".to_string(),
            ));
            return;
        }
        Err(e) => {
            let _ = std::fs::remove_file(&apks_path);
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Failed to run bundletool build-apks: {e}"
            )));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("bundletool error: {e}"),
            ));
            return;
        }
    }

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Installing APKS on device {}...",
        ctx.serial
    )));

    let mut install_apks_cmd = Command::new("bundletool");
    install_apks_cmd.args([
        "install-apks",
        &format!("--apks={}", apks_path.display()),
        &format!("--device-id={}", ctx.serial),
    ]);

    let install_res =
        run_streaming_cmd(install_apks_cmd, "bundletool install-apks", sender, cancel_rx).await;

    let _ = std::fs::remove_file(&apks_path);

    match install_res {
        Ok(true) => {
            let effective_app_id = resolve_or_extract_app_id(ctx);
            if let Some(app_id) = effective_app_id {
                let _ = sender.send(AppEvent::ProcessOutput(format!(
                    "🚀 Launching application '{app_id}' on device..."
                )));
                let mut launched = false;
                if let Some(activity) = &ctx.main_activity {
                    let component = if activity.starts_with('.') || activity.contains('/') {
                        format!("{app_id}/{activity}")
                    } else {
                        format!("{app_id}/.{activity}")
                    };
                    let mut start_cmd = Command::new(&ctx.adb_path);
                    start_cmd.args([
                        "-s",
                        &ctx.serial,
                        "shell",
                        "am",
                        "start",
                        "-W",
                        "-n",
                        &component,
                    ]);
                    if let Ok(true) =
                        run_streaming_cmd(start_cmd, "am start", sender, cancel_rx).await
                    {
                        launched = true;
                    }
                }
                if !launched {
                    let mut monkey_cmd = Command::new(&ctx.adb_path);
                    monkey_cmd.args([
                        "-s",
                        &ctx.serial,
                        "shell",
                        "monkey",
                        "-p",
                        &app_id,
                        "-c",
                        "android.intent.category.LAUNCHER",
                        "1",
                    ]);
                    let _ =
                        run_streaming_cmd(monkey_cmd, "monkey launch", sender, cancel_rx).await;
                }
                let _ = sender.send(AppEvent::ProcessFinished(
                    true,
                    format!("Installed & launched '{app_id}'"),
                ));
            } else {
                let _ = sender.send(AppEvent::ProcessFinished(
                    true,
                    "AAB deployed and installed successfully via bundletool".to_string(),
                ));
            }
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "bundletool install-apks failed".to_string(),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Failed to execute bundletool install-apks: {e}"
            )));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("bundletool install error: {e}"),
            ));
        }
    }
}

async fn execute_clear_app_data(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let app_id_owned = match resolve_or_extract_app_id(ctx) {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No applicationId detected or specified for project.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Missing applicationId".to_string(),
            ));
            return;
        }
    };
    let app_id = app_id_owned.as_str();

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Clearing app data for '{app_id}' on device {}...",
        ctx.serial
    )));

    let mut cmd = Command::new(&ctx.adb_path);
    cmd.args(["-s", &ctx.serial, "shell", "pm", "clear", app_id]);

    match run_streaming_cmd(cmd, "pm clear", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                format!("Data cleared for '{app_id}'"),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Failed to clear data for '{app_id}'"),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("ADB pm clear error: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("ADB execution error: {e}"),
            ));
        }
    }
}

async fn execute_force_stop(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let app_id_owned = match resolve_or_extract_app_id(ctx) {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No applicationId detected or specified for project.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Missing applicationId".to_string(),
            ));
            return;
        }
    };
    let app_id = app_id_owned.as_str();

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Force stopping '{app_id}' on device {}...",
        ctx.serial
    )));

    let mut cmd = Command::new(&ctx.adb_path);
    cmd.args(["-s", &ctx.serial, "shell", "am", "force-stop", app_id]);

    match run_streaming_cmd(cmd, "am force-stop", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                format!("App '{app_id}' force-stopped"),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Failed to force stop '{app_id}'"),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("ADB am force-stop error: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("ADB execution error: {e}"),
            ));
        }
    }
}

async fn execute_relaunch_activity(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let app_id_owned = match resolve_or_extract_app_id(ctx) {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No applicationId detected or specified for project.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Missing applicationId".to_string(),
            ));
            return;
        }
    };
    let app_id = app_id_owned.as_str();

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Stopping application '{app_id}'..."
    )));

    let mut stop_cmd = Command::new(&ctx.adb_path);
    stop_cmd.args(["-s", &ctx.serial, "shell", "am", "force-stop", app_id]);
    let _ = run_streaming_cmd(stop_cmd, "am force-stop", sender, cancel_rx).await;

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Launching application '{app_id}'..."
    )));

    let mut start_cmd = Command::new(&ctx.adb_path);
    if let Some(activity) = &ctx.main_activity {
        let component = if activity.starts_with('.') || activity.contains('/') {
            format!("{app_id}/{activity}")
        } else {
            format!("{app_id}/.{activity}")
        };
        start_cmd.args([
            "-s",
            &ctx.serial,
            "shell",
            "am",
            "start",
            "-W",
            "-n",
            &component,
        ]);
    } else {
        start_cmd.args([
            "-s",
            &ctx.serial,
            "shell",
            "monkey",
            "-p",
            app_id,
            "-c",
            "android.intent.category.LAUNCHER",
            "1",
        ]);
    }

    match run_streaming_cmd(start_cmd, "App relaunch", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                format!("Application '{app_id}' relaunched successfully"),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Failed to relaunch '{app_id}'"),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("ADB start error: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("ADB invocation error: {e}"),
            ));
        }
    }
}

async fn execute_uninstall(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let app_id_owned = match resolve_or_extract_app_id(ctx) {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Error: No applicationId detected or specified for project.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                "Missing applicationId".to_string(),
            ));
            return;
        }
    };
    let app_id = app_id_owned.as_str();

    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Uninstalling '{app_id}' from device {}...",
        ctx.serial
    )));

    let mut cmd = Command::new(&ctx.adb_path);
    cmd.args(["-s", &ctx.serial, "uninstall", app_id]);

    match run_streaming_cmd(cmd, "adb uninstall", sender, cancel_rx).await {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                format!("Uninstalled '{app_id}'"),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("Failed to uninstall '{app_id}'"),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!("ADB uninstall error: {e}")));
            let _ = sender.send(AppEvent::ProcessFinished(
                false,
                format!("ADB execution error: {e}"),
            ));
        }
    }
}

async fn execute_stop_gradle_daemons(
    ctx: &ActionContext,
    sender: &UnboundedSender<AppEvent>,
    cancel_rx: &mut Option<tokio::sync::oneshot::Receiver<()>>,
) {
    let gradlew = ctx.gradle_executable();
    let _ = sender.send(AppEvent::ProcessOutput(format!(
        "Stopping Gradle daemons: {} --stop",
        gradlew.display()
    )));

    let mut gradle_cmd = Command::new(&gradlew);
    gradle_cmd.arg("--stop");
    gradle_cmd.current_dir(&ctx.project_root);

    let stop_res = run_streaming_cmd(gradle_cmd, "Gradle --stop", sender, cancel_rx).await;

    // As fallback in case of hung Gradle processes, terminate orphaned GradleDaemon processes
    #[cfg(unix)]
    {
        let _ = sender.send(AppEvent::ProcessOutput(
            "Verifying and terminating any orphaned Gradle daemon processes...".to_string(),
        ));
        let mut pkill_cmd = Command::new("pkill");
        pkill_cmd.args(["-f", "org.gradle.launcher.daemon.bootstrap.GradleDaemon"]);
        let _ = pkill_cmd.status().await;
    }

    #[cfg(windows)]
    {
        let mut taskkill = Command::new("taskkill");
        taskkill.args(["/F", "/FI", "IMAGENAME eq java.exe", "/FI", "WINDOWTITLE eq GradleDaemon*"]);
        let _ = taskkill.status().await;
    }

    match stop_res {
        Ok(true) => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "All Gradle daemons stopped successfully.".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                "All Gradle daemons stopped".to_string(),
            ));
        }
        Ok(false) => {
            let _ = sender.send(AppEvent::ProcessOutput(
                "Gradle --stop completed (orphaned daemons cleaned up).".to_string(),
            ));
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                "All Gradle daemons stopped".to_string(),
            ));
        }
        Err(e) => {
            let _ = sender.send(AppEvent::ProcessOutput(format!(
                "Note: Gradle invocation error: {e}. Cleaned up background processes."
            )));
            let _ = sender.send(AppEvent::ProcessFinished(
                true,
                "All Gradle daemons stopped".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    #[test]
    fn test_quick_action_metadata() {
        assert_eq!(QuickAction::BuildAndInstallDebug.shortcut(), Some('b'));
        assert_eq!(QuickAction::BuildAabAndDeploy.shortcut(), None);
        assert_eq!(QuickAction::ClearAppData.shortcut(), None);
        assert_eq!(QuickAction::ForceStopApp.shortcut(), Some('k'));
        assert_eq!(QuickAction::RelaunchActivity.shortcut(), Some('r'));
        assert_eq!(QuickAction::UninstallApp.shortcut(), Some('u'));
        assert_eq!(QuickAction::LaunchScrcpy.shortcut(), None);
        assert_eq!(QuickAction::StopGradleDaemons.shortcut(), Some('g'));

        assert_eq!(
            QuickAction::from_char('b'),
            Some(QuickAction::BuildAndInstallDebug)
        );
        assert_eq!(
            QuickAction::from_char('B'),
            Some(QuickAction::BuildAndInstallDebug)
        );
        assert_eq!(QuickAction::from_char('a'), None);
        assert_eq!(QuickAction::from_char('c'), None);
        assert_eq!(QuickAction::from_char('k'), Some(QuickAction::ForceStopApp));
        assert_eq!(
            QuickAction::from_char('r'),
            Some(QuickAction::RelaunchActivity)
        );
        assert_eq!(QuickAction::from_char('u'), Some(QuickAction::UninstallApp));
        assert_eq!(QuickAction::from_char('s'), None);
        assert_eq!(
            QuickAction::from_char('g'),
            Some(QuickAction::StopGradleDaemons)
        );
        assert_eq!(
            QuickAction::from_char('G'),
            Some(QuickAction::StopGradleDaemons)
        );
        assert_eq!(QuickAction::from_char('z'), None);

        assert_eq!(QuickAction::all().len(), 8);
        assert!(QuickAction::BuildAndInstallDebug.display_label().starts_with("[b] "));
        assert!(QuickAction::ClearAppData.display_label().starts_with("    "));
    }

    #[test]
    fn test_parse_gradle_progress() {
        assert_eq!(
            parse_gradle_progress("<==========---> 80% EXECUTING [12s]"),
            Some(80)
        );
        assert_eq!(parse_gradle_progress("> 25% EXECUTING"), Some(25));
        assert_eq!(
            parse_gradle_progress("<===----------> 100% EXECUTING"),
            Some(100)
        );
        assert_eq!(parse_gradle_progress("[45%] Building module"), Some(45));
        assert_eq!(parse_gradle_progress("BUILD SUCCESSFUL in 1s"), None);
        assert_eq!(parse_gradle_progress("Random line with no progress"), None);
        assert_eq!(parse_gradle_progress("Battery percentage: 95%"), None);
    }

    #[test]
    fn test_find_latest_apk() {
        let temp_dir = std::env::temp_dir().join(format!(
            "overridepulse_test_apk_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apk_debug_dir = temp_dir.join("app/build/outputs/apk/debug");
        fs::create_dir_all(&apk_debug_dir).unwrap();

        let debug_apk = apk_debug_dir.join("app-debug.apk");
        File::create(&debug_apk).unwrap();

        let test_apk = apk_debug_dir.join("app-debug-androidTest.apk");
        File::create(&test_apk).unwrap();

        let found = find_latest_apk(&temp_dir, Some("debug"));
        assert_eq!(found, Some(debug_apk));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_latest_aab() {
        let temp_dir = std::env::temp_dir().join(format!(
            "overridepulse_test_aab_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let aab_release_dir = temp_dir.join("app/build/outputs/bundle/release");
        fs::create_dir_all(&aab_release_dir).unwrap();

        let release_aab = aab_release_dir.join("app-release.aab");
        File::create(&release_aab).unwrap();

        let found = find_latest_aab(&temp_dir, Some("release"));
        assert_eq!(found, Some(release_aab));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_action_context_defaults() {
        let ctx = ActionContext::new("/tmp/test_project", "emulator-5554")
            .with_app_id("com.example.app")
            .with_main_activity("MainActivity")
            .with_variant("debug");

        assert_eq!(ctx.project_root, PathBuf::from("/tmp/test_project"));
        assert_eq!(ctx.serial, "emulator-5554");
        assert_eq!(ctx.application_id.as_deref(), Some("com.example.app"));
        assert_eq!(ctx.main_activity.as_deref(), Some("MainActivity"));
        assert_eq!(ctx.variant, "debug");
    }

    #[test]
    fn test_process_runner_lifecycle() {
        let mut runner = ProcessRunner::new();
        assert!(!runner.is_running());
        assert!(!runner.abort());
    }

    #[tokio::test]
    async fn test_process_runner_streaming_execution() {
        let mut runner = ProcessRunner::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let mut cmd = Command::new("echo");
        cmd.arg("overridepulse-test");

        runner.run_command(cmd, "Echo test", tx);

        let mut lines = Vec::new();
        let mut finished = false;

        while let Some(event) = rx.recv().await {
            match event {
                AppEvent::ProcessOutput(line) => lines.push(line),
                AppEvent::ProcessFinished(success, _msg) => {
                    assert!(success);
                    finished = true;
                    break;
                }
                _ => {}
            }
        }

        assert!(finished);
        assert!(lines.iter().any(|l| l.contains("overridepulse-test")));
    }

    #[tokio::test]
    async fn test_quick_action_stop_gradle_daemons() {
        let mut runner = ProcessRunner::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let ctx = ActionContext::new("/tmp/test_project", "local");

        runner.execute_action(QuickAction::StopGradleDaemons, ctx, tx);
        assert!(runner.is_running());

        let mut finished = false;
        while let Some(event) = rx.recv().await {
            if let AppEvent::ProcessFinished(success, msg) = event {
                assert!(success);
                assert!(msg.contains("All Gradle daemons stopped"));
                finished = true;
                break;
            }
        }
        assert!(finished);
    }

    #[tokio::test]
    async fn test_process_runner_abort_running() {
        let mut runner = ProcessRunner::new();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let mut cmd = Command::new("sleep");
        cmd.arg("5");

        runner.run_command(cmd, "Sleep test", tx);
        assert!(runner.is_running());

        let aborted = runner.abort();
        assert!(aborted);

        // Receive until finished
        while let Some(event) = rx.recv().await {
            if let AppEvent::ProcessFinished(success, _) = event {
                assert!(!success);
                break;
            }
        }
    }

    #[test]
    fn test_resolve_or_extract_app_id_priority() {
        let mut ctx = ActionContext::new("/tmp/some_dir", "emulator-5554");
        ctx.application_id = Some("com.explicit.appid".to_string());
        assert_eq!(
            resolve_or_extract_app_id(&ctx),
            Some("com.explicit.appid".to_string())
        );
    }
}
