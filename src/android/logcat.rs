use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::AbortHandle;

use crate::event::AppEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    pub fn priority(&self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Verbose => 1,
            Self::Debug => 2,
            Self::Info => 3,
            Self::Warn => 4,
            Self::Error => 5,
            Self::Fatal => 6,
        }
    }

    pub fn as_char(&self) -> char {
        match self {
            Self::Verbose => 'V',
            Self::Debug => 'D',
            Self::Info => 'I',
            Self::Warn => 'W',
            Self::Error => 'E',
            Self::Fatal => 'F',
            Self::Unknown => '?',
        }
    }

    pub fn from_char(c: char) -> Self {
        match c.to_ascii_uppercase() {
            'V' => Self::Verbose,
            'D' => Self::Debug,
            'I' => Self::Info,
            'W' => Self::Warn,
            'E' => Self::Error,
            'F' | 'A' => Self::Fatal,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Verbose => "VERBOSE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub raw: String,
    pub timestamp: Option<String>,
    pub pid: Option<u32>,
    pub tid: Option<u32>,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
}

static MODERN_THREADTIME_RE: OnceLock<Regex> = OnceLock::new();
static TIME_PID_RE: OnceLock<Regex> = OnceLock::new();
static TIME_NOPID_RE: OnceLock<Regex> = OnceLock::new();

/// `threadtime` as printed by current platform-tools:
/// `MM-DD HH:MM:SS.mmm PID TID L Tag: msg` and the Android 14+ form that
/// inserts a UID and may use microsecond timestamps or a leading year.
fn modern_threadtime_regex() -> &'static Regex {
    MODERN_THREADTIME_RE.get_or_init(|| {
        Regex::new(
            r"^(?P<ts>(?:\d{4}-)?\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3,9})\s+(?P<a>\d+)\s+(?P<b>\d+)\s+(?:(?P<c>\d+)\s+)?(?P<lvl>[VDIWEFAvdiwefa])\s+(?P<tag>.*?):\s?(?P<msg>.*)$",
        )
        .expect("Invalid modern threadtime regex")
    })
}

fn time_pid_regex() -> &'static Regex {
    TIME_PID_RE.get_or_init(|| {
        Regex::new(
            r"^((?:\d{4}-)?\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3,9})\s+([VDIWEFAvdiwefa])/(.*?)\(\s*(\d+)\s*\):\s?(.*)$",
        )
        .expect("Invalid time_pid regex")
    })
}

fn time_nopid_regex() -> &'static Regex {
    TIME_NOPID_RE.get_or_init(|| {
        Regex::new(
            r"^((?:\d{4}-)?\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3,9})\s+([VDIWEFAvdiwefa])/(.*?):\s?(.*)$",
        )
        .expect("Invalid time_nopid regex")
    })
}

fn parse_modern_threadtime(line: &str) -> Option<LogEntry> {
    let caps = modern_threadtime_regex().captures(line)?;
    let timestamp = caps.name("ts").map(|m| m.as_str().to_string());
    let level_char = caps
        .name("lvl")
        .and_then(|m| m.as_str().chars().next())
        .unwrap_or('?');
    let has_uid = caps.name("c").is_some();
    let pid = if has_uid {
        caps.name("b").and_then(|m| m.as_str().parse::<u32>().ok())
    } else {
        caps.name("a").and_then(|m| m.as_str().parse::<u32>().ok())
    };
    let tid = if has_uid {
        caps.name("c").and_then(|m| m.as_str().parse::<u32>().ok())
    } else {
        caps.name("b").and_then(|m| m.as_str().parse::<u32>().ok())
    };
    Some(LogEntry {
        raw: line.to_string(),
        timestamp,
        pid,
        tid,
        level: LogLevel::from_char(level_char),
        tag: caps
            .name("tag")
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default(),
        message: caps
            .name("msg")
            .map(|m| m.as_str().to_string())
            .unwrap_or_default(),
    })
}

impl LogEntry {
    pub fn new_raw(raw: impl Into<String>) -> Self {
        let raw_str = raw.into();
        Self::parse_line(&raw_str).unwrap_or_else(|| Self {
            raw: raw_str.clone(),
            timestamp: None,
            pid: None,
            tid: None,
            level: LogLevel::Unknown,
            tag: String::new(),
            message: raw_str,
        })
    }

    /// Parses a line of logcat output, supporting both `-v threadtime` and `-v time` formats.
    ///
    /// Examples:
    /// - Threadtime: `"10-23 15:30:12.345  1234  5678 D MyActivity: Application started"`
    /// - Time with PID: `"10-23 15:30:12.345 D/MyTag( 1234): Mensaje"`
    /// - Time without PID: `"10-23 15:30:12.345 I/SystemServer: System ready"`
    pub fn parse_line(line: &str) -> Option<Self> {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            return None;
        }

        // `-v threadtime`, including the UID column and microsecond timestamps.
        if let Some(entry) = parse_modern_threadtime(trimmed) {
            return Some(entry);
        }

        // Try `-v time` format with PID: "10-23 15:30:12.345 D/MyTag( 1234): Msg"
        if let Some(caps) = time_pid_regex().captures(trimmed) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string());
            let level_char = caps
                .get(2)
                .and_then(|m| m.as_str().chars().next())
                .unwrap_or('?');
            let level = LogLevel::from_char(level_char);
            let tag = caps
                .get(3)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let pid = caps.get(4).and_then(|m| m.as_str().parse::<u32>().ok());
            let message = caps
                .get(5)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            return Some(Self {
                raw: trimmed.to_string(),
                timestamp,
                pid,
                tid: None,
                level,
                tag,
                message,
            });
        }

        // Try `-v time` format without PID: "10-23 15:30:12.345 I/System: Msg"
        if let Some(caps) = time_nopid_regex().captures(trimmed) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string());
            let level_char = caps
                .get(2)
                .and_then(|m| m.as_str().chars().next())
                .unwrap_or('?');
            let level = LogLevel::from_char(level_char);
            let tag = caps
                .get(3)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let message = caps
                .get(4)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            return Some(Self {
                raw: trimmed.to_string(),
                timestamp,
                pid: None,
                tid: None,
                level,
                tag,
                message,
            });
        }

        // Fallback for unformatted lines, raw dumps or section headers
        Some(Self {
            raw: trimmed.to_string(),
            timestamp: None,
            pid: None,
            tid: None,
            level: LogLevel::Unknown,
            tag: String::new(),
            message: trimmed.to_string(),
        })
    }

    /// Evaluates if the log entry matches in-memory filter criteria (query, min_level, PID, and package).
    pub fn matches_filter(
        &self,
        query: &str,
        min_level: Option<LogLevel>,
        app_pid: Option<u32>,
        package_name: Option<&str>,
    ) -> bool {
        // 1. Min level filter
        if let Some(min) = min_level {
            if self.level.priority() < min.priority() {
                return false;
            }
        }

        // 2. PID or package filtering
        if let Some(target_pid) = app_pid {
            match self.pid {
                Some(pid) => {
                    if pid != target_pid {
                        return false;
                    }
                }
                None => {
                    // If entry doesn't have PID, check if package_name matches
                    if let Some(pkg) = package_name {
                        if !self.raw.contains(pkg) && !self.tag.contains(pkg) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
        } else if let Some(pkg) = package_name {
            if !self.raw.contains(pkg) && !self.tag.contains(pkg) && !self.message.contains(pkg) {
                return false;
            }
        }

        // 3. Query search filter (case-insensitive across tag, message, raw)
        let q = query.trim();
        if !q.is_empty() {
            let q_lower = q.to_lowercase();
            let matches_query = self.tag.to_lowercase().contains(&q_lower)
                || self.message.to_lowercase().contains(&q_lower)
                || self.raw.to_lowercase().contains(&q_lower);
            if !matches_query {
                return false;
            }
        }

        true
    }
}

/// Asynchronous logcat stream manager that polls logs via ADB and streams them to the UI.
#[derive(Debug)]
pub struct LogcatStream {
    abort_handle: Option<AbortHandle>,
    current_serial: Option<String>,
    filter_by_app: Arc<AtomicBool>,
    package_name: Option<String>,
    build_variant: Option<String>,
}

impl Default for LogcatStream {
    fn default() -> Self {
        Self::new()
    }
}

impl LogcatStream {
    pub fn new() -> Self {
        Self {
            abort_handle: None,
            current_serial: None,
            filter_by_app: Arc::new(AtomicBool::new(false)),
            package_name: None,
            build_variant: None,
        }
    }

    pub fn is_running(&self) -> bool {
        if let Some(ref handle) = self.abort_handle {
            !handle.is_finished()
        } else {
            false
        }
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
        }
        self.current_serial = None;
        self.build_variant = None;
    }

    pub fn set_filter_by_app(&self, enabled: bool) {
        self.filter_by_app.store(enabled, Ordering::SeqCst);
    }

    pub fn is_filtering_by_app(&self) -> bool {
        self.filter_by_app.load(Ordering::SeqCst)
    }

    pub fn current_serial(&self) -> Option<&str> {
        self.current_serial.as_deref()
    }

    pub fn package_name(&self) -> Option<&str> {
        self.package_name.as_deref()
    }

    pub fn build_variant(&self) -> Option<&str> {
        self.build_variant.as_deref()
    }

    /// Clears the logcat buffer on the connected device (`adb -s <serial> logcat -c`).
    pub async fn clear_buffer(serial: &str) -> Result<(), std::io::Error> {
        let status = Command::new("adb")
            .args(["-s", serial, "logcat", "-c"])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "adb logcat -c exited with status {status}"
            )))
        }
    }

    /// Starts streaming logcat for one device.
    ///
    /// App-only mode follows the package UID, so every process of the app is
    /// included (`:push`, WebView, workers). Debug variants also match
    /// `applicationId + ".debug"`. Devices without `--uid` fall back to the
    /// live PID list from `pidof` / `ps`.
    pub fn start(
        &mut self,
        adb_path: impl Into<PathBuf>,
        serial: impl Into<String>,
        package_name: Option<String>,
        build_variant: impl Into<String>,
        filter_by_app: bool,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.stop();

        let serial = serial.into();
        let variant = build_variant.into();
        self.current_serial = Some(serial.clone());
        self.package_name = package_name.clone();
        self.build_variant = Some(variant.clone());
        self.filter_by_app.store(filter_by_app, Ordering::SeqCst);

        let adb_path = adb_path.into();
        let join_handle = tokio::spawn(async move {
            run_logcat_loop(
                adb_path,
                serial,
                package_name,
                variant,
                filter_by_app,
                sender,
            )
            .await;
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }
}

enum LogcatMode {
    All,
    Uid(u32),
    Pid(u32),
    Pids(Vec<u32>),
}

struct SessionOutcome {
    exited_immediately: bool,
    disconnected: bool,
}

fn info_entry(message: impl Into<String>) -> LogEntry {
    let message = message.into();
    LogEntry {
        raw: message.clone(),
        timestamp: None,
        pid: None,
        tid: None,
        level: LogLevel::Info,
        tag: "OverridePulse".to_string(),
        message,
    }
}

async fn run_logcat_loop(
    adb: PathBuf,
    serial: String,
    package_name: Option<String>,
    build_variant: String,
    filter_by_app: bool,
    sender: UnboundedSender<AppEvent>,
) {
    let package = package_name.and_then(|pkg| {
        let trimmed = pkg.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    if filter_by_app && package.is_none() {
        let _ = sender.send(AppEvent::LogcatLine(info_entry(
            "No applicationId detected yet. Build the app first to stream its Logcat.",
        )));
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    let prefer_debug = build_variant.to_ascii_lowercase().contains("debug");
    let mut uid_unsupported = false;
    let mut announced = String::new();

    loop {
        let mode = if !filter_by_app {
            LogcatMode::All
        } else if let Some(pkg) = package.as_deref() {
            let uid = if uid_unsupported {
                None
            } else {
                resolve_app_uid(&adb, &serial, pkg, prefer_debug).await
            };
            if let Some(uid) = uid {
                LogcatMode::Uid(uid)
            } else {
                let pids = resolve_app_pids(&adb, &serial, pkg).await;
                if pids.len() == 1 {
                    LogcatMode::Pid(pids[0])
                } else if pids.len() > 1 {
                    LogcatMode::Pids(pids)
                } else {
                    let message = format!("Waiting for '{pkg}' on the device…");
                    if announced != message {
                        if sender
                            .send(AppEvent::LogcatLine(info_entry(message.clone())))
                            .is_err()
                        {
                            return;
                        }
                        announced = message;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        } else {
            LogcatMode::All
        };

        let description = match &mode {
            LogcatMode::All => "Streaming device logcat".to_string(),
            LogcatMode::Uid(uid) => format!(
                "Streaming {} (uid {uid})",
                package.as_deref().unwrap_or("app")
            ),
            LogcatMode::Pid(pid) => format!(
                "Streaming {} (pid {pid})",
                package.as_deref().unwrap_or("app")
            ),
            LogcatMode::Pids(pids) => format!(
                "Streaming {} ({} processes)",
                package.as_deref().unwrap_or("app"),
                pids.len()
            ),
        };
        if announced != description {
            if sender
                .send(AppEvent::LogcatLine(info_entry(description.clone())))
                .is_err()
            {
                return;
            }
            announced = description;
        }

        let outcome = run_logcat_session(&adb, &serial, &mode, package.as_deref(), &sender).await;
        if outcome.disconnected {
            return;
        }
        if matches!(mode, LogcatMode::Uid(_)) && outcome.exited_immediately {
            uid_unsupported = true;
            announced.clear();
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
}

async fn run_logcat_session(
    adb: &Path,
    serial: &str,
    mode: &LogcatMode,
    package: Option<&str>,
    sender: &UnboundedSender<AppEvent>,
) -> SessionOutcome {
    let mut cmd = Command::new(adb);
    cmd.args(["-s", serial, "logcat", "-v", "threadtime", "-T", "250"]);
    match mode {
        LogcatMode::Uid(uid) => {
            cmd.arg(format!("--uid={uid}"));
        }
        LogcatMode::Pid(pid) => {
            cmd.arg(format!("--pid={pid}"));
        }
        LogcatMode::All | LogcatMode::Pids(_) => {}
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    cmd.kill_on_drop(true);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            let _ = sender.send(AppEvent::LogcatLine(info_entry(format!(
                "Failed to spawn adb logcat: {err}"
            ))));
            return SessionOutcome {
                exited_immediately: false,
                disconnected: false,
            };
        }
    };

    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill().await;
        return SessionOutcome {
            exited_immediately: true,
            disconnected: false,
        };
    };

    let mut reader = BufReader::new(stdout).lines();
    let mut saw_line = false;
    let started = Instant::now();
    let mut pids = match mode {
        LogcatMode::Pids(pids) => pids.clone(),
        _ => Vec::new(),
    };
    let mut pid_check = tokio::time::interval(Duration::from_millis(1500));
    pid_check.tick().await;

    loop {
        tokio::select! {
            line_res = reader.next_line() => {
                match line_res {
                    Ok(Some(line)) => {
                        saw_line = true;
                        if let Some(entry) = LogEntry::parse_line(&line) {
                            if accept_log_line(&entry, mode, package, &pids)
                                && sender.send(AppEvent::LogcatLine(entry)).is_err()
                            {
                                let _ = child.kill().await;
                                return SessionOutcome {
                                    exited_immediately: false,
                                    disconnected: true,
                                };
                            }
                        }
                    }
                    Ok(None) | Err(_) => break,
                }
            }
            _ = pid_check.tick() => {
                match mode {
                    LogcatMode::Pid(pid) => {
                        if let Some(pkg) = package {
                            let active = resolve_app_pids(adb, serial, pkg).await;
                            if !active.contains(pid) {
                                let _ = child.kill().await;
                                break;
                            }
                        }
                    }
                    LogcatMode::Pids(_) => {
                        if let Some(pkg) = package {
                            let active = resolve_app_pids(adb, serial, pkg).await;
                            if !active.is_empty() {
                                pids = active;
                            }
                        }
                    }
                    LogcatMode::All | LogcatMode::Uid(_) => {}
                }
            }
        }
    }

    let _ = child.kill().await;
    let _ = child.wait().await;
    SessionOutcome {
        exited_immediately: !saw_line && started.elapsed() < Duration::from_millis(700),
        disconnected: false,
    }
}

fn accept_log_line(
    entry: &LogEntry,
    mode: &LogcatMode,
    package: Option<&str>,
    pids: &[u32],
) -> bool {
    match mode {
        LogcatMode::All | LogcatMode::Uid(_) | LogcatMode::Pid(_) => true,
        LogcatMode::Pids(_) => {
            if entry.pid.is_some_and(|pid| pids.contains(&pid)) {
                return true;
            }
            package.is_some_and(|pkg| {
                entry.raw.contains(pkg) || entry.tag.contains(pkg) || entry.message.contains(pkg)
            })
        }
    }
}

async fn adb_shell(adb: &Path, serial: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(adb)
        .arg("-s")
        .arg(serial)
        .arg("shell")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

async fn resolve_app_uid(
    adb: &Path,
    serial: &str,
    package: &str,
    prefer_debug: bool,
) -> Option<u32> {
    if let Some(out) = adb_shell(
        adb,
        serial,
        &["cmd", "package", "list", "packages", "-U", package],
    )
    .await
    {
        if let Some(uid) = parse_package_uid(&out, package, prefer_debug) {
            return Some(uid);
        }
    }
    None
}

fn parse_package_uid(output: &str, package: &str, prefer_debug: bool) -> Option<u32> {
    let debug_name = format!("{package}.debug");
    let mut exact = None;
    let mut debug = None;
    let mut only = None;
    let mut count = 0u32;
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix("package:") else {
            continue;
        };
        let mut parts = rest.split_whitespace();
        let name = parts.next().unwrap_or("");
        let uid = parts
            .next()
            .and_then(|token| token.strip_prefix("uid:"))
            .and_then(|value| value.parse::<u32>().ok())
            .or_else(|| {
                rest.split("uid:")
                    .nth(1)
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse().ok())
            })?;
        count += 1;
        only = Some(uid);
        if name == package {
            exact = Some(uid);
        } else if name == debug_name {
            debug = Some(uid);
        }
    }
    if prefer_debug {
        debug.or(exact).or(if count == 1 { only } else { None })
    } else {
        exact.or(debug).or(if count == 1 { only } else { None })
    }
}

async fn resolve_app_pids(adb: &Path, serial: &str, package: &str) -> Vec<u32> {
    let mut packages = vec![package.to_string()];
    if !package.ends_with(".debug") {
        packages.push(format!("{package}.debug"));
    }
    for pkg in &packages {
        if let Some(out) = adb_shell(adb, serial, &["pidof", pkg]).await {
            let pids: Vec<u32> = out
                .split_whitespace()
                .filter_map(|token| token.parse().ok())
                .collect();
            if !pids.is_empty() {
                return pids;
            }
        }
    }

    let ps_output = match adb_shell(adb, serial, &["ps", "-A"]).await {
        Some(output) => Some(output),
        None => adb_shell(adb, serial, &["ps"]).await,
    };
    ps_output
        .map(|output| parse_ps_pids(&output, &packages))
        .unwrap_or_default()
}

fn parse_ps_pids(output: &str, packages: &[String]) -> Vec<u32> {
    let mut pids = Vec::new();
    for line in output.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let name = cols[cols.len() - 1];
        let matches_package = packages.iter().any(|pkg| {
            name == pkg.as_str()
                || name
                    .strip_prefix(pkg.as_str())
                    .is_some_and(|rest| rest.starts_with(':'))
        });
        if !matches_package {
            continue;
        }
        if let Some(pid) = cols.iter().skip(1).find_map(|col| col.parse::<u32>().ok()) {
            if !pids.contains(&pid) {
                pids.push(pid);
            }
        }
    }
    pids
}

impl Drop for LogcatStream {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logcat_line_parsing_threadtime() {
        let line = "10-23 15:30:12.345  1234  5678 D MyActivity: Application started";
        let entry = LogEntry::new_raw(line);

        assert_eq!(entry.level, LogLevel::Debug);
        assert_eq!(entry.tag, "MyActivity");
        assert_eq!(entry.message, "Application started");
        assert_eq!(entry.pid, Some(1234));
        assert_eq!(entry.tid, Some(5678));
        assert_eq!(entry.timestamp, Some("10-23 15:30:12.345".to_string()));
    }

    #[test]
    fn test_logcat_line_parsing_time_format() {
        let line = "10-23 15:30:12.345 D/MyTag( 1234): Mensaje de depuracion";
        let entry = LogEntry::parse_line(line).expect("Should parse -v time line");

        assert_eq!(entry.level, LogLevel::Debug);
        assert_eq!(entry.tag, "MyTag");
        assert_eq!(entry.message, "Mensaje de depuracion");
        assert_eq!(entry.pid, Some(1234));
        assert_eq!(entry.tid, None);
        assert_eq!(entry.timestamp, Some("10-23 15:30:12.345".to_string()));
    }

    #[test]
    fn test_logcat_line_parsing_time_format_no_pid() {
        let line = "10-23 15:30:12.345 I/SystemServer: System is ready";
        let entry = LogEntry::parse_line(line).expect("Should parse -v time no-pid line");

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.tag, "SystemServer");
        assert_eq!(entry.message, "System is ready");
        assert_eq!(entry.pid, None);
        assert_eq!(entry.timestamp, Some("10-23 15:30:12.345".to_string()));
    }

    #[test]
    fn test_logcat_raw_fallback() {
        let line = "--------- beginning of main";
        let entry = LogEntry::new_raw(line);

        assert_eq!(entry.level, LogLevel::Unknown);
        assert_eq!(entry.message, "--------- beginning of main");
        assert_eq!(entry.pid, None);
    }

    #[test]
    fn test_logcat_matches_filter() {
        let entry = LogEntry {
            raw: "10-23 15:30:12.345  1234  5678 W MyTag: Warning occurred in com.example.app"
                .to_string(),
            timestamp: Some("10-23 15:30:12.345".to_string()),
            pid: Some(1234),
            tid: Some(5678),
            level: LogLevel::Warn,
            tag: "MyTag".to_string(),
            message: "Warning occurred in com.example.app".to_string(),
        };

        // Query tests
        assert!(entry.matches_filter("warning", None, None, None));
        assert!(entry.matches_filter("mytag", None, None, None));
        assert!(!entry.matches_filter("fatal error", None, None, None));

        // Level tests
        assert!(entry.matches_filter("", Some(LogLevel::Info), None, None));
        assert!(entry.matches_filter("", Some(LogLevel::Warn), None, None));
        assert!(!entry.matches_filter("", Some(LogLevel::Error), None, None));

        // PID tests
        assert!(entry.matches_filter("", None, Some(1234), None));
        assert!(!entry.matches_filter("", None, Some(9999), None));

        // Package tests
        assert!(entry.matches_filter("", None, None, Some("com.example.app")));
        assert!(!entry.matches_filter("", None, None, Some("com.other.app")));
    }

    #[test]
    fn test_logcat_stream_lifecycle() {
        let mut stream = LogcatStream::new();
        assert!(!stream.is_running());
        assert!(!stream.is_filtering_by_app());

        stream.set_filter_by_app(true);
        assert!(stream.is_filtering_by_app());

        stream.stop();
        assert!(!stream.is_running());
    }

    #[test]
    fn test_logcat_threadtime_with_uid_and_microseconds() {
        let line = "11-04 10:43:12.345678 10157  4321  4321 D MyApp: onCreate";
        let entry = LogEntry::parse_line(line).expect("uid threadtime should parse");
        assert_eq!(entry.level, LogLevel::Debug);
        assert_eq!(entry.pid, Some(4321));
        assert_eq!(entry.tid, Some(4321));
        assert_eq!(entry.tag, "MyApp");
        assert_eq!(entry.message, "onCreate");
        assert_eq!(entry.timestamp.as_deref(), Some("11-04 10:43:12.345678"));
    }

    #[test]
    fn test_logcat_threadtime_with_year() {
        let line = "2026-11-04 10:43:12.345  1234  5678 I System.out: hello";
        let entry = LogEntry::parse_line(line).expect("year threadtime should parse");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.pid, Some(1234));
        assert_eq!(entry.tid, Some(5678));
        assert_eq!(entry.tag, "System.out");
        assert_eq!(entry.message, "hello");
    }

    #[test]
    fn test_app_log_is_not_hidden_without_package_substring() {
        let entry = LogEntry::parse_line("11-04 10:43:12.345  4321  4321 D MyApp: onCreate")
            .expect("classic threadtime");
        assert!(entry.matches_filter("", None, None, None));
        assert!(!entry.matches_filter("", None, None, Some("com.example.app")));
    }

    #[test]
    fn test_parse_package_uid_prefers_debug_variant() {
        let output = "\
package:com.example.app uid:10100
package:com.example.app.debug uid:10157
";
        assert_eq!(
            parse_package_uid(output, "com.example.app", true),
            Some(10157)
        );
        assert_eq!(
            parse_package_uid(output, "com.example.app", false),
            Some(10100)
        );
    }

    #[test]
    fn test_parse_ps_pids_includes_secondary_processes() {
        let output = "\
USER           PID  PPID        VSZ    RSS WCHAN            ADDR S NAME
u0_a157       4321   551  12345678 123456 0                   0 S com.example.app
u0_a157       4400   551  12345678  80000 0                   0 S com.example.app:push
u0_a12         100     1     1000   1000 0                   0 S system_server
";
        let pids = parse_ps_pids(output, &["com.example.app".to_string()]);
        assert_eq!(pids, vec![4321, 4400]);
    }
}
