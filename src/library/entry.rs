use serde::{Deserialize, Serialize};

use super::paths::{applications_dir, data_dir, desktop_file, library_file};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Entry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) exe: String,
    pub(crate) installer: String,
    pub(crate) prefix: String,
    pub(crate) gameid: String,
    pub(crate) icon: String,
    pub(crate) kind: String,
    pub(crate) description: String,
    pub(crate) store: String,
    pub(crate) categories: String,
    pub(crate) shortcut: bool,
}

#[derive(Serialize, Clone, Default)]
pub(crate) struct Staged {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) kind: String,
    pub(crate) icon: String,
    pub(crate) description: String,
    pub(crate) store: String,
    pub(crate) gameid: String,
}

pub(crate) fn load_entries() -> Vec<Entry> {
    std::fs::read_to_string(library_file())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_entries(entries: &[Entry]) {
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(s) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(library_file(), s);
    }
}

pub(crate) fn slugify(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        format!(
            "app-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        )
    } else {
        s
    }
}

pub(crate) const DEFAULT_CATEGORIES: &str = "Utility;";

pub(crate) fn write_desktop_entry(entry: &Entry) -> std::io::Result<()> {
    std::fs::create_dir_all(applications_dir())?;
    let icon = if entry.icon.is_empty() {
        "org.blossomos.winapps".to_string()
    } else {
        entry.icon.clone()
    };
    let categories = if entry.categories.is_empty() {
        DEFAULT_CATEGORIES
    } else {
        entry.categories.as_str()
    };
    let comment_line = if entry.description.is_empty() {
        String::new()
    } else {
        format!("Comment={}\n", entry.description)
    };
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={}\n\
         {comment_line}\
         Icon={icon}\n\
         Exec=winapps --launch {}\n\
         Categories={categories}\n\
         Terminal=false\n\
         StartupNotify=true\n\
         X-WinApps-Id={}\n",
        entry.name, entry.id, entry.id
    );
    std::fs::write(desktop_file(&entry.id), contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_tolerates_missing_fields() {
        let legacy = r#"[{
            "id": "app",
            "name": "App",
            "exe": "",
            "installer": "/tmp/App.exe",
            "prefix": "/tmp/prefix",
            "gameid": "umu-app"
        }]"#;
        let entries: Vec<Entry> = serde_json::from_str(legacy).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].icon, "");
        assert!(!entries[0].shortcut);
    }

    #[test]
    fn slugify_is_stable_and_safe() {
        assert_eq!(slugify("Epic Games Launcher"), "epic-games-launcher");
        assert_eq!(slugify("App 1.0 (x64)"), "app-1-0--x64");
        assert!(slugify("///").starts_with("app-"));
    }
}
