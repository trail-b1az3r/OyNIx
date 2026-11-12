//! XDG base-directory helpers. Every profile gets its own sub-directory so
//! profiles are fully isolated (cookies, history, bookmarks, settings).

use std::path::PathBuf;
use std::sync::OnceLock;

static PROFILE: OnceLock<String> = OnceLock::new();

/// Select the active profile. Must be called once, before any path lookup.
pub fn set_profile(name: &str) {
    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let _ = PROFILE.set(if clean.is_empty() { "default".into() } else { clean });
}

pub fn profile() -> &'static str {
    PROFILE.get().map(String::as_str).unwrap_or("default")
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    match std::env::var_os(var) {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => home().join(fallback),
    }
}

fn ensure(p: PathBuf) -> PathBuf {
    let _ = std::fs::create_dir_all(&p);
    p
}

/// `~/.config/oynix/<profile>`
pub fn config_dir() -> PathBuf {
    ensure(xdg("XDG_CONFIG_HOME", ".config").join("oynix").join(profile()))
}

/// `~/.local/share/oynix/<profile>`
pub fn data_dir() -> PathBuf {
    ensure(xdg("XDG_DATA_HOME", ".local/share").join("oynix").join(profile()))
}

/// `~/.cache/oynix/<profile>`
pub fn cache_dir() -> PathBuf {
    ensure(xdg("XDG_CACHE_HOME", ".cache").join("oynix").join(profile()))
}

/// User supplied content-blocker lists (EasyList / uBlock syntax subset).
pub fn filters_dir() -> PathBuf {
    ensure(config_dir().join("filters"))
}

/// User scripts (`*.user.js`) and user styles (`*.user.css`).
pub fn userscripts_dir() -> PathBuf {
    ensure(config_dir().join("userscripts"))
}

pub fn downloads_dir() -> PathBuf {
    let d = xdg("XDG_DOWNLOAD_DIR", "Downloads");
    ensure(d)
}

/// All profiles that exist on disk.
pub fn list_profiles() -> Vec<String> {
    let root = xdg("XDG_CONFIG_HOME", ".config").join("oynix");
    let mut out: Vec<String> = std::fs::read_dir(root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    if !out.iter().any(|p| p == "default") {
        out.push("default".into());
    }
    out.sort();
    out
}

/// Atomically write a file (write to temp + rename) so a crash never leaves
/// a half-written session or settings file behind.
pub fn write_atomic(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, path)
}
