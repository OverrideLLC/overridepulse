pub mod device;
pub mod logcat;
pub mod project;
pub mod runner;

pub use device::{spawn_device_monitor, ConnectionType, Device, DeviceAction, DeviceState};
pub use logcat::{LogEntry, LogLevel, LogcatStream};
pub use project::{AndroidProject, KmpTarget, TargetPlatform};
pub use runner::{
    parse_gradle_progress, resolve_kmp_gradle_task, ActionContext, ProcessRunner, QuickAction,
};

/// Alias for projects that include Kotlin Multiplatform targets alongside Android.
pub type GradleProject = AndroidProject;
