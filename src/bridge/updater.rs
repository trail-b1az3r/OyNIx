//! `Updater` — the built-in updater as seen from QML: checks for a new
//! release, downloads and verifies it in the background, installs it with
//! the system package manager and restarts OyNIx. See core/updater.rs.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// idle | checking | current | available | downloading | ready | installing | installed | error
        #[qproperty(QString, state)]
        /// The newer release as JSON ({version, title, notes, page, published, package}), or "".
        #[qproperty(QString, release)]
        #[qproperty(f64, progress)]
        #[qproperty(QString, error)]
        /// deb | pacman | manual
        #[qproperty(QString, kind)]
        #[qproperty(QString, current_version)]
        /// For copying into a terminal when pkexec is missing or the install failed.
        #[qproperty(QString, manual_command)]
        type Updater = super::UpdaterRust;

        /// Ask for a new release. `quiet`: a scheduled check (errors aren't shown).
        #[qinvokable]
        fn check(self: Pin<&mut Updater>, include_prerelease: bool, quiet: bool);
        /// Seconds since the last successful check.
        #[qinvokable]
        fn seconds_since_check(self: &Updater) -> f64;
        #[qinvokable]
        fn download(self: Pin<&mut Updater>);
        #[qinvokable]
        fn install(self: Pin<&mut Updater>);
        #[qinvokable]
        fn cancel(self: Pin<&mut Updater>);
        /// Start the updated OyNIx (restoring this session); QML then quits.
        #[qinvokable]
        fn relaunch(self: &Updater) -> bool;
    }

    impl cxx_qt::Threading for Updater {}
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::core::updater::{self, Kind, Release};

pub struct UpdaterRust {
    state: QString,
    release: QString,
    progress: f64,
    error: QString,
    kind: QString,
    current_version: QString,
    manual_command: QString,
    found: Option<Release>,
    file: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    busy: bool,
}

impl Default for UpdaterRust {
    fn default() -> Self {
        Self {
            state: QString::from("idle"),
            release: QString::default(),
            progress: 0.0,
            error: QString::default(),
            kind: QString::from(updater::install_kind().name()),
            current_version: QString::from(crate::VERSION),
            manual_command: QString::default(),
            found: None,
            file: None,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: false,
        }
    }
}

fn kind() -> Kind {
    updater::install_kind()
}

impl qobject::Updater {
    fn fail(mut self: Pin<&mut Self>, msg: String) {
        self.as_mut().rust_mut().busy = false;
        self.as_mut().set_error(QString::from(&msg));
        self.set_state(QString::from("error"));
    }

    pub fn check(mut self: Pin<&mut Self>, include_prerelease: bool, quiet: bool) {
        if self.busy {
            return;
        }
        self.as_mut().rust_mut().busy = true;
        self.as_mut().set_error(QString::default());
        let before = self.state().clone();
        self.as_mut().set_state(QString::from("checking"));
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let r = updater::check(include_prerelease);
            let _ = thread.queue(move |mut o: Pin<&mut qobject::Updater>| {
                o.as_mut().rust_mut().busy = false;
                match r {
                    Ok(Some(rel)) => {
                        updater::mark_checked();
                        let same = o.found.as_ref().is_some_and(|f| f.version == rel.version);
                        o.as_mut().set_release(QString::from(&rel.to_json(kind()).to_string()));
                        o.as_mut().rust_mut().found = Some(rel);
                        // Keep a finished download of the same version.
                        if same && o.file.is_some() {
                            o.set_state(QString::from("ready"));
                        } else {
                            o.as_mut().rust_mut().file = None;
                            o.set_state(QString::from("available"));
                        }
                    }
                    Ok(None) => {
                        updater::mark_checked();
                        o.as_mut().rust_mut().found = None;
                        o.as_mut().set_release(QString::default());
                        o.set_state(QString::from("current"));
                    }
                    Err(e) if quiet => {
                        eprintln!("oynix: update check: {e}");
                        o.set_state(before);
                    }
                    Err(e) => o.fail(e),
                }
            });
        });
    }

    pub fn seconds_since_check(&self) -> f64 {
        updater::since_last_check() as f64
    }

    pub fn download(mut self: Pin<&mut Self>) {
        let Some(rel) = self.found.clone() else { return };
        if self.busy {
            return;
        }
        if rel.package_for(kind()).is_none() {
            return self.fail("This release has no package for your system. Download it from the release page.".into());
        }
        self.as_mut().rust_mut().busy = true;
        let cancel = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().cancel = cancel.clone();
        self.as_mut().set_progress(0.0);
        self.as_mut().set_error(QString::default());
        self.as_mut().set_state(QString::from("downloading"));
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let mut last = -1i64;
            let t2 = thread.clone();
            let r = updater::download(&rel, kind(), &cancel, |got, total| {
                let pct = if total > 0 { (got * 1000 / total) as i64 } else { 0 };
                if pct != last {
                    last = pct;
                    let p = pct as f64 / 1000.0;
                    let _ = t2.queue(move |o: Pin<&mut qobject::Updater>| o.set_progress(p));
                }
            });
            let _ = thread.queue(move |mut o: Pin<&mut qobject::Updater>| {
                o.as_mut().rust_mut().busy = false;
                match r {
                    Ok(file) => {
                        let cmd = updater::manual_command(kind(), &file);
                        o.as_mut().set_manual_command(QString::from(&cmd));
                        o.as_mut().rust_mut().file = Some(file);
                        o.as_mut().set_progress(1.0);
                        o.set_state(QString::from("ready"));
                    }
                    Err(e) if e == "Cancelled" => o.set_state(QString::from("available")),
                    Err(e) => o.fail(e),
                }
            });
        });
    }

    pub fn install(mut self: Pin<&mut Self>) {
        let Some(file) = self.file.clone() else { return };
        if self.busy {
            return;
        }
        self.as_mut().rust_mut().busy = true;
        self.as_mut().set_error(QString::default());
        self.as_mut().set_state(QString::from("installing"));
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let r = updater::install(kind(), &file);
            let _ = thread.queue(move |mut o: Pin<&mut qobject::Updater>| {
                o.as_mut().rust_mut().busy = false;
                match r {
                    Ok(()) => {
                        let _ = std::fs::remove_dir_all(updater::download_dir());
                        o.as_mut().rust_mut().file = None;
                        o.set_state(QString::from("installed"));
                    }
                    Err(e) => {
                        // The download is still good; let the user try again.
                        o.as_mut().set_error(QString::from(&e));
                        o.set_state(QString::from("ready"));
                    }
                }
            });
        });
    }

    pub fn cancel(self: Pin<&mut Self>) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn relaunch(&self) -> bool {
        // After a package upgrade the running executable has been replaced;
        // start the installed one.
        let exe = ["/usr/bin/oynix"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.exists() && kind() != Kind::Manual)
            .or_else(|| std::env::current_exe().ok().map(|p| PathBuf::from(p.to_string_lossy().trim_end_matches(" (deleted)"))));
        let Some(exe) = exe else { return false };
        std::process::Command::new(exe)
            .args(["--profile", crate::core::paths::profile(), "--restore-session", "--wait-for-pid", &std::process::id().to_string()])
            .spawn()
            .is_ok()
    }
}
