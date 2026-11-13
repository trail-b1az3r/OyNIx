//! Built-in content-blocker subscriptions.
//!
//! OyNIx ships a catalogue of the standard community filter lists (the same
//! families mainstream content blockers use) and downloads them on first
//! run, then refreshes them periodically. Nothing is bundled in the binary:
//! the lists are fetched from their maintainers and stored per profile in
//! `~/.local/share/oynix/<profile>/filter-lists/`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{library::now, paths};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Ads,
    Privacy,
    Annoyances,
    Cookies,
    Social,
    Security,
    Custom,
}

impl Category {
    pub fn id(self) -> &'static str {
        match self {
            Self::Ads => "ads",
            Self::Privacy => "privacy",
            Self::Annoyances => "annoyances",
            Self::Cookies => "cookies",
            Self::Social => "social",
            Self::Security => "security",
            Self::Custom => "custom",
        }
    }
    /// Blocks from privacy / social / security lists are counted as trackers.
    pub fn is_tracking(self) -> bool {
        matches!(self, Self::Privacy | Self::Social | Self::Security)
    }
}

pub struct List {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub url: &'static str,
    pub homepage: &'static str,
    pub default_on: bool,
    pub description: &'static str,
}

pub const CATALOG: &[List] = &[
    List { id: "easylist", name: "EasyList", category: Category::Ads, default_on: true,
        url: "https://easylist.to/easylist/easylist.txt", homepage: "https://easylist.to/",
        description: "The primary list for removing adverts from international websites." },
    List { id: "easyprivacy", name: "EasyPrivacy", category: Category::Privacy, default_on: true,
        url: "https://easylist.to/easylist/easyprivacy.txt", homepage: "https://easylist.to/",
        description: "Removes tracking: web bugs, tracking scripts and information collectors." },
    List { id: "ublock-filters", name: "uBlock filters", category: Category::Ads, default_on: true,
        url: "https://ublockorigin.github.io/uAssets/filters/filters.txt", homepage: "https://github.com/uBlockOrigin/uAssets",
        description: "Complements EasyList with fixes for ad and anti-adblock breakage." },
    List { id: "ublock-privacy", name: "uBlock filters – Privacy", category: Category::Privacy, default_on: true,
        url: "https://ublockorigin.github.io/uAssets/filters/privacy.txt", homepage: "https://github.com/uBlockOrigin/uAssets",
        description: "Additional tracker and telemetry blocking." },
    List { id: "peter-lowe", name: "Peter Lowe’s Ad and tracking server list", category: Category::Privacy, default_on: true,
        url: "https://pgl.yoyo.org/adservers/serverlist.php?hostformat=adblockplus&showintro=0&mimetype=plaintext", homepage: "https://pgl.yoyo.org/adservers/",
        description: "Compact list of ad and tracking servers." },
    List { id: "adguard-base", name: "AdGuard Base", category: Category::Ads, default_on: false,
        url: "https://filters.adtidy.org/extension/ublock/filters/2.txt", homepage: "https://adguard.com/kb/general/ad-filtering/adguard-filters/",
        description: "AdGuard’s general ad-blocking list (large)." },
    List { id: "adguard-tracking", name: "AdGuard Tracking Protection", category: Category::Privacy, default_on: false,
        url: "https://filters.adtidy.org/extension/ublock/filters/3.txt", homepage: "https://adguard.com/kb/general/ad-filtering/adguard-filters/",
        description: "AdGuard’s comprehensive tracker list (very large)." },
    List { id: "fanboy-annoyance", name: "Fanboy’s Annoyance List", category: Category::Annoyances, default_on: false,
        url: "https://secure.fanboy.co.nz/fanboy-annoyance.txt", homepage: "https://easylist.to/",
        description: "Hides in-page pop-ups, newsletter prompts, chat boxes and other clutter." },
    List { id: "fanboy-cookie", name: "EasyList Cookie List", category: Category::Cookies, default_on: false,
        url: "https://secure.fanboy.co.nz/fanboy-cookiemonster.txt", homepage: "https://easylist.to/",
        description: "Hides cookie notices and consent banners." },
    List { id: "fanboy-social", name: "Fanboy’s Social Blocking List", category: Category::Social, default_on: false,
        url: "https://easylist.to/easylist/fanboy-social.txt", homepage: "https://easylist.to/",
        description: "Removes social media share buttons and widgets." },
    List { id: "urlhaus", name: "Online Malicious URL Blocklist", category: Category::Security, default_on: false,
        url: "https://malware-filter.gitlab.io/malware-filter/urlhaus-filter-online.txt", homepage: "https://gitlab.com/malware-filter/urlhaus-filter",
        description: "Blocks known malware distribution sites (from abuse.ch URLhaus)." },
];

pub fn dir() -> PathBuf {
    let d = paths::data_dir().join("filter-lists");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn file(id: &str) -> PathBuf {
    let safe: String = id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    dir().join(format!("{safe}.txt"))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Meta {
    pub updated: i64,
    pub size: u64,
    pub rules: u64,
    pub title: String,
    pub error: String,
}

fn meta_file() -> PathBuf {
    dir().join("lists.json")
}

pub fn load_meta() -> BTreeMap<String, Meta> {
    std::fs::read(meta_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_meta(m: &BTreeMap<String, Meta>) {
    if let Ok(b) = serde_json::to_vec_pretty(m) {
        let _ = paths::write_atomic(&meta_file(), &b);
    }
}

/// Stable id for a user-added list URL.
pub fn custom_id(url: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("custom-{:012x}", h & 0xffff_ffff_ffff)
}

/// All subscriptions (catalogue + custom URLs) with their enabled state.
pub fn subscriptions(enabled: &BTreeMap<String, bool>, custom: &[String]) -> Vec<(String, String, Category, String, bool)> {
    let mut v: Vec<(String, String, Category, String, bool)> = CATALOG
        .iter()
        .map(|l| (l.id.to_string(), l.name.to_string(), l.category, l.url.to_string(), *enabled.get(l.id).unwrap_or(&l.default_on)))
        .collect();
    for u in custom {
        let id = custom_id(u);
        let on = *enabled.get(&id).unwrap_or(&true);
        v.push((id, u.clone(), Category::Custom, u.clone(), on));
    }
    v
}

/// Validate and store a downloaded list. Returns the number of rule lines.
pub fn store(id: &str, text: &str) -> Result<u64, String> {
    let t = text.trim_start_matches('\u{feff}');
    // Reject HTML error pages and obviously wrong downloads.
    let head: String = t.chars().take(512).collect::<String>().to_lowercase();
    if head.contains("<html") || head.contains("<!doctype") {
        return Err("the server returned a web page, not a filter list".into());
    }
    let rules = t
        .lines()
        .filter(|l| {
            let l = l.trim();
            !l.is_empty() && !l.starts_with('!') && !l.starts_with('[') && !l.starts_with('#')
        })
        .count() as u64;
    if rules == 0 {
        return Err("the list is empty".into());
    }
    paths::write_atomic(&file(id), t.as_bytes()).map_err(|e| e.to_string())?;
    let title = t
        .lines()
        .take(40)
        .find_map(|l| l.strip_prefix("! Title:").map(|s| s.trim().to_string()))
        .unwrap_or_default();
    let mut meta = load_meta();
    meta.insert(id.to_string(), Meta { updated: now(), size: t.len() as u64, rules, title, error: String::new() });
    save_meta(&meta);
    Ok(rules)
}

pub fn record_error(id: &str, error: &str) {
    let mut meta = load_meta();
    let e = meta.entry(id.to_string()).or_default();
    e.error = error.to_string();
    save_meta(&meta);
}

pub fn remove(id: &str) {
    let _ = std::fs::remove_file(file(id));
    let mut meta = load_meta();
    meta.remove(id);
    save_meta(&meta);
}

/// Enabled lists whose copy is missing or older than `max_age_days`.
pub fn due(enabled: &BTreeMap<String, bool>, custom: &[String], max_age_days: i64) -> Vec<(String, String)> {
    let meta = load_meta();
    let limit = now() - max_age_days.max(1) * 86_400;
    subscriptions(enabled, custom)
        .into_iter()
        .filter(|(id, _, _, _, on)| *on && (meta.get(id).map(|m| m.updated < limit).unwrap_or(true) || !file(id).exists()))
        .map(|(id, _, _, url, _)| (id, url))
        .collect()
}

/// JSON for the settings UI.
pub fn describe(enabled: &BTreeMap<String, bool>, custom: &[String]) -> Value {
    let meta = load_meta();
    let info = |id: &str| -> (&'static str, &'static str) {
        CATALOG.iter().find(|l| l.id == id).map(|l| (l.description, l.homepage)).unwrap_or(("Custom filter list", ""))
    };
    Value::Array(
        subscriptions(enabled, custom)
            .into_iter()
            .map(|(id, name, cat, url, on)| {
                let m = meta.get(&id).cloned().unwrap_or_default();
                let (desc, home) = info(&id);
                json!({
                    "id": id, "name": if cat == Category::Custom && !m.title.is_empty() { m.title.clone() } else { name },
                    "category": cat.id(), "url": url, "enabled": on, "description": desc, "homepage": home,
                    "updated": m.updated, "rules": m.rules, "size": m.size, "error": m.error,
                    "downloaded": file(&id).exists()
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_unique_and_defaults() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|l| l.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
        let subs = subscriptions(&BTreeMap::new(), &["https://example.org/list.txt".into()]);
        assert!(subs.iter().any(|s| s.0 == "easylist" && s.4));
        assert!(subs.iter().any(|s| s.0 == "fanboy-cookie" && !s.4));
        assert!(subs.iter().any(|s| s.0.starts_with("custom-") && s.4));
    }
}
