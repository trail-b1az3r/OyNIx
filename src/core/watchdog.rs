//! UI freeze detector. QML calls `beat()` twice a second; a background
//! thread notices when the beats stop (the UI thread is blocked) and, once
//! they resume, records how long it was stuck. The log tells apart freezes
//! that happen while OyNIx is hidden (typically graphics presentation
//! waiting on the compositor) from ones in the foreground (work on the UI
//! thread).
//!
//! `Instant` doesn't advance while the computer sleeps, so suspend/resume
//! isn't mistaken for a freeze.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use super::paths;

/// Freezes shorter than this aren't recorded.
const THRESHOLD: Duration = Duration::from_millis(1500);
const MAX_LINES: usize = 200;

static EPOCH: OnceLock<Instant> = OnceLock::new();
static LAST_BEAT_MS: AtomicU64 = AtomicU64::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(true);

fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Called from the UI thread. `active`: an OyNIx window has focus.
pub fn beat(active: bool) {
    ACTIVE.store(active, Ordering::Relaxed);
    LAST_BEAT_MS.store(now_ms(), Ordering::Relaxed);
}

pub fn log_path() -> std::path::PathBuf {
    paths::cache_dir().join("freezes.log")
}

fn record(line: &str) {
    eprintln!("oynix: {line}");
    let path = log_path();
    let mut lines: Vec<String> = std::fs::read_to_string(&path).unwrap_or_default().lines().map(String::from).collect();
    lines.push(line.to_string());
    let start = lines.len().saturating_sub(MAX_LINES);
    if let Ok(mut f) = std::fs::File::create(&path) {
        for l in &lines[start..] {
            let _ = writeln!(f, "{l}");
        }
    }
}

pub fn start(graphics: String) {
    beat(true);
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some() && !std::env::var("QT_QPA_PLATFORM").unwrap_or_default().starts_with("xcb");
    let session = if wayland { "Wayland" } else { "X11" };
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let _ = std::thread::Builder::new().name("oynix-watchdog".into()).spawn(move || {
        let mut stalled_since: Option<(u64, bool)> = None;
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let last = LAST_BEAT_MS.load(Ordering::Relaxed);
            let now = now_ms();
            match stalled_since {
                None if now.saturating_sub(last) > THRESHOLD.as_millis() as u64 => {
                    stalled_since = Some((last, ACTIVE.load(Ordering::Relaxed)));
                }
                Some((from, was_active)) if last > from => {
                    let secs = (last - from) as f64 / 1000.0;
                    let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    record(&format!(
                        "{when} UI unresponsive for {secs:.1}s · {} · graphics {graphics} · {session} {desktop}",
                        if was_active { "window focused" } else { "window in background" }
                    ));
                    stalled_since = None;
                }
                _ => {}
            }
        }
    });
}
