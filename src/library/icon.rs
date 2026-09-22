use std::io::Read;
use std::path::PathBuf;

use super::classify::is_msi;

pub(crate) fn ico_to_png(bytes: &[u8], dest: &PathBuf) -> Option<String> {
    let dir = ico::IconDir::read(std::io::Cursor::new(bytes)).ok()?;
    let best = dir
        .entries()
        .iter()
        .max_by_key(|e| u32::from(e.width()) * u32::from(e.height()))?;
    let image = best.decode().ok()?;
    let file = std::fs::File::create(dest).ok()?;
    image.write_png(file).ok()?;
    Some(dest.to_string_lossy().to_string())
}

pub(crate) fn icon_from_pe(source: &PathBuf, dest: &PathBuf) -> Option<String> {
    let map = pelite::FileMap::open(source).ok()?;
    icon_from_pe_bytes(map.as_ref(), dest)
}

pub(crate) fn icon_from_pe_bytes(bytes: &[u8], dest: &PathBuf) -> Option<String> {
    let pe = pelite::PeFile::from_bytes(bytes).ok()?;
    let resources = pe.resources().ok()?;
    for entry in resources.icons() {
        let Ok((_, group)) = entry else {
            continue;
        };
        let mut ico = Vec::new();
        if group.write(&mut ico).is_err() {
            continue;
        }
        if let Some(path) = ico_to_png(&ico, dest) {
            return Some(path);
        }
    }
    None
}

pub(crate) fn icon_from_msi(source: &PathBuf, dest: &PathBuf) -> Option<String> {
    let mut package = msi::open(source).ok()?;

    let mut names: Vec<String> = Vec::new();
    if package.has_table("Icon") {
        if let Ok(rows) = package.select_rows(msi::Select::table("Icon")) {
            for row in rows {
                if let msi::Value::Str(name) = &row[0] {
                    names.push(name.clone());
                }
            }
        }
    }
    names.extend(package.streams().map(|s| s.to_string()));

    for name in names {
        let mut bytes = Vec::new();
        let Ok(mut stream) = package.read_stream(&name) else {
            continue;
        };
        if stream.read_to_end(&mut bytes).is_err() {
            continue;
        }
        if let Some(path) = ico_to_png(&bytes, dest) {
            return Some(path);
        }
        if bytes.starts_with(b"MZ") {
            if let Some(path) = icon_from_pe_bytes(&bytes, dest) {
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn extract_icon(source: &PathBuf, dest_dir: &PathBuf) -> Option<String> {
    let _ = std::fs::create_dir_all(dest_dir);
    let dest = dest_dir.join("icon.png");
    if is_msi(source) {
        icon_from_msi(source, &dest)
    } else {
        icon_from_pe(source, &dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pe_resources_yield_a_png() {
        let Some(exe) = std::env::var("WINAPPS_TEST_EXE").ok().map(PathBuf::from) else {
            return;
        };
        let dir = std::env::temp_dir().join("winapps-icon-pe");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_png(&extract_icon(&exe, &dir).expect("no icon extracted"));
    }

    #[test]
    fn msi_streams_yield_a_png() {
        let Some(path) = std::env::var("WINAPPS_TEST_MSI").ok().map(PathBuf::from) else {
            return;
        };
        let dir = std::env::temp_dir().join("winapps-icon-msi");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_png(&extract_icon(&path, &dir).expect("no icon extracted"));
        let files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(files.len(), 1, "unexpected leftovers: {files:?}");
    }

    fn assert_png(path: &str) {
        let bytes = std::fs::read(path).expect("icon file missing");
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
    }
}
