use std::path::{Path, PathBuf};

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
        let Some(candidate) = find_ignoring_case(&base, rest) else {
            continue;
        };
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn find_ignoring_case(base: &Path, rest: &str) -> Option<PathBuf> {
    let mut path = base.to_path_buf();
    for part in rest.split('/').filter(|p| !p.is_empty() && *p != ".") {
        if part == ".." {
            return None;
        }
        let exact = path.join(part);
        if exact.exists() {
            path = exact;
            continue;
        }
        let wanted = part.to_lowercase();
        path = std::fs::read_dir(&path)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().to_lowercase() == wanted)?
            .path();
    }
    Some(path)
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
    pub(crate) args: Vec<String>,
    /// The .lnk's own NAME_STRING field, shown by Windows as the shortcut's
    /// tooltip and its "Comment" in Properties. Many installers never set it.
    pub(crate) description: String,
    pub(crate) icon: Option<(PathBuf, i32)>,
}

fn lnk_text(text: &Option<String>) -> String {
    text.as_deref()
        .unwrap_or("")
        .trim_matches(['\0', ' '])
        .to_string()
}

pub(crate) fn shortcut_targets(prefix: &PathBuf) -> Vec<ShortcutInfo> {
    let mut links = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for root in [prefix.join("drive_c"), prefix.join("pfx").join("drive_c")] {
        let Ok(base) = root.canonicalize() else {
            continue;
        };
        if seen.contains(&base) {
            continue;
        }
        seen.push(base.clone());

        let mut dirs = vec![base.join("ProgramData/Microsoft/Windows/Start Menu/Programs")];
        for user in std::fs::read_dir(base.join("users"))
            .into_iter()
            .flatten()
            .flatten()
        {
            if user.file_type().is_ok_and(|t| t.is_symlink()) {
                continue;
            }
            dirs.push(
                user.path()
                    .join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs"),
            );
            dirs.push(user.path().join("Desktop"));
        }
        for dir in dirs {
            if std::fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
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
        let target_lower = target.to_lowercase();
        let file_name = target_lower.rsplit('\\').next().unwrap_or_default();
        if !file_name.ends_with(".exe") {
            continue;
        }
        if file_name.contains("unins") || target_lower.starts_with("c:\\windows\\") {
            continue;
        }
        let Some(resolved) = windows_to_prefix(prefix, &target) else {
            continue;
        };
        let name = link
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let data = shell.string_data();
        let description = lnk_text(data.name_string());
        let args = split_args(&lnk_text(data.command_line_arguments()));
        let icon = windows_to_prefix(prefix, &lnk_text(data.icon_location()))
            .map(|file| (file, *shell.header().icon_index()));
        let exe = resolved.to_string_lossy().to_string();
        if !targets.iter().any(|t| t.exe == exe && t.args == args) {
            targets.push(ShortcutInfo {
                name,
                exe,
                args,
                description,
                icon,
            });
        }
    }
    targets.sort_by_key(|t| t.name.to_lowercase());
    targets
}

pub(crate) fn suite_name(prefix: &PathBuf, programs: &[ShortcutInfo]) -> Option<String> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for program in programs {
        let Ok(inside) = Path::new(&program.exe).strip_prefix(prefix) else {
            continue;
        };
        let parts: Vec<String> = inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        let Some(drive) = parts.iter().position(|p| p == "drive_c") else {
            continue;
        };
        let (Some(top), Some(folder)) = (parts.get(drive + 1), parts.get(drive + 2)) else {
            continue;
        };
        if !top.to_lowercase().starts_with("program files") || parts.len() <= drive + 3 {
            continue;
        }
        match counts.iter_mut().find(|(name, _)| name == folder) {
            Some((_, count)) => *count += 1,
            None => counts.push((folder.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    counts.into_iter().next().map(|(name, _)| name)
}

pub(crate) fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current: Option<String> = None;
    let mut quoted = false;
    let mut backslashes = 0;
    for c in line.chars() {
        if c == '\\' {
            backslashes += 1;
            continue;
        }
        if c == '"' {
            let arg = current.get_or_insert_with(String::new);
            arg.push_str(&"\\".repeat(backslashes / 2));
            if backslashes % 2 == 1 {
                arg.push('"');
            } else {
                quoted = !quoted;
            }
            backslashes = 0;
            continue;
        }
        if backslashes > 0 {
            current
                .get_or_insert_with(String::new)
                .push_str(&"\\".repeat(backslashes));
            backslashes = 0;
        }
        if (c == ' ' || c == '\t') && !quoted {
            args.extend(current.take());
        } else {
            current.get_or_insert_with(String::new).push(c);
        }
    }
    if backslashes > 0 {
        current
            .get_or_insert_with(String::new)
            .push_str(&"\\".repeat(backslashes));
    }
    args.extend(current);
    args
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
    fn windows_paths_ignore_letter_case() {
        let root = std::env::temp_dir().join("winapps-case");
        let _ = std::fs::remove_dir_all(&root);
        let office = root.join("drive_c/Program Files/Microsoft Office/root/Office16");
        std::fs::create_dir_all(&office).unwrap();
        std::fs::write(office.join("WINWORD.EXE"), b"MZ").unwrap();

        assert_eq!(
            windows_to_prefix(
                &root,
                "C:\\Program Files\\Microsoft Office\\Root\\Office16\\winword.exe"
            ),
            Some(office.join("WINWORD.EXE"))
        );
        assert_eq!(
            windows_to_prefix(
                &root,
                "C:\\Program Files\\..\\..\\winapps-case\\drive_c\\Program Files\\Microsoft Office\\root\\Office16\\WINWORD.EXE"
            ),
            None
        );
    }

    #[test]
    fn arguments_split_like_windows_programs_read_them() {
        assert_eq!(
            split_args("/memoryWindow start"),
            ["/memoryWindow", "start"]
        );
        assert_eq!(
            split_args(r#"--profile "My Profile"  -x"#),
            ["--profile", "My Profile", "-x"]
        );
        assert_eq!(split_args(r"C:\Games\save dir"), [r"C:\Games\save", "dir"]);
        assert_eq!(
            split_args(r#"say\"hi\" "a\\" b"#),
            [r#"say"hi""#, r"a\", "b"]
        );
        assert_eq!(split_args(r#""""#), [""]);
        assert!(split_args("  ").is_empty());
    }

    fn write_lnk(path: &Path, target: &str, args: &str, icon: &str, icon_index: i32) {
        let mut flags: u32 = 0x02 | 0x80;
        if !args.is_empty() {
            flags |= 0x20;
        }
        if !icon.is_empty() {
            flags |= 0x40;
        }

        let mut lnk = Vec::new();
        lnk.extend(0x4Cu32.to_le_bytes());
        lnk.extend([
            0x01, 0x14, 0x02, 0, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46,
        ]);
        lnk.extend(flags.to_le_bytes());
        lnk.extend([0u8; 32]);
        lnk.extend(icon_index.to_le_bytes());
        lnk.extend(1u32.to_le_bytes());
        lnk.extend([0u8; 12]);

        let volume: Vec<u8> = [17u32, 3, 0, 0x10]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .chain([0])
            .collect();
        let path_offset = 0x1C + volume.len();
        let suffix_offset = path_offset + target.len() + 1;
        for value in [
            suffix_offset + 1,
            0x1C,
            1,
            0x1C,
            path_offset,
            0,
            suffix_offset,
        ] {
            lnk.extend((value as u32).to_le_bytes());
        }
        lnk.extend(volume);
        lnk.extend(target.as_bytes());
        lnk.extend([0, 0]);

        for text in [args, icon].into_iter().filter(|t| !t.is_empty()) {
            let wide: Vec<u16> = text.encode_utf16().collect();
            lnk.extend((wide.len() as u16).to_le_bytes());
            lnk.extend(wide.iter().flat_map(|c| c.to_le_bytes()));
        }
        lnk.extend(0u32.to_le_bytes());
        std::fs::write(path, lnk).unwrap();
    }

    #[test]
    fn a_suite_gives_one_target_per_program() {
        let prefix = std::env::temp_dir().join("winapps-suite/prefix");
        let _ = std::fs::remove_dir_all(&prefix);
        let office = prefix.join("drive_c/Program Files/Microsoft Office/root/Office16");
        let menu = prefix.join("drive_c/ProgramData/Microsoft/Windows/Start Menu/Programs");
        let desktop = prefix.join("drive_c/users/steamuser/Desktop");
        for dir in [&office, &menu, &desktop, &prefix.join("drive_c/windows")] {
            std::fs::create_dir_all(dir).unwrap();
        }
        for exe in ["WINWORD.EXE", "EXCEL.EXE", "ONENOTE.EXE", "unins000.exe"] {
            std::fs::write(office.join(exe), b"MZ").unwrap();
        }
        std::fs::write(prefix.join("drive_c/windows/notepad.exe"), b"MZ").unwrap();
        std::os::unix::fs::symlink("steamuser", prefix.join("drive_c/users/someone")).unwrap();
        std::os::unix::fs::symlink(".", prefix.join("pfx")).unwrap();

        let root = "C:\\Program Files\\Microsoft Office\\Root\\Office16\\";
        let onenote = format!("{root}ONENOTE.EXE");
        write_lnk(
            &menu.join("Word.lnk"),
            &format!("{root}WINWORD.EXE"),
            "",
            "",
            0,
        );
        write_lnk(
            &menu.join("Excel.lnk"),
            &format!("{root}EXCEL.EXE"),
            "",
            "",
            0,
        );
        write_lnk(&menu.join("OneNote.lnk"), &onenote, "", "", 0);
        write_lnk(
            &menu.join("Sticky Notes.lnk"),
            &onenote,
            "/memoryWindow start",
            &onenote,
            5,
        );
        write_lnk(
            &menu.join("Uninstall.lnk"),
            &format!("{root}unins000.exe"),
            "",
            "",
            0,
        );
        write_lnk(
            &menu.join("Readme.lnk"),
            "C:\\windows\\notepad.exe",
            "readme.txt",
            "",
            0,
        );
        write_lnk(
            &desktop.join("Word.lnk"),
            &format!("{root}WINWORD.EXE"),
            "",
            "",
            0,
        );

        let targets = shortcut_targets(&prefix);
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Excel", "OneNote", "Sticky Notes", "Word"]);

        let sticky = &targets[2];
        assert_eq!(sticky.exe, office.join("ONENOTE.EXE").to_string_lossy());
        assert_eq!(sticky.args, ["/memoryWindow", "start"]);
        assert_eq!(sticky.icon, Some((office.join("ONENOTE.EXE"), 5)));
        assert!(targets[1].args.is_empty());
        assert_eq!(targets[1].icon, None);
        assert_eq!(
            suite_name(&prefix, &targets).as_deref(),
            Some("Microsoft Office")
        );
    }

    #[test]
    fn suite_name_needs_a_folder_under_program_files() {
        let prefix = PathBuf::from("/prefix");
        let program = |exe: &str| ShortcutInfo {
            name: String::new(),
            exe: exe.to_string(),
            args: Vec::new(),
            description: String::new(),
            icon: None,
        };
        let loose = [
            program("/prefix/drive_c/Program Files/tool.exe"),
            program("/prefix/drive_c/Games/Game/game.exe"),
        ];
        assert_eq!(suite_name(&prefix, &loose), None);

        let mixed = [
            program("/prefix/drive_c/Program Files (x86)/Vendor/a.exe"),
            program("/prefix/pfx/drive_c/program files/Vendor/bin/b.exe"),
            program("/prefix/drive_c/Program Files/Other/c.exe"),
        ];
        assert_eq!(suite_name(&prefix, &mixed).as_deref(), Some("Vendor"));
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
            if let Some((file, _)) = &t.icon {
                assert!(file.is_file(), "icon {file:?} of {} does not exist", t.name);
            }
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
