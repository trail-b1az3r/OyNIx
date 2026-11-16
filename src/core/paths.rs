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

/// Is there a default route (IPv4 or IPv6)? Without one the computer is
/// offline, whatever an individual site's error says. Linux only; true
/// when the routing tables can't be read.
pub fn has_default_route() -> bool {
    // Testing hook: pretend the computer is offline.
    if std::env::var("OYNIX_OFFLINE").is_ok_and(|v| v == "1") {
        return false;
    }
    let v4 = std::fs::read_to_string("/proc/net/route");
    let v6 = std::fs::read_to_string("/proc/net/ipv6_route");
    if v4.is_err() && v6.is_err() {
        return true;
    }
    let v4_default = v4.is_ok_and(|t| {
        t.lines().skip(1).any(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // Iface Destination Gateway Flags …; flag 0x1 = route is up.
            f.len() > 3 && f[0] != "lo" && f[1] == "00000000" && u32::from_str_radix(f[3], 16).is_ok_and(|fl| fl & 1 == 1)
        })
    });
    let v6_default = v6.is_ok_and(|t| {
        t.lines().any(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            // dest/len … ifname last; ::/0 is the default route.
            f.len() >= 10 && f[0] == "00000000000000000000000000000000" && f[1] == "00" && f[9] != "lo"
        })
    });
    v4_default || v6_default
}
