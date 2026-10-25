# OverridePulse ⚡

[![Crates.io](https://img.shields.io/crates/v/overridepulse.svg)](https://crates.io/crates/overridepulse)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange.svg)](https://www.rust-lang.org/)
[![GitHub](https://img.shields.io/github/v/release/OverrideLLC/overridepulse?include_prereleases&label=release)](https://github.com/OverrideLLC/overridepulse/releases)

> **Ultralightweight TUI for Android & Gradle development (Lazygit for Android developers)**

OverridePulse is an ultra-fast, asynchronous terminal user interface (TUI) designed to streamline Android and Gradle development workflows. Stop wrestling with bloated IDE windows or typing repetitive Gradle/ADB commands—manage devices, build variants, logcat streams, ADB hardware toggles, and Gradle daemons with single-key shortcuts.

---

## 🌟 What's New in v0.2.1

- 🎯 **Strict Android Application Module Detection**: Complete multi-module and Kotlin Multiplatform (KMP / Clean Architecture) resolution. Accurately prioritizes application modules (`:composeApp`, `:androidApp`, `:app`) over library domain/data modules (`:auth:domain`, `:core:data`), completely preventing library namespace misclassification.
- 📦 **Infallible Direct APK Package Resolution**: Automatic detection of `output-metadata.json` directly adjacent to the compiled APK file.
- 📂 **Multi-Line Settings & KMP Manifest Support**: Robust multi-line `include(...)` parser for `settings.gradle[.kts]` and full support for Kotlin Multiplatform `src/androidMain/AndroidManifest.xml`.

---

## 🌟 What's New in v0.2.0

- 🎯 **Deep `applicationId` Detection**: Multi-pass resolution inspecting Kotlin DSL (`build.gradle.kts`), Groovy DSL (`build.gradle`), `namespace`, `gradle.properties`, and automatic fallback to `build/outputs/apk/**/output-metadata.json`.
- 🔍 **App-Only Exclusive Logcat (`--pid=<pid>`)**: Logcat streaming is 100% isolated to the target application's active process ID. No system spam, no unwanted third-party logs.
- 🚀 **Auto-Launch After Install**: Building and deploying APK (`[b]`) or AAB automatically detects the main launcher activity or launcher alias and launches the app immediately upon install.
- 📶 **Interactive ADB Wireless Pair & Connect Modal (`[w]`)**: Pair with 6-digit wireless debugging codes (Android 11+) and connect seamlessly without leaving the terminal.
- 🎛️ **Direct ADB Hardware & System Controls**: Dedicated upper panel for toggling Wi-Fi, Mobile Data, Stay Awake, Dark Theme, Touch feedback taps, taking screenshots, rebooting, and enabling `tcpip 5555`.
- 📊 **Configurable Multi-Pane Console**: Toggle Simplified event summary (`[1]`), Raw Build output (`[2]`), and Live Logcat (`[3]`), or cycle between view presets (`[m]`/`[l]`).
- 🧹 **Optimized Headerless UI**: Space-saving layout with project status and active variant consolidated cleanly into the bottom footer bar.

---

## ✨ Core Features

- **⚡ Blazing Fast & Memory Efficient**: Built natively in Rust (Ratatui + Tokio). Consumes negligible CPU and RAM compared to standard IDEs.
- **📱 Real-time ADB Device Monitor**: Automatic auto-discovery and state tracking for physical USB devices, emulators, and wireless ADB targets.
- **🚀 One-Key Quick Actions**:
  - `[b]` Assemble & install debug APK (auto-launches on completion)
  - `[r]` Relaunch main launcher activity (`am start -W`)
  - `[k]` Force-stop application process (`am force-stop`)
  - `[u]` Completely uninstall application (`adb uninstall`)
  - `[g]` **Stop all running Gradle daemons (`./gradlew --stop`) & terminate orphan processes**
  - `[w]` Open ADB Wireless Pair & Connect modal
  - `Enter / Space` on **App & Gradle Actions**: Clear app data (`pm clear`), Build AAB & deploy, Launch `scrcpy`.
  - `Enter / Space` on **Device Controls**: Wi-Fi, Mobile Data, Stay Awake, Dark Theme, Taps, Settings, Screenshot, Reboot.
- **📜 Live Non-blocking Logcat**: High-throughput log viewer with level colorization, real-time query filtering, and automatic per-app process PID isolation.
- **📂 Multi-Project Discovery**: Interactive project selector (`[p]`) with automatic detection of Gradle modules, application IDs, and build variants.
- **🛑 Zero UI Blocking**: All ADB and Gradle operations run in asynchronous Tokio background tasks with live progress indicators.

---

## 📦 Installation

### From Crates.io

```bash
cargo install overridepulse
```

### From GitHub

```bash
cargo install --git https://github.com/OverrideLLC/overridepulse
```

### From Source

```bash
git clone https://github.com/OverrideLLC/overridepulse.git
cd overridepulse
cargo build --release
sudo cp target/release/overridepulse /usr/local/bin/
```

### Prerequisites

- **Android SDK Platform-Tools** (`adb` in your `PATH` or standard SDK location).
- **Gradle** / Gradle Wrapper (`./gradlew` in project root).
- *(Optional)* **scrcpy** for screen mirroring.
- *(Optional)* **bundletool** for Android App Bundle deployments.

---

## 🚀 Usage

Launch OverridePulse inside any Android project directory:

```bash
overridepulse
```

Or specify options explicitly:

```bash
# Target a specific Android project directory
overridepulse --project /path/to/android/project

# Target a specific ADB device by serial
overridepulse --device emulator-5554

# Custom UI refresh rate (in ms)
overridepulse --tick-rate 100
```

---

## ⌨️ Keyboard Shortcuts

### Navigation & Panes

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle focus between panels (`Devices` → `Device Controls` → `Actions` → `Console`) |
| `↑` / `↓` or `k` / `j` | Navigate active list or scroll console logs |
| `1` | Toggle Simplified console pane |
| `2` | Toggle Raw Build Output pane |
| `3` | Toggle Live Logcat pane |
| `m` or `l` | Cycle console view presets (Simplified / Build / Logcat / Parity / Triple) |
| `w` | Open ADB Wireless Pair & Connect modal |
| `v` | Cycle active build variant (`debug`, `release`, etc.) |
| `p` | Open Android project selector modal |
| `?` | Toggle help overlay modal |
| `q` / `Ctrl+C` | Quit OverridePulse |
| `Esc` / `Ctrl+X` | Cancel running background task or dismiss modal |

### Actions & Device Controls

| Key | Action | Description |
|---|---|---|
| `Enter` / `Space` | **Execute Highlighted Action** | Executes selected Action or Device Control / Selects device |
| `[b]` | **Build & Install Debug APK** | Runs `./gradlew assembleDebug`, installs via ADB, and auto-launches |
| `[r]` | **Relaunch Activity** | Stops and relaunches main launcher activity (`am start -W`) |
| `[k]` | **Force Stop App** | Terminates app process immediately (`adb shell am force-stop`) |
| `[u]` | **Uninstall App** | Uninstalls package from device (`adb uninstall`) |
| `[g]` | **Stop All Gradle Daemons** | Stops daemons with `./gradlew --stop` and cleans orphaned processes |
| `[w]` | **Wireless ADB Connect** | Interactive modal for pairing & connecting over Wi-Fi |

*Note*: Actions like **Clear App Data**, **Build AAB & Deploy**, and **Launch Scrcpy** are safely selected from the **App & Gradle Actions** list via `Enter` or `Space` to avoid unintentional key collisions.

---

## 🏗️ Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                 OVERRIDEPULSE TUI ENGINE                    │
│   (TerminalGuard, PanicHook, Crossterm Raw Mode, Ratatui)   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                ┌──────────────┴──────────────┐
                ▼                             ▼
   ┌─────────────────────────┐   ┌─────────────────────────┐
   │    AppState & Events    │   │  Asynchronous Engines   │
   │ - Ring buffers (Deq)    │   │ - DeviceMonitor (ADB)   │
   │ - Multi-project state   │   │ - LogcatStream (PID-ex) │
   │ - Focus & Modal states  │   │ - ProcessRunner (Tokio) │
   │ - Multi-pane Parity     │   │ - DeviceAction Toggles  │
   └─────────────────────────┘   └─────────────────────────┘
```

- **Ring Buffers**: Output terminals maintain memory-bounded `VecDeque` buffers (2,000 lines for raw build, 100 entries for simplified view) preventing memory exhaustion.
- **Safety First**: Implements RAII `TerminalGuard` and custom panic hooks ensuring your terminal emulator is cleanly restored to normal mode under all circumstances.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2026 Override LLC & ChristopherTwo.
