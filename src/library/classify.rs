use std::path::PathBuf;

pub(crate) fn is_msi(path: &PathBuf) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("msi"))
}

pub(crate) fn mentions_setup(text: &str) -> bool {
    ["setup", "install", "_inst", "updater"]
        .iter()
        .any(|needle| text.contains(needle))
}

pub(crate) fn looks_like_installer(path: &PathBuf) -> bool {
    if is_msi(path) {
        return true;
    }

    if let Ok(map) = pelite::FileMap::open(path) {
        if let Ok(pe) = pelite::PeFile::from_bytes(map.as_ref()) {
            if let Ok(resources) = pe.resources() {
                if let Ok(manifest) = resources.manifest() {
                    if manifest.contains("requireAdministrator") {
                        return true;
                    }
                }
                if let Ok(info) = resources.version_info() {
                    let mut hit = false;
                    for lang in info.translation() {
                        info.strings(*lang, |key, value| {
                            if matches!(
                                key,
                                "FileDescription"
                                    | "OriginalFilename"
                                    | "InternalName"
                                    | "ProductName"
                            ) && mentions_setup(&value.to_lowercase())
                            {
                                hit = true;
                            }
                        });
                    }
                    if hit {
                        return true;
                    }
                }
            }
        }
    }

    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    mentions_setup(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msi_detected_by_extension() {
        assert!(is_msi(&PathBuf::from("/tmp/Setup.msi")));
        assert!(is_msi(&PathBuf::from("/tmp/Setup.MSI")));
        assert!(!is_msi(&PathBuf::from("/tmp/Setup.exe")));
        assert!(!is_msi(&PathBuf::from("/tmp/msi")));
    }

    #[test]
    fn describes_from_version_resource() {
        let cases = std::env::var("WINAPPS_TEST_CLASSIFY").unwrap_or_default();
        if cases.is_empty() {
            return;
        }
        for case in cases.split(',') {
            let (path, _) = case.rsplit_once('=').expect("path=expected");
            println!("{:?} {path}", describe(&PathBuf::from(path)));
        }
    }

    #[test]
    fn installer_detection_on_samples() {
        let cases = std::env::var("WINAPPS_TEST_CLASSIFY").unwrap_or_default();
        if cases.is_empty() {
            return;
        }
        for case in cases.split(',') {
            let (path, want) = case.rsplit_once('=').expect("path=expected");
            let got = if looks_like_installer(&PathBuf::from(path)) {
                "installer"
            } else {
                "portable"
            };
            assert_eq!(got, want, "misclassified {path}");
        }
    }
}

pub(crate) fn describe(path: &PathBuf) -> Option<String> {
    if is_msi(path) {
        return msi_property(path, "ProductName");
    }
    let map = pelite::FileMap::open(path).ok()?;
    let pe = pelite::PeFile::from_bytes(map.as_ref()).ok()?;
    let info = pe.resources().ok()?.version_info().ok()?;

    let mut description = None;
    let mut product = None;
    for lang in info.translation() {
        info.strings(*lang, |key, value| {
            let value = value.trim().to_string();
            if value.is_empty() {
                return;
            }
            match key {
                "FileDescription" if description.is_none() => description = Some(value),
                "ProductName" if product.is_none() => product = Some(value),
                _ => {}
            }
        });
    }
    description.or(product)
}

fn msi_property(path: &PathBuf, wanted: &str) -> Option<String> {
    let mut package = msi::open(path).ok()?;
    if !package.has_table("Property") {
        return None;
    }
    let rows = package.select_rows(msi::Select::table("Property")).ok()?;
    for row in rows {
        let (msi::Value::Str(key), msi::Value::Str(value)) = (&row[0], &row[1]) else {
            continue;
        };
        if key == wanted && !value.trim().is_empty() {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod describe_tests {
    use super::*;

    #[test]
    fn describe_returns_none_without_resources() {
        let path = std::env::temp_dir().join("sangria-not-a-pe.exe");
        std::fs::write(&path, b"not a pe file at all").unwrap();
        assert_eq!(describe(&path), None);
    }
}
