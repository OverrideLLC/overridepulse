# OverridePulse ⚡

[![Crates.io](https://img.shields.io/crates/v/overridepulse.svg)](https://crates.io/crates/overridepulse)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange.svg)](https://www.rust-lang.org/)
[![GitHub](https://img.shields.io/github/v/release/OverrideLLC/overridepulse?include_prereleases&label=release)](https://github.com/OverrideLLC/overridepulse/releases)

> **Ultralightweight TUI for Android & Gradle development (Lazygit for Android developers)**

OverridePulse is an ultra-fast, asynchronous terminal user interface (TUI) designed to streamline Android and Gradle development workflows. Stop wrestling with bloated IDE windows or typing repetitive Gradle/ADB commands—manage devices, build variants, logcat streams, and Gradle daemons with single-key shortcuts.

---

## ✨ Features

- **⚡ Blazing Fast & Memory Efficient**: Built natively in Rust (Ratatui + Tokio). Consumes negligible CPU and RAM compared to standard IDEs.
- **📱 Real-time ADB Device Monitor**: Automatic auto-discovery and state tracking for physical USB devices, emulators, and wireless ADB targets.
- **🚀 One-Key Quick Actions**:
  - `[b]` Assemble & install debug APK
  - `[a]` Assemble AAB & deploy via Bundletool
  - `[g]` **Stop all running Gradle daemons (`./gradlew --stop`) & terminate orphan processes**
  - `[k]` Force-stop app process
  - `[r]` Relaunch launcher activity
  - `[c]` Clear app data & cache (`pm clear`)
  - `[u]` Completely uninstall application
  - `[s]` Launch screen mirroring via `scrcpy`
- **📜 Live Non-blocking Logcat**: High-throughput log viewer with level colorization (Error, Warning, Info, Debug, Verbose), real-time query filtering, and per-app package isolation (`[f]`).
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
- *(Optional)* **scrcpy** for screen mirroring (`[s]`).
- *(Optional)* **bundletool** for Android App Bundle deployments (`[a]`).

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

### Global & Navigation

| Key | Action |
|---|---|
| `Tab` / `BackTab` | Cycle focus between panels (Devices, Actions, Output / Logcat) |
| `↑` / `↓` or `k` / `j` | Navigate items in list / scroll output buffers |
| `PageUp` / `PageDown` | Fast scroll (10 lines) |
| `v` | Cycle active build variant (`debug`, `release`, etc.) |
| `p` | Open Android project selector modal |
| `?` | Toggle help modal |
| `q` / `Ctrl+C` | Quit OverridePulse |
| `Esc` / `Ctrl+X` | Cancel running background task or dismiss modal |

### Quick Actions

| Key | Action | Description |
|---|---|---|
| `[b]` | **Build & Install Debug APK** | Runs `./gradlew assembleDebug` and installs via `adb install -r` |
| `[a]` | **Build AAB & Deploy** | Generates App Bundle, extracts device APKS via `bundletool`, and deploys |
| `[g]` | **Stop All Gradle Daemons** | Stops daemons with `./gradlew --stop` and terminates orphaned processes |
| `[k]` | **Force Stop App** | Terminates app immediately (`adb shell am force-stop`) |
| `[r]` | **Relaunch Activity** | Stops and relaunches main launcher activity (`am start -W`) |
| `[c]` | **Clear App Data** | Wipes app storage and runtime cache (`adb shell pm clear`) |
| `[u]` | **Uninstall App** | Uninstalls package from device (`adb uninstall`) |
| `[s]` | **Launch Scrcpy** | Opens mirrored display session in background |

### Logcat & Filtering

| Key | Action | Description |
|---|---|---|
| `[l]` | **Toggle Logcat View** | Switch between Build Output terminal and Live Logcat stream |
| `[f]` | **Toggle App Filter** | Restrict Logcat output strictly to the active project's package |
| `[/]` | **Search / Filter** | Filter log lines in real-time by search query |

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
   │ - Multi-project state   │   │ - LogcatStream (Async)  │
   │ - Focus & Modal states  │   │ - ProcessRunner (Tokio) │
   └─────────────────────────┘   └─────────────────────────┘
```

- **Ring Buffers**: Output terminals maintain a memory-bounded `VecDeque` buffer (default: 2,000 lines) preventing memory exhaustion during heavy compilation or verbose logcat output.
- **Safety First**: Implements RAII `TerminalGuard` and a custom panic hook ensuring your terminal emulator is cleanly restored to normal mode under all circumstances.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2026 Override LLC & ChristopherTwo.
