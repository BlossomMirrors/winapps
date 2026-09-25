use std::path::PathBuf;

pub(crate) fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("sangria")
}

pub(crate) fn library_file() -> PathBuf {
    data_dir().join("library.json")
}

pub(crate) fn app_dir(id: &str) -> PathBuf {
    data_dir().join("apps").join(id)
}

pub(crate) fn staging_dir() -> PathBuf {
    data_dir().join("staging")
}

pub(crate) fn scan_dir() -> PathBuf {
    data_dir().join("scan")
}

pub(crate) fn applications_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("applications")
}

pub(crate) fn desktop_file(id: &str) -> PathBuf {
    applications_dir().join(format!("sangria-{id}.desktop"))
}

pub(crate) fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub(crate) fn read_settings() -> serde_json::Value {
    std::fs::read_to_string(settings_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub(crate) fn read_setting(key: &str) -> Option<String> {
    read_settings()
        .get(key)?
        .as_str()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

pub(crate) fn save_setting(key: &str, value: &str) {
    let _ = std::fs::create_dir_all(data_dir());
    let mut settings = read_settings();
    settings[key] = serde_json::Value::String(value.to_string());
    if let Ok(text) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_file(), text);
    }
}

pub(crate) fn read_flag(key: &str, fallback: bool) -> bool {
    read_settings()
        .get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(fallback)
}

pub(crate) fn save_flag(key: &str, value: bool) {
    let _ = std::fs::create_dir_all(data_dir());
    let mut settings = read_settings();
    settings[key] = serde_json::Value::Bool(value);
    if let Ok(text) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_file(), text);
    }
}

pub(crate) fn default_prefix_root() -> String {
    read_setting("prefix_root")
        .unwrap_or_else(|| data_dir().join("prefixes").to_string_lossy().to_string())
}
