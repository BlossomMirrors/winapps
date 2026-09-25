use sha2::{Digest, Sha512};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

//FIXME change it to own wine fork
pub const RELEASES_API: &str =
    "https://api.github.com/repos/nanomatters/proton-cachyos/releases/latest";
pub const TAG_PREFIX: &str = "wineland-";
pub const ASSET_GLOB: &str = "proton-wineland-*-x86_64.tar.xz";
pub const DISPLAY_NAME: &str = "Proton-Wineland";

pub fn install_root() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("sangria").join("proton")
}

pub fn installed_tag() -> Option<String> {
    installed().and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
}

pub fn installed() -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(install_root())
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("proton").is_file())
        .collect();
    found.sort();
    found.pop()
}

#[derive(serde::Serialize)]
pub struct Tool {
    pub label: String,
    pub value: String,
    pub recommended: bool,
}

/// Compatibility tools a library can run with. The managed build comes first
/// with an empty value, so picking it keeps following its updates.
pub fn tools() -> Vec<Tool> {
    let mut tools = vec![
        Tool {
            label: DISPLAY_NAME.to_string(),
            value: String::new(),
            recommended: true,
        },
        // umu fetches these by name on first use.
        Tool {
            label: "GE-Proton (newest release)".to_string(),
            value: "GE-Proton".to_string(),
            recommended: false,
        },
        Tool {
            label: "UMU-Proton (newest release)".to_string(),
            value: "UMU-Proton".to_string(),
            recommended: false,
        },
    ];

    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    // Native Steam first: that is also where umu puts the builds it fetches.
    for root in [
        home.join(".local/share/Steam/compatibilitytools.d"),
        home.join(".steam/root/compatibilitytools.d"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d"),
        PathBuf::from("/usr/share/steam/compatibilitytools.d"),
    ] {
        let Ok(dir) = std::fs::read_dir(&root) else {
            continue;
        };
        let mut found: Vec<PathBuf> = dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("proton").is_file())
            .collect();
        found.sort();
        for path in found {
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            // The Steam folders link to each other or hold copies of the same
            // build, which would otherwise show up twice under one name.
            if tools.iter().any(|t| t.label == label) {
                continue;
            }
            tools.push(Tool {
                label,
                value: path.to_string_lossy().to_string(),
                recommended: false,
            });
        }
    }
    tools
}

/// Wraps a reader and reports how much of it has been consumed.
struct Counting<R> {
    inner: R,
    seen: u64,
    total: u64,
    last: u64,
    report: Arc<dyn Fn(&str, f64) + Send + Sync>,
    label: String,
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.seen += read as u64;
        if self.total > 0 {
            let percent = self.seen * 100 / self.total;
            if percent != self.last {
                self.last = percent;
                (self.report)(
                    &format!("{} {percent}%", self.label),
                    self.seen as f64 / self.total as f64,
                );
            }
        }
        Ok(read)
    }
}

pub struct Release {
    pub tag: String,
    pub url: String,
    pub checksum_url: String,
}

fn get(url: &str) -> Result<Vec<u8>, String> {
    let mut body = ureq::get(url)
        .header("User-Agent", "sangria")
        .call()
        .map_err(|e| format!("{url}: {e}"))?
        .into_body();
    let mut bytes = Vec::new();
    body.as_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{url}: {e}"))?;
    Ok(bytes)
}

fn matches_glob(pattern: &str, name: &str) -> bool {
    let mut rest = name;
    let mut parts = pattern.split('*');

    let Some(first) = parts.next() else {
        return false;
    };
    let Some(stripped) = rest.strip_prefix(first) else {
        return false;
    };
    rest = stripped;

    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

fn assets_of(release: &serde_json::Value) -> Vec<(String, String)> {
    let assets = &release["assets"];

    if let Some(list) = assets.as_array() {
        return list
            .iter()
            .map(|a| {
                (
                    a["name"].as_str().unwrap_or_default().to_string(),
                    a["browser_download_url"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                )
            })
            .collect();
    }

    assets["links"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|a| {
                    (
                        a["name"].as_str().unwrap_or_default().to_string(),
                        a["direct_asset_url"]
                            .as_str()
                            .or_else(|| a["url"].as_str())
                            .unwrap_or_default()
                            .to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn latest_release() -> Result<Release, String> {
    let body = get(RELEASES_API)?;
    let releases: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("malformed release list: {e}"))?;
    let single = [releases.clone()];
    let releases: &[serde_json::Value] = releases.as_array().map(Vec::as_slice).unwrap_or(&single);

    for release in releases {
        let tag = release["tag_name"].as_str().unwrap_or_default();
        if !tag.starts_with(TAG_PREFIX) {
            continue;
        }
        let assets = assets_of(release);
        let Some((name, url)) = assets.iter().find(|(n, _)| matches_glob(ASSET_GLOB, n)) else {
            continue;
        };
        let stem = name.strip_suffix(".tar.xz").unwrap_or(name);
        let checksum_url = assets
            .iter()
            .find(|(n, _)| n == &format!("{stem}.sha512sum") || n == &format!("{name}.sha512sum"))
            .map(|(_, u)| u.clone())
            .unwrap_or_default();

        return Ok(Release {
            tag: tag.to_string(),
            url: url.clone(),
            checksum_url,
        });
    }
    Err(format!("no release tagged {TAG_PREFIX}* at {RELEASES_API}"))
}

pub fn download(
    release: &Release,
    progress: impl Fn(&str, f64) + Send + Sync + 'static,
) -> Result<PathBuf, String> {
    let target = install_root().join(&release.tag);
    if target.join("proton").is_file() {
        return Ok(target);
    }
    let progress: Arc<dyn Fn(&str, f64) + Send + Sync> = Arc::new(progress);

    let expected = if release.checksum_url.is_empty() {
        None
    } else {
        progress("fetching the checksum", -1.0);
        let text = String::from_utf8_lossy(&get(&release.checksum_url)?).to_string();
        text.split_whitespace().next().map(str::to_lowercase)
    };

    progress(&format!("downloading {}", release.tag), -1.0);
    let response = ureq::get(&release.url)
        .header("User-Agent", "sangria")
        .call()
        .map_err(|e| format!("{}: {e}", release.url))?;
    let total: u64 = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut reader = response.into_body().into_reader();
    let mut archive = Vec::with_capacity(total as usize);
    let mut buffer = vec![0u8; 1 << 20];
    let mut last = u64::MAX;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("download failed: {e}"))?;
        if read == 0 {
            break;
        }
        archive.extend_from_slice(&buffer[..read]);
        if total > 0 {
            let done = archive.len() as u64;
            let percent = done * 100 / total;
            if percent != last {
                last = percent;
                progress(
                    &format!("downloading {percent}%"),
                    done as f64 / total as f64,
                );
            }
        }
    }

    if let Some(expected) = expected {
        let mut hasher = Sha512::new();
        let mut last = u64::MAX;
        for (index, chunk) in archive.chunks(4 << 20).enumerate() {
            hasher.update(chunk);
            let done = ((index + 1) * (4 << 20)).min(archive.len()) as u64;
            let percent = done * 100 / archive.len().max(1) as u64;
            if percent != last {
                last = percent;
                progress(
                    &format!("verifying {percent}%"),
                    done as f64 / archive.len() as f64,
                );
            }
        }
        let actual: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != expected {
            return Err("checksum mismatch, the download was corrupted".to_string());
        }
    }

    let staging = install_root().join(format!(".{}.part", release.tag));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;

    // Progress is measured on the compressed input, whose size is known, rather
    // than on the unpacked tree, whose size is not.
    let compressed = archive.len() as u64;
    let counting = Counting {
        inner: std::io::Cursor::new(archive),
        seen: 0,
        total: compressed,
        last: u64::MAX,
        report: Arc::clone(&progress),
        label: "unpacking".to_string(),
    };
    tar::Archive::new(xz2::read::XzDecoder::new(counting))
        .unpack(&staging)
        .map_err(|e| format!("unpacking failed: {e}"))?;

    let inner = std::fs::read_dir(&staging)
        .map_err(|e| format!("{}: {e}", staging.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .ok_or("the archive had no directory in it")?;

    let _ = std::fs::remove_dir_all(&target);
    std::fs::rename(&inner, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    let _ = std::fs::remove_dir_all(&staging);

    if !target.join("proton").is_file() {
        return Err(format!("{} has no proton script", target.display()));
    }
    progress(&format!("{} is ready", release.tag), 1.0);
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_matches_asset_names() {
        assert!(matches_glob(
            "proton-wineland-*-x86_64.tar.xz",
            "proton-wineland-11.0-20260917-x86_64.tar.xz"
        ));
        assert!(!matches_glob(
            "proton-wineland-*-x86_64.tar.xz",
            "proton-wineland-11.0-20260917-x86_64_v3.tar.xz"
        ));
        assert!(!matches_glob(
            "proton-wineland-*-x86_64.tar.xz",
            "proton-wineland-11.0-20260917-x86_64.sha512sum"
        ));
        assert!(matches_glob("*.tar.xz", "anything.tar.xz"));
        assert!(matches_glob("exact.tar.xz", "exact.tar.xz"));
        assert!(!matches_glob("exact.tar.xz", "other.tar.xz"));
        assert!(matches_glob("a-*-b-*.xz", "a-1-b-2.xz"));
    }

    #[test]
    fn assets_parse_from_both_forges() {
        let github = serde_json::json!({
            "assets": [{ "name": "a.tar.xz", "browser_download_url": "https://h/a.tar.xz" }]
        });
        let gitlab = serde_json::json!({
            "assets": { "links": [
                { "name": "a.tar.xz", "direct_asset_url": "https://h/a.tar.xz", "url": "https://h/other" }
            ]}
        });
        assert_eq!(
            assets_of(&github),
            vec![("a.tar.xz".to_string(), "https://h/a.tar.xz".to_string())]
        );
        assert_eq!(assets_of(&gitlab), assets_of(&github));
    }

    #[test]
    fn latest_release_resolves() {
        if std::env::var("WINAPPS_TEST_NETWORK").is_err() {
            eprintln!("set WINAPPS_TEST_NETWORK=1 to exercise the release lookup");
            return;
        }
        let release = latest_release().expect("no release found");
        assert!(release.tag.starts_with(TAG_PREFIX));
        assert!(
            matches_glob(
                ASSET_GLOB,
                release.url.rsplit('/').next().unwrap_or_default()
            ),
            "url {}",
            release.url
        );
        assert!(
            release.checksum_url.ends_with(".sha512sum"),
            "checksum {}",
            release.checksum_url
        );

        let sum = get(&release.checksum_url).expect("checksum not reachable");
        let text = String::from_utf8_lossy(&sum);
        let digest = text.split_whitespace().next().unwrap_or_default();
        assert_eq!(digest.len(), 128, "not a sha512: {text}");
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn counting_reader_reports_every_percent() {
        let seen: Arc<Mutex<Vec<(String, f64)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);

        let data = vec![7u8; 10_000];
        let total = data.len() as u64;
        let mut reader = Counting {
            inner: std::io::Cursor::new(data),
            seen: 0,
            total,
            last: u64::MAX,
            report: Arc::new(move |label: &str, fraction: f64| {
                sink.lock().unwrap().push((label.to_string(), fraction));
            }),
            label: "unpacking".to_string(),
        };

        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), 10_000);

        let seen = seen.lock().unwrap();
        assert!(!seen.is_empty(), "no progress reported");
        assert!(
            seen.iter()
                .all(|(label, _)| label.starts_with("unpacking "))
        );
        let last = seen.last().unwrap();
        assert!((last.1 - 1.0).abs() < f64::EPSILON, "ended at {}", last.1);
        assert!(
            seen.windows(2).all(|w| w[1].1 >= w[0].1),
            "progress went backwards"
        );
    }
}
