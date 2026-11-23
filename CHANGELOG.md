# Changelog

All notable changes to **OverridePulse** are documented in this file.

> The [README](README.md) displays only the latest release. Older versions are tracked here.

---

## [0.3.1] — 2026-11-23

### Added
- **First-Run Onboarding Wizard**: Launches automatically on initial startup (or anytime via `Shift+O` / `--onboarding`) with a 2-step guide covering OverridePulse workflows and theme setup.
- **6 Live-Preview Color Themes**: `Override Cyan`, `Tokyo Night`, `Catppuccin Mocha`, `Dracula`, `Gruvbox Dark`, and `Emerald Matrix`, applied across all panels, grids, modals, and logs in real time (`Shift+T` cycles themes anytime).
- **Persistent User Configuration**: Saves `onboarding_completed` and active `theme` to `~/.config/overridepulse/config` (respecting `$XDG_CONFIG_HOME` / `%APPDATA%`).

---

## [0.3.0] — 2026-11-23

### Added
- **Kotlin Multiplatform (KMP) Target Support (`[t]`)**: Detects `Android`, `JVM / Compose Desktop`, `WasmJS`, and `JS` targets across Gradle modules and runs host builds, dev servers (`wasmJsBrowserDevelopmentRun`, `jsBrowserDevelopmentRun`, `run`), distribution bundles, and tests without requiring an attached ADB device.
- **Responsive Card-Grid Actions & Simplified Workspace**: Renders Actions and ADB Controls as rounded cards (`←/→/↑/↓`), while the default screen shows only the Actions grid and `[1] Status` pane.
- **Separate Devices & ADB View (`[d]`)**: Moves connected devices and ADB hardware controls into a dedicated view toggled with `[d]` (`d` or `Esc` to return).
- **Clean Dev-Server Cancellation**: Isolates subprocesses into Unix process groups so `Esc` / `Ctrl+X` terminates Gradle dev servers cleanly.

---

## [0.2.2] — 2026-11-04

### Added
- **Non-blocking project load**: Gradle and build-output scans run off the UI thread so opening large projects no longer freezes the TUI.
- **In-app project switching (`[p]`)**: Lists the current project, siblings, and one nested level without blocking the interface.
- **App logcat by UID**: Streams every process of the application (`--uid`), including secondary workers and `applicationId` suffixes (`[f]` toggles all-device logcat).
- **Copy consoles (`[y]` / `[Y]`)**: Copies live logcat or build output to the system clipboard (`wl-copy`, `xclip`, `xsel`) or terminal clipboard.

---

## [0.2.1] — 2026-10-25

### Fixed
- **Strict Android Application Module Detection**: Prioritizes application modules (`:composeApp`, `:androidApp`, `:app`) over library domain/data modules (`:auth:domain`, `:core:data`).
- **Direct APK Package Resolution**: Reads `output-metadata.json` adjacent to compiled APK outputs.
- **Multi-Line Settings & KMP Manifest Support**: Supports multi-line `include(...)` blocks and KMP `src/androidMain/AndroidManifest.xml`.

---

## [0.2.0] — 2026-10-25

### Added
- **Deep `applicationId` Detection**: Inspects Kotlin DSL, Groovy DSL, `namespace`, `gradle.properties`, and `output-metadata.json`.
- **App-Only Exclusive Logcat**: Isolates logcat to the target app process.
- **Auto-Launch After Install**: Detects the main launcher activity or alias and launches immediately after APK/AAB install.
- **Interactive ADB Wireless Pair & Connect Modal (`[w]`)**: Supports Android 11+ 6-digit wireless pairing codes.
- **Direct ADB Hardware & System Controls**: Toggles Wi-Fi, Mobile Data, Stay Awake, Dark Theme, Show Taps, Screenshots, Reboot, and `tcpip 5555`.
- **Configurable Multi-Pane Console**: Toggles Simplified (`[1]`), Raw Build (`[2]`), and Live Logcat (`[3]`) panes (`[m]`/`[l]` presets).

---

## [0.1.0] — Initial Release

### Added
- Initial asynchronous Rust TUI for Android & Gradle workflows using `ratatui`, `crossterm`, and `tokio`.
