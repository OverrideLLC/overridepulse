use std::path::{Path, PathBuf};
use regex::Regex;

/// Represents an Android project and its configuration extracted from Gradle and Manifest files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidProject {
    pub name: String,
    pub root_path: PathBuf,
    pub application_id: Option<String>,
    pub namespace: Option<String>,
    pub launcher_activity: Option<String>,
    pub active_variant: String,
    pub variants: Vec<String>,
    pub modules: Vec<String>,
    pub active_module: Option<String>,
    pub is_valid_android_project: bool,
}

impl AndroidProject {
    /// Creates an `AndroidProject` by detecting configuration at `root_path`.
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        let path = root_path.into();
        Self::detect(&path)
    }

    /// Checks if the project is a valid detected Android project.
    pub fn is_valid(&self) -> bool {
        self.is_valid_android_project
    }

    /// Detects an Android project by inspecting `start_path` and its ancestors.
    ///
    /// Searches upward for Gradle wrapper or settings/build files. If found,
    /// parses `settings.gradle[.kts]`, `build.gradle[.kts]`, and `AndroidManifest.xml`.
    pub fn detect(start_path: &Path) -> Self {
        let root = match find_project_root(start_path) {
            Some(r) => r,
            None => {
                let name = start_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("AndroidProject")
                    .to_string();

                return Self {
                    name,
                    root_path: start_path.to_path_buf(),
                    application_id: None,
                    namespace: None,
                    launcher_activity: None,
                    active_variant: "debug".to_string(),
                    variants: vec!["debug".to_string(), "release".to_string()],
                    modules: Vec::new(),
                    active_module: None,
                    is_valid_android_project: false,
                };
            }
        };

        // 1. Settings parsing (Project Name and Modules)
        let (parsed_name, parsed_modules) = parse_settings_file(&root);

        let project_name = parsed_name.unwrap_or_else(|| {
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("AndroidProject")
                .to_string()
        });

        let mut modules = parsed_modules;
        if modules.is_empty() && (root.join("app/build.gradle").exists() || root.join("app/build.gradle.kts").exists()) {
            modules.push(":app".to_string());
        }

        let active_module = if modules.contains(&":app".to_string()) {
            Some(":app".to_string())
        } else {
            modules.first().cloned()
        };

        // 2. Build Gradle parsing (Application ID, Namespace, Variants)
        let module_dir = match &active_module {
            Some(m) => {
                let rel_path = m.trim_start_matches(':').replace(':', "/");
                let candidate = root.join(&rel_path);
                if candidate.exists() {
                    candidate
                } else if root.join("app").exists() {
                    root.join("app")
                } else {
                    root.clone()
                }
            }
            None => {
                if root.join("app").exists() {
                    root.join("app")
                } else {
                    root.clone()
                }
            }
        };

        let (mut app_id, mut namespace, variants) = parse_build_file(&module_dir);

        // Fallback cross-assignment between namespace and applicationId if one is missing
        if app_id.is_none() && namespace.is_some() {
            app_id = namespace.clone();
        } else if namespace.is_none() && app_id.is_some() {
            namespace = app_id.clone();
        }

        // Active variant selection
        let active_variant = if variants.contains(&"debug".to_string()) {
            "debug".to_string()
        } else if let Some(first_debug) = variants.iter().find(|v| v.ends_with("Debug") || v.ends_with("debug")) {
            first_debug.clone()
        } else {
            variants.first().cloned().unwrap_or_else(|| "debug".to_string())
        };

        // 3. AndroidManifest.xml parsing (Launcher Activity)
        let launcher_activity = parse_manifest_file(&module_dir)
            .or_else(|| parse_manifest_file(&root.join("app")))
            .or_else(|| parse_manifest_file(&root));

        Self {
            name: project_name,
            root_path: root,
            application_id: app_id,
            namespace,
            launcher_activity,
            active_variant,
            variants,
            modules,
            active_module,
            is_valid_android_project: true,
        }
    }

    /// Discovers Android projects within `search_root` up to `max_depth` levels.
    ///
    /// Skips hidden directories and standard build/dependency folders (`build`, `target`, `node_modules`).
    pub fn discover_nearby_projects(search_root: &Path, max_depth: usize) -> Vec<PathBuf> {
        let mut discovered = Vec::new();
        scan_directory(search_root, 0, max_depth, &mut discovered);
        discovered.sort();
        discovered.dedup();
        discovered
    }

    pub fn has_gradle_wrapper(&self) -> bool {
        let gradlew = if cfg!(windows) {
            self.root_path.join("gradlew.bat")
        } else {
            self.root_path.join("gradlew")
        };
        gradlew.exists()
    }

    pub fn gradle_executable(&self) -> PathBuf {
        if cfg!(windows) {
            let gradlew = self.root_path.join("gradlew.bat");
            if gradlew.exists() {
                gradlew
            } else {
                PathBuf::from("gradle.bat")
            }
        } else {
            let gradlew = self.root_path.join("gradlew");
            if gradlew.exists() {
                gradlew
            } else {
                PathBuf::from("gradle")
            }
        }
    }
}

impl Default for AndroidProject {
    fn default() -> Self {
        Self::new(Path::new("."))
    }
}

// ============================================================================
// Internal Helpers & Parsers
// ============================================================================

/// Searches upward starting at `start_path` for an Android project root.
pub fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let start = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    for ancestor in start.ancestors() {
        if is_android_root_dir(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

/// Checks if a directory contains Gradle markers indicative of an Android root.
pub fn is_android_root_dir(dir: &Path) -> bool {
    let has_wrapper = dir.join("gradlew").exists() || dir.join("gradlew.bat").exists();
    let has_settings = dir.join("settings.gradle").exists() || dir.join("settings.gradle.kts").exists();
    let has_build = dir.join("build.gradle").exists()
        || dir.join("build.gradle.kts").exists()
        || dir.join("app").join("build.gradle").exists()
        || dir.join("app").join("build.gradle.kts").exists();

    (has_wrapper || has_settings) && has_build
}

/// Recursive scanner for `discover_nearby_projects`.
fn scan_directory(current: &Path, current_depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if current_depth > max_depth || !current.is_dir() {
        return;
    }

    if current_depth > 0 && is_android_root_dir(current) {
        out.push(current.to_path_buf());
        return;
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with('.')
                || name_str == "build"
                || name_str == "target"
                || name_str == "node_modules"
                || name_str == "dist"
                || name_str == "out"
            {
                continue;
            }
            scan_directory(&path, current_depth + 1, max_depth, out);
        }
    }
}

/// Reads and parses `settings.gradle` or `settings.gradle.kts` in `dir`.
fn parse_settings_file(dir: &Path) -> (Option<String>, Vec<String>) {
    let kts_path = dir.join("settings.gradle.kts");
    let groovy_path = dir.join("settings.gradle");

    let content = if kts_path.exists() {
        std::fs::read_to_string(&kts_path).unwrap_or_default()
    } else if groovy_path.exists() {
        std::fs::read_to_string(&groovy_path).unwrap_or_default()
    } else {
        return (None, Vec::new());
    };

    parse_settings_content(&content)
}

/// Parses the contents of a `settings.gradle` or `settings.gradle.kts` file.
pub fn parse_settings_content(content: &str) -> (Option<String>, Vec<String>) {
    // 1. rootProject.name
    let name_regex = Regex::new(r#"rootProject\.name\s*=\s*['"]([^'"]+)['"]"#).unwrap();
    let name = name_regex.captures(content).map(|cap| cap[1].to_string());

    // 2. include lines
    let include_regex = Regex::new(r#"(?m)^\s*include\s*\(?([^)\r\n]+)\)?"#).unwrap();
    let module_regex = Regex::new(r#"['"](:?[\w\-:]+)['"]"#).unwrap();
    let mut modules = Vec::new();

    for cap in include_regex.captures_iter(content) {
        let args = &cap[1];
        for mod_cap in module_regex.captures_iter(args) {
            let mut m = mod_cap[1].to_string();
            if !m.starts_with(':') {
                m = format!(":{m}");
            }
            if !modules.contains(&m) {
                modules.push(m);
            }
        }
    }

    (name, modules)
}

/// Reads and parses `build.gradle` or `build.gradle.kts` in `dir`.
fn parse_build_file(dir: &Path) -> (Option<String>, Option<String>, Vec<String>) {
    let kts_path = dir.join("build.gradle.kts");
    let groovy_path = dir.join("build.gradle");

    let content = if kts_path.exists() {
        std::fs::read_to_string(&kts_path).unwrap_or_default()
    } else if groovy_path.exists() {
        std::fs::read_to_string(&groovy_path).unwrap_or_default()
    } else {
        return (None, None, vec!["debug".to_string(), "release".to_string()]);
    };

    parse_build_gradle_content(&content)
}

/// Parses `applicationId`, `namespace`, and build variants from `build.gradle[.kts]` content.
pub fn parse_build_gradle_content(content: &str) -> (Option<String>, Option<String>, Vec<String>) {
    // 1. applicationId
    let app_id_regex = Regex::new(r#"applicationId\s*(?:=\s*)?['"]([^'"]+)['"]"#).unwrap();
    let application_id = app_id_regex.captures(content).map(|cap| cap[1].to_string());

    // 2. namespace
    let namespace_regex = Regex::new(r#"namespace\s*(?:=\s*)?['"]([^'"]+)['"]"#).unwrap();
    let namespace = namespace_regex.captures(content).map(|cap| cap[1].to_string());

    // 3. buildTypes & productFlavors
    let mut build_types = Vec::new();
    let mut flavors = Vec::new();

    // Extract buildTypes block
    let build_types_regex = Regex::new(r#"(?s)buildTypes\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}"#).unwrap();
    if let Some(bt_block) = build_types_regex.captures(content) {
        let block_content = &bt_block[1];
        // Match Groovy block names like `release {` or `debug {` or `staging {`
        let block_name_regex = Regex::new(r#"(?m)^\s*([a-zA-Z][a-zA-Z0-9_]*)\s*\{"#).unwrap();
        for cap in block_name_regex.captures_iter(block_content) {
            let name = &cap[1];
            if !matches!(name, "getByName" | "create" | "register" | "named" | "all" | "each") {
                build_types.push(name.to_string());
            }
        }
        // Match Kotlin DSL: getByName("release"), create("staging"), register("benchmark")
        let kts_name_regex = Regex::new(r#"(?:getByName|create|register|named)\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in kts_name_regex.captures_iter(block_content) {
            build_types.push(cap[1].to_string());
        }
    }

    // Extract productFlavors block
    let flavors_regex = Regex::new(r#"(?s)productFlavors\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}"#).unwrap();
    if let Some(pf_block) = flavors_regex.captures(content) {
        let block_content = &pf_block[1];
        let block_name_regex = Regex::new(r#"(?m)^\s*([a-zA-Z][a-zA-Z0-9_]*)\s*\{"#).unwrap();
        for cap in block_name_regex.captures_iter(block_content) {
            let name = &cap[1];
            if !matches!(name, "getByName" | "create" | "register" | "named" | "all" | "each") {
                flavors.push(name.to_string());
            }
        }
        let kts_name_regex = Regex::new(r#"(?:getByName|create|register|named)\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in kts_name_regex.captures_iter(block_content) {
            flavors.push(cap[1].to_string());
        }
    }

    // Ensure baseline buildTypes
    if !build_types.contains(&"debug".to_string()) {
        build_types.push("debug".to_string());
    }
    if !build_types.contains(&"release".to_string()) {
        build_types.push("release".to_string());
    }

    // Combine flavors with buildTypes
    let mut variants = Vec::new();
    if flavors.is_empty() {
        variants = build_types;
    } else {
        for flavor in &flavors {
            for bt in &build_types {
                let mut capitalized_bt = String::new();
                let mut chars = bt.chars();
                if let Some(first) = chars.next() {
                    capitalized_bt.push_str(&first.to_uppercase().to_string());
                    capitalized_bt.push_str(chars.as_str());
                }
                variants.push(format!("{flavor}{capitalized_bt}"));
            }
        }
    }

    // Deduplicate preserving order
    let mut deduped_variants = Vec::new();
    for v in variants {
        if !deduped_variants.contains(&v) {
            deduped_variants.push(v);
        }
    }

    (application_id, namespace, deduped_variants)
}

/// Reads and parses `AndroidManifest.xml` in `module_dir` or `module_dir/src/main`.
fn parse_manifest_file(dir: &Path) -> Option<String> {
    let direct_manifest = dir.join("AndroidManifest.xml");
    let src_manifest = dir.join("src/main/AndroidManifest.xml");

    let path = if src_manifest.exists() {
        src_manifest
    } else if direct_manifest.exists() {
        direct_manifest
    } else {
        return None;
    };

    let content = std::fs::read_to_string(path).ok()?;
    parse_manifest_launcher_activity(&content)
}

/// Extracts the launcher activity (`android.intent.action.MAIN` + `android.intent.category.LAUNCHER`) from manifest XML.
pub fn parse_manifest_launcher_activity(content: &str) -> Option<String> {
    let activity_regex = Regex::new(r#"(?s)<activity\b.*?</activity>"#).unwrap();
    let alias_regex = Regex::new(r#"(?s)<activity-alias\b.*?</activity-alias>"#).unwrap();
    let name_attr_regex = Regex::new(r#"android:name\s*=\s*['"]([^'"]+)['"]"#).unwrap();

    let check_block = |block: &str| -> Option<String> {
        let has_main = block.contains("android.intent.action.MAIN");
        let has_launcher = block.contains("android.intent.category.LAUNCHER");
        if has_main && has_launcher {
            name_attr_regex.captures(block).map(|cap| cap[1].to_string())
        } else {
            None
        }
    };

    for cap in activity_regex.captures_iter(content) {
        if let Some(name) = check_block(&cap[0]) {
            return Some(name);
        }
    }

    for cap in alias_regex.captures_iter(content) {
        if let Some(name) = check_block(&cap[0]) {
            return Some(name);
        }
    }

    None
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    #[test]
    fn test_parse_settings_groovy() {
        let content = r#"
            rootProject.name = 'MyAwesomeApp'
            include ':app'
            include ':core:network', ':feature:dashboard'
        "#;
        let (name, modules) = parse_settings_content(content);
        assert_eq!(name, Some("MyAwesomeApp".to_string()));
        assert_eq!(
            modules,
            vec![
                ":app".to_string(),
                ":core:network".to_string(),
                ":feature:dashboard".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_settings_kts() {
        let content = r#"
            rootProject.name = "KotlinAndroidApp"
            include(":app")
            include(":lib-ui", ":data:repository")
        "#;
        let (name, modules) = parse_settings_content(content);
        assert_eq!(name, Some("KotlinAndroidApp".to_string()));
        assert_eq!(
            modules,
            vec![
                ":app".to_string(),
                ":lib-ui".to_string(),
                ":data:repository".to_string()
            ]
        );
    }

    #[test]
    fn test_parse_build_gradle_groovy() {
        let content = r#"
            plugins {
                id 'com.android.application'
            }

            android {
                namespace 'com.example.pulse'
                compileSdk 34

                defaultConfig {
                    applicationId "com.example.pulse.app"
                    minSdk 24
                    targetSdk 34
                    versionCode 1
                    versionName "1.0"
                }

                buildTypes {
                    release {
                        minifyEnabled false
                    }
                    debug {
                        applicationIdSuffix ".debug"
                    }
                    staging {
                        initWith debug
                    }
                }
            }
        "#;
        let (app_id, namespace, variants) = parse_build_gradle_content(content);
        assert_eq!(app_id, Some("com.example.pulse.app".to_string()));
        assert_eq!(namespace, Some("com.example.pulse".to_string()));
        assert!(variants.contains(&"debug".to_string()));
        assert!(variants.contains(&"release".to_string()));
        assert!(variants.contains(&"staging".to_string()));
    }

    #[test]
    fn test_parse_build_gradle_kts() {
        let content = r#"
            plugins {
                alias(libs.plugins.android.application)
                alias(libs.plugins.kotlin.android)
            }

            android {
                namespace = "com.droid.pulse.kts"
                compileSdk = 34

                defaultConfig {
                    applicationId = "com.droid.pulse.kts"
                    minSdk = 26
                    targetSdk = 34
                }

                buildTypes {
                    getByName("release") {
                        isMinifyEnabled = true
                    }
                    getByName("debug") {
                        isDebuggable = true
                    }
                    create("benchmark") {
                        initWith(getByName("release"))
                    }
                }
            }
        "#;
        let (app_id, namespace, variants) = parse_build_gradle_content(content);
        assert_eq!(app_id, Some("com.droid.pulse.kts".to_string()));
        assert_eq!(namespace, Some("com.droid.pulse.kts".to_string()));
        assert!(variants.contains(&"debug".to_string()));
        assert!(variants.contains(&"release".to_string()));
        assert!(variants.contains(&"benchmark".to_string()));
    }

    #[test]
    fn test_parse_build_gradle_with_flavors() {
        let content = r#"
            android {
                namespace = "com.example.flavor"
                flavorDimensions += listOf("version")
                productFlavors {
                    create("demo") {
                        dimension = "version"
                    }
                    create("full") {
                        dimension = "version"
                    }
                }
                buildTypes {
                    getByName("debug") {}
                    getByName("release") {}
                }
            }
        "#;
        let (_, _, variants) = parse_build_gradle_content(content);
        assert!(variants.contains(&"demoDebug".to_string()));
        assert!(variants.contains(&"demoRelease".to_string()));
        assert!(variants.contains(&"fullDebug".to_string()));
        assert!(variants.contains(&"fullRelease".to_string()));
    }

    #[test]
    fn test_parse_manifest_launcher_activity() {
        let manifest_xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <manifest xmlns:android="http://schemas.android.com/apk/res/android"
            package="com.example.app">

            <application
                android:label="@string/app_name">
                <activity
                    android:name=".MainActivity"
                    android:exported="true">
                    <intent-filter>
                        <action android:name="android.intent.action.MAIN" />
                        <category android:name="android.intent.category.LAUNCHER" />
                    </intent-filter>
                </activity>
                <activity android:name=".DetailActivity" />
            </application>
        </manifest>"#;

        let launcher = parse_manifest_launcher_activity(manifest_xml);
        assert_eq!(launcher, Some(".MainActivity".to_string()));
    }

    #[test]
    fn test_parse_manifest_launcher_activity_alias() {
        let manifest_xml = r#"
            <manifest xmlns:android="http://schemas.android.com/apk/res/android">
                <application>
                    <activity android:name="com.example.app.SplashActivity" />
                    <activity-alias
                        android:name="com.example.app.LauncherAlias"
                        android:targetActivity="com.example.app.SplashActivity">
                        <intent-filter>
                            <action android:name="android.intent.action.MAIN"/>
                            <category android:name="android.intent.category.LAUNCHER"/>
                        </intent-filter>
                    </activity-alias>
                </application>
            </manifest>
        "#;
        let launcher = parse_manifest_launcher_activity(manifest_xml);
        assert_eq!(launcher, Some("com.example.app.LauncherAlias".to_string()));
    }

    #[test]
    fn test_detect_in_filesystem() {
        let temp_dir = std::env::temp_dir().join(format!("overridepulse_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("app/src/main")).unwrap();

        // Write gradlew
        File::create(temp_dir.join("gradlew")).unwrap();

        // Write settings.gradle.kts
        let mut settings = File::create(temp_dir.join("settings.gradle.kts")).unwrap();
        writeln!(
            settings,
            "rootProject.name = \"DetectedTestApp\"\ninclude(\":app\")"
        )
        .unwrap();

        // Write app/build.gradle.kts
        let mut build_file = File::create(temp_dir.join("app/build.gradle.kts")).unwrap();
        writeln!(
            build_file,
            "namespace = \"com.detected.app\"\napplicationId = \"com.detected.app\"\nbuildTypes {{\n getByName(\"debug\") {{}}\n getByName(\"release\") {{}}\n}}"
        )
        .unwrap();

        // Write app/src/main/AndroidManifest.xml
        let mut manifest = File::create(temp_dir.join("app/src/main/AndroidManifest.xml")).unwrap();
        writeln!(
            manifest,
            "<manifest><application><activity android:name=\".HomeActivity\"><intent-filter><action android:name=\"android.intent.action.MAIN\"/><category android:name=\"android.intent.category.LAUNCHER\"/></intent-filter></activity></application></manifest>"
        )
        .unwrap();

        // Test detecting from deep subdirectory (app/src/main)
        let deep_dir = temp_dir.join("app/src/main");
        let project = AndroidProject::detect(&deep_dir);

        assert!(project.is_valid());
        assert_eq!(project.name, "DetectedTestApp");
        assert_eq!(project.application_id, Some("com.detected.app".to_string()));
        assert_eq!(project.namespace, Some("com.detected.app".to_string()));
        assert_eq!(project.launcher_activity, Some(".HomeActivity".to_string()));
        assert_eq!(project.modules, vec![":app".to_string()]);
        assert_eq!(project.active_module, Some(":app".to_string()));
        assert_eq!(project.active_variant, "debug");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_discover_nearby_projects() {
        let temp_base = std::env::temp_dir().join(format!("overridepulse_discover_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_base);

        let p1 = temp_base.join("project_alpha");
        let p2 = temp_base.join("workspace/project_beta");
        let not_project = temp_base.join("non_android");

        fs::create_dir_all(p1.join("app")).unwrap();
        fs::create_dir_all(p2.join("app")).unwrap();
        fs::create_dir_all(&not_project).unwrap();

        File::create(p1.join("gradlew")).unwrap();
        File::create(p1.join("app/build.gradle")).unwrap();

        File::create(p2.join("gradlew.bat")).unwrap();
        File::create(p2.join("app/build.gradle.kts")).unwrap();

        let nearby = AndroidProject::discover_nearby_projects(&temp_base, 3);
        assert_eq!(nearby.len(), 2);
        assert!(nearby.contains(&p1));
        assert!(nearby.contains(&p2));

        let _ = fs::remove_dir_all(&temp_base);
    }
}
