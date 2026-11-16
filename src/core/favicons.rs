//! A small favicon cache, one file per site.
//!
//! Qt WebEngine only hands out icons for pages open right now, so
//! bookmarks, history and tabs that haven't loaded yet would show letter
//! tiles. When a tab shows an icon, OyNIx downloads the same icon file once
//! (refreshed weekly) into `~/.cache/oynix/<profile>/favicons/`, and
//! `Favicon.qml` falls back to it. Private windows never add to it.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const MAX_BYTES: u64 = 256 * 1024;
const REFRESH: Duration = Duration::from_secs(7 * 24 * 3600);

fn dir() -> PathBuf {
    super::paths::cache_dir().join("favicons")
}

/// Cache key for a page URL: its host without "www.", made file-name safe.
fn key(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    let host = u.host_str()?.trim_start_matches("www.").to_ascii_lowercase();
    let safe: String = host.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')).collect();
    (!safe.is_empty() && !safe.starts_with('.')).then_some(safe)
}

/// "en.wikipedia.org" -> ["en.wikipedia.org", "wikipedia.org"].
fn and_parents(k: &str) -> Vec<String> {
    let parts: Vec<&str> = k.split('.').collect();
    (0..parts.len().saturating_sub(1).max(1)).map(|i| parts[i..].join(".")).collect()
}

/// The cached icon for the site of `url` (or a parent domain), if any.
pub fn cached(url: &str) -> Option<PathBuf> {
    and_parents(&key(url)?).into_iter().map(|k| dir().join(k)).find(|p| p.is_file())
}

fn looks_like_image(b: &[u8]) -> bool {
    b.starts_with(b"\x89PNG")
        || b.starts_with(b"GIF8")
        || b.starts_with(b"\xff\xd8\xff")
        || b.starts_with(&[0, 0, 1, 0])
        || (b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP"))
        || {
            let head = String::from_utf8_lossy(&b[..b.len().min(512)]).to_ascii_lowercase();
            head.contains("<svg")
        }
}

/// Save the icon a page shows (`icon` as Qt reports it, usually
/// `image://favicon/https://…/favicon.ico`). Runs in the background.
pub fn remember(page_url: &str, icon: &str) {
    let Some(k) = key(page_url) else { return };
    let icon = icon.strip_prefix("image://favicon/").unwrap_or(icon).to_string();
    if !(icon.starts_with("https://") || icon.starts_with("http://")) {
        return;
    }
    let dest = dir().join(&k);
    let fresh = std::fs::metadata(&dest)
        .and_then(|m| m.modified())
        .is_ok_and(|t| SystemTime::now().duration_since(t).unwrap_or_default() < REFRESH);
    if fresh {
        return;
    }
    std::thread::Builder::new()
        .name("oynix-favicon".into())
        .spawn(move || {
            let Ok(mut resp) = super::updater::agent().get(&icon).call() else { return };
            let Ok(bytes) = resp.body_mut().with_config().limit(MAX_BYTES).read_to_vec() else { return };
            if bytes.is_empty() || !looks_like_image(&bytes) {
                return;
            }
            let _ = std::fs::create_dir_all(dir());
            let _ = super::paths::write_atomic(&dest, &bytes);
            // Also stand in for the parent domain (a bookmark to
            // wikipedia.org gets en.wikipedia.org's icon) until it has its own.
            for parent in and_parents(&k).into_iter().skip(1) {
                let p = dir().join(parent);
                if !p.exists() {
                    let _ = super::paths::write_atomic(&p, &bytes);
                }
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys() {
        assert_eq!(super::key("https://www.GitHub.com/x?y").as_deref(), Some("github.com"));
        assert_eq!(super::key("oynix://start"), None);
        assert_eq!(super::key("file:///tmp/a"), None);
        assert_eq!(super::and_parents("en.wikipedia.org"), ["en.wikipedia.org", "wikipedia.org"]);
        assert_eq!(super::and_parents("localhost"), ["localhost"]);
        assert!(super::looks_like_image(b"\x89PNG\r\n"));
        assert!(super::looks_like_image(b"<?xml version='1.0'?><svg xmlns="));
        assert!(!super::looks_like_image(b"<html>not found</html>"));
    }
}
