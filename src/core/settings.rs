//! Persistent user preferences (`settings.json`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use super::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    // ── Appearance ────────────────────────────────────────────────
    /// "auto" | "light" | "dark"
    pub theme: String,
    /// Accent colour (hex).
    pub accent: String,
    /// "standard" (tabs under toolbar) | "compact" (tabs inside toolbar) | "vertical" (sidebar)
    pub tab_layout: String,
    pub show_favorites_bar: bool,
    pub show_status_bar: bool,
    pub use_system_title_bar: bool,
    /// "tab": Settings opens as a tab; "window": as a dialog window.
    pub settings_open_in: String,
    /// "left" (macOS style) | "right"
    pub window_controls_side: String,
    pub show_full_url: bool,
    pub default_zoom: f64,

    // ── General ───────────────────────────────────────────────────
    /// "startpage" | "homepage" | "restore"
    pub on_launch: String,
    pub homepage: String,
    /// "startpage" | "blank" | "homepage"
    pub new_tab_page: String,
    pub download_dir: String,
    pub ask_where_to_save: bool,

    // ── Tabs ──────────────────────────────────────────────────────
    pub open_links_in_background: bool,
    pub suspend_inactive_tabs: bool,
    pub suspend_after_minutes: i32,
    pub close_window_with_last_tab: bool,
    pub tab_hover_previews: bool,

    // ── Search ────────────────────────────────────────────────────
    pub search_engine: String,
    pub search_suggestions: bool,
    pub bangs_enabled: bool,

    // ── Privacy ───────────────────────────────────────────────────
    pub content_blocker: bool,
    pub block_trackers: bool,
    pub block_ads: bool,
    pub block_third_party_cookies: bool,
    pub send_do_not_track: bool,
    pub send_gpc: bool,
    pub strip_tracking_params: bool,
    pub https_upgrade: bool,
    pub fingerprint_protection: bool,
    /// Hide page elements using the lists' cosmetic (`##`) rules.
    pub cosmetic_filtering: bool,
    /// Subscription id -> enabled (absent = catalogue default).
    pub filter_lists: BTreeMap<String, bool>,
    /// User-added filter list URLs.
    pub custom_filter_urls: Vec<String>,
    /// Refresh subscribed lists when older than this many days.
    pub filter_update_days: i32,
    /// Darken websites that have no dark theme of their own (when OyNIx is dark).
    pub force_dark_websites: bool,
    /// Tell websites to use their dark theme when OyNIx is dark.
    pub dark_websites: bool,
    /// Website appearance: "auto" (dark when OyNIx is dark: the site's own
    /// dark theme, otherwise automatic darkening), "dark" or "light".
    pub website_appearance: String,
    /// Let secure websites ask for access to USB devices (WebUSB).
    pub webusb: bool,

    // ── Claude for OyNIx ──────────────────────────────────────────
    /// Allow Claude (MCP clients and the built-in panel) to control the browser.
    pub agent_enabled: bool,
    /// "ask": ask before Claude uses a site not in the allowed list;
    /// "auto": act on any site except blocked ones.
    pub agent_mode: String,
    pub agent_allowed_sites: Vec<String>,
    pub agent_blocked_sites: Vec<String>,
    /// Model for the built-in Claude panel.
    pub agent_model: String,
    /// "low" | "medium" | "high" | "xhigh" | "max"
    pub agent_effort: String,

    // ── Advanced ──────────────────────────────────────────────────
    pub javascript: bool,
    pub autoplay_media: bool,
    pub smooth_scrolling: bool,
    pub developer_mode: bool,
    pub user_agent: String,
    pub low_power_mode: bool,

    /// Per-site overrides: host -> { zoom, javascript, blocker, popups, readerByDefault }
    pub sites: BTreeMap<String, Value>,
    /// Favourites shown on the start page / favourites bar are stored with bookmarks.
    pub start_page_background: String,
    pub start_page_show_privacy_report: bool,
    pub start_page_show_reading_list: bool,
    pub start_page_show_recent: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "auto".into(),
            accent: "#2f7cf6".into(),
            tab_layout: "standard".into(),
            show_favorites_bar: false,
            show_status_bar: false,
            use_system_title_bar: false,
            settings_open_in: "tab".into(),
            window_controls_side: "right".into(),
            show_full_url: false,
            default_zoom: 1.0,
            on_launch: "restore".into(),
            homepage: "oynix://start".into(),
            new_tab_page: "startpage".into(),
            download_dir: paths::downloads_dir().to_string_lossy().into_owned(),
            ask_where_to_save: false,
            open_links_in_background: true,
            suspend_inactive_tabs: true,
            suspend_after_minutes: 30,
            close_window_with_last_tab: false,
            tab_hover_previews: true,
            search_engine: "duckduckgo".into(),
            search_suggestions: true,
            bangs_enabled: true,
            content_blocker: true,
            block_trackers: true,
            block_ads: true,
            block_third_party_cookies: true,
            send_do_not_track: false,
            send_gpc: true,
            strip_tracking_params: true,
            https_upgrade: true,
            fingerprint_protection: true,
            cosmetic_filtering: true,
            filter_lists: BTreeMap::new(),
            custom_filter_urls: Vec::new(),
            filter_update_days: 4,
            force_dark_websites: false,
            dark_websites: true,
            website_appearance: "auto".into(),
            webusb: true,
            agent_enabled: false,
            agent_mode: "ask".into(),
            agent_allowed_sites: Vec::new(),
            agent_blocked_sites: Vec::new(),
            agent_model: "claude-opus-5-5".into(),
            agent_effort: "high".into(),
            javascript: true,
            autoplay_media: false,
            smooth_scrolling: true,
            developer_mode: false,
            user_agent: String::new(),
            low_power_mode: false,
            sites: BTreeMap::new(),
            start_page_background: "default".into(),
            start_page_show_privacy_report: true,
            start_page_show_reading_list: true,
            start_page_show_recent: true,
        }
    }
}

impl Settings {
    fn file() -> std::path::PathBuf {
        paths::config_dir().join("settings.json")
    }

    pub fn load() -> Self {
        std::fs::read(Self::file())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(b) = serde_json::to_vec_pretty(self) {
            let _ = paths::write_atomic(&Self::file(), &b);
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    /// Set a single camelCase key from a JSON value. Unknown keys and
    /// type mismatches are rejected so a bad call can never corrupt the file.
    pub fn set_key(&mut self, key: &str, value: Value) -> bool {
        let mut obj = match serde_json::to_value(&*self) {
            Ok(Value::Object(o)) => o,
            _ => return false,
        };
        if !obj.contains_key(key) {
            return false;
        }
        obj.insert(key.to_string(), value);
        match serde_json::from_value::<Settings>(Value::Object(obj)) {
            Ok(s) => {
                *self = s;
                true
            }
            Err(_) => false,
        }
    }

    pub fn site(&self, host: &str) -> Value {
        self.sites.get(host).cloned().unwrap_or(Value::Object(Default::default()))
    }

    pub fn set_site(&mut self, host: &str, key: &str, value: Value) {
        if host.is_empty() {
            return;
        }
        let entry = self
            .sites
            .entry(host.to_string())
            .or_insert_with(|| Value::Object(Default::default()));
        if let Value::Object(o) = entry {
            if value.is_null() {
                o.remove(key);
            } else {
                o.insert(key.to_string(), value);
            }
            if o.is_empty() {
                self.sites.remove(host);
            }
        }
    }

    /// Is the content blocker active for pages whose top-level host is `host`?
    pub fn blocker_enabled_for(&self, host: &str) -> bool {
        if !self.content_blocker {
            return false;
        }
        !matches!(self.sites.get(host).and_then(|v| v.get("blocker")), Some(Value::Bool(false)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_key_validates() {
        let mut s = Settings::default();
        assert!(s.set_key("tabLayout", Value::String("vertical".into())));
        assert_eq!(s.tab_layout, "vertical");
        assert!(!s.set_key("tabLayout", Value::Bool(true)));
        assert!(!s.set_key("doesNotExist", Value::Bool(true)));
    }

    #[test]
    fn site_overrides() {
        let mut s = Settings::default();
        assert!(s.blocker_enabled_for("example.com"));
        s.set_site("example.com", "blocker", Value::Bool(false));
        assert!(!s.blocker_enabled_for("example.com"));
        s.set_site("example.com", "blocker", Value::Null);
        assert!(s.sites.is_empty());
    }
}
