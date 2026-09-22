use std::path::PathBuf;

pub(crate) fn find_exes(root: &PathBuf, out: &mut Vec<String>, depth: usize) {
    if depth > 8 || out.len() > 500 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    for entry in rd.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if file_type.is_dir() {
            if matches!(name.as_str(), "windows" | "winsxs" | "dosdevices") {
                continue;
            }
            find_exes(&entry.path(), out, depth + 1);
        } else if file_type.is_file() && name.ends_with(".exe") && !name.contains("unins") {
            out.push(entry.path().to_string_lossy().to_string());
        }
    }
}

pub(crate) fn looks_like_a_prefix(path: &PathBuf) -> bool {
    path.is_dir()
        && path.components().count() > 3
        && (path.join("drive_c").is_dir() || path.join("pfx").exists())
}

pub(crate) fn find_links(root: &PathBuf, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 6 || out.len() > 200 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else {
        return;
    };
    for entry in rd.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            find_links(&entry.path(), out, depth + 1);
        } else if entry
            .file_name()
            .to_string_lossy()
            .to_lowercase()
            .ends_with(".lnk")
        {
            out.push(entry.path());
        }
    }
}

pub(crate) fn windows_to_prefix(prefix: &PathBuf, windows: &str) -> Option<PathBuf> {
    let path = windows.replace('\\', "/");
    let (drive, rest) = path.split_once(":/")?;
    if !drive.eq_ignore_ascii_case("c") {
        return None;
    }
    for base in [prefix.join("drive_c"), prefix.join("pfx").join("drive_c")] {
        let candidate = base.join(rest);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// What a Start Menu or Desktop shortcut says about the program it launches.
#[derive(Debug)]
pub(crate) struct ShortcutInfo {
    /// The shortcut's own file name, e.g. "Epic Games Launcher" from
    /// "Epic Games Launcher.lnk". This is the name the installer chose to show
    /// end users, more reliable than either the installer package's own
    /// metadata or the installed executable's version resource.
    pub(crate) name: String,
    pub(crate) exe: String,
    /// The .lnk's own NAME_STRING field, shown by Windows as the shortcut's
    /// tooltip and its "Comment" in Properties. Many installers never set it.
    pub(crate) description: String,
}

pub(crate) fn shortcut_targets(prefix: &PathBuf) -> Vec<ShortcutInfo> {
    let mut links = Vec::new();
    for base in [prefix.join("drive_c"), prefix.join("pfx").join("drive_c")] {
        for dir in [
            base.join("ProgramData/Microsoft/Windows/Start Menu/Programs"),
            base.join("users"),
        ] {
            if dir.is_dir() {
                find_links(&dir, &mut links, 0);
            }
        }
    }

    let mut targets: Vec<ShortcutInfo> = Vec::new();
    for link in links {
        let Ok(shell) = lnk::ShellLink::open(&link, lnk::encoding::WINDOWS_1252) else {
            continue;
        };
        let Some(target) = shell.link_target() else {
            continue;
        };
        if !target.to_lowercase().ends_with(".exe") {
            continue;
        }
        let Some(resolved) = windows_to_prefix(prefix, &target) else {
            continue;
        };
        let name = link
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let description = shell
            .string_data()
            .name_string()
            .as_deref()
            .unwrap_or("")
            .trim_matches(['\0', ' '])
            .to_string();
        let exe = resolved.to_string_lossy().to_string();
        if !targets.iter().any(|t| t.exe == exe) {
            targets.push(ShortcutInfo {
                name,
                exe,
                description,
            });
        }
    }
    targets
}

pub(crate) fn prefix_exes(prefix: &PathBuf) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for root in [prefix.join("drive_c"), prefix.join("pfx").join("drive_c")] {
        let Ok(real) = root.canonicalize() else {
            continue;
        };
        if !real.is_dir() || seen.contains(&real) {
            continue;
        }
        seen.push(real.clone());
        find_exes(&real, &mut found, 0);
    }
    found.sort();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_guard_rejects_shallow_paths() {
        assert!(!looks_like_a_prefix(&PathBuf::from("/")));
        assert!(!looks_like_a_prefix(&PathBuf::from("/home")));
        assert!(!looks_like_a_prefix(&PathBuf::from("/tmp")));

        let root = std::env::temp_dir().join("winapps-guard/deep/prefix");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("drive_c")).unwrap();
        assert!(looks_like_a_prefix(&root));

        let bare = std::env::temp_dir().join("winapps-guard/deep/empty");
        std::fs::create_dir_all(&bare).unwrap();
        assert!(!looks_like_a_prefix(&bare));
    }

    #[test]
    fn windows_paths_map_to_drive_c_only() {
        let root = std::env::temp_dir().join("winapps-winpath");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("drive_c/Program Files/App")).unwrap();
        std::fs::write(root.join("drive_c/Program Files/App/App.exe"), b"MZ").unwrap();

        assert_eq!(
            windows_to_prefix(&root, "C:\\Program Files\\App\\App.exe"),
            Some(root.join("drive_c/Program Files/App/App.exe"))
        );
        assert_eq!(windows_to_prefix(&root, "C:\\nope.exe"), None);
        assert_eq!(windows_to_prefix(&root, "Z:\\home\\u\\bait.exe"), None);
    }

    #[test]
    fn exe_scan_stays_inside_the_prefix() {
        let root = std::env::temp_dir().join("winapps-symlink");
        let _ = std::fs::remove_dir_all(&root);
        let prefix = root.join("prefix");
        let outside = root.join("outside");
        std::fs::create_dir_all(prefix.join("drive_c/Program Files")).unwrap();
        std::fs::create_dir_all(prefix.join("dosdevices")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();

        std::fs::write(prefix.join("drive_c/Program Files/Real.exe"), b"MZ").unwrap();
        std::fs::write(outside.join("Bait.exe"), b"MZ").unwrap();
        std::os::unix::fs::symlink(&outside, prefix.join("dosdevices/z:")).unwrap();
        std::os::unix::fs::symlink(&outside, prefix.join("drive_c/Links")).unwrap();
        std::os::unix::fs::symlink(".", prefix.join("pfx")).unwrap();

        let found = prefix_exes(&prefix);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(found[0].ends_with("Real.exe"));
    }

    #[test]
    fn shortcuts_point_at_real_executables() {
        let Some(prefix) = std::env::var("WINAPPS_TEST_PREFIX").ok().map(PathBuf::from) else {
            return;
        };
        let mut links = Vec::new();
        for base in [prefix.join("drive_c"), prefix.join("pfx").join("drive_c")] {
            if base.is_dir() {
                find_links(&base.join("users"), &mut links, 0);
            }
        }
        if links.is_empty() {
            eprintln!("{prefix:?} holds no shortcuts, nothing to resolve");
            return;
        }

        let targets = shortcut_targets(&prefix);
        assert!(
            !targets.is_empty(),
            "{} shortcuts, none resolved",
            links.len()
        );
        for t in &targets {
            assert!(PathBuf::from(&t.exe).is_file(), "{} does not exist", t.exe);
            assert!(t.exe.starts_with(prefix.to_string_lossy().as_ref()));
        }
    }

    #[test]
    fn shortcut_description_is_trimmed_of_padding() {
        let Some(prefix) = std::env::var("WINAPPS_TEST_PREFIX").ok().map(PathBuf::from) else {
            return;
        };
        for t in shortcut_targets(&prefix) {
            assert!(!t.description.contains('\0'), "leftover NUL in {t:?}");
        }
    }
}
