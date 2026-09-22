use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::PathBuf;

use super::entry::{Entry, save_entries, write_desktop_entry};
use super::icon::extract_icon;
use super::paths::{app_dir, desktop_file};
use super::prefix::{looks_like_a_prefix, prefix_exes};
use super::qobject;
use super::runner::{default_proton, env_for, host_command};

impl qobject::Library {
    pub(crate) fn launch(mut self: core::pin::Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            self.append_log(&format!("unknown app: {id}"));
            return;
        };
        if entry.exe.is_empty() {
            self.append_log(&format!("{}: no executable picked yet", entry.name));
            return;
        }
        if self.as_mut().prepare_proton(Some(id)) {
            return;
        }
        self.run_now(&entry);
    }

    pub(crate) fn run_now(self: core::pin::Pin<&mut Self>, entry: &Entry) {
        super::runner::sync_theme(entry);
        super::runner::sync_dpi(entry);
        let overrides = self.rust().overrides();
        let env = env_for(entry, &overrides);
        self.spawn_detached(vec![entry.exe.clone()], env, entry.name.clone());
    }

    pub(crate) fn winetricks(self: core::pin::Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            return;
        };
        super::runner::sync_theme(&entry);
        super::runner::sync_dpi(&entry);
        let overrides = self.rust().overrides();
        let env = env_for(&entry, &overrides);
        let status = format!("winetricks in {}…", entry.name);
        self.spawn(
            "umu-run".to_string(),
            vec!["winetricks".to_string(), "--gui".to_string()],
            env,
            status,
        );
    }

    pub(crate) fn open_prefix(self: core::pin::Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            return;
        };
        match host_command("xdg-open").arg(&entry.prefix).spawn() {
            Ok(_) => self.append_log(&format!("opened {}", entry.prefix)),
            Err(e) => self.append_log(&format!("xdg-open failed: {e}")),
        }
    }

    pub(crate) fn candidates(self: core::pin::Pin<&mut Self>, id: &QString) -> QString {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id) else {
            return QString::from("[]");
        };
        let found = prefix_exes(&PathBuf::from(&entry.prefix));
        QString::from(&serde_json::to_string(&found).unwrap_or_else(|_| "[]".to_string()))
    }

    pub(crate) fn set_exe(mut self: core::pin::Pin<&mut Self>, id: &QString, exe: &QString) {
        let id = id.to_string();
        let exe = exe.to_string();
        let needs_icon = self.rust().entry(&id).is_some_and(|e| e.icon.is_empty());
        let icon = if needs_icon {
            extract_icon(&PathBuf::from(&exe), &app_dir(&id)).unwrap_or_default()
        } else {
            String::new()
        };
        let mut changed = None;
        if let Some(entry) = self
            .as_mut()
            .rust_mut()
            .entries
            .iter_mut()
            .find(|e| e.id == id)
        {
            entry.exe = exe;
            if !icon.is_empty() {
                entry.icon = icon;
            }
            if entry.shortcut {
                changed = Some(entry.clone());
            }
        }
        if let Some(entry) = changed {
            let _ = write_desktop_entry(&entry);
        }
        save_entries(&self.rust().entries);
        self.sync_entries();
    }

    pub(crate) fn search_database(&self, needle: &QString) -> QString {
        let hits = super::database::search(&needle.to_string(), 12);
        QString::from(&serde_json::to_string(&hits).unwrap_or_else(|_| "[]".to_string()))
    }

    pub(crate) fn set_details(
        mut self: core::pin::Pin<&mut Self>,
        id: &QString,
        description: &QString,
        categories: &QString,
    ) {
        let id = id.to_string();
        let description = description.to_string();
        let categories = categories.to_string();
        let mut changed = None;
        if let Some(entry) = self
            .as_mut()
            .rust_mut()
            .entries
            .iter_mut()
            .find(|e| e.id == id)
        {
            entry.description = description;
            entry.categories = categories;
            if entry.shortcut {
                changed = Some(entry.clone());
            }
        }
        if let Some(entry) = changed {
            let _ = write_desktop_entry(&entry);
        }
        save_entries(&self.rust().entries);
        self.sync_entries();
    }

    pub(crate) fn set_shortcut(mut self: core::pin::Pin<&mut Self>, id: &QString, wanted: bool) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            return;
        };
        if wanted {
            if let Err(e) = write_desktop_entry(&entry) {
                self.append_log(&format!("could not write the desktop entry: {e}"));
                return;
            }
        } else {
            let _ = std::fs::remove_file(desktop_file(&id));
        }
        if let Some(stored) = self
            .as_mut()
            .rust_mut()
            .entries
            .iter_mut()
            .find(|e| e.id == id)
        {
            stored.shortcut = wanted;
        }
        save_entries(&self.rust().entries);
        self.sync_entries();
    }

    pub(crate) fn uninstall(mut self: core::pin::Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        let Some(entry) = self.rust().entry(&id).cloned() else {
            return;
        };

        let _ = std::fs::remove_file(desktop_file(&id));
        let _ = std::fs::remove_dir_all(app_dir(&id));

        let prefix = PathBuf::from(&entry.prefix);
        if looks_like_a_prefix(&prefix) {
            match std::fs::remove_dir_all(&prefix) {
                Ok(()) => self
                    .as_mut()
                    .append_log(&format!("removed {}", prefix.display())),
                Err(e) => self
                    .as_mut()
                    .append_log(&format!("could not remove {}: {e}", prefix.display())),
            }
        } else {
            self.as_mut().append_log(&format!(
                "{} does not look like a prefix, leaving it alone",
                prefix.display()
            ));
        }

        self.as_mut().rust_mut().entries.retain(|e| e.id != id);
        save_entries(&self.rust().entries);
        self.as_mut().sync_entries();
        self.append_log(&format!("uninstalled {}", entry.name));
    }

    pub(crate) fn clear_log(self: core::pin::Pin<&mut Self>) {
        self.set_log(QString::default());
    }

    pub(crate) fn ensure_proton(mut self: core::pin::Pin<&mut Self>) {
        self.as_mut().prepare_proton(None);
    }

    pub(crate) fn prepare_proton_for_install(mut self: core::pin::Pin<&mut Self>) -> bool {
        if crate::proton::installed_tag().is_some()
            && !super::paths::read_flag("proton_auto_update", true)
        {
            return false;
        }
        self.as_mut().rust_mut().resume_installer = true;
        if self.prepare_proton(None) {
            return true;
        }
        false
    }

    pub(crate) fn prepare_proton(
        mut self: core::pin::Pin<&mut Self>,
        launch_after: Option<String>,
    ) -> bool {
        if *self.busy() || *self.preparing() {
            return true;
        }

        let installed = crate::proton::installed_tag();
        let forced = launch_after.is_none() && !self.rust().resume_installer;
        if installed.is_some() && !forced && !super::paths::read_flag("proton_auto_update", true) {
            return false;
        }

        self.as_mut().set_preparing(true);
        self.as_mut().set_status(QString::from(&format!(
            "Checking {}…",
            crate::proton::DISPLAY_NAME
        )));
        let thread = self.qt_thread();

        std::thread::spawn(move || {
            let release = crate::proton::latest_release();
            let up_to_date = match (&release, &installed) {
                (Ok(release), Some(tag)) => &release.tag == tag,
                _ => installed.is_some(),
            };

            if up_to_date {
                let _ = thread.queue(move |mut qobject| {
                    qobject.as_mut().set_preparing(false);
                    qobject.as_mut().set_status(QString::default());
                    qobject.as_mut().set_progress(-1.0);
                    qobject.resume(launch_after);
                });
                return;
            }

            let outcome = release.and_then(|release| {
                let thread = thread.clone();
                crate::proton::download(&release, move |line, fraction| {
                    let line = line.to_string();
                    let _ = thread.queue(move |mut qobject| {
                        qobject.as_mut().set_status(QString::from(&line));
                        qobject.as_mut().set_progress(fraction);
                        qobject.append_log(&line);
                    });
                })
            });

            let message = match outcome {
                Ok(path) => format!("compatibility tool ready: {}", path.display()),
                Err(e) => format!("could not fetch the compatibility tool: {e}"),
            };
            let _ = thread.queue(move |mut qobject| {
                qobject.as_mut().append_log(&message);
                qobject.as_mut().set_preparing(false);
                qobject.as_mut().set_status(QString::default());
                qobject.as_mut().set_progress(-1.0);
                qobject
                    .as_mut()
                    .set_proton_build(QString::from(&super::runner::installed_build()));
                qobject.resume(launch_after);
            });
        });

        true
    }

    pub(crate) fn resume(mut self: core::pin::Pin<&mut Self>, id: Option<String>) {
        if self.rust().resume_installer {
            self.as_mut().rust_mut().resume_installer = false;
            if let Some(entry) = self.rust().pending.clone() {
                self.run_installer_now(entry);
                return;
            }
        }
        let Some(id) = id else {
            return;
        };
        let Some(entry) = self.rust().entry(&id).cloned() else {
            return;
        };
        self.run_now(&entry);
    }

    pub(crate) fn default_proton_name(&self) -> QString {
        QString::from(&default_proton())
    }

    pub(crate) fn env_preview(&self) -> QString {
        let sample = Entry {
            prefix: "<prefix>".to_string(),
            gameid: "<gameid>".to_string(),
            ..Default::default()
        };
        let lines: Vec<String> = env_for(&sample, &self.rust().overrides())
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        QString::from(&lines.join("\n"))
    }
}
