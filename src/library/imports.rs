use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::classify::is_msi;
use super::entry::{Entry, Staged, save_entries, slugify, write_desktop_entry};
use super::icon::extract_icon;
use super::paths::{app_dir, desktop_file, staging_dir};
use super::prefix::{prefix_exes, shortcut_targets};
use super::runner::env_for;
use super::{classify::looks_like_installer, qobject};

impl qobject::Library {
    pub(crate) fn prepare(mut self: core::pin::Pin<&mut Self>, file: &QString) {
        let picked = file.to_string();
        let picked = picked
            .strip_prefix("file://")
            .unwrap_or(&picked)
            .to_string();
        let source = PathBuf::from(&picked);

        let file_name_stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "App".to_string());

        let staging = staging_dir();
        let _ = std::fs::remove_dir_all(&staging);
        if let Err(e) = std::fs::create_dir_all(&staging) {
            self.append_log(&format!("could not create {}: {e}", staging.display()));
            return;
        }

        let file_name = source
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "app.exe".to_string());
        let local = staging.join(&file_name);
        if let Err(e) = std::fs::copy(&source, &local) {
            self.append_log(&format!("could not copy {}: {e}", source.display()));
            return;
        }

        // version resource carries the real product name
        let name = super::classify::describe(&local).unwrap_or(file_name_stem);

        let staged = Staged {
            kind: if looks_like_installer(&local) {
                "installer"
            } else {
                "portable"
            }
            .to_string(),
            icon: extract_icon(&local, &staging).unwrap_or_default(),
            description: String::new(),
            store: String::new(),
            gameid: String::new(),
            source: local.to_string_lossy().to_string(),
            name,
        };
        let mut staged = staged;
        if let Some(hit) = super::database::lookup(&staged.name) {
            staged.store = hit.store;
            staged.gameid = hit.gameid;
        }
        self.as_mut().rust_mut().staged = Some(staged);
        self.sync_staged();
    }

    pub(crate) fn discard_staged(mut self: core::pin::Pin<&mut Self>) {
        let _ = std::fs::remove_dir_all(staging_dir());
        self.as_mut().rust_mut().staged = None;
        self.sync_staged();
    }

    pub(crate) fn adopt_staged(
        mut self: core::pin::Pin<&mut Self>,
        name: &str,
        prefix_root: &str,
    ) -> Option<Entry> {
        let staged = self.rust().staged.clone()?;
        let id = slugify(name);
        let dir = app_dir(&id);
        let prefix = PathBuf::from(prefix_root).join(&id);
        if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::create_dir_all(&prefix))
        {
            self.as_mut()
                .append_log(&format!("could not create {}: {e}", prefix.display()));
            return None;
        }

        let source = PathBuf::from(&staged.source);
        let file_name = source
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "app.exe".to_string());
        let local = dir.join(&file_name);
        if let Err(e) =
            std::fs::rename(&source, &local).or_else(|_| std::fs::copy(&source, &local).map(|_| ()))
        {
            self.as_mut()
                .append_log(&format!("could not move {}: {e}", source.display()));
            return None;
        }

        let icon = if staged.icon.is_empty() {
            String::new()
        } else {
            let target = dir.join("icon.png");
            let moved = std::fs::rename(&staged.icon, &target).is_ok()
                || std::fs::copy(&staged.icon, &target).is_ok();
            if moved {
                target.to_string_lossy().to_string()
            } else {
                String::new()
            }
        };

        Some(Entry {
            id: id.clone(),
            name: name.to_string(),
            exe: String::new(),
            installer: local.to_string_lossy().to_string(),
            prefix: prefix.to_string_lossy().to_string(),
            gameid: if staged.gameid.is_empty() {
                format!("umu-{id}")
            } else {
                staged.gameid.clone()
            },
            icon,
            kind: staged.kind,
            description: staged.description,
            store: String::new(),
            categories: String::new(),
            shortcut: false,
        })
    }

    pub(crate) fn register(mut self: core::pin::Pin<&mut Self>, mut entry: Entry, shortcut: bool) {
        if shortcut {
            match write_desktop_entry(&entry) {
                Ok(()) => {
                    entry.shortcut = true;
                    self.as_mut().append_log(&format!(
                        "desktop entry written to {}",
                        desktop_file(&entry.id).display()
                    ));
                }
                Err(e) => self
                    .as_mut()
                    .append_log(&format!("could not write the desktop entry: {e}")),
            }
        }
        let id = entry.id.clone();
        self.as_mut().rust_mut().entries.retain(|e| e.id != id);
        self.as_mut().rust_mut().entries.push(entry);
        save_entries(&self.rust().entries);
        self.sync_entries();
    }

    pub(crate) fn commit_portable(
        mut self: core::pin::Pin<&mut Self>,
        name: &QString,
        description: &QString,
        categories: &QString,
        store: &QString,
        shortcut: bool,
    ) {
        let name = name.to_string();
        let root = self.prefix_root().to_string();
        let Some(mut entry) = self.as_mut().adopt_staged(&name, &root) else {
            return;
        };
        entry.exe = entry.installer.clone();
        entry.kind = "portable".to_string();
        entry.categories = categories.to_string();
        entry.store = store.to_string();
        if !description.to_string().is_empty() {
            entry.description = description.to_string();
        }
        self.as_mut().register(entry, shortcut);
        self.as_mut()
            .append_log(&format!("{name} added as a standalone program"));
        self.discard_staged();
    }

    pub(crate) fn commit_installer(
        mut self: core::pin::Pin<&mut Self>,
        name: &QString,
        root: &QString,
        description: &QString,
        categories: &QString,
        store: &QString,
        shortcut: bool,
    ) {
        let name = name.to_string();
        let mut root = root.to_string();
        if root.is_empty() {
            root = self.prefix_root().to_string();
        }
        let Some(mut entry) = self.as_mut().adopt_staged(&name, &root) else {
            return;
        };
        entry.categories = categories.to_string();
        entry.store = store.to_string();
        if !description.to_string().is_empty() {
            entry.description = description.to_string();
        }
        self.as_mut().rust_mut().pending_shortcut = shortcut;
        self.as_mut().rust_mut().staged = Some(Staged {
            name: entry.name.clone(),
            source: entry.installer.clone(),
            kind: entry.kind.clone(),
            icon: entry.icon.clone(),
            description: entry.description.clone(),
            store: entry.store.clone(),
            gameid: entry.gameid.clone(),
        });
        self.as_mut().sync_staged();
        self.start_installer(entry);
    }

    pub(crate) fn start_installer(mut self: core::pin::Pin<&mut Self>, entry: Entry) {
        self.as_mut().rust_mut().pending = Some(entry.clone());
        if self.as_mut().prepare_proton_for_install() {
            return;
        }
        self.run_installer_now(entry);
    }

    pub(crate) fn run_installer_now(mut self: core::pin::Pin<&mut Self>, entry: Entry) {
        let prefix = PathBuf::from(&entry.prefix);
        let before = prefix_exes(&prefix);
        let overrides = self.rust().overrides();
        let env = env_for(&entry, &overrides);
        let installer = PathBuf::from(&entry.installer);
        let args = if is_msi(&installer) {
            vec![
                "msiexec".to_string(),
                "/i".to_string(),
                entry.installer.clone(),
            ]
        } else {
            vec![entry.installer.clone()]
        };
        let status = format!("Installing {}…", entry.name);

        self.as_mut().rust_mut().pending_before = before;
        self.as_mut().rust_mut().pending = Some(entry);
        self.as_mut()
            .spawn("umu-run".to_string(), args, env, status);
        self.watch_for_shortcut(prefix);
    }

    pub(crate) fn watch_for_shortcut(self: core::pin::Pin<&mut Self>, prefix: PathBuf) {
        let watching = Arc::clone(&self.rust().watching);
        if watching.swap(true, Ordering::SeqCst) {
            return;
        }
        let thread = self.qt_thread();

        std::thread::spawn(move || {
            let mut settled = 0;
            for _ in 0..1800 {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if !watching.load(Ordering::SeqCst) {
                    return;
                }
                if shortcut_targets(&prefix).is_empty() {
                    settled = 0;
                    continue;
                }
                settled += 1;
                if settled < 3 {
                    continue;
                }
                watching.store(false, Ordering::SeqCst);
                let _ = thread.queue(|mut qobject| {
                    qobject.as_mut().append_log(
                        "the installer created a shortcut, treating the install as finished",
                    );
                    qobject.finish_install(0);
                });
                return;
            }
            watching.store(false, Ordering::SeqCst);
        });
    }

    pub(crate) fn run_installer(mut self: core::pin::Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            self.append_log(&format!("unknown app: {id}"));
            return;
        };
        let shortcut = entry.shortcut;
        self.as_mut().rust_mut().pending_shortcut = shortcut;
        self.start_installer(entry);
    }

    pub(crate) fn finish_install(mut self: core::pin::Pin<&mut Self>, code: i32) {
        let Some(entry) = self.as_mut().rust_mut().pending.take() else {
            return;
        };
        self.as_mut().set_busy(false);
        self.as_mut().set_status(QString::default());
        self.rust().watching.store(false, Ordering::SeqCst);
        let before = std::mem::take(&mut self.as_mut().rust_mut().pending_before);
        let shortcut = self.rust().pending_shortcut;

        if code != 0 {
            self.append_log(&format!(
                "{}: the installer exited with {code}, nothing was added",
                entry.name
            ));
            return;
        }

        let after = prefix_exes(&PathBuf::from(&entry.prefix));
        let fresh: Vec<String> = after.into_iter().filter(|p| !before.contains(p)).collect();

        let matched_shortcut = shortcut_targets(&PathBuf::from(&entry.prefix))
            .into_iter()
            .next();
        let shortcut_exe = matched_shortcut.as_ref().map(|s| s.exe.clone());

        if shortcut_exe.is_none() && fresh.is_empty() {
            self.append_log(&format!(
                "{}: the installer left nothing behind, nothing was added",
                entry.name
            ));
            return;
        }

        let wanted = slugify(&entry.name).replace('-', "");
        let best = shortcut_exe.or_else(|| {
            fresh
                .iter()
                .find(|p| {
                    PathBuf::from(p)
                        .file_stem()
                        .map(|s| {
                            s.to_string_lossy()
                                .to_lowercase()
                                .replace(['-', '_', ' '], "")
                        })
                        .is_some_and(|stem| stem == wanted)
                })
                .cloned()
                .or_else(|| {
                    fresh
                        .iter()
                        .max_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
                        .cloned()
                })
        });

        let Some(best) = best else {
            return;
        };
        self.as_mut()
            .append_log(&format!("{}: launching {best}", entry.name));

        let mut entry = entry;
        let installed = PathBuf::from(&best);

        if let Some(icon) = extract_icon(&installed, &app_dir(&entry.id)) {
            entry.icon = icon;
        }

        if let Some(shortcut) = &matched_shortcut {
            if !shortcut.name.is_empty() {
                entry.name = shortcut.name.clone();
            }
            if entry.description.is_empty() && !shortcut.description.is_empty() {
                entry.description = shortcut.description.clone();
            }
        }
        entry.exe = best;
        self.as_mut().register(entry, shortcut);
        self.as_mut().rust_mut().staged = None;
        self.sync_staged();
    }
}
