<div align="center">

# OverridePulse ⚡

**Ultralightweight terminal workspace for Android, Kotlin Multiplatform (KMP) & Gradle**

*Compile Android, JVM/Desktop, WasmJS, and JS targets, stream app logs, and manage ADB devices — at keyboard speed.*

[![Crates.io](https://img.shields.io/crates/v/overridepulse.svg?style=flat-square&color=4FC3F7)](https://crates.io/crates/overridepulse)
[![GitHub Release](https://img.shields.io/github/v/release/OverrideLLC/overridepulse?style=flat-square&color=66BB6A)](https://github.com/OverrideLLC/overridepulse/releases)
[![Rust](https://img.shields.io/badge/rust-1.74%2B-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)

[✨ Latest Release](#-latest-release--v031) • [🧩 KMP Targets](#-supported-targets) • [🎨 Themes](#-themes--onboarding) • [📦 Install](#-installation) • [⌨️ Shortcuts](#️-keyboard-shortcuts) • [📜 Changelog](CHANGELOG.md)

</div>

---

```text
╭─ Actions · 🕸 WasmJS (:composeApp) ───────────────────────────────────────────╮
│ ╭──────────────────────╮ ╭──────────────────────╮ ╭──────────────────────╮   │
│ │ ▶ [b] Compile WasmJS │ │   [r] Run WasmJS Dev │ │   Bundle WasmJS Dist │   │
│ ╰──────────────────────╯ ╰──────────────────────╯ ╰──────────────────────╯   │
│ ╭──────────────────────╮ ╭──────────────────────╮ ╭──────────────────────╮   │
│ │   Run WasmJS Tests   │ │   Assemble All       │ │   [g] Stop Gradle    │   │
│ ╰──────────────────────╯ ╰──────────────────────╯ ╰──────────────────────╯   │
╰──────────────────────────────────────────────────────────────────────────────╯
╭─ [1] Status │ [2] Build  [3] Logcat ─────────────────────────────────────────╮
│ ● Target: WasmJS (:composeApp)                                               │
│   ✅ :composeApp:wasmJsBrowserDistribution completed (3.4s)                  │
╰──────────────────────────────────────────────────────────────────────────────╯
╭──────────────────────────────────────────────────────────────────────────────╮
│ [t] Target  [p] Project  [d] Devices  [1/2/3] Views  [?] Help │ 📦 PulseApp  │
╰──────────────────────────────────────────────────────────────────────────────╯
```

---

## ✨ Latest Release — v0.3.1

- 👋 **First-Run Onboarding Wizard**: Greets new users with a concise quick-start guide and interactive setup (`Shift+O` or `--onboarding` to reopen anytime).
- 🎨 **6 Live-Preview Color Themes**: Switch between `Override Cyan`, `Tokyo Night`, `Catppuccin Mocha`, `Dracula`, `Gruvbox Dark`, and `Emerald Matrix` with real-time preview (`Shift+T` cycles themes anytime).
- 💾 **Persistent User Preferences**: Automatically saves your theme and onboarding state to `~/.config/overridepulse/config`.

> Looking for earlier releases (`v0.3.0`, `v0.2.2`, `v0.2.1`, `v0.2.0`, `v0.1.0`)? See the full history in **[CHANGELOG.md](CHANGELOG.md)**.

---

## 🧩 Supported Targets

OverridePulse scans `settings.gradle[.kts]` and module `build.gradle[.kts]` files asynchronously off the UI thread and lets you switch targets with **`[t]`**:

| Target | Compile / Build (`[b]`) | Run / Dev Server (`[r]`) | Distribution & Tests | ADB Required? |
|---|---|---|---|---|
| 🤖 **Android** | `assembleDebug` + `adb install` + auto-launch | Relaunch main launcher activity | `bundletool` AAB deploy, clear data, force-stop, uninstall, `scrcpy` | Yes (`[d]` view) |
| ☕ **JVM / Desktop** | `<target>Jar` / `jar` | `run` (Compose Desktop / Application) or `<target>Run` | `packageDistributionForCurrentOS` / `installDist` / `test` | No |
| 🕸 **WasmJS** | `compileKotlinWasmJs` | `wasmJsBrowserDevelopmentRun` / Node dev run | `wasmJsBrowserDistribution` / `wasmJsTest` | No |
| 🟨 **JS** | `compileKotlinJs` | `jsBrowserDevelopmentRun` / Node dev run | `jsBrowserDistribution` / `jsTest` | No |

---

## ⚡ Why OverridePulse?

- **Zero IDE Bloat**: Built in Rust (`ratatui` + `crossterm` + `tokio`) with minimal CPU and memory overhead.
- **Calm, Focused Workspace**: Opens straight to your **Actions Card Grid** and **`[1] Status`** pane—raw Gradle logs (`[2]`), live UID-scoped Logcat (`[3]`), and **Devices & ADB Controls** (`[d]`) stay one keypress away.
- **Clean Dev-Server Lifecycle**: Cancelling with `Esc` or `Ctrl+X` terminates the Unix process group cleanly so background Webpack or Desktop processes don't linger.
- **Multi-Project Ready**: Press **`[p]`** to switch between sibling or nested Gradle/KMP projects without restarting.

---

## 🎨 Themes & Onboarding

On first launch, OverridePulse walks you through a 2-step setup:
1. Quick overview of KMP targets, card-grid actions, and views.
2. Live theme selector with color swatches:
   - `Override Cyan` · `Tokyo Night` · `Catppuccin Mocha` · `Dracula` · `Gruvbox Dark` · `Emerald Matrix`

You can cycle themes anytime with **`Shift+T`** or reopen the wizard with **`Shift+O`** (`--onboarding`).

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
cargo install --path .
```

### Prerequisites
- **Gradle** / Gradle Wrapper (`./gradlew` in your project root).
- *(For Android targets)* **Android SDK Platform-Tools** (`adb` in `PATH` or standard SDK location).
- *(Optional)* **scrcpy** for screen mirroring and **bundletool** for `.aab` deployments.

---

## 🚀 Usage

```bash
# Open in the current Android / KMP project
overridepulse

# Target a specific project directory
overridepulse --project /path/to/kmp-or-android-project

# Target a specific ADB device serial
overridepulse --device emulator-5554

# Reopen the onboarding & theme setup wizard
overridepulse --onboarding
```

---

## ⌨️ Keyboard Shortcuts

### Navigation & Workspace
| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle panels (`Actions` ↔ `Console`, or `Devices` ↔ `ADB Controls`) |
| `←` `→` `↑` `↓` | Navigate card grid, device list, or scroll console logs |
| `Enter` / `Space` | Run highlighted card / select active device |
| `t` | Cycle KMP target (`Android` / `JVM` / `WasmJS` / `JS`) |
| `d` | Open / close separate **Devices & ADB Controls** view |
| `p` | Switch Gradle / KMP project |
| `1` / `2` / `3` | Toggle **Status**, **Build Output**, or **Live Logcat** pane |
| `m` / `l` | Cycle console view presets |
| `T` (`Shift+T`) | Cycle color theme with live preview |
| `O` (`Shift+O`) | Reopen onboarding & theme wizard |
| `?` | Toggle help overlay |
| `q` / `Ctrl+C` | Quit OverridePulse |

### Build, Target & ADB Actions
| Key | Action |
|---|---|
| `b` | Build & install APK (Android) or compile active KMP target |
| `r` | Relaunch app (Android) or start run / dev server (JVM / WasmJS / JS) |
| `g` | Stop all Gradle daemons (`./gradlew --stop`) |
| `k` / `u` | Force-stop (`k`) or uninstall (`u`) Android app |
| `v` | Cycle Android build variant (`debug`, `release`, flavors) |
| `w` | Open ADB Wireless Pair & Connect modal |
| `f` / `/` | Toggle app-only UID Logcat (`f`) or filter logs (`/`) |
| `y` / `Y` | Copy Logcat (`y`) or Build Output (`Y`) to clipboard |
| `Esc` / `Ctrl+X` | Cancel running build/dev server, close Devices view, or dismiss modal |

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
   │    AppState & Config    │   │  Asynchronous Engines   │
   │ - KMP target state      │   │ - Gradle & KMP scanner  │
   │ - Theme & user config   │   │ - ProcessRunner (PGID)  │
   │ - Ring buffers (Deq)    │   │ - DeviceMonitor (ADB)   │
   │ - Card-grid navigation  │   │ - LogcatStream (UID)    │
   └─────────────────────────┘   └─────────────────────────┘
```

---

## 📄 License

Licensed under the [MIT License](LICENSE).

Copyright (c) 2026 Override LLC & ChristopherTwo.
