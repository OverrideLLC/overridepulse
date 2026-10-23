use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct SdkConfig {
    pub sdk_root: Option<PathBuf>,
    pub adb_path: Option<PathBuf>,
    pub bundletool_path: Option<PathBuf>,
    pub scrcpy_path: Option<PathBuf>,
}

impl SdkConfig {
    /// Detects SDK root and associated tool paths from environment variables,
    /// standard system locations, and PATH.
    pub fn detect() -> Self {
        let sdk_root = Self::find_sdk_root();
        let adb_path = Self::find_adb(sdk_root.as_deref());
        let bundletool_path = Self::find_bundletool(sdk_root.as_deref());
        let scrcpy_path = Self::find_scrcpy();

        Self {
            sdk_root,
            adb_path,
            bundletool_path,
            scrcpy_path,
        }
    }

    /// Finds Android SDK root directory checking ANDROID_HOME, ANDROID_SDK_ROOT,
    /// and standard platform-specific directories.
    pub fn find_sdk_root() -> Option<PathBuf> {
        // 1. Check ANDROID_HOME
        if let Ok(path) = env::var("ANDROID_HOME") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }

        // 2. Check ANDROID_SDK_ROOT
        if let Ok(path) = env::var("ANDROID_SDK_ROOT") {
            let p = PathBuf::from(path);
            if p.is_dir() {
                return Some(p);
            }
        }

        // 3. Check standard OS directories
        Self::sdk_candidate_dirs()
            .into_iter()
            .find(|candidate| candidate.is_dir())
    }

    fn sdk_candidate_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(home) = env::var("HOME") {
            let home_path = PathBuf::from(home);
            // Linux standard
            dirs.push(home_path.join("Android/Sdk"));
            dirs.push(home_path.join(".android/sdk"));
            // macOS standard
            dirs.push(home_path.join("Library/Android/sdk"));
        }

        #[cfg(windows)]
        {
            if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
                dirs.push(PathBuf::from(local_app_data).join("Android").join("Sdk"));
            }
        }

        // Linux package managers
        dirs.push(PathBuf::from("/usr/lib/android-sdk"));
        dirs.push(PathBuf::from("/opt/android-sdk"));
        dirs.push(PathBuf::from("/var/lib/android-sdk"));

        dirs
    }

    /// Finds adb binary in SDK platform-tools or system PATH
    pub fn find_adb(sdk_root: Option<&Path>) -> Option<PathBuf> {
        let binary_name = exe_name("adb");

        // 1. Check inside SDK platform-tools
        if let Some(root) = sdk_root {
            let path = root.join("platform-tools").join(&binary_name);
            if path.is_file() {
                return Some(path);
            }
        }

        // 2. Search in PATH
        Self::find_in_path(&binary_name)
    }

    /// Finds bundletool binary or jar in PATH, home bin, or SDK directory
    pub fn find_bundletool(sdk_root: Option<&Path>) -> Option<PathBuf> {
        let binary_name = exe_name("bundletool");

        // 1. Check in PATH
        if let Some(path) = Self::find_in_path(&binary_name) {
            return Some(path);
        }

        // 2. Check ~/.local/bin or ~/bin
        if let Ok(home) = env::var("HOME") {
            let home_path = PathBuf::from(home);
            let local_bin = home_path.join(".local/bin").join(&binary_name);
            if local_bin.is_file() {
                return Some(local_bin);
            }
            let user_bin = home_path.join("bin").join(&binary_name);
            if user_bin.is_file() {
                return Some(user_bin);
            }
        }

        // 3. Check inside SDK cmdline-tools or platform-tools
        if let Some(root) = sdk_root {
            let candidates = [
                root.join("cmdline-tools/latest/bin").join(&binary_name),
                root.join("platform-tools").join(&binary_name),
                root.join(&binary_name),
                root.join("bundletool.jar"),
            ];

            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        None
    }

    /// Finds scrcpy binary in PATH or standard system directories
    pub fn find_scrcpy() -> Option<PathBuf> {
        let binary_name = exe_name("scrcpy");

        if let Some(path) = Self::find_in_path(&binary_name) {
            return Some(path);
        }

        // Common install paths
        let standard_paths = [
            PathBuf::from("/usr/bin").join(&binary_name),
            PathBuf::from("/usr/local/bin").join(&binary_name),
            PathBuf::from("/opt/homebrew/bin").join(&binary_name),
        ];

        standard_paths.into_iter().find(|path| path.is_file())
    }

    /// Search for an executable in directories listed in the PATH environment variable.
    pub fn find_in_path(binary_name: &str) -> Option<PathBuf> {
        if let Some(paths) = env::var_os("PATH") {
            for dir in env::split_paths(&paths) {
                let candidate = dir.join(binary_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    pub fn is_adb_available(&self) -> bool {
        self.adb_path.is_some()
    }

    pub fn is_bundletool_available(&self) -> bool {
        self.bundletool_path.is_some()
    }

    pub fn is_scrcpy_available(&self) -> bool {
        self.scrcpy_path.is_some()
    }

    pub fn status_summary(&self) -> String {
        format!(
            "SDK: {} | adb: {} | bundletool: {} | scrcpy: {}",
            self.sdk_root
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Not found".to_string()),
            self.adb_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Missing".to_string()),
            self.bundletool_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Missing".to_string()),
            self.scrcpy_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Missing".to_string())
        )
    }
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_detect_smoke() {
        let config = SdkConfig::detect();
        // Just verify it doesn't panic and prints a summary string
        let summary = config.status_summary();
        assert!(summary.contains("SDK:"));
        assert!(summary.contains("adb:"));
    }
}
