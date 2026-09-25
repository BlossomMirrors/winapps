use std::path::Path;

use pelite::resources::{Entry, Name};

use super::theme::{BLACK, Rgb, Scheme, WHITE, mix};

pub(crate) const REGISTRY_PATH: &str =
    "C:\\\\windows\\\\resources\\\\themes\\\\sangria\\\\sangria.msstyles";

const THEMES: &str = "drive_c/windows/resources/themes";

const STYLE_BLUE: Rgb = (48, 150, 250);
const MAGENTA: Rgb = (255, 0, 255);

const BASE_GREY: f32 = 245.0;

const SYSMETRICS: &[(&str, &str)] = &[
    ("Scrollbar", "Scrollbar"),
    ("Background", "Background"),
    ("ActiveCaption", "ActiveTitle"),
    ("InactiveCaption", "InactiveTitle"),
    ("Menu", "Menu"),
    ("Window", "Window"),
    ("WindowFrame", "WindowFrame"),
    ("MenuText", "MenuText"),
    ("WindowText", "WindowText"),
    ("CaptionText", "TitleText"),
    ("ActiveBorder", "ActiveBorder"),
    ("InactiveBorder", "InactiveBorder"),
    ("AppWorkSpace", "AppWorkSpace"),
    ("Highlight", "Hilight"),
    ("HighlightText", "HilightText"),
    ("BtnFace", "ButtonFace"),
    ("BtnShadow", "ButtonShadow"),
    ("GrayText", "GrayText"),
    ("BtnText", "ButtonText"),
    ("InactiveCaptionText", "InactiveTitleText"),
    ("BtnHighlight", "ButtonHilight"),
    ("DkShadow3d", "ButtonDkShadow"),
    ("Light3d", "ButtonLight"),
    ("InfoText", "InfoText"),
    ("InfoBk", "InfoWindow"),
    ("ButtonAlternateFace", "ButtonAlternateFace"),
    ("HotTracking", "HotTrackingColor"),
    ("GradientActiveCaption", "GradientActiveTitle"),
    ("GradientInactiveCaption", "GradientInactiveTitle"),
    ("MenuHilight", "MenuHilight"),
    ("MenuBar", "MenuBar"),
];

pub(crate) fn generate(prefix: &Path, scheme: &Scheme) -> Result<(), String> {
    let themes = prefix.join(THEMES);
    let source = themes.join("light").join("light.msstyles");
    let dir = themes.join("sangria");
    let target = dir.join("sangria.msstyles");
    let stamp_path = dir.join("sangria.stamp");

    let meta = std::fs::metadata(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    let stamp = format!(
        "{:?} {} {:?} {scheme:?}",
        std::fs::canonicalize(&source).unwrap_or_default(),
        meta.len(),
        meta.modified().ok(),
    );
    if target.is_file() && std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp) {
        return Ok(());
    }

    let mut bytes = std::fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    for (offset, data) in patches(&bytes, scheme)? {
        bytes[offset..offset + data.len()].copy_from_slice(&data);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
    std::fs::write(&stamp_path, stamp).map_err(|e| format!("{}: {e}", stamp_path.display()))
}

fn patches(bytes: &[u8], scheme: &Scheme) -> Result<Vec<(usize, Vec<u8>)>, String> {
    let pe = pelite::PeFile::from_bytes(bytes).map_err(|e| e.to_string())?;
    let root = pe
        .resources()
        .and_then(|r| r.root())
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for kind in root.entries() {
        let Ok(name) = kind.name() else { continue };
        let is_bitmap = matches!(name, Name::Id(2));
        let is_text = matches!(name, Name::Wide(w) if String::from_utf16_lossy(w) == "TEXTFILE");
        if !is_bitmap && !is_text {
            continue;
        }
        let Ok(Entry::Directory(names)) = kind.entry() else {
            continue;
        };
        for resource in names.entries() {
            let Ok(Entry::Directory(languages)) = resource.entry() else {
                continue;
            };
            for language in languages.entries() {
                let Ok(Entry::DataEntry(data)) = language.entry() else {
                    continue;
                };
                let Ok(slice) = data.bytes() else { continue };
                let offset = slice.as_ptr() as usize - bytes.as_ptr() as usize;
                let patched = if is_bitmap {
                    recolor_bitmap(slice, scheme)
                } else {
                    rewrite_ini(slice, scheme)
                };
                if let Some(patched) = patched {
                    out.push((offset, patched));
                }
            }
        }
    }
    Ok(out)
}

fn recolor(color: Rgb, scheme: &Scheme) -> Rgb {
    let max = color.0.max(color.1).max(color.2);
    let min = color.0.min(color.1).min(color.2);
    let lightness = (max + min) as f32 / 2.0;
    let neutral = if lightness >= BASE_GREY {
        mix(
            scheme.base,
            scheme.surface,
            (lightness - BASE_GREY) / (255.0 - BASE_GREY),
        )
    } else {
        mix(scheme.text, scheme.base, lightness / BASE_GREY)
    };
    let chroma = (max - min) as f32;
    if chroma < 4.0 {
        return neutral;
    }
    let style_chroma = (STYLE_BLUE.2 - STYLE_BLUE.0) as f32;
    let weight = chroma / style_chroma;
    let target = if (180.0..=250.0).contains(&hue(color)) {
        accent_at(lightness, scheme.accent)
    } else {
        color
    };
    mix(neutral, target, weight)
}

fn accent_at(lightness: f32, accent: Rgb) -> Rgb {
    let style = (STYLE_BLUE.0.max(STYLE_BLUE.2) + STYLE_BLUE.0.min(STYLE_BLUE.2)) as f32 / 2.0;
    if lightness >= style {
        mix(accent, WHITE, (lightness - style) / (255.0 - style) * 0.7)
    } else {
        mix(accent, BLACK, (style - lightness) / style * 0.7)
    }
}

fn hue(color: Rgb) -> f32 {
    let (r, g, b) = (color.0 as f32, color.1 as f32, color.2 as f32);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    if delta == 0.0 {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    h * 60.0
}

fn recolor_bitmap(dib: &[u8], scheme: &Scheme) -> Option<Vec<u8>> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(dib.get(at..at + 4)?.try_into().ok()?));
    let header = u32_at(0)? as usize;
    let width = u32_at(4)? as i32;
    let height = u32_at(8)? as i32;
    let bits = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let compression = u32_at(16)?;
    let pixel_size = match (bits, compression) {
        (32, 0) | (32, 3) => 4,
        (24, 0) => 3,
        _ => return None,
    };
    let masks_after = if compression == 3 && header == 40 {
        12
    } else {
        0
    };
    if compression == 3 {
        let at = if header >= 56 { 40 } else { header };
        if (u32_at(at)?, u32_at(at + 4)?, u32_at(at + 8)?) != (0xff0000, 0xff00, 0xff) {
            return None;
        }
    }
    let start = header + masks_after;
    let row_bytes = width.unsigned_abs() as usize * pixel_size;
    let stride = (row_bytes + 3) & !3;
    let rows = height.unsigned_abs() as usize;

    let mut out = dib.to_vec();
    let pixels = out.get_mut(start..start + stride * rows)?;
    for row in pixels.chunks_exact_mut(stride) {
        for px in row[..row_bytes].chunks_exact_mut(pixel_size) {
            let color = (px[2] as u32, px[1] as u32, px[0] as u32);
            if color == MAGENTA {
                continue;
            }
            let (r, g, b) = recolor(color, scheme);
            px[0] = b as u8;
            px[1] = g as u8;
            px[2] = r as u8;
        }
    }
    Some(out)
}

fn rewrite_ini(data: &[u8], scheme: &Scheme) -> Option<Vec<u8>> {
    let units: Vec<u16> = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;

    let mut out = String::with_capacity(text.len());
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = line.to_ascii_lowercase();
            out.push_str(line);
        } else if let Some((key, value)) = line.split_once('=') {
            let (key, value) = (key.trim(), value.trim());
            out.push_str(key);
            out.push('=');
            match edit(&section, key, value, scheme) {
                Some(value) => out.push_str(&value),
                None => out.push_str(value),
            }
        } else {
            out.push_str(line);
        }
        out.push_str("\r\n");
    }

    let mut encoded: Vec<u8> = out.encode_utf16().flat_map(u16::to_le_bytes).collect();
    if encoded.len() > data.len() {
        return None;
    }
    while data.len() - encoded.len() >= 4 {
        encoded.extend_from_slice(&[b'\r', 0, b'\n', 0]);
    }
    if data.len() > encoded.len() {
        encoded.extend_from_slice(&[b' ', 0]);
    }
    Some(encoded)
}

fn edit(section: &str, key: &str, value: &str, scheme: &Scheme) -> Option<String> {
    let lower = key.to_ascii_lowercase();
    if section == "[documentation]" {
        match lower.as_str() {
            "displayname" => return Some("Sangria".to_string()),
            "tooltip" => return Some("Sangria Visual Style".to_string()),
            _ => {}
        }
    }
    if section == "[sysmetrics]" {
        if let Some((_, name)) = SYSMETRICS.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
            return scheme.system(name).map(format_rgb);
        }
    }
    if !lower.ends_with("color") || lower.contains("transparent") {
        return None;
    }
    let parts: Vec<u32> = value
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    let [r, g, b] = parts[..] else { return None };
    let color = (r, g, b);
    let mapped = if lower.contains("text") && color == WHITE {
        scheme.accent_text
    } else {
        recolor(color, scheme)
    };
    Some(format_rgb(mapped))
}

fn format_rgb((r, g, b): Rgb) -> String {
    format!("{r} {g} {b}")
}
