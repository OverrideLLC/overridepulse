use crate::android::{DeviceAction, KmpTarget, QuickAction, TargetPlatform};

/// Supported interface languages for OverridePulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    Es,
}

impl Language {
    pub fn all() -> &'static [Language] {
        &[Self::En, Self::Es]
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
        }
    }

    pub fn native_tagline(&self) -> &'static str {
        match self {
            Self::En => "Configure the entire workspace in English",
            Self::Es => "Configura toda la interfaz en Español",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().trim_matches('"').trim_matches('\'').to_ascii_lowercase().as_str() {
            "en" | "english" => Some(Self::En),
            "es" | "spanish" | "español" => Some(Self::Es),
            _ => None,
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Self::En => Self::Es,
            Self::Es => Self::En,
        }
    }
}

/// Returns a concise, localized title for an action card.
pub fn localized_action_title(
    action: QuickAction,
    target: Option<&KmpTarget>,
    lang: Language,
) -> String {
    match (lang, action) {
        (Language::En, QuickAction::BuildAndInstallDebug) => "Build & Install".to_string(),
        (Language::Es, QuickAction::BuildAndInstallDebug) => "Compilar e Instalar".to_string(),

        (Language::En, QuickAction::BuildAabAndDeploy) => "Build AAB".to_string(),
        (Language::Es, QuickAction::BuildAabAndDeploy) => "Generar AAB".to_string(),

        (Language::En, QuickAction::ClearAppData) => "Clear App Data".to_string(),
        (Language::Es, QuickAction::ClearAppData) => "Limpiar Datos".to_string(),

        (Language::En, QuickAction::ForceStopApp) => "Force Stop".to_string(),
        (Language::Es, QuickAction::ForceStopApp) => "Detener App".to_string(),

        (Language::En, QuickAction::RelaunchActivity) => "Relaunch App".to_string(),
        (Language::Es, QuickAction::RelaunchActivity) => "Relanzar App".to_string(),

        (Language::En, QuickAction::UninstallApp) => "Uninstall App".to_string(),
        (Language::Es, QuickAction::UninstallApp) => "Desinstalar".to_string(),

        (Language::En, QuickAction::LaunchScrcpy) => "Scrcpy Mirror".to_string(),
        (Language::Es, QuickAction::LaunchScrcpy) => "Espejo Scrcpy".to_string(),

        (Language::En, QuickAction::StopGradleDaemons) => "Stop Gradle".to_string(),
        (Language::Es, QuickAction::StopGradleDaemons) => "Detener Gradle".to_string(),

        (Language::En, _) => action.title_for(target),
        (Language::Es, QuickAction::KmpCompileTarget) => match target.map(|t| t.platform) {
            Some(TargetPlatform::Jvm) => "Compilar JVM / Jar".to_string(),
            Some(TargetPlatform::WasmJs) => "Compilar WasmJS".to_string(),
            Some(TargetPlatform::Js) => "Compilar JS".to_string(),
            _ => "Compilar Target".to_string(),
        },
        (Language::Es, QuickAction::KmpRunTarget) => match target {
            Some(t) if t.platform == TargetPlatform::Jvm && t.has_compose_desktop => {
                "Ejecutar Desktop".to_string()
            }
            Some(t) if t.platform == TargetPlatform::Jvm => "Ejecutar JVM".to_string(),
            Some(t) if t.platform == TargetPlatform::WasmJs && t.has_nodejs && !t.has_browser => {
                "Ejecutar WasmJS Node".to_string()
            }
            Some(t) if t.platform == TargetPlatform::WasmJs => "Servidor WasmJS".to_string(),
            Some(t) if t.platform == TargetPlatform::Js && t.has_nodejs && !t.has_browser => {
                "Ejecutar JS Node".to_string()
            }
            Some(t) if t.platform == TargetPlatform::Js => "Servidor JS".to_string(),
            _ => "Ejecutar Target".to_string(),
        },
        (Language::Es, QuickAction::KmpBundleDist) => match target {
            Some(t) if t.platform == TargetPlatform::Jvm && t.has_compose_desktop => {
                "Empaquetar Desktop".to_string()
            }
            Some(t) if t.platform == TargetPlatform::Jvm && t.has_application_plugin => {
                "Distribución JVM".to_string()
            }
            Some(t) if t.platform == TargetPlatform::Jvm => "Generar Jar JVM".to_string(),
            Some(t) if t.platform == TargetPlatform::WasmJs => "Bundle WasmJS".to_string(),
            Some(t) if t.platform == TargetPlatform::Js => "Bundle JS".to_string(),
            _ => "Empaquetar Dist".to_string(),
        },
        (Language::Es, QuickAction::KmpTestTarget) => match target {
            Some(t) => format!("Tests {}", t.platform.label()),
            None => "Ejecutar Tests".to_string(),
        },
        (Language::Es, QuickAction::GradleBuildAll) => "Compilar Todo".to_string(),
    }
}

/// Returns a localized title for an ADB device control card.
pub fn localized_device_action_title(action: DeviceAction, lang: Language) -> &'static str {
    match lang {
        Language::En => action.title(),
        Language::Es => match action {
            DeviceAction::WirelessPairAndConnect => "Emparejar Wi-Fi",
            DeviceAction::EnableTcpip5555 => "Activar TCP/IP 5555",
            DeviceAction::ToggleWifi => "Alternar Wi-Fi",
            DeviceAction::ToggleMobileData => "Datos Móviles",
            DeviceAction::ToggleStayAwake => "Pantalla Activa",
            DeviceAction::ToggleDarkMode => "Tema Oscuro",
            DeviceAction::ToggleShowTaps => "Mostrar Toques",
            DeviceAction::OpenWirelessDebuggingSettings => "Ajustes Dev",
            DeviceAction::TakeScreenshot => "Captura Pantalla",
            DeviceAction::RebootDevice => "Reiniciar Equipo",
        },
    }
}

/// Localized static UI labels across panels and modals.
pub struct UiText {
    pub actions_title: &'static str,
    pub status_tab_title: &'static str,
    pub build_tab_title: &'static str,
    pub logcat_tab_title: &'static str,
    pub devices_view_controls_title: &'static str,
    pub no_devices: &'static str,
    pub no_operations: &'static str,
    pub ready: &'static str,
    pub loading_project: &'static str,
    pub running_prefix: &'static str,
    pub paused_tag: &'static str,
    pub copy_hint: &'static str,
    pub scope_copy_hint: &'static str,
    pub scope_app: &'static str,
    pub scope_all: &'static str,
    pub footer_target: &'static str,
    pub footer_project: &'static str,
    pub footer_devices: &'static str,
    pub footer_settings: &'static str,
    pub footer_views: &'static str,
    pub footer_help: &'static str,
    pub footer_quit: &'static str,
    pub footer_back: &'static str,
    pub footer_switch_pane: &'static str,
    pub footer_select_run: &'static str,
    pub footer_navigate: &'static str,
    pub footer_apply_edit: &'static str,
}

pub fn ui_text(lang: Language) -> UiText {
    match lang {
        Language::En => UiText {
            actions_title: "⚡ Actions",
            status_tab_title: " 📊 [1] Status │ 🔨 [2] Build  📜 [3] Logcat ",
            build_tab_title: "Build",
            logcat_tab_title: "Logcat",
            devices_view_controls_title: " 📱 Device Controls (ADB) ",
            no_devices: "  📵 No devices · [w] 📶",
            no_operations: "  💤 Idle · [b] 🔨  [r] 🚀  [t] 🎯",
            ready: "Ready",
            loading_project: "⏳ Loading…",
            running_prefix: "⚡ ",
            paused_tag: " ⏸",
            copy_hint: " │ 📋 ",
            scope_copy_hint: " │ 📦 📋 ",
            scope_app: " [App]",
            scope_all: " [All]",
            footer_target: "Target",
            footer_project: "Project",
            footer_devices: "Devices",
            footer_settings: "Settings",
            footer_views: "Views",
            footer_help: "Help",
            footer_quit: "Quit",
            footer_back: "Back",
            footer_switch_pane: "Switch Pane",
            footer_select_run: "Select / Run",
            footer_navigate: "Navigate",
            footer_apply_edit: "Apply / Edit",
        },
        Language::Es => UiText {
            actions_title: "⚡ Acciones",
            status_tab_title: " 📊 [1] Estado │ 🔨 [2] Build  📜 [3] Logcat ",
            build_tab_title: "Build",
            logcat_tab_title: "Logcat",
            devices_view_controls_title: " 📱 Controles ADB ",
            no_devices: "  📵 Sin equipos · [w] 📶",
            no_operations: "  💤 En espera · [b] 🔨  [r] 🚀  [t] 🎯",
            ready: "Listo",
            loading_project: "⏳ Cargando…",
            running_prefix: "⚡ ",
            paused_tag: " ⏸",
            copy_hint: " │ 📋 ",
            scope_copy_hint: " │ 📦 📋 ",
            scope_app: " [App]",
            scope_all: " [Todo]",
            footer_target: "Target",
            footer_project: "Proyecto",
            footer_devices: "Equipos",
            footer_settings: "Ajustes",
            footer_views: "Vistas",
            footer_help: "Ayuda",
            footer_quit: "Salir",
            footer_back: "Volver",
            footer_switch_pane: "Cambiar Panel",
            footer_select_run: "Elegir / Ejecutar",
            footer_navigate: "Navegar",
            footer_apply_edit: "Aplicar / Editar",
        },
    }
}
