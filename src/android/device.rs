use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Device,
    Offline,
    Unauthorized,
    Bootloader,
    Recovery,
    Sideload,
    Unknown(String),
}

impl DeviceState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Device => "device",
            Self::Offline => "offline",
            Self::Unauthorized => "unauthorized",
            Self::Bootloader => "bootloader",
            Self::Recovery => "recovery",
            Self::Sideload => "sideload",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for DeviceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    Usb,
    Wifi,
    Emulator,
    Unknown,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usb => write!(f, "USB"),
            Self::Wifi => write!(f, "Wi-Fi"),
            Self::Emulator => write!(f, "Emulator"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub model: String,
    pub product: String,
    pub state: DeviceState,
    pub connection_type: ConnectionType,
    pub battery_level: Option<u8>,
    pub android_version: Option<String>,
    pub api_level: Option<u32>,
}

impl Device {
    pub fn new(serial: impl Into<String>, state: DeviceState) -> Self {
        let serial_str = serial.into();
        let conn_type = if serial_str.starts_with("emulator-") {
            ConnectionType::Emulator
        } else if serial_str.contains(':') {
            ConnectionType::Wifi
        } else {
            ConnectionType::Usb
        };

        Self {
            serial: serial_str,
            model: "Unknown".to_string(),
            product: String::new(),
            state,
            connection_type: conn_type,
            battery_level: None,
            android_version: None,
            api_level: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, DeviceState::Device)
    }

    pub fn display_name(&self) -> String {
        if self.model != "Unknown" && !self.model.is_empty() {
            format!("{} ({})", self.model, self.serial)
        } else {
            self.serial.clone()
        }
    }

    /// Parses the stdout of `adb devices -l` into a list of `Device` instances.
    pub fn parse_adb_devices_output(output: &str) -> Vec<Device> {
        let mut devices = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with("List of devices attached")
                || line.starts_with('*')
            {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let serial = parts[0];
            let state_str = parts[1];

            let state = match state_str.to_lowercase().as_str() {
                "device" => DeviceState::Device,
                "offline" => DeviceState::Offline,
                "unauthorized" => DeviceState::Unauthorized,
                "bootloader" => DeviceState::Bootloader,
                "recovery" => DeviceState::Recovery,
                "sideload" => DeviceState::Sideload,
                other => DeviceState::Unknown(other.to_string()),
            };

            let mut device = Device::new(serial, state);

            for part in &parts[2..] {
                if let Some(val) = part.strip_prefix("model:") {
                    device.model = val.to_string();
                } else if let Some(val) = part.strip_prefix("product:") {
                    device.product = val.to_string();
                } else if part.starts_with("usb:")
                    && device.connection_type == ConnectionType::Unknown
                {
                    device.connection_type = ConnectionType::Usb;
                }
            }

            devices.push(device);
        }

        devices
    }
}

/// Actions for interacting directly with and configuring the connected Android device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceAction {
    WirelessPairAndConnect,
    EnableTcpip5555,
    ToggleWifi,
    ToggleMobileData,
    ToggleStayAwake,
    ToggleDarkMode,
    ToggleShowTaps,
    OpenWirelessDebuggingSettings,
    TakeScreenshot,
    RebootDevice,
}

impl DeviceAction {
    /// Returns all supported device control actions.
    pub fn all() -> &'static [DeviceAction] {
        &[
            DeviceAction::WirelessPairAndConnect,
            DeviceAction::EnableTcpip5555,
            DeviceAction::ToggleWifi,
            DeviceAction::ToggleMobileData,
            DeviceAction::ToggleStayAwake,
            DeviceAction::ToggleDarkMode,
            DeviceAction::ToggleShowTaps,
            DeviceAction::OpenWirelessDebuggingSettings,
            DeviceAction::TakeScreenshot,
            DeviceAction::RebootDevice,
        ]
    }

    /// User-facing action title for menus and lists.
    pub fn title(&self) -> &'static str {
        match self {
            Self::WirelessPairAndConnect => "Pair & Connect Wireless ADB",
            Self::EnableTcpip5555 => "Enable ADB over Wi-Fi (tcpip 5555)",
            Self::ToggleWifi => "Toggle Device Wi-Fi (ON/OFF)",
            Self::ToggleMobileData => "Toggle Mobile Data (ON/OFF)",
            Self::ToggleStayAwake => "Toggle Stay Awake While Plugged",
            Self::ToggleDarkMode => "Toggle Dark / Light Theme",
            Self::ToggleShowTaps => "Toggle Show Taps on Screen",
            Self::OpenWirelessDebuggingSettings => "Open Developer / Wireless Debug Settings",
            Self::TakeScreenshot => "Take Screenshot to Project Folder",
            Self::RebootDevice => "Reboot Device",
        }
    }

    /// Action description explaining effect on target device.
    pub fn description(&self) -> &'static str {
        match self {
            Self::WirelessPairAndConnect => {
                "Pair with 6-digit code and connect to wireless ADB port"
            }
            Self::EnableTcpip5555 => {
                "Restart ADB daemon listening on TCP port 5555 for Wi-Fi debugging"
            }
            Self::ToggleWifi => "Toggle device Wi-Fi interface enabled or disabled via svc/cmd",
            Self::ToggleMobileData => "Toggle device cellular data connection via svc data",
            Self::ToggleStayAwake => "Toggle keep screen awake while USB/AC charging is active",
            Self::ToggleDarkMode => "Switch device system night mode / dark theme on or off",
            Self::ToggleShowTaps => "Toggle visual touch feedback indicators on the screen",
            Self::OpenWirelessDebuggingSettings => {
                "Open Developer options and Wireless debugging settings on device screen"
            }
            Self::TakeScreenshot => "Capture device display to a PNG file in project directory",
            Self::RebootDevice => "Reboot the connected Android device via ADB",
        }
    }
}

/// Parses the battery level percentage from `dumpsys battery` output.
pub fn parse_battery_level(output: &str) -> Option<u8> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("level:") {
            if let Ok(lvl) = rest.trim().parse::<u8>() {
                return Some(lvl);
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
struct CachedDeviceInfo {
    android_version: Option<String>,
    api_level: Option<u32>,
    battery_level: Option<u8>,
}

/// Parses batched ADB shell output containing `__VER__=...`, `__SDK__=...`, and `dumpsys battery`.
fn parse_combined_device_details(output: &str) -> (Option<u8>, Option<String>, Option<u32>) {
    let mut android_version = None;
    let mut api_level = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(ver) = trimmed.strip_prefix("__VER__=") {
            let v = ver.trim();
            if !v.is_empty() {
                android_version = Some(v.to_string());
            }
        } else if let Some(sdk) = trimmed.strip_prefix("__SDK__=") {
            if let Ok(parsed) = sdk.trim().parse::<u32>() {
                api_level = Some(parsed);
            }
        }
    }

    let battery_level = parse_battery_level(output);
    (battery_level, android_version, api_level)
}

/// Asynchronously fetches device details (battery level, Android OS version, and API level)
/// for a connected device using minimal `adb shell` invocations.
async fn populate_device_details(
    adb_path: &Path,
    device: &mut Device,
    fetch_static: bool,
    fetch_battery: bool,
) {
    let serial = &device.serial;
    let timeout_dur = Duration::from_millis(1500);

    if fetch_static {
        let script = "echo __VER__=$(getprop ro.build.version.release); echo __SDK__=$(getprop ro.build.version.sdk); dumpsys battery";
        let cmd = tokio::process::Command::new(adb_path)
            .args(["-s", serial, "shell", script])
            .output();
        if let Ok(Ok(out)) = tokio::time::timeout(timeout_dur, cmd).await {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let (battery, ver, sdk) = parse_combined_device_details(&text);
                if battery.is_some() {
                    device.battery_level = battery;
                }
                if ver.is_some() {
                    device.android_version = ver;
                }
                if sdk.is_some() {
                    device.api_level = sdk;
                }
            }
        }
    } else if fetch_battery {
        let battery_cmd = tokio::process::Command::new(adb_path)
            .args(["-s", serial, "shell", "dumpsys", "battery"])
            .output();
        if let Ok(Ok(out)) = tokio::time::timeout(timeout_dur, battery_cmd).await {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                if let Some(lvl) = parse_battery_level(&text) {
                    device.battery_level = Some(lvl);
                }
            }
        }
    }
}

/// Asynchronously queries `adb devices -l` and enriches online devices with battery and OS metrics.
pub async fn fetch_devices(adb_path: &Path) -> anyhow::Result<Vec<Device>> {
    let mut cache = HashMap::new();
    fetch_devices_with_cache(adb_path, &mut cache, true).await
}

async fn fetch_devices_with_cache(
    adb_path: &Path,
    cache: &mut HashMap<String, CachedDeviceInfo>,
    refresh_battery: bool,
) -> anyhow::Result<Vec<Device>> {
    let output = tokio::process::Command::new(adb_path)
        .args(["devices", "-l"])
        .output()
        .await?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("adb devices -l failed: {err}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices = Device::parse_adb_devices_output(&stdout);

    let online_serials: HashSet<String> = devices
        .iter()
        .filter(|d| d.state == DeviceState::Device)
        .map(|d| d.serial.clone())
        .collect();
    cache.retain(|serial, _| online_serials.contains(serial));

    // Concurrently fetch missing/refreshed metrics for online devices
    let mut tasks = Vec::with_capacity(devices.len());
    for mut dev in devices {
        if dev.state == DeviceState::Device {
            let cached = cache.get(&dev.serial).cloned();
            let fetch_static = cached.is_none();
            if let Some(ref info) = cached {
                dev.android_version = info.android_version.clone();
                dev.api_level = info.api_level;
                dev.battery_level = info.battery_level;
            }

            if fetch_static || refresh_battery {
                let adb = adb_path.to_path_buf();
                tasks.push(tokio::spawn(async move {
                    populate_device_details(&adb, &mut dev, fetch_static, refresh_battery).await;
                    dev
                }));
            } else {
                tasks.push(tokio::spawn(async move { dev }));
            }
        } else {
            tasks.push(tokio::spawn(async move { dev }));
        }
    }

    let mut populated_devices = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(dev) = task.await {
            if dev.state == DeviceState::Device {
                cache.insert(
                    dev.serial.clone(),
                    CachedDeviceInfo {
                        android_version: dev.android_version.clone(),
                        api_level: dev.api_level,
                        battery_level: dev.battery_level,
                    },
                );
            }
            populated_devices.push(dev);
        }
    }

    Ok(populated_devices)
}

/// Spawns a background task that periodically monitors ADB devices and emits
/// `AppEvent::DevicesUpdated` when changes are detected.
pub fn spawn_device_monitor(
    adb_path: PathBuf,
    sender: tokio::sync::mpsc::UnboundedSender<crate::event::AppEvent>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        let mut last_devices: Option<Vec<Device>> = None;
        let mut cache: HashMap<String, CachedDeviceInfo> = HashMap::new();
        let mut tick_count: u64 = 0;

        loop {
            ticker.tick().await;
            let refresh_battery = tick_count.is_multiple_of(5);
            tick_count = tick_count.wrapping_add(1);

            match fetch_devices_with_cache(&adb_path, &mut cache, refresh_battery).await {
                Ok(devices) => {
                    let changed = match &last_devices {
                        Some(prev) => prev != &devices,
                        None => true,
                    };

                    if changed {
                        if sender
                            .send(crate::event::AppEvent::DevicesUpdated(devices.clone()))
                            .is_err()
                        {
                            break; // Channel disconnected, stop monitoring
                        }
                        last_devices = Some(devices);
                    }
                }
                Err(err) => {
                    // Back off polling if adb executable itself is missing to avoid idle CPU churn
                    if err
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
                    {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        ticker.reset();
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_connection_types() {
        let usb = Device::new("12345678", DeviceState::Device);
        assert_eq!(usb.connection_type, ConnectionType::Usb);
        assert!(usb.is_ready());

        let emu = Device::new("emulator-5554", DeviceState::Device);
        assert_eq!(emu.connection_type, ConnectionType::Emulator);

        let wifi = Device::new("192.168.1.50:5555", DeviceState::Device);
        assert_eq!(wifi.connection_type, ConnectionType::Wifi);
    }

    #[test]
    fn test_parse_adb_devices_output_comprehensive() {
        let sample_output = r#"
            * daemon not running; starting now at tcp:5037
            * daemon started successfully
            List of devices attached
            emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emulator64_x86_64 transport_id:1
            192.168.1.50:5555      device product:husky model:Pixel_8_Pro device:husky transport_id:2
            0123456789ABCDEF       unauthorized usb:1-1 transport_id:3
            emulator-5556          offline transport_id:4
            FASTBOOT123            bootloader
            RECDEV999              recovery product:rec_prod model:Recovery_Phone
            SIDELOAD1              sideload
            UNKNOWN_DEV            some_strange_state
        "#;

        let devices = Device::parse_adb_devices_output(sample_output);
        assert_eq!(devices.len(), 8);

        // 1. Emulator
        let emu = &devices[0];
        assert_eq!(emu.serial, "emulator-5554");
        assert_eq!(emu.state, DeviceState::Device);
        assert_eq!(emu.connection_type, ConnectionType::Emulator);
        assert_eq!(emu.model, "sdk_gphone64_x86_64");
        assert_eq!(emu.product, "sdk_gphone64_x86_64");
        assert!(emu.is_ready());

        // 2. Wi-Fi device
        let wifi = &devices[1];
        assert_eq!(wifi.serial, "192.168.1.50:5555");
        assert_eq!(wifi.state, DeviceState::Device);
        assert_eq!(wifi.connection_type, ConnectionType::Wifi);
        assert_eq!(wifi.model, "Pixel_8_Pro");
        assert_eq!(wifi.product, "husky");
        assert_eq!(wifi.display_name(), "Pixel_8_Pro (192.168.1.50:5555)");

        // 3. Unauthorized USB device
        let unauth = &devices[2];
        assert_eq!(unauth.serial, "0123456789ABCDEF");
        assert_eq!(unauth.state, DeviceState::Unauthorized);
        assert_eq!(unauth.connection_type, ConnectionType::Usb);
        assert!(!unauth.is_ready());

        // 4. Offline emulator
        let offline = &devices[3];
        assert_eq!(offline.serial, "emulator-5556");
        assert_eq!(offline.state, DeviceState::Offline);
        assert_eq!(offline.connection_type, ConnectionType::Emulator);

        // 5. Bootloader
        let bootloader = &devices[4];
        assert_eq!(bootloader.serial, "FASTBOOT123");
        assert_eq!(bootloader.state, DeviceState::Bootloader);

        // 6. Recovery
        let recovery = &devices[5];
        assert_eq!(recovery.serial, "RECDEV999");
        assert_eq!(recovery.state, DeviceState::Recovery);
        assert_eq!(recovery.model, "Recovery_Phone");

        // 7. Sideload
        let sideload = &devices[6];
        assert_eq!(sideload.serial, "SIDELOAD1");
        assert_eq!(sideload.state, DeviceState::Sideload);

        // 8. Unknown
        let unknown = &devices[7];
        assert_eq!(unknown.serial, "UNKNOWN_DEV");
        assert_eq!(
            unknown.state,
            DeviceState::Unknown("some_strange_state".to_string())
        );
    }

    #[test]
    fn test_parse_battery_level() {
        let dumpsys_battery = r#"
            Current Battery Service state:
              AC powered: false
              USB powered: true
              Wireless powered: false
              Max charging current: 500000
              Max charging voltage: 5000000
              Charge counter: 3824000
              status: 2
              health: 2
              present: true
              level: 87
              scale: 100
              voltage: 4120
              temperature: 295
              technology: Li-ion
        "#;

        assert_eq!(parse_battery_level(dumpsys_battery), Some(87));
        assert_eq!(parse_battery_level("invalid output"), None);
    }

    #[test]
    fn test_device_action_metadata() {
        let actions = DeviceAction::all();
        assert_eq!(actions.len(), 10);

        for action in actions {
            assert!(!action.title().is_empty());
            assert!(!action.description().is_empty());
        }

        assert_eq!(
            DeviceAction::WirelessPairAndConnect.title(),
            "Pair & Connect Wireless ADB"
        );
        assert_eq!(
            DeviceAction::EnableTcpip5555.title(),
            "Enable ADB over Wi-Fi (tcpip 5555)"
        );
        assert_eq!(
            DeviceAction::ToggleWifi.title(),
            "Toggle Device Wi-Fi (ON/OFF)"
        );
        assert_eq!(
            DeviceAction::TakeScreenshot.title(),
            "Take Screenshot to Project Folder"
        );
        assert_eq!(DeviceAction::RebootDevice.title(), "Reboot Device");
    }

    #[test]
    fn test_parse_batched_device_details() {
        let output = "\
__VER__=15
__SDK__=35
Current Battery Service state:
  AC powered: false
  USB powered: true
  level: 92
  scale: 100
";
        let (battery, ver, sdk) = parse_combined_device_details(output);
        assert_eq!(battery, Some(92));
        assert_eq!(ver.as_deref(), Some("15"));
        assert_eq!(sdk, Some(35));

        let empty_output = "__VER__=\n__SDK__=not_a_num\n";
        let (battery2, ver2, sdk2) = parse_combined_device_details(empty_output);
        assert_eq!(battery2, None);
        assert_eq!(ver2, None);
        assert_eq!(sdk2, None);
    }
}
