use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use regex::Regex;
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

static THREADTIME_RE: OnceLock<Regex> = OnceLock::new();
static TIME_PID_RE: OnceLock<Regex> = OnceLock::new();
static TIME_NOPID_RE: OnceLock<Regex> = OnceLock::new();

fn threadtime_regex() -> &'static Regex {
    THREADTIME_RE.get_or_init(|| {
        Regex::new(
            r"^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+(\d+)\s+(\d+)\s+([VDIWEFAvdiwefa])\s+(.*?):\s?(.*)$",
        )
        .expect("Invalid threadtime regex")
    })
}

fn time_pid_regex() -> &'static Regex {
    TIME_PID_RE.get_or_init(|| {
        Regex::new(
            r"^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+([VDIWEFAvdiwefa])/(.*?)\(\s*(\d+)\s*\):\s?(.*)$",
        )
        .expect("Invalid time_pid regex")
    })
}

fn time_nopid_regex() -> &'static Regex {
    TIME_NOPID_RE.get_or_init(|| {
        Regex::new(
            r"^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+([VDIWEFAvdiwefa])/(.*?):\s?(.*)$",
        )
        .expect("Invalid time_nopid regex")
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

        // Try `-v threadtime` format: "10-23 15:30:12.345  1234  5678 D MyActivity: Msg"
        if let Some(caps) = threadtime_regex().captures(trimmed) {
            let timestamp = caps.get(1).map(|m| m.as_str().to_string());
            let pid = caps.get(2).and_then(|m| m.as_str().parse::<u32>().ok());
            let tid = caps.get(3).and_then(|m| m.as_str().parse::<u32>().ok());
            let level_char = caps
                .get(4)
                .and_then(|m| m.as_str().chars().next())
                .unwrap_or('?');
            let level = LogLevel::from_char(level_char);
            let tag = caps
                .get(5)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let message = caps
                .get(6)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            return Some(Self {
                raw: trimmed.to_string(),
                timestamp,
                pid,
                tid,
                level,
                tag,
                message,
            });
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

    /// Resolves the PID of an application running on the specified device.
    pub async fn resolve_pid(serial: &str, package_name: &str) -> Option<u32> {
        if package_name.trim().is_empty() {
            return None;
        }

        // Try `pidof -s <package>`
        if let Ok(output) = Command::new("adb")
            .args(["-s", serial, "shell", "pidof", "-s", package_name])
            .output()
            .await
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                if let Some(pid) = out_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    return Some(pid);
                }
            }
        }

        // Fallback to `pidof <package>`
        if let Ok(output) = Command::new("adb")
            .args(["-s", serial, "shell", "pidof", package_name])
            .output()
            .await
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                if let Some(pid) = out_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    return Some(pid);
                }
            }
        }

        None
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

    /// Starts streaming logcat strictly filtered for the project's application package.
    pub fn start(
        &mut self,
        serial: impl Into<String>,
        package_name: Option<String>,
        filter_by_app: bool,
        sender: UnboundedSender<AppEvent>,
    ) {
        self.stop();

        let serial = serial.into();
        self.current_serial = Some(serial.clone());
        self.package_name = package_name.clone();
        self.filter_by_app.store(filter_by_app, Ordering::SeqCst);

        let task_serial = serial.clone();
        let task_pkg = package_name.clone();

        let join_handle = tokio::spawn(async move {
            let pkg = match task_pkg {
                Some(p) if !p.trim().is_empty() => p.trim().to_string(),
                _ => {
                    let entry = LogEntry {
                        raw: "No applicationId detected yet. Build the app first to stream its Logcat.".to_string(),
                        timestamp: None,
                        pid: None,
                        tid: None,
                        level: LogLevel::Info,
                        tag: "OverridePulse".to_string(),
                        message: "No applicationId detected yet. Build the app first to stream its Logcat.".to_string(),
                    };
                    let _ = sender.send(AppEvent::LogcatLine(entry));
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            };

            let mut waiting_emitted = false;
            loop {
                // 1. Wait until resolving PID
                let mut pid = Self::resolve_pid(&task_serial, &pkg).await;
                while pid.is_none() {
                    if !waiting_emitted {
                        let info = LogEntry {
                            raw: format!("Waiting for process '{pkg}' to start on device..."),
                            timestamp: None,
                            pid: None,
                            tid: None,
                            level: LogLevel::Info,
                            tag: "OverridePulse".to_string(),
                            message: format!("Waiting for process '{pkg}' to start on device..."),
                        };
                        if sender.send(AppEvent::LogcatLine(info)).is_err() {
                            return;
                        }
                        waiting_emitted = true;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    pid = Self::resolve_pid(&task_serial, &pkg).await;
                }

                let current_pid = match pid {
                    Some(p) => p,
                    None => continue,
                };
                waiting_emitted = false;

                // 2. Stream logcat exclusively for this PID
                let mut cmd = Command::new("adb");
                cmd.args([
                    "-s",
                    &task_serial,
                    "logcat",
                    "-v",
                    "threadtime",
                    "-T",
                    "100",
                    &format!("--pid={current_pid}"),
                ]);
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::null());
                cmd.kill_on_drop(true);

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = sender.send(AppEvent::ProcessOutput(format!(
                            "Failed to spawn adb logcat: {e}"
                        )));
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        continue;
                    }
                };

                if let Some(stdout) = child.stdout.take() {
                    let mut reader = BufReader::new(stdout).lines();
                    let mut pid_check_interval =
                        tokio::time::interval(tokio::time::Duration::from_millis(1500));
                    pid_check_interval.tick().await;

                    loop {
                        tokio::select! {
                            line_res = reader.next_line() => {
                                match line_res {
                                    Ok(Some(line)) => {
                                        if let Some(entry) = LogEntry::parse_line(&line) {
                                            if sender.send(AppEvent::LogcatLine(entry)).is_err() {
                                                return;
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        break;
                                    }
                                    Err(_) => {
                                        break;
                                    }
                                }
                            }
                            _ = pid_check_interval.tick() => {
                                let active_pid = Self::resolve_pid(&task_serial, &pkg).await;
                                if active_pid != Some(current_pid) {
                                    let _ = child.kill().await;
                                    break;
                                }
                            }
                        }
                    }
                }

                let _ = child.kill().await;
                let _ = child.wait().await;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        self.abort_handle = Some(join_handle.abort_handle());
    }
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
}
