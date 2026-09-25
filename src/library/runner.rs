use std::path::PathBuf;
use std::process::Command;

use super::entry::{Entry, load_entries};

pub(crate) fn installed_build() -> String {
    crate::proton::installed_tag().unwrap_or_default()
}

pub(crate) fn chosen_proton() -> Option<String> {
    let tool = crate::library::paths::read_setting("proton_tool")?;
    if tool.starts_with('/') && !std::path::Path::new(&tool).join("proton").is_file() {
        return None;
    }
    Some(tool)
}

pub(crate) fn default_proton() -> String {
    chosen_proton().unwrap_or_else(|| {
        crate::proton::installed()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    })
}

pub(crate) fn sandboxed() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

pub(crate) fn host_command(program: &str) -> Command {
    if sandboxed() {
        let mut cmd = Command::new("flatpak-spawn");
        cmd.arg("--host").arg(program);
        cmd
    } else {
        Command::new(program)
    }
}

pub(crate) fn sync_theme(entry: &Entry) {
    if !crate::library::paths::read_flag("follow_system_theme", true) {
        return;
    }
    let prefix = PathBuf::from(&entry.prefix);
    if !prefix.join("user.reg").is_file() {
        return;
    }
    let Some(scheme) = crate::wine::theme::system_scheme() else {
        return;
    };
    let _ = crate::wine::theme::apply(&prefix, &scheme);
}

pub(crate) fn launch_target(entry: &Entry) -> String {
    let exe = PathBuf::from(&entry.exe);
    super::prefix::prefix_to_windows(&PathBuf::from(&entry.prefix), &exe)
        .unwrap_or_else(|| entry.exe.clone())
}

pub(crate) fn env_for(entry: &Entry, overrides: &[(String, String)]) -> Vec<(String, String)> {
    let mut env = vec![
        ("WINEPREFIX".to_string(), entry.prefix.clone()),
        ("GAMEID".to_string(), entry.gameid.clone()),
        (
            "STORE".to_string(),
            if entry.store.is_empty() {
                "none".to_string()
            } else {
                entry.store.clone()
            },
        ),
    ];
    if crate::library::paths::read_flag("proton_wayland", true) {
        env.push(("PROTON_ENABLE_WAYLAND".to_string(), "1".to_string()));
    }
    let proton = default_proton();
    if !proton.is_empty() {
        env.push(("PROTONPATH".to_string(), proton));
    }
    for (key, value) in overrides {
        env.retain(|(k, _)| k != key);
        env.push((key.clone(), value.clone()));
    }
    env
}

pub fn launch_headless(id: &str) -> i32 {
    let Some(entry) = load_entries().into_iter().find(|e| e.id == id) else {
        eprintln!("sangria: no app with id {id}");
        return 1;
    };
    if entry.exe.is_empty() {
        eprintln!("sangria: {} has no executable set", entry.name);
        return 1;
    }
    sync_theme(&entry);
    let mut cmd = host_command("umu-run");
    for (key, value) in env_for(&entry, &[]) {
        if sandboxed() {
            cmd.arg(format!("--env={key}={value}"));
        } else {
            cmd.env(key, value);
        }
    }
    cmd.arg(launch_target(&entry)).args(&entry.args);
    match cmd.status() {
        Ok(status) => status.code().unwrap_or(0),
        Err(e) => {
            eprintln!("sangria: could not start umu-run: {e}");
            1
        }
    }
}
