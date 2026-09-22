use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::msstyles;
use super::userreg::{self, Values};

pub(crate) type Rgb = (u32, u32, u32);

pub(crate) const WHITE: Rgb = (255, 255, 255);
pub(crate) const BLACK: Rgb = (0, 0, 0);

pub(crate) fn kdeglobals() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
        });
    base.join("kdeglobals")
}

fn parse(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = name.to_string();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            sections
                .entry(current.clone())
                .or_default()
                .insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    sections
}

fn rgb(value: &str) -> Option<Rgb> {
    let parts: Vec<u32> = value
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    match parts[..] {
        [r, g, b, ..] => Some((r, g, b)),
        _ => None,
    }
}

pub(crate) fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let channel = |x: u32, y: u32| (x as f32 + (y as f32 - x as f32) * t).round() as u32;
    (channel(a.0, b.0), channel(a.1, b.1), channel(a.2, b.2))
}

fn is_dark(rgb: Rgb) -> bool {
    (rgb.0 * 299 + rgb.1 * 587 + rgb.2 * 114) / 1000 < 128
}

#[derive(Debug)]
pub(crate) struct Scheme {
    pub(crate) dark: bool,
    pub(crate) base: Rgb,
    pub(crate) surface: Rgb,
    pub(crate) text: Rgb,
    pub(crate) accent: Rgb,
    pub(crate) accent_text: Rgb,
    pub(crate) colors: Vec<(&'static str, Rgb)>,
}

impl Scheme {
    pub(crate) fn system(&self, name: &str) -> Option<Rgb> {
        self.colors
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| *c)
    }
}

pub(crate) fn system_scheme() -> Option<Scheme> {
    let ini = parse(&std::fs::read_to_string(kdeglobals()).ok()?);
    let get = |section: &str, key: &str| ini.get(section)?.get(key).and_then(|v| rgb(v));

    let window = get("Colors:Window", "BackgroundNormal")?;
    let window_text = get("Colors:Window", "ForegroundNormal").unwrap_or(WHITE);
    let view = get("Colors:View", "BackgroundNormal").unwrap_or(window);
    let view_text = get("Colors:View", "ForegroundNormal").unwrap_or(window_text);
    let button = get("Colors:Button", "BackgroundNormal").unwrap_or(window);
    let button_alt = get("Colors:Button", "BackgroundAlternate").unwrap_or(button);
    let button_text = get("Colors:Button", "ForegroundNormal").unwrap_or(window_text);
    let accent = get("General", "AccentColor")
        .or_else(|| get("Colors:Selection", "BackgroundNormal"))
        .unwrap_or((61, 174, 233));
    let accent_text = get("Colors:Selection", "ForegroundNormal").unwrap_or(WHITE);
    let tooltip = get("Colors:Tooltip", "BackgroundNormal").unwrap_or(view);
    let tooltip_text = get("Colors:Tooltip", "ForegroundNormal").unwrap_or(view_text);
    let title = get("WM", "activeBackground").unwrap_or(window);
    let title_text = get("WM", "activeForeground").unwrap_or(window_text);
    let inactive_title = get("WM", "inactiveBackground").unwrap_or(window);
    let inactive_title_text = get("WM", "inactiveForeground").unwrap_or(window_text);

    let dark = is_dark(window);
    let face = window;
    let (face_light, face_hilight) = if dark {
        (mix(face, window_text, 0.1), mix(face, window_text, 0.2))
    } else {
        (mix(face, WHITE, 0.5), mix(face, WHITE, 0.8))
    };

    let colors = vec![
        ("ActiveBorder", window),
        ("ActiveTitle", title),
        ("AppWorkSpace", window),
        ("Background", window),
        ("ButtonAlternateFace", button_alt),
        ("ButtonDkShadow", mix(face, BLACK, 0.7)),
        ("ButtonFace", face),
        ("ButtonHilight", face_hilight),
        ("ButtonLight", face_light),
        ("ButtonShadow", mix(face, BLACK, 0.35)),
        ("ButtonText", button_text),
        ("GradientActiveTitle", title),
        ("GradientInactiveTitle", inactive_title),
        ("GrayText", mix(view_text, view, 0.5)),
        ("Hilight", accent),
        ("HilightText", accent_text),
        ("HotTrackingColor", accent),
        ("InactiveBorder", window),
        ("InactiveTitle", inactive_title),
        ("InactiveTitleText", inactive_title_text),
        ("InfoText", tooltip_text),
        ("InfoWindow", tooltip),
        ("Menu", view),
        ("MenuBar", window),
        ("MenuHilight", accent),
        ("MenuText", view_text),
        ("Scrollbar", window),
        ("TitleText", title_text),
        ("Window", view),
        ("WindowFrame", mix(window, window_text, 0.3)),
        ("WindowText", view_text),
    ];
    Some(Scheme {
        dark,
        base: window,
        surface: button,
        text: window_text,
        accent,
        accent_text,
        colors,
    })
}

pub(crate) fn apply(prefix: &Path, scheme: &Scheme) -> Result<(), String> {
    let light = if scheme.dark {
        "dword:00000000"
    } else {
        "dword:00000001"
    };
    let colors: Values = scheme
        .colors
        .iter()
        .map(|(name, (r, g, b))| (*name, format!("\"{r} {g} {b}\"")))
        .collect();
    let style: Values = match msstyles::generate(prefix, scheme) {
        Ok(()) => vec![
            ("DllName", format!("\"{}\"", msstyles::REGISTRY_PATH)),
            ("ColorName", "\"Blue\"".to_string()),
            ("SizeName", "\"NormalSize\"".to_string()),
            ("ThemeActive", "\"1\"".to_string()),
        ],
        Err(_) => vec![(
            "ThemeActive",
            if scheme.dark { "\"0\"" } else { "\"1\"" }.to_string(),
        )],
    };

    userreg::set(
        prefix,
        &[
            (
                "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                vec![
                    ("AppsUseLightTheme", light.to_string()),
                    ("SystemUsesLightTheme", light.to_string()),
                ],
            ),
            (
                "Software\\Microsoft\\Windows\\CurrentVersion\\ThemeManager",
                style,
            ),
            ("Control Panel\\Colors", colors),
        ],
    )
}
