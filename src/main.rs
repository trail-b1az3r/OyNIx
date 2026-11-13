//! OyNIx — a native, privacy-first desktop browser.
//!
//! Architecture
//! ------------
//! * `core/`    – pure Rust: settings, history/bookmarks, address-bar logic,
//!                content blocker, injected scripts.
//! * `bridge/`  – CXX-Qt QObjects exposing the core to QML.
//! * `native`   – tiny C++ layer for things Qt only offers in C++
//!                (graphics API, WebEngine init, request interceptor).
//! * `gpu`      – Vulkan 1.2/1.3 probe (ash) and backend selection.
//! * `qml/`     – the whole user interface, rendered by Qt Quick on
//!                Vulkan or OpenGL ES.

mod bridge {
    pub mod core;
    pub mod extbridge;
    pub mod native;
}
mod core;
mod gpu;

use bridge::native;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};
use std::sync::OnceLock;

/// Release version: `OYNIX_VERSION` at build time (set by the packaging
/// scripts / CI when a version is given), else the Cargo version.
pub const VERSION: &str = match option_env!("OYNIX_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

pub struct LaunchEnv {
    pub graphics: String,
    pub qt_version: String,
    pub start_url: String,
    pub private: bool,
    pub gpu_arg: Option<String>,
}

static LAUNCH: OnceLock<LaunchEnv> = OnceLock::new();

pub fn launch_env() -> &'static LaunchEnv {
    LAUNCH.get_or_init(|| LaunchEnv {
        graphics: String::new(),
        qt_version: String::new(),
        start_url: String::new(),
        private: false,
        gpu_arg: None,
    })
}

const HELP: &str = "\
OyNIx — fast, private web browser

USAGE:
    oynix [OPTIONS] [URL]

OPTIONS:
    --profile <NAME>     Use (or create) an isolated profile
    --private            Open a private window (nothing is written to disk)
    --gpu <BACKEND>      vulkan | gles | opengl | software   (default: auto)
    --wayland | --x11    Force the windowing platform
    --install-extension <PATH>
                         Install a .crx / .xpi / .zip / folder (Chrome, Firefox
                         or OyNIx format) into the profile and exit
    --list-extensions    Print installed extensions and exit
    --list-profiles      Print existing profiles and exit
    --gpu-info           Print the detected Vulkan device and exit
    -h, --help           Show this help
    -V, --version        Show version
";

struct Args {
    install: Vec<String>,
    list_extensions: bool,
    profile: String,
    private: bool,
    gpu: Option<String>,
    url: String,
}

fn parse_args() -> Args {
    let mut a = Args { install: vec![], list_extensions: false, profile: "default".into(), private: false, gpu: None, url: String::new() };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("OyNIx {VERSION}");
                std::process::exit(0);
            }
            "--profile" => a.profile = it.next().unwrap_or_else(|| "default".into()),
            "--private" | "--incognito" => a.private = true,
            "--install-extension" => a.install.extend(it.next()),
            "--list-extensions" => a.list_extensions = true,
            "--gpu" => a.gpu = it.next(),
            "--wayland" => std::env::set_var("QT_QPA_PLATFORM", "wayland;xcb"),
            "--x11" => std::env::set_var("QT_QPA_PLATFORM", "xcb"),
            "--list-profiles" => {
                for p in core::paths::list_profiles() {
                    println!("{p}");
                }
                std::process::exit(0);
            }
            "--gpu-info" => {
                match gpu::probe_vulkan() {
                    Some(v) => println!("Vulkan {} on {}{}", v.version(), v.device, if v.driver_is_cpu { " (CPU)" } else { "" }),
                    None => println!("No Vulkan 1.2+ device found; OyNIx will use OpenGL ES"),
                }
                std::process::exit(0);
            }
            s if s.starts_with("--profile=") => a.profile = s["--profile=".len()..].to_string(),
            s if s.starts_with("--gpu=") => a.gpu = Some(s["--gpu=".len()..].to_string()),
            // Unknown flags (e.g. Chromium switches) are passed through to Qt.
            s if s.starts_with('-') => {}
            s => a.url = s.to_string(),
        }
    }
    a
}

fn main() {
    let args = parse_args();
    core::paths::set_profile(&args.profile);

    if !args.install.is_empty() || args.list_extensions {
        let mut list = core::extensions::load();
        let mut failed = false;
        for p in &args.install {
            match core::extensions::install(std::path::Path::new(p), &mut list) {
                Ok(e) => {
                    println!("installed {} {} ({}{})", e.name, e.version, e.kind, if e.manifest_version > 0 { format!(", MV{}", e.manifest_version) } else { String::new() });
                    for w in &e.warnings {
                        println!("  warning: {w}");
                    }
                }
                Err(err) => {
                    eprintln!("{p}: {err}");
                    failed = true;
                }
            }
        }
        if args.list_extensions {
            for e in &list {
                println!("{:<28} {:<8} {:<8} {}  {}", e.id, e.version, e.kind, if e.enabled { "on " } else { "off" }, e.path);
            }
        }
        std::process::exit(if failed { 1 } else { 0 });
    }

    let requested = args
        .gpu
        .clone()
        .or_else(|| std::env::var("OYNIX_GPU").ok())
        .and_then(|s| gpu::Backend::parse(&s));
    let selection = gpu::select(requested);

    // Chromium switches that depend on user settings must be set before
    // Qt WebEngine initialises.
    let early = core::settings::Settings::load();
    let mut chromium = std::env::var("QTWEBENGINE_CHROMIUM_FLAGS").unwrap_or_default();
    chromium.push_str(if early.smooth_scrolling { " --enable-smooth-scrolling" } else { " --disable-smooth-scrolling" });
    if selection.backend == gpu::Backend::Vulkan {
        chromium.push_str(" --enable-features=Vulkan");
    }
    std::env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", chromium.trim());
    native::ffi::init_webengine();
    native::ffi::prepare_graphics(selection.backend.id());

    let mut app = QGuiApplication::new();
    native::ffi::set_app_metadata(&args.profile);

    // Content blocker + privacy defaults before any page loads.
    core::blocker::init();
    let settings = core::settings::Settings::load();
    bridge::core::apply_privacy(&settings);


    let start_url = if args.url.is_empty() {
        String::new()
    } else {
        core::omnibox::resolve(&args.url, &settings.search_engine, settings.bangs_enabled, settings.https_upgrade)
    };
    let _ = LAUNCH.set(LaunchEnv {
        graphics: format!("{} — {}", native::ffi::active_graphics_api(), selection.detail),
        qt_version: native::ffi::qt_runtime_version(),
        start_url,
        private: args.private,
        gpu_arg: args.gpu.clone(),
    });

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/OyNIx/qml/Main.qml"));
    }

    let code = app.as_mut().map(|a| a.exec()).unwrap_or(1);
    core::blocker::save_stats();
    std::process::exit(code);
}
