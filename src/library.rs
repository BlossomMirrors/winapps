use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

mod classify;
mod database;
mod entry;
mod icon;
mod imports;
mod ops;
mod paths;
mod prefix;
mod runner;

use entry::{Entry, Staged, load_entries};
use paths::{default_prefix_root, save_setting};
use runner::installed_build;
use runner::{host_command, sandboxed};

pub use runner::launch_headless;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(QString, entries_json, cxx_name = "entriesJson")]
        #[qproperty(QString, staged_json, cxx_name = "stagedJson")]
        #[qproperty(QString, log)]
        #[qproperty(QString, status)]
        #[qproperty(QString, prefix_root, cxx_name = "prefixRoot")]
        #[qproperty(bool, busy)]
        #[qproperty(bool, preparing)]
        #[qproperty(f64, progress)]
        #[qproperty(QString, proton_build, cxx_name = "protonBuild")]
        #[qproperty(bool, auto_update, cxx_name = "autoUpdate")]
        #[qproperty(bool, wayland)]
        #[qproperty(bool, follow_theme, cxx_name = "followTheme")]
        #[qproperty(QString, primary_monitor, cxx_name = "primaryMonitor")]
        #[qproperty(QString, dpi_override, cxx_name = "dpiOverride")]
        #[qproperty(QString, umu_log, cxx_name = "umuLog")]
        #[qproperty(QString, proton_verb, cxx_name = "protonVerb")]
        #[qproperty(QString, proton_path, cxx_name = "protonPath")]
        #[qproperty(bool, no_runtime, cxx_name = "noRuntime")]
        #[qproperty(bool, skip_runtime_update, cxx_name = "skipRuntimeUpdate")]
        #[qproperty(bool, no_proton, cxx_name = "noProton")]
        type Library = super::LibraryRust;

        #[qinvokable]
        fn prepare(self: Pin<&mut Library>, file: &QString);

        #[qinvokable]
        #[cxx_name = "commitPortable"]
        fn commit_portable(
            self: Pin<&mut Library>,
            name: &QString,
            description: &QString,
            categories: &QString,
            store: &QString,
            shortcut: bool,
        );

        #[qinvokable]
        #[cxx_name = "commitInstaller"]
        fn commit_installer(
            self: Pin<&mut Library>,
            name: &QString,
            root: &QString,
            description: &QString,
            categories: &QString,
            store: &QString,
            shortcut: bool,
        );

        #[qinvokable]
        #[cxx_name = "discardStaged"]
        fn discard_staged(self: Pin<&mut Library>);

        #[qinvokable]
        fn cancel(self: Pin<&mut Library>);

        #[qinvokable]
        fn launch(self: Pin<&mut Library>, id: &QString);

        #[qinvokable]
        #[cxx_name = "runInstaller"]
        fn run_installer(self: Pin<&mut Library>, id: &QString);

        #[qinvokable]
        fn winetricks(self: Pin<&mut Library>, id: &QString);

        #[qinvokable]
        #[cxx_name = "openPrefix"]
        fn open_prefix(self: Pin<&mut Library>, id: &QString);

        #[qinvokable]
        fn candidates(self: Pin<&mut Library>, id: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "setExe"]
        fn set_exe(self: Pin<&mut Library>, id: &QString, exe: &QString);

        #[qinvokable]
        #[cxx_name = "searchDatabase"]
        fn search_database(self: &Library, needle: &QString) -> QString;

        #[qinvokable]
        #[cxx_name = "setDetails"]
        fn set_details(
            self: Pin<&mut Library>,
            id: &QString,
            description: &QString,
            categories: &QString,
        );

        #[qinvokable]
        #[cxx_name = "setShortcut"]
        fn set_shortcut(self: Pin<&mut Library>, id: &QString, wanted: bool);

        #[qinvokable]
        fn uninstall(self: Pin<&mut Library>, id: &QString);

        #[qinvokable]
        #[cxx_name = "clearLog"]
        fn clear_log(self: Pin<&mut Library>);

        #[qinvokable]
        #[cxx_name = "ensureProton"]
        fn ensure_proton(self: Pin<&mut Library>);

        #[qinvokable]
        #[cxx_name = "defaultProton"]
        fn default_proton_name(self: &Library) -> QString;

        #[qinvokable]
        #[cxx_name = "envPreview"]
        fn env_preview(self: &Library) -> QString;
    }

    impl cxx_qt::Threading for Library {}
    impl cxx_qt::Initialize for Library {}
}

pub struct LibraryRust {
    entries_json: QString,
    staged_json: QString,
    log: QString,
    status: QString,
    prefix_root: QString,
    busy: bool,
    preparing: bool,
    progress: f64,
    proton_build: QString,
    auto_update: bool,
    wayland: bool,
    follow_theme: bool,
    primary_monitor: QString,
    dpi_override: QString,
    umu_log: QString,
    proton_verb: QString,
    proton_path: QString,
    no_runtime: bool,
    skip_runtime_update: bool,
    no_proton: bool,
    entries: Vec<Entry>,
    staged: Option<Staged>,
    pending: Option<Entry>,
    pending_before: Vec<String>,
    pending_shortcut: bool,
    resume_installer: bool,
    running_pgid: Arc<AtomicI32>,
    watching: Arc<AtomicBool>,
}

impl Default for LibraryRust {
    fn default() -> Self {
        Self {
            entries_json: QString::from("[]"),
            staged_json: QString::from("null"),
            log: QString::default(),
            status: QString::default(),
            prefix_root: QString::from(&default_prefix_root()),
            busy: false,
            preparing: false,
            progress: -1.0,
            proton_build: QString::from(&installed_build()),
            auto_update: paths::read_flag("proton_auto_update", true),
            wayland: paths::read_flag("proton_wayland", true),
            follow_theme: paths::read_flag("follow_system_theme", true),
            primary_monitor: QString::from(&paths::read_setting("primary_monitor").unwrap_or_default()),
            dpi_override: QString::from(&paths::read_setting("dpi_override").unwrap_or_default()),
            umu_log: QString::default(),
            proton_verb: QString::from("waitforexitandrun"),
            proton_path: QString::default(),
            no_runtime: false,
            skip_runtime_update: false,
            no_proton: false,
            entries: Vec::new(),
            staged: None,
            pending: None,
            pending_before: Vec::new(),
            pending_shortcut: false,
            resume_installer: false,
            running_pgid: Arc::new(AtomicI32::new(0)),
            watching: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl LibraryRust {
    fn entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn overrides(&self) -> Vec<(String, String)> {
        let mut env = Vec::new();
        let umu_log = self.umu_log.to_string();
        if !umu_log.is_empty() {
            env.push(("UMU_LOG".to_string(), umu_log));
        }
        let verb = self.proton_verb.to_string();
        if !verb.is_empty() {
            env.push(("PROTON_VERB".to_string(), verb));
        }
        let proton = self.proton_path.to_string();
        if !proton.is_empty() {
            env.push(("PROTONPATH".to_string(), proton));
        }
        if self.no_runtime {
            env.push(("UMU_NO_RUNTIME".to_string(), "1".to_string()));
        }
        if self.skip_runtime_update {
            env.push(("UMU_RUNTIME_UPDATE".to_string(), "0".to_string()));
        }
        if self.no_proton {
            env.push(("UMU_NO_PROTON".to_string(), "1".to_string()));
        }
        env
    }
}

impl cxx_qt::Initialize for qobject::Library {
    fn initialize(mut self: core::pin::Pin<&mut Self>) {
        let entries = load_entries();
        self.as_mut().rust_mut().entries = entries;
        self.as_mut()
            .on_prefix_root_changed(|qobject| {
                save_setting("prefix_root", &qobject.prefix_root().to_string());
            })
            .release();
        self.as_mut()
            .on_auto_update_changed(|qobject| {
                paths::save_flag("proton_auto_update", *qobject.auto_update());
            })
            .release();
        self.as_mut()
            .on_wayland_changed(|qobject| {
                paths::save_flag("proton_wayland", *qobject.wayland());
            })
            .release();
        self.as_mut()
            .on_follow_theme_changed(|qobject| {
                paths::save_flag("follow_system_theme", *qobject.follow_theme());
            })
            .release();
        self.as_mut()
            .on_primary_monitor_changed(|qobject| {
                save_setting("primary_monitor", &qobject.primary_monitor().to_string());
            })
            .release();
        self.as_mut()
            .on_dpi_override_changed(|qobject| {
                save_setting("dpi_override", &qobject.dpi_override().to_string());
            })
            .release();
        self.sync_entries();
    }
}

impl qobject::Library {
    fn sync_entries(mut self: core::pin::Pin<&mut Self>) {
        let json = serde_json::to_string(&self.rust().entries).unwrap_or_else(|_| "[]".to_string());
        self.as_mut().set_entries_json(QString::from(&json));
    }

    fn sync_staged(mut self: core::pin::Pin<&mut Self>) {
        let json = self
            .rust()
            .staged
            .as_ref()
            .and_then(|s| serde_json::to_string(s).ok())
            .unwrap_or_else(|| "null".to_string());
        self.as_mut().set_staged_json(QString::from(&json));
    }

    fn append_log(mut self: core::pin::Pin<&mut Self>, line: &str) {
        let mut text = self.rust().log.to_string();
        text.push_str(line);
        text.push('\n');
        if text.len() > 200_000 {
            let cut = text.len() - 150_000;
            text = text[cut..].to_string();
        }
        self.as_mut().set_log(QString::from(&text));
    }

    fn spawn(
        mut self: core::pin::Pin<&mut Self>,
        program: String,
        args: Vec<String>,
        env: Vec<(String, String)>,
        status: String,
    ) {
        if *self.busy() {
            self.as_mut()
                .append_log("busy: another run is still in progress");
            return;
        }
        self.as_mut().set_busy(true);
        self.as_mut().set_status(QString::from(&status));

        let thread = self.qt_thread();
        let pgid = Arc::clone(&self.rust().running_pgid);
        let shown: Vec<String> = env
            .iter()
            .map(|(k, v)| {
                if v.contains(' ') {
                    format!("{k}=\"{v}\"")
                } else {
                    format!("{k}={v}")
                }
            })
            .collect();
        self.as_mut().append_log(&format!(
            "$ {} {} {}",
            shown.join(" "),
            program,
            args.join(" ")
        ));

        std::thread::spawn(move || {
            let mut cmd = host_command(&program);
            if sandboxed() {
                for (key, value) in &env {
                    cmd.arg(format!("--env={key}={value}"));
                }
            } else {
                cmd.envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
            }
            cmd.process_group(0)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => {
                    pgid.store(c.id() as i32, Ordering::SeqCst);
                    c
                }
                Err(e) => {
                    let msg = format!("failed to start {program}: {e}");
                    let _ = thread.queue(move |mut qobject| {
                        qobject.as_mut().append_log(&msg);
                        qobject.as_mut().set_busy(false);
                        qobject.set_status(QString::default());
                    });
                    return;
                }
            };

            let mut pipes: Vec<Box<dyn Read + Send>> = Vec::new();
            if let Some(out) = child.stdout.take() {
                pipes.push(Box::new(out));
            }
            if let Some(err) = child.stderr.take() {
                pipes.push(Box::new(err));
            }

            for pipe in pipes {
                let thread = thread.clone();
                std::thread::spawn(move || {
                    for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                        let _ = thread.queue(move |qobject| qobject.append_log(&line));
                    }
                });
            }

            let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
            pgid.store(0, Ordering::SeqCst);
            let _ = thread.queue(move |mut qobject| {
                qobject.as_mut().append_log(&format!("exit {code}"));
                qobject.as_mut().set_busy(false);
                qobject.as_mut().set_status(QString::default());
                qobject.finish_install(code);
            });
        });
    }

    /// launch an already-installed program without holding the shared "busy"
    pub(crate) fn spawn_detached(
        self: core::pin::Pin<&mut Self>,
        args: Vec<String>,
        env: Vec<(String, String)>,
        label: String,
    ) {
        let thread = self.qt_thread();
        let shown: Vec<String> = env
            .iter()
            .map(|(k, v)| {
                if v.contains(' ') {
                    format!("{k}=\"{v}\"")
                } else {
                    format!("{k}={v}")
                }
            })
            .collect();
        let cmdline = format!("$ {} umu-run {}", shown.join(" "), args.join(" "));
        let _ = thread.queue(move |qobject| qobject.append_log(&cmdline));

        let thread = thread.clone();
        std::thread::spawn(move || {
            let mut cmd = host_command("umu-run");
            if sandboxed() {
                for (key, value) in &env {
                    cmd.arg(format!("--env={key}={value}"));
                }
            } else {
                cmd.envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
            }
            cmd.process_group(0)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let msg = format!("failed to start {label}: {e}");
                    let _ = thread.queue(move |qobject| qobject.append_log(&msg));
                    return;
                }
            };

            let mut pipes: Vec<Box<dyn Read + Send>> = Vec::new();
            if let Some(out) = child.stdout.take() {
                pipes.push(Box::new(out));
            }
            if let Some(err) = child.stderr.take() {
                pipes.push(Box::new(err));
            }
            for pipe in pipes {
                let thread = thread.clone();
                std::thread::spawn(move || {
                    for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                        let _ = thread.queue(move |qobject| qobject.append_log(&line));
                    }
                });
            }

            let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(-1);
            let _ = thread.queue(move |qobject| {
                qobject.append_log(&format!("{label} exited with {code}"));
            });
        });
    }

    fn cancel(mut self: core::pin::Pin<&mut Self>) {
        let pgid = self.rust().running_pgid.load(Ordering::SeqCst);
        if pgid <= 0 {
            return;
        }
        self.as_mut().append_log("cancelled");
        unsafe {
            libc::kill(-pgid, libc::SIGTERM);
        }
    }
}
