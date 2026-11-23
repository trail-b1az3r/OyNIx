//! `Core` — the QML singleton that exposes the Rust browser core to the UI.
//!
//! Structured data crosses the boundary as JSON strings (parsed once in QML
//! with `JSON.parse`), which keeps the FFI surface small and stable.

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
        #[qml_singleton]
        // appearance
        #[qproperty(QString, theme)]
        #[qproperty(QString, accent)]
        #[qproperty(QString, tab_layout)]
        #[qproperty(bool, show_favorites_bar)]
        #[qproperty(bool, show_status_bar)]
        #[qproperty(bool, use_system_title_bar)]
        #[qproperty(QString, window_controls_side)]
        #[qproperty(bool, show_full_url)]
        #[qproperty(f64, default_zoom)]
        // behaviour
        #[qproperty(QString, on_launch)]
        #[qproperty(QString, new_tab_page)]
        #[qproperty(QString, homepage)]
        #[qproperty(QString, download_dir)]
        #[qproperty(bool, open_links_in_background)]
        #[qproperty(bool, suspend_inactive_tabs)]
        #[qproperty(i32, suspend_after_minutes)]
        #[qproperty(bool, close_window_with_last_tab)]
        #[qproperty(bool, tab_hover_previews)]
        #[qproperty(bool, tree_tabs)]
        #[qproperty(QString, offline_game)]
        #[qproperty(QString, search_engine)]
        #[qproperty(bool, search_suggestions)]
        #[qproperty(bool, content_blocker)]
        #[qproperty(bool, block_third_party_cookies)]
        #[qproperty(bool, javascript)]
        #[qproperty(bool, autoplay_media)]
        #[qproperty(bool, smooth_scrolling)]
        #[qproperty(bool, developer_mode)]
        #[qproperty(bool, low_power_mode)]
        #[qproperty(bool, dark_websites)]
        #[qproperty(bool, force_dark_websites)]
        #[qproperty(QString, website_appearance)]
        #[qproperty(bool, cosmetic_filtering)]
        #[qproperty(bool, agent_enabled)]
        #[qproperty(QString, agent_mode)]
        #[qproperty(QString, user_agent)]
        #[qproperty(bool, start_page_show_privacy_report)]
        #[qproperty(bool, start_page_show_reading_list)]
        #[qproperty(bool, start_page_show_recent)]
        #[qproperty(QString, start_page_background)]
        #[qproperty(QString, settings_json)]
        // live data
        #[qproperty(i32, bookmarks_revision)]
        #[qproperty(i32, reading_revision)]
        #[qproperty(i32, history_revision)]
        #[qproperty(i32, extensions_revision)]
        #[qproperty(f64, total_blocked)]
        #[qproperty(f64, total_ads)]
        #[qproperty(f64, total_trackers)]
        // environment (read-only for QML)
        #[qproperty(QString, profile, READ, CONSTANT)]
        #[qproperty(QString, version, READ, CONSTANT)]
        #[qproperty(QString, graphics, READ, CONSTANT)]
        #[qproperty(QString, qt_version, READ, CONSTANT)]
        #[qproperty(QString, start_url, READ, CONSTANT)]
        #[qproperty(bool, start_private, READ, CONSTANT)]
        #[qproperty(bool, force_restore, READ, CONSTANT)]
        type Core = super::CoreRust;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        // ── settings ──
        #[qinvokable]
        fn set_setting(self: Pin<&mut Core>, key: &QString, json_value: &QString) -> bool;
        #[qinvokable]
        fn reset_settings(self: Pin<&mut Core>);
        #[qinvokable]
        fn site_settings(self: &Core, host: &QString) -> QString;
        #[qinvokable]
        fn set_site_setting(self: Pin<&mut Core>, host: &QString, key: &QString, json_value: &QString);

        // ── address bar ──
        #[qinvokable]
        fn resolve_input(self: &Core, text: &QString) -> QString;
        #[qinvokable]
        fn display_url(self: &Core, url: &QString) -> QString;
        #[qinvokable]
        fn host_of(self: &Core, url: &QString) -> QString;
        #[qinvokable]
        fn is_secure(self: &Core, url: &QString) -> bool;
        #[qinvokable]
        fn suggestions(self: &Core, text: &QString) -> QString;
        #[qinvokable]
        fn suggest_url(self: &Core, text: &QString) -> QString;
        #[qinvokable]
        fn pretty_url(self: &Core, url: &QString) -> QString;
        #[qinvokable]
        fn is_navigation(self: &Core, text: &QString) -> bool;
        #[qinvokable]
        fn inline_complete(self: &Core, typed: &QString) -> QString;
        #[qinvokable]
        fn search_engines(self: &Core) -> QString;
        #[qinvokable]
        fn bangs(self: &Core) -> QString;

        // ── history ──
        #[qinvokable]
        fn record_visit(self: Pin<&mut Core>, url: &QString, title: &QString);
        #[qinvokable]
        fn update_title(self: Pin<&mut Core>, url: &QString, title: &QString);
        #[qinvokable]
        fn history(self: &Core, query: &QString, limit: i32) -> QString;
        #[qinvokable]
        fn remove_history(self: Pin<&mut Core>, url: &QString);
        #[qinvokable]
        fn clear_history(self: Pin<&mut Core>, since_seconds_ago: i32);
        #[qinvokable]
        fn most_visited(self: &Core, limit: i32) -> QString;

        // ── bookmarks / favourites ──
        #[qinvokable]
        fn is_bookmarked(self: &Core, url: &QString) -> bool;
        #[qinvokable]
        fn toggle_bookmark(self: Pin<&mut Core>, url: &QString, title: &QString) -> bool;
        #[qinvokable]
        fn add_bookmark(self: Pin<&mut Core>, url: &QString, title: &QString, folder: &QString);
        #[qinvokable]
        fn remove_bookmark(self: Pin<&mut Core>, id: f64);
        #[qinvokable]
        fn edit_bookmark(self: Pin<&mut Core>, id: f64, title: &QString, url: &QString, folder: &QString);
        #[qinvokable]
        fn move_bookmark(self: Pin<&mut Core>, id: f64, index: i32);
        #[qinvokable]
        fn bookmarks(self: &Core, folder: &QString) -> QString;
        #[qinvokable]
        fn bookmark_folders(self: &Core) -> QString;
        /// The bookmark for `url` ({id, url, title, folder}) or "null".
        #[qinvokable]
        fn bookmark_for(self: &Core, url: &QString) -> QString;
        #[qinvokable]
        fn import_bookmarks(self: Pin<&mut Core>, path_or_url: &QString) -> i32;

        // ── passwords (core/passwords.rs, core/firefox.rs) ──
        /// [{id, username, password}] saved for the page at `url`, most recent first.
        #[qinvokable]
        fn logins_for(self: &Core, url: &QString) -> QString;
        /// What saving would do: "saved" | "updated" | "same" | "never".
        #[qinvokable]
        fn login_would(self: &Core, url: &QString, username: &QString, password: &QString) -> QString;
        #[qinvokable]
        fn save_login(self: Pin<&mut Core>, url: &QString, username: &QString, password: &QString) -> QString;
        #[qinvokable]
        fn login_used(self: Pin<&mut Core>, id: &QString);
        #[qinvokable]
        fn remove_login(self: Pin<&mut Core>, id: &QString) -> bool;
        /// Every login without its password: [{id, origin, username, lastUsed, timesUsed}].
        #[qinvokable]
        fn login_list(self: &Core) -> QString;
        #[qinvokable]
        fn reveal_login(self: &Core, id: &QString) -> QString;
        #[qinvokable]
        fn never_save_login(self: Pin<&mut Core>, url: &QString);
        #[qinvokable]
        fn never_save_list(self: &Core) -> QString;
        #[qinvokable]
        fn forget_never_save(self: Pin<&mut Core>, origin: &QString);
        /// Import a CSV of passwords. {ok, added, error}
        #[qinvokable]
        fn import_logins_csv(self: Pin<&mut Core>, path_or_url: &QString) -> QString;
        /// Firefox profiles found on this computer.
        #[qinvokable]
        fn firefox_profiles(self: &Core) -> QString;
        /// Import from a Firefox profile. {ok, passwords, bookmarks, errors: []}
        #[qinvokable]
        fn import_firefox(self: Pin<&mut Core>, profile: &QString, passwords: bool, bookmarks: bool) -> QString;
        #[qsignal]
        fn logins_changed(self: Pin<&mut Core>);
        #[qinvokable]
        fn export_bookmarks(self: &Core, path_or_url: &QString) -> bool;

        // ── reading list ──
        #[qinvokable]
        fn reading_list(self: &Core) -> QString;
        #[qinvokable]
        fn add_to_reading_list(self: Pin<&mut Core>, url: &QString, title: &QString);
        #[qinvokable]
        fn remove_from_reading_list(self: Pin<&mut Core>, url: &QString);
        #[qinvokable]
        fn mark_read(self: Pin<&mut Core>, url: &QString, read: bool);

        // ── closed tabs / sessions ──
        #[qinvokable]
        fn push_closed_tab(self: Pin<&mut Core>, url: &QString, title: &QString);
        #[qinvokable]
        fn pop_closed_tab(self: Pin<&mut Core>) -> QString;
        #[qinvokable]
        fn closed_tabs(self: &Core) -> QString;
        #[qinvokable]
        fn save_session(self: Pin<&mut Core>, json: &QString);
        #[qinvokable]
        fn load_session(self: &Core) -> QString;
        #[qinvokable]
        fn flush(self: Pin<&mut Core>);
        #[qinvokable]
        fn heartbeat(self: &Core, active: bool);
        #[qinvokable]
        fn freeze_log(self: &Core) -> QString;

        // ── privacy ──
        #[qinvokable]
        fn refresh_stats(self: Pin<&mut Core>);
        #[qinvokable]
        fn site_report(self: &Core, host: &QString) -> QString;
        #[qinvokable]
        fn privacy_report(self: &Core) -> QString;
        #[qinvokable]
        fn reset_privacy_stats(self: Pin<&mut Core>);
        #[qinvokable]
        fn reload_filter_lists(self: Pin<&mut Core>) -> QString;
        #[qinvokable]
        fn filter_lists(self: &Core) -> QString;
        #[qinvokable]
        fn set_filter_list_enabled(self: Pin<&mut Core>, id: &QString, enabled: bool);
        #[qinvokable]
        fn add_filter_list_url(self: Pin<&mut Core>, url: &QString) -> QString;
        #[qinvokable]
        fn remove_filter_list(self: Pin<&mut Core>, id: &QString);
        #[qinvokable]
        fn due_filter_lists(self: &Core, force: bool) -> QString;
        #[qinvokable]
        fn store_filter_list(self: Pin<&mut Core>, id: &QString, text: &QString) -> QString;
        #[qinvokable]
        fn filter_list_failed(self: Pin<&mut Core>, id: &QString, error: &QString);
        #[qinvokable]
        fn cosmetic_script(self: &Core, host: &QString) -> QString;
        #[qinvokable]
        fn apply_color_scheme(self: &Core, dark: bool);
        #[qinvokable]
        unsafe fn clear_cookies(self: &Core, profile: *mut QObject);
        #[qinvokable]
        unsafe fn load_extension_panel(self: &Core, host: *mut QObject, id: &QString, api: *mut QObject) -> QString;
        #[qinvokable]
        unsafe fn unload_extension_panel(self: &Core, host: *mut QObject);
        #[qinvokable]
        fn extension_data(self: &Core, id: &QString) -> QString;
        #[qinvokable]
        fn extension_page_url(self: &Core, id: &QString) -> QString;
        /// file:// URL of the selected browser style's theme-shape.qml (power extensions), or "".
        #[qinvokable]
        fn theme_shape_url(self: &Core) -> QString;
        #[qinvokable]
        fn extension_has_permission(self: &Core, id: &QString, permission: &QString) -> bool;
        /// For the WebExtension runtime: {id, name, permissions, origins,
        /// developer, powerful} of the extension OyNIx knows as `web_id`.
        #[qinvokable]
        fn webext_info(self: &Core, web_id: &QString) -> QString;
        /// [{ext, name, key, description}] keyboard shortcuts from manifests' "commands".
        #[qinvokable]
        fn webext_commands(self: &Core) -> QString;
        /// [{ext, keyword}] address-field keywords from manifests' "omnibox".
        #[qinvokable]
        fn webext_omnibox(self: &Core) -> QString;
        #[qinvokable]
        fn webext_set_observing(self: &Core, requests: bool, cookies: bool);
        #[qinvokable]
        fn webext_drain_events(self: &Core) -> QString;
        /// A file as a data: URL (deletes it if `consume`).
        #[qinvokable]
        fn file_data_url(self: &Core, path: &QString, mime: &QString, consume: bool) -> QString;
        /// Delete a downloaded file (only inside the downloads folder).
        #[qinvokable]
        fn remove_download_file(self: &Core, path: &QString) -> bool;
        /// Search the default engine for `text` (never treated as an address).
        #[qinvokable]
        fn search_url(self: &Core, text: &QString) -> QString;
        #[qinvokable]
        unsafe fn webext_cookies(self: &Core, web_id: &QString, profile: *mut QObject) -> QString;
        #[qinvokable]
        unsafe fn webext_cookie_set(self: &Core, web_id: &QString, profile: *mut QObject, cookie: &QString) -> QString;
        #[qinvokable]
        unsafe fn webext_cookie_remove(self: &Core, web_id: &QString, profile: *mut QObject, url: &QString, name: &QString) -> QString;
        /// The active browser style (validated theme.json of the chosen extension), or "{}".
        #[qinvokable]
        fn browser_style_json(self: &Core) -> QString;
        // ── hyperthemes (5.4.2) ──
        /// The chosen hypertheme for light or dark: { file, name, tokens,
        /// panels, warnings }, or {} when none (or it didn't load).
        #[qinvokable]
        fn hypertheme_json(self: &Core, dark: bool) -> QString;
        /// [{ file, name, description, builtin }] of built-in and installed hyperthemes.
        #[qinvokable]
        fn hyperthemes(self: &Core) -> QString;
        /// Install a .hypertheme file or folder: { file } or { error }.
        #[qinvokable]
        fn install_hypertheme(self: &Core, path: &QString) -> QString;
        /// Theme Studio's page, written to the cache with its bridge; its file:// URL.
        #[qinvokable]
        fn theme_studio_url(self: &Core) -> QString;
        /// Remove the saved site icons (see core/favicons.rs).
        #[qinvokable]
        fn clear_favicons(self: &Core) -> i32;
        /// [{id, name}] of enabled extensions that have a theme.json.
        #[qinvokable]
        fn browser_styles(self: &Core) -> QString;
        #[qinvokable]
        fn extension_page_api(self: &Core, id: &QString) -> QString;
        #[qinvokable]
        fn webchannel_script(self: &Core) -> QString;
        #[qinvokable]
        fn blocker_generation(self: &Core) -> i32;
        #[qinvokable]
        unsafe fn send_mouse(self: &Core, item: *mut QObject, x: f64, y: f64, button: i32, clicks: i32, modifiers: i32, kind: i32);
        #[qinvokable]
        unsafe fn send_key(self: &Core, item: *mut QObject, key: i32, modifiers: i32, text: &QString);
        #[qinvokable]
        unsafe fn send_text(self: &Core, item: *mut QObject, text: &QString);
        #[qinvokable]
        unsafe fn send_wheel(self: &Core, item: *mut QObject, x: f64, y: f64, dx: f64, dy: f64);
        #[qinvokable]
        fn notify(self: &Core, title: &QString, body: &QString);
        #[qinvokable]
        fn material_scheme(self: &Core, seed: &QString, dark: bool) -> QString;
        #[qinvokable]
        fn theme_scheme(self: &Core, seed: &QString, mode: &QString, style: &QString) -> QString;
        #[qinvokable]
        unsafe fn attach_profile(self: &Core, profile: *mut QObject);

        // ── scripts ──
        #[qinvokable]
        fn reader_script(self: &Core) -> QString;
        #[qinvokable]
        fn reader_probe_script(self: &Core) -> QString;
        #[qinvokable]
        fn page_scripts(self: &Core) -> QString;

        // ── extensions ──
        #[qinvokable]
        fn extensions(self: &Core) -> QString;
        #[qinvokable]
        fn install_extension(self: Pin<&mut Core>, path_or_url: &QString) -> QString;
        #[qinvokable]
        fn remove_extension(self: Pin<&mut Core>, id: &QString) -> bool;
        #[qinvokable]
        fn set_extension_enabled(self: Pin<&mut Core>, id: &QString, enabled: bool);
        /// Is the extension pinned to the toolbar?
        #[qinvokable]
        fn is_extension_pinned(self: &Core, id: &QString) -> bool;
        #[qinvokable]
        fn set_extension_pinned(self: Pin<&mut Core>, id: &QString, pinned: bool);
        /// Icon file of the extension a chrome-extension:// URL belongs to, or "".
        #[qinvokable]
        fn extension_icon_for(self: &Core, url: &QString) -> QString;
        /// Does the computer have a route to the internet (a default route)?
        #[qinvokable]
        fn network_online(self: &Core) -> bool;
        /// Keep a copy of the icon a page shows (see core/favicons.rs).
        #[qinvokable]
        fn remember_favicon(self: &Core, page_url: &QString, icon: &QString);
        /// "file://…" of the saved icon for the site of `url`, or "".
        #[qinvokable]
        fn cached_favicon(self: &Core, url: &QString) -> QString;
        #[qinvokable]
        fn create_extension(self: Pin<&mut Core>, name: &QString) -> QString;
        #[qinvokable]
        fn extensions_dir(self: &Core) -> QString;
        /// network.ts: one call from an extension's background page
        /// ({op: "block"|"header"|"clearHeaders"|"observe", …}). Returns {ok, error}.
        #[qinvokable]
        fn net_call(self: Pin<&mut Core>, ext: &QString, call_json: &QString) -> QString;
        /// Requests observed for network.ts since the last call (JSON array).
        #[qinvokable]
        fn net_drain(self: &Core) -> QString;
        /// The hidden page network.ts runs in; `token` authenticates its messages.
        #[qinvokable]
        fn network_html(self: &Core, id: &QString, token: &QString) -> QString;
        /// The page for an extension's widget.tsx, styled with `theme_css`.
        #[qinvokable]
        fn widget_html(self: &Core, id: &QString, theme_css: &QString) -> QString;
        #[qinvokable]
        fn set_extension_web_id(self: Pin<&mut Core>, id: &QString, web_id: &QString);
        #[qinvokable]
        fn consume_first_run(self: Pin<&mut Core>, id: &QString) -> bool;
        #[qinvokable]
        fn extension_scripts(self: &Core, tab_id: i32) -> QString;
        #[qinvokable]
        fn extension_page_scripts(self: &Core, kind: &QString) -> QString;
        /// For tabs: the chrome.* runtime for extension pages opened in a tab
        /// (chrome-extension://…). Inert on any other page.
        #[qinvokable]
        fn extension_tab_page_scripts(self: &Core, tab_id: i32) -> QString;

        // ── system ──
        #[qinvokable]
        fn profiles(self: &Core) -> QString;
        #[qinvokable]
        fn launch_profile(self: &Core, name: &QString, private_window: bool) -> bool;
        #[qinvokable]
        fn open_externally(self: &Core, path_or_url: &QString) -> bool;
        #[qinvokable]
        fn make_default_browser(self: &Core) -> bool;
        #[qinvokable]
        fn config_path(self: &Core, which: &QString) -> QString;
        #[qinvokable]
        fn format_bytes(self: &Core, bytes: f64) -> QString;
        #[qinvokable]
        fn local_path(self: &Core, url: &QString) -> QString;
    }

    impl cxx_qt::Initialize for Core {}
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{json, Value};

use crate::core::{blocker, extensions, filterlists, library::Library, omnibox, paths, scripts, settings::Settings};

pub struct CoreRust {
    theme: QString,
    accent: QString,
    tab_layout: QString,
    show_favorites_bar: bool,
    show_status_bar: bool,
    use_system_title_bar: bool,
    window_controls_side: QString,
    show_full_url: bool,
    default_zoom: f64,
    on_launch: QString,
    new_tab_page: QString,
    homepage: QString,
    download_dir: QString,
    open_links_in_background: bool,
    suspend_inactive_tabs: bool,
    suspend_after_minutes: i32,
    close_window_with_last_tab: bool,
    tab_hover_previews: bool,
    tree_tabs: bool,
    offline_game: QString,
    search_engine: QString,
    search_suggestions: bool,
    content_blocker: bool,
    block_third_party_cookies: bool,
    javascript: bool,
    autoplay_media: bool,
    smooth_scrolling: bool,
    developer_mode: bool,
    low_power_mode: bool,
    dark_websites: bool,
    force_dark_websites: bool,
    website_appearance: QString,
    cosmetic_filtering: bool,
    agent_enabled: bool,
    agent_mode: QString,
    user_agent: QString,
    start_page_show_privacy_report: bool,
    start_page_show_reading_list: bool,
    start_page_show_recent: bool,
    start_page_background: QString,
    settings_json: QString,

    bookmarks_revision: i32,
    reading_revision: i32,
    history_revision: i32,
    extensions_revision: i32,
    total_blocked: f64,
    total_ads: f64,
    total_trackers: f64,

    profile: QString,
    version: QString,
    graphics: QString,
    qt_version: QString,
    start_url: QString,
    start_private: bool,
    force_restore: bool,

    settings: Settings,
    library: Library,
    extensions: Vec<extensions::Entry>,
}

impl Default for CoreRust {
    fn default() -> Self {
        let env = crate::launch_env();
        let settings = Settings::load();
        let q = |s: &str| QString::from(s);
        let mut me = Self {
            theme: q(""),
            accent: q(""),
            tab_layout: q(""),
            show_favorites_bar: false,
            show_status_bar: false,
            use_system_title_bar: false,
            window_controls_side: q(""),
            show_full_url: false,
            default_zoom: 1.0,
            on_launch: q(""),
            new_tab_page: q(""),
            homepage: q(""),
            download_dir: q(""),
            open_links_in_background: true,
            suspend_inactive_tabs: true,
            suspend_after_minutes: 30,
            close_window_with_last_tab: false,
            tab_hover_previews: true,
            tree_tabs: false,
            offline_game: q("go"),
            search_engine: q(""),
            search_suggestions: true,
            content_blocker: true,
            block_third_party_cookies: true,
            javascript: true,
            autoplay_media: false,
            smooth_scrolling: true,
            developer_mode: false,
            low_power_mode: false,
            dark_websites: true,
            force_dark_websites: false,
            website_appearance: q("auto"),
            cosmetic_filtering: true,
            agent_enabled: false,
            agent_mode: q("ask"),
            user_agent: q(""),
            start_page_show_privacy_report: true,
            start_page_show_reading_list: true,
            start_page_show_recent: true,
            start_page_background: q(""),
            settings_json: q("{}"),
            bookmarks_revision: 0,
            reading_revision: 0,
            history_revision: 0,
            extensions_revision: 0,
            total_blocked: 0.0,
            total_ads: 0.0,
            total_trackers: 0.0,
            profile: q(paths::profile()),
            version: q(crate::VERSION),
            graphics: q(&env.graphics),
            qt_version: q(&env.qt_version),
            start_url: q(&env.start_url),
            start_private: env.private,
            force_restore: env.restore,
            settings,
            library: Library::load(),
            extensions: { let mut l = extensions::load(); crate::core::builtin::sync(&mut l); l },
        };
        me.copy_settings_to_fields();
        me
    }
}

fn s(q: &QString) -> String {
    String::from(q)
}

fn qs(s: impl AsRef<str>) -> QString {
    QString::from(s.as_ref())
}

fn to_qjson(v: &Value) -> QString {
    qs(serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
}

/// Accept either a filesystem path or a `file://` URL.
fn to_path(p: &str) -> std::path::PathBuf {
    if let Ok(u) = url::Url::parse(p) {
        if u.scheme() == "file" {
            if let Ok(pb) = u.to_file_path() {
                return pb;
            }
        }
    }
    std::path::PathBuf::from(p)
}

impl CoreRust {
    fn copy_settings_to_fields(&mut self) {
        let st = &self.settings;
        self.theme = qs(&st.theme);
        self.accent = qs(&st.accent);
        self.tab_layout = qs(&st.tab_layout);
        self.show_favorites_bar = st.show_favorites_bar;
        self.show_status_bar = st.show_status_bar;
        self.use_system_title_bar = st.use_system_title_bar;
        self.window_controls_side = qs(&st.window_controls_side);
        self.show_full_url = st.show_full_url;
        self.default_zoom = st.default_zoom;
        self.on_launch = qs(&st.on_launch);
        self.new_tab_page = qs(&st.new_tab_page);
        self.homepage = qs(&st.homepage);
        self.download_dir = qs(&st.download_dir);
        self.open_links_in_background = st.open_links_in_background;
        self.suspend_inactive_tabs = st.suspend_inactive_tabs;
        self.suspend_after_minutes = st.suspend_after_minutes;
        self.close_window_with_last_tab = st.close_window_with_last_tab;
        self.tab_hover_previews = st.tab_hover_previews;
        self.tree_tabs = st.tree_tabs;
        self.offline_game = qs(&st.offline_game);
        self.search_engine = qs(&st.search_engine);
        self.search_suggestions = st.search_suggestions;
        self.content_blocker = st.content_blocker;
        self.block_third_party_cookies = st.block_third_party_cookies;
        self.javascript = st.javascript;
        self.autoplay_media = st.autoplay_media;
        self.smooth_scrolling = st.smooth_scrolling;
        self.developer_mode = st.developer_mode;
        self.low_power_mode = st.low_power_mode;
        self.dark_websites = st.dark_websites;
        self.force_dark_websites = st.force_dark_websites;
        self.website_appearance = qs(&st.website_appearance);
        self.cosmetic_filtering = st.cosmetic_filtering;
        self.agent_enabled = st.agent_enabled;
        self.agent_mode = qs(&st.agent_mode);
        self.user_agent = qs(&st.user_agent);
        self.start_page_show_privacy_report = st.start_page_show_privacy_report;
        self.start_page_show_reading_list = st.start_page_show_reading_list;
        self.start_page_show_recent = st.start_page_show_recent;
        self.start_page_background = qs(&st.start_page_background);
        self.settings_json = qs(st.to_json());
        apply_privacy(st);
    }
}

/// Subscriptions the blocker should load: (id, category) of enabled lists.
pub fn active_lists(st: &Settings) -> Vec<(String, filterlists::Category)> {
    filterlists::subscriptions(&st.filter_lists, &st.custom_filter_urls)
        .into_iter()
        .filter(|x| x.4)
        .map(|(id, _, cat, _, _)| (id, cat))
        .collect()
}

/// Push privacy settings down into the blocker and the network interceptor.
pub fn apply_privacy(st: &Settings) {
    blocker::configure(st.content_blocker, st.block_ads, st.block_trackers, st.cosmetic_filtering);
    blocker::set_allowlist(
        st.sites
            .iter()
            .filter(|(_, v)| v.get("blocker") == Some(&Value::Bool(false)))
            .map(|(k, _)| k.clone()),
    );
    crate::bridge::native::set_privacy_flags(st.send_do_not_track, st.send_gpc, st.strip_tracking_params, st.block_third_party_cookies);
}

impl qobject::Core {
    /// Re-emit every property after the settings struct changed.
    fn sync(mut self: Pin<&mut Self>) {
        let st = self.rust().settings.clone();
        self.as_mut().set_theme(qs(&st.theme));
        self.as_mut().set_accent(qs(&st.accent));
        self.as_mut().set_tab_layout(qs(&st.tab_layout));
        self.as_mut().set_show_favorites_bar(st.show_favorites_bar);
        self.as_mut().set_show_status_bar(st.show_status_bar);
        self.as_mut().set_use_system_title_bar(st.use_system_title_bar);
        self.as_mut().set_window_controls_side(qs(&st.window_controls_side));
        self.as_mut().set_show_full_url(st.show_full_url);
        self.as_mut().set_default_zoom(st.default_zoom);
        self.as_mut().set_on_launch(qs(&st.on_launch));
        self.as_mut().set_new_tab_page(qs(&st.new_tab_page));
        self.as_mut().set_homepage(qs(&st.homepage));
        self.as_mut().set_download_dir(qs(&st.download_dir));
        self.as_mut().set_open_links_in_background(st.open_links_in_background);
        self.as_mut().set_suspend_inactive_tabs(st.suspend_inactive_tabs);
        self.as_mut().set_suspend_after_minutes(st.suspend_after_minutes);
        self.as_mut().set_close_window_with_last_tab(st.close_window_with_last_tab);
        self.as_mut().set_tab_hover_previews(st.tab_hover_previews);
        self.as_mut().set_tree_tabs(st.tree_tabs);
        self.as_mut().set_offline_game(qs(&st.offline_game));
        self.as_mut().set_search_engine(qs(&st.search_engine));
        self.as_mut().set_search_suggestions(st.search_suggestions);
        self.as_mut().set_content_blocker(st.content_blocker);
        self.as_mut().set_block_third_party_cookies(st.block_third_party_cookies);
        self.as_mut().set_javascript(st.javascript);
        self.as_mut().set_autoplay_media(st.autoplay_media);
        self.as_mut().set_smooth_scrolling(st.smooth_scrolling);
        self.as_mut().set_developer_mode(st.developer_mode);
        self.as_mut().set_low_power_mode(st.low_power_mode);
        self.as_mut().set_dark_websites(st.dark_websites);
        self.as_mut().set_force_dark_websites(st.force_dark_websites);
        self.as_mut().set_website_appearance(qs(&st.website_appearance));
        self.as_mut().set_cosmetic_filtering(st.cosmetic_filtering);
        self.as_mut().set_agent_enabled(st.agent_enabled);
        self.as_mut().set_agent_mode(qs(&st.agent_mode));
        self.as_mut().set_user_agent(qs(&st.user_agent));
        self.as_mut().set_start_page_show_privacy_report(st.start_page_show_privacy_report);
        self.as_mut().set_start_page_show_reading_list(st.start_page_show_reading_list);
        self.as_mut().set_start_page_show_recent(st.start_page_show_recent);
        self.as_mut().set_start_page_background(qs(&st.start_page_background));
        self.as_mut().set_settings_json(qs(st.to_json()));
        apply_privacy(&st);
    }

    fn bump_bookmarks(mut self: Pin<&mut Self>) {
        let r = self.bookmarks_revision + 1;
        self.as_mut().set_bookmarks_revision(r);
    }

    fn bump_reading(mut self: Pin<&mut Self>) {
        let r = self.reading_revision + 1;
        self.as_mut().set_reading_revision(r);
    }

    fn bump_history(mut self: Pin<&mut Self>) {
        let r = self.history_revision + 1;
        self.as_mut().set_history_revision(r);
    }

    // ── settings ────────────────────────────────────────────────────
    pub fn set_setting(mut self: Pin<&mut Self>, key: &QString, json_value: &QString) -> bool {
        let Ok(v) = serde_json::from_str::<Value>(&s(json_value)) else { return false };
        let ok = self.as_mut().rust_mut().settings.set_key(&s(key), v);
        if ok {
            self.rust().settings.save();
            self.sync();
        }
        ok
    }

    pub fn reset_settings(mut self: Pin<&mut Self>) {
        let sites = self.rust().settings.sites.clone();
        let mut fresh = Settings::default();
        fresh.sites = sites;
        self.as_mut().rust_mut().settings = fresh;
        self.rust().settings.save();
        self.sync();
    }

    pub fn site_settings(&self, host: &QString) -> QString {
        to_qjson(&self.settings.site(&s(host)))
    }

    pub fn set_site_setting(mut self: Pin<&mut Self>, host: &QString, key: &QString, json_value: &QString) {
        let v = serde_json::from_str::<Value>(&s(json_value)).unwrap_or(Value::Null);
        self.as_mut().rust_mut().settings.set_site(&s(host), &s(key), v);
        self.rust().settings.save();
        self.sync();
    }

    // ── address bar ─────────────────────────────────────────────────
    pub fn resolve_input(&self, text: &QString) -> QString {
        let st = &self.settings;
        qs(omnibox::resolve(&s(text), &st.search_engine, st.bangs_enabled, st.https_upgrade))
    }

    pub fn display_url(&self, url: &QString) -> QString {
        qs(omnibox::display(&s(url), self.settings.show_full_url))
    }

    pub fn host_of(&self, url: &QString) -> QString {
        qs(omnibox::host_of(&s(url)))
    }

    pub fn is_secure(&self, url: &QString) -> bool {
        omnibox::is_secure(&s(url))
    }

    /// Local suggestions: bang completions, then bookmarks, then history.
    pub fn suggestions(&self, text: &QString) -> QString {
        let t = s(text);
        let tl = t.trim().to_lowercase();
        let mut out: Vec<Value> = Vec::new();
        if tl.is_empty() {
            return qs("[]");
        }
        if self.settings.bangs_enabled {
            if let Some(word) = tl.split_whitespace().last().and_then(|w| w.strip_prefix('!')) {
                for (k, name, _) in omnibox::BANGS.iter().filter(|(k, _, _)| k.starts_with(word)).take(4) {
                    out.push(json!({ "kind": "bang", "title": format!("!{k}"), "subtitle": format!("Search {name}"), "url": "" }));
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        for b in self.library.bookmarks.iter() {
            if out.len() >= 4 {
                break;
            }
            if (b.url.to_lowercase().contains(&tl) || b.title.to_lowercase().contains(&tl)) && seen.insert(b.url.clone()) {
                out.push(json!({ "kind": "bookmark", "title": b.title, "subtitle": omnibox::pretty(&b.url), "url": b.url }));
            }
        }
        for h in self.library.search_history(&tl, 40) {
            if out.len() >= 8 {
                break;
            }
            if seen.insert(h.url.clone()) {
                out.push(json!({ "kind": "history", "title": if h.title.is_empty() { h.url.clone() } else { h.title.clone() }, "subtitle": omnibox::pretty(&h.url), "url": h.url }));
            }
        }
        to_qjson(&Value::Array(out))
    }

    /// Does this input open a page (URL / host / path), rather than search?
    pub fn is_navigation(&self, text: &QString) -> bool {
        let t = s(text);
        let st = &self.settings;
        let r = omnibox::resolve(&t, &st.search_engine, st.bangs_enabled, st.https_upgrade);
        !t.trim().is_empty() && r != omnibox::search_url(&st.search_engine, t.trim()) && omnibox::expand_bang(&t).is_none()
    }

    pub fn pretty_url(&self, url: &QString) -> QString {
        qs(omnibox::pretty(&s(url)))
    }

    pub fn inline_complete(&self, typed: &QString) -> QString {
        qs(omnibox::inline_complete(&s(typed), &self.library.completion_candidates()))
    }

    pub fn suggest_url(&self, text: &QString) -> QString {
        if !self.settings.search_suggestions {
            return qs("");
        }
        qs(omnibox::suggest_url(&self.settings.search_engine, &s(text)))
    }

    pub fn search_engines(&self) -> QString {
        let v: Vec<Value> = omnibox::ENGINES.iter().map(|e| json!({ "id": e.id, "name": e.name })).collect();
        to_qjson(&Value::Array(v))
    }

    pub fn bangs(&self) -> QString {
        let v: Vec<Value> = omnibox::BANGS.iter().map(|(k, n, _)| json!({ "key": k, "name": n })).collect();
        to_qjson(&Value::Array(v))
    }

    // ── history ─────────────────────────────────────────────────────
    pub fn record_visit(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        self.as_mut().rust_mut().library.record_visit(&s(url), &s(title));
        self.bump_history();
    }

    pub fn update_title(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        self.as_mut().rust_mut().library.update_title(&s(url), &s(title));
    }

    pub fn history(&self, query: &QString, limit: i32) -> QString {
        let v: Vec<Value> = self
            .library
            .search_history(&s(query), limit.max(1) as usize)
            .into_iter()
            .map(|h| json!({ "url": h.url, "title": h.title, "visits": h.visits, "lastVisit": h.last_visit, "host": omnibox::host_of(&h.url) }))
            .collect();
        to_qjson(&Value::Array(v))
    }

    pub fn remove_history(mut self: Pin<&mut Self>, url: &QString) {
        self.as_mut().rust_mut().library.remove_history(&s(url));
        self.bump_history();
    }

    pub fn clear_history(mut self: Pin<&mut Self>, since_seconds_ago: i32) {
        let since = if since_seconds_ago <= 0 { 0 } else { crate::core::library::now() - since_seconds_ago as i64 };
        self.as_mut().rust_mut().library.clear_history(since);
        self.bump_history();
    }

    pub fn most_visited(&self, limit: i32) -> QString {
        let v: Vec<Value> = self
            .library
            .most_visited(limit.max(1) as usize)
            .into_iter()
            .map(|h| json!({ "url": h.url, "title": h.title, "host": omnibox::display(&h.url, false) }))
            .collect();
        to_qjson(&Value::Array(v))
    }

    // ── bookmarks ───────────────────────────────────────────────────
    pub fn is_bookmarked(&self, url: &QString) -> bool {
        self.library.is_bookmarked(&s(url))
    }

    pub fn toggle_bookmark(mut self: Pin<&mut Self>, url: &QString, title: &QString) -> bool {
        let u = s(url);
        let now_marked = if self.library.is_bookmarked(&u) {
            self.as_mut().rust_mut().library.remove_bookmark_url(&u);
            false
        } else {
            self.as_mut().rust_mut().library.add_bookmark(&u, &s(title), "");
            true
        };
        self.bump_bookmarks();
        now_marked
    }

    pub fn add_bookmark(mut self: Pin<&mut Self>, url: &QString, title: &QString, folder: &QString) {
        self.as_mut().rust_mut().library.add_bookmark(&s(url), &s(title), &s(folder));
        self.bump_bookmarks();
    }

    pub fn remove_bookmark(mut self: Pin<&mut Self>, id: f64) {
        self.as_mut().rust_mut().library.remove_bookmark(id as u64);
        self.bump_bookmarks();
    }

    pub fn edit_bookmark(mut self: Pin<&mut Self>, id: f64, title: &QString, url: &QString, folder: &QString) {
        self.as_mut().rust_mut().library.edit_bookmark(id as u64, &s(title), &s(url), &s(folder));
        self.bump_bookmarks();
    }

    pub fn move_bookmark(mut self: Pin<&mut Self>, id: f64, index: i32) {
        self.as_mut().rust_mut().library.move_bookmark(id as u64, index.max(0) as usize);
        self.bump_bookmarks();
    }

    /// `folder` = "*" for all bookmarks.
    pub fn bookmarks(&self, folder: &QString) -> QString {
        let f = s(folder);
        let v: Vec<Value> = self
            .library
            .bookmarks
            .iter()
            .filter(|b| f == "*" || b.folder == f)
            .map(|b| json!({ "id": b.id, "url": b.url, "title": b.title, "folder": b.folder, "host": omnibox::display(&b.url, false) }))
            .collect();
        to_qjson(&Value::Array(v))
    }

    pub fn bookmark_for(&self, url: &QString) -> QString {
        let u = s(url);
        match self.library.bookmarks.iter().find(|b| b.url == u) {
            Some(b) => to_qjson(&json!({ "id": b.id, "url": b.url, "title": b.title, "folder": b.folder })),
            None => qs("null"),
        }
    }

    pub fn bookmark_folders(&self) -> QString {
        to_qjson(&json!(self.library.folders()))
    }

    // ── passwords ──
    pub fn logins_for(&self, url: &QString) -> QString {
        let st = crate::core::passwords::load();
        let v: Vec<Value> = st.for_url(&s(url)).iter().map(|l| json!({ "id": l.id, "username": l.username, "password": l.password })).collect();
        to_qjson(&Value::Array(v))
    }

    pub fn login_would(&self, url: &QString, username: &QString, password: &QString) -> QString {
        let st = crate::core::passwords::load();
        let o = crate::core::passwords::origin_of(&s(url));
        if st.never.contains(&o) {
            return qs("never");
        }
        qs(st.would(&s(url), &s(username), &s(password)))
    }

    pub fn save_login(self: Pin<&mut Self>, url: &QString, username: &QString, password: &QString) -> QString {
        let mut st = crate::core::passwords::load();
        let r = st.put(&s(url), &s(username), &s(password));
        if r != "same" {
            if let Err(e) = crate::core::passwords::save(&st) {
                return qs(format!("error: {e}"));
            }
            self.logins_changed();
        }
        qs(r)
    }

    pub fn login_used(self: Pin<&mut Self>, id: &QString) {
        let mut st = crate::core::passwords::load();
        st.used(&s(id));
        let _ = crate::core::passwords::save(&st);
    }

    pub fn remove_login(self: Pin<&mut Self>, id: &QString) -> bool {
        let mut st = crate::core::passwords::load();
        let ok = st.remove(&s(id)) && crate::core::passwords::save(&st).is_ok();
        if ok {
            self.logins_changed();
        }
        ok
    }

    pub fn login_list(&self) -> QString {
        let mut st = crate::core::passwords::load();
        st.logins.sort_by(|a, b| a.origin.cmp(&b.origin).then(a.username.cmp(&b.username)));
        let v: Vec<Value> = st
            .logins
            .iter()
            .map(|l| json!({ "id": l.id, "origin": l.origin, "username": l.username, "lastUsed": l.last_used, "timesUsed": l.times_used }))
            .collect();
        to_qjson(&Value::Array(v))
    }

    pub fn reveal_login(&self, id: &QString) -> QString {
        let id = s(id);
        qs(crate::core::passwords::load().logins.iter().find(|l| l.id == id).map(|l| l.password.as_str()).unwrap_or(""))
    }

    pub fn never_save_login(self: Pin<&mut Self>, url: &QString) {
        let mut st = crate::core::passwords::load();
        let o = crate::core::passwords::origin_of(&s(url));
        if !o.is_empty() && !st.never.contains(&o) {
            st.never.push(o);
            let _ = crate::core::passwords::save(&st);
            self.logins_changed();
        }
    }

    pub fn never_save_list(&self) -> QString {
        to_qjson(&json!(crate::core::passwords::load().never))
    }

    pub fn forget_never_save(self: Pin<&mut Self>, origin: &QString) {
        let mut st = crate::core::passwords::load();
        let o = s(origin);
        st.never.retain(|x| *x != o);
        let _ = crate::core::passwords::save(&st);
        self.logins_changed();
    }

    pub fn import_logins_csv(self: Pin<&mut Self>, path_or_url: &QString) -> QString {
        let text = match std::fs::read_to_string(to_path(&s(path_or_url))) {
            Ok(t) => t,
            Err(e) => return to_qjson(&json!({ "ok": false, "added": 0, "error": e.to_string() })),
        };
        match crate::core::passwords::parse_csv(&text) {
            Ok(rows) => {
                let mut st = crate::core::passwords::load();
                let n = st.merge(rows);
                if let Err(e) = crate::core::passwords::save(&st) {
                    return to_qjson(&json!({ "ok": false, "added": 0, "error": e }));
                }
                self.logins_changed();
                to_qjson(&json!({ "ok": true, "added": n, "error": "" }))
            }
            Err(e) => to_qjson(&json!({ "ok": false, "added": 0, "error": format!("This doesn’t look like a passwords CSV ({e}).") })),
        }
    }

    pub fn firefox_profiles(&self) -> QString {
        to_qjson(&json!(crate::core::firefox::profiles()))
    }

    pub fn import_firefox(mut self: Pin<&mut Self>, profile: &QString, passwords: bool, bookmarks: bool) -> QString {
        let dir = std::path::PathBuf::from(s(profile));
        let (mut np, mut nb, mut errors) = (0usize, 0usize, Vec::<String>::new());
        if passwords {
            match crate::core::firefox::logins(&dir) {
                Ok(rows) => {
                    let mut st = crate::core::passwords::load();
                    np = st.merge(rows);
                    if let Err(e) = crate::core::passwords::save(&st) {
                        errors.push(e);
                    }
                }
                Err(e) => errors.push(e),
            }
        }
        if bookmarks {
            match crate::core::firefox::bookmarks(&dir) {
                Ok(rows) => nb = self.as_mut().rust_mut().library.import_list(rows),
                Err(e) => errors.push(format!("Bookmarks: {e}")),
            }
        }
        if np > 0 {
            self.as_mut().logins_changed();
        }
        if nb > 0 {
            self.as_mut().bump_bookmarks();
        }
        to_qjson(&json!({ "ok": errors.is_empty(), "passwords": np, "bookmarks": nb, "errors": errors }))
    }

    pub fn import_bookmarks(mut self: Pin<&mut Self>, path_or_url: &QString) -> i32 {
        let p = to_path(&s(path_or_url));
        let Ok(html) = std::fs::read_to_string(p) else { return -1 };
        let n = self.as_mut().rust_mut().library.import_html(&html);
        self.bump_bookmarks();
        n as i32
    }

    pub fn export_bookmarks(&self, path_or_url: &QString) -> bool {
        std::fs::write(to_path(&s(path_or_url)), self.library.export_html()).is_ok()
    }

    // ── reading list ────────────────────────────────────────────────
    pub fn reading_list(&self) -> QString {
        to_qjson(&serde_json::to_value(&self.library.reading).unwrap_or(Value::Null))
    }

    pub fn add_to_reading_list(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        self.as_mut().rust_mut().library.add_reading(&s(url), &s(title));
        self.bump_reading();
    }

    pub fn remove_from_reading_list(mut self: Pin<&mut Self>, url: &QString) {
        self.as_mut().rust_mut().library.remove_reading(&s(url));
        self.bump_reading();
    }

    pub fn mark_read(mut self: Pin<&mut Self>, url: &QString, read: bool) {
        self.as_mut().rust_mut().library.set_read(&s(url), read);
        self.bump_reading();
    }

    // ── closed tabs / session ───────────────────────────────────────
    pub fn push_closed_tab(mut self: Pin<&mut Self>, url: &QString, title: &QString) {
        self.as_mut().rust_mut().library.push_closed(&s(url), &s(title));
    }

    pub fn pop_closed_tab(mut self: Pin<&mut Self>) -> QString {
        match self.as_mut().rust_mut().library.pop_closed() {
            Some(c) => to_qjson(&serde_json::to_value(c).unwrap_or(Value::Null)),
            None => qs(""),
        }
    }

    pub fn closed_tabs(&self) -> QString {
        to_qjson(&serde_json::to_value(&self.library.closed).unwrap_or(Value::Null))
    }

    pub fn save_session(self: Pin<&mut Self>, json: &QString) {
        if self.start_private {
            return;
        }
        let _ = paths::write_atomic(&paths::data_dir().join("session.json"), s(json).as_bytes());
    }

    pub fn load_session(&self) -> QString {
        qs(std::fs::read_to_string(paths::data_dir().join("session.json")).unwrap_or_default())
    }

    pub fn heartbeat(&self, active: bool) {
        crate::core::watchdog::beat(active);
    }

    /// Recent UI freezes (newest last), for Settings › About.
    pub fn freeze_log(&self) -> QString {
        let t = std::fs::read_to_string(crate::core::watchdog::log_path()).unwrap_or_default();
        let lines: Vec<&str> = t.lines().collect();
        qs(lines[lines.len().saturating_sub(20)..].join("\n"))
    }

    pub fn flush(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().library.flush();
        blocker::save_stats();
    }

    // ── privacy ─────────────────────────────────────────────────────
    pub fn refresh_stats(mut self: Pin<&mut Self>) {
        let (b, a, t) = blocker::totals();
        self.as_mut().set_total_blocked(b as f64);
        self.as_mut().set_total_ads(a as f64);
        self.as_mut().set_total_trackers(t as f64);
    }

    pub fn site_report(&self, host: &QString) -> QString {
        let h = s(host);
        let (n, domains) = blocker::site_report(&h);
        to_qjson(&json!({ "host": h, "blocked": n, "domains": domains, "enabled": self.settings.blocker_enabled_for(&h) }))
    }

    pub fn privacy_report(&self) -> QString {
        let (b, a, t) = blocker::totals();
        let (rules, cosmetic, lists) = blocker::rule_count();
        let top: Vec<Value> = blocker::top_trackers(8).into_iter().map(|(d, n)| json!({ "domain": d, "sites": n })).collect();
        to_qjson(&json!({ "blocked": b, "ads": a, "trackers": t, "rules": rules, "cosmetic": cosmetic, "lists": lists, "loading": blocker::loading(), "top": top }))
    }

    pub fn reset_privacy_stats(self: Pin<&mut Self>) {
        blocker::reset_stats();
        self.refresh_stats();
    }

    pub fn reload_filter_lists(self: Pin<&mut Self>) -> QString {
        blocker::init(active_lists(&self.settings));
        apply_privacy(&self.settings);
        self.privacy_report()
    }

    pub fn filter_lists(&self) -> QString {
        to_qjson(&filterlists::describe(&self.settings.filter_lists, &self.settings.custom_filter_urls))
    }

    pub fn set_filter_list_enabled(mut self: Pin<&mut Self>, id: &QString, enabled: bool) {
        self.as_mut().rust_mut().settings.filter_lists.insert(s(id), enabled);
        self.rust().settings.save();
        self.as_mut().sync();
        blocker::init(active_lists(&self.settings));
    }

    /// Returns the new list's id, or "" if the URL is invalid / already added.
    pub fn add_filter_list_url(mut self: Pin<&mut Self>, url: &QString) -> QString {
        let u = s(url).trim().to_string();
        let ok = url::Url::parse(&u).map(|x| matches!(x.scheme(), "https" | "http")).unwrap_or(false);
        if !ok || self.settings.custom_filter_urls.contains(&u) {
            return qs("");
        }
        self.as_mut().rust_mut().settings.custom_filter_urls.push(u.clone());
        self.rust().settings.save();
        self.as_mut().sync();
        qs(filterlists::custom_id(&u))
    }

    pub fn remove_filter_list(mut self: Pin<&mut Self>, id: &QString) {
        let id = s(id);
        {
            let mut r = self.as_mut().rust_mut();
            r.settings.custom_filter_urls.retain(|u| filterlists::custom_id(u) != id);
            r.settings.filter_lists.remove(&id);
        }
        filterlists::remove(&id);
        self.rust().settings.save();
        self.as_mut().sync();
        blocker::init(active_lists(&self.settings));
    }

    /// `[{id, url}]` of enabled lists that need downloading.
    pub fn due_filter_lists(&self, force: bool) -> QString {
        let st = &self.settings;
        let days = if force { 0 } else { st.filter_update_days.max(1) as i64 };
        let list: Vec<Value> = if force {
            filterlists::subscriptions(&st.filter_lists, &st.custom_filter_urls)
                .into_iter()
                .filter(|x| x.4)
                .map(|(id, _, _, url, _)| json!({ "id": id, "url": url }))
                .collect()
        } else {
            filterlists::due(&st.filter_lists, &st.custom_filter_urls, days).into_iter().map(|(id, url)| json!({ "id": id, "url": url })).collect()
        };
        to_qjson(&Value::Array(list))
    }

    /// Save a downloaded list. Returns "" on success or an error message.
    pub fn store_filter_list(self: Pin<&mut Self>, id: &QString, text: &QString) -> QString {
        let id = s(id);
        match filterlists::store(&id, &String::from(text)) {
            Ok(_) => qs(""),
            Err(e) => {
                filterlists::record_error(&id, &e);
                qs(e)
            }
        }
    }

    pub fn filter_list_failed(self: Pin<&mut Self>, id: &QString, error: &QString) {
        filterlists::record_error(&s(id), &s(error));
    }

    /// # Safety
    /// `item` must be a live QQuickItem handed over by QML.
    pub unsafe fn send_mouse(&self, item: *mut cxx_qt::QObject, x: f64, y: f64, button: i32, clicks: i32, modifiers: i32, kind: i32) {
        if !item.is_null() {
            crate::bridge::native::ffi::send_mouse(item, x, y, button, clicks, modifiers, kind);
        }
    }

    /// # Safety
    /// `item` must be a live QQuickItem handed over by QML.
    pub unsafe fn send_key(&self, item: *mut cxx_qt::QObject, key: i32, modifiers: i32, text: &QString) {
        if !item.is_null() {
            crate::bridge::native::ffi::send_key(item, key, modifiers, &s(text));
        }
    }

    /// # Safety
    /// `item` must be a live QQuickItem handed over by QML.
    pub unsafe fn send_text(&self, item: *mut cxx_qt::QObject, text: &QString) {
        if !item.is_null() {
            crate::bridge::native::ffi::send_text(item, &s(text));
        }
    }

    /// # Safety
    /// `item` must be a live QQuickItem handed over by QML.
    pub unsafe fn send_wheel(&self, item: *mut cxx_qt::QObject, x: f64, y: f64, dx: f64, dy: f64) {
        if !item.is_null() {
            crate::bridge::native::ffi::send_wheel(item, x, y, dx, dy);
        }
    }

    /// Material 3 colour roles for a seed colour (JSON object of #rrggbb).
    pub fn material_scheme(&self, seed: &QString, dark: bool) -> QString {
        to_qjson(&crate::core::material::scheme(&s(seed), dark))
    }

    /// The colour roles for a mode and style (see core/themes.rs and
    /// assets/themes.zsp, written in Z#++).
    pub fn theme_scheme(&self, seed: &QString, mode: &QString, style: &QString) -> QString {
        to_qjson(&crate::core::themes::scheme(&s(seed), &s(mode), &s(style)))
    }

    /// Desktop notification (used by extensions' notifications API).
    pub fn notify(&self, title: &QString, body: &QString) {
        let _ = std::process::Command::new("notify-send").args(["-a", "OyNIx", &s(title), &s(body)]).spawn();
    }

    fn oynix_dir(&self, id: &QString) -> Option<std::path::PathBuf> {
        let id = s(id);
        self.extensions.iter().find(|e| e.id == id && e.kind == "oynix").map(|e| std::path::PathBuf::from(&e.path))
    }

    /// # Safety
    /// `host` must be a live QQuickItem and `api` a live ExtensionPanel, both from QML.
    pub unsafe fn load_extension_panel(&self, host: *mut cxx_qt::QObject, id: &QString, api: *mut cxx_qt::QObject) -> QString {
        let Some(dir) = self.oynix_dir(id) else { return qs("extension not found") };
        let qml = dir.join("graphics.qt");
        // Checked again here: the files may have been edited since install.
        match std::fs::read_to_string(&qml) {
            Ok(src) => {
                if let Err(e) = crate::core::qmlcheck::check(&src) {
                    return qs(e);
                }
            }
            Err(e) => return qs(format!("graphics.qt: {e}")),
        }
        let err = crate::bridge::native::ffi::load_panel(host, &qml.to_string_lossy(), &dir.to_string_lossy(), api);
        qs(err)
    }

    /// # Safety
    /// `host` must be a live QQuickItem from QML.
    pub unsafe fn unload_extension_panel(&self, host: *mut cxx_qt::QObject) {
        crate::bridge::native::ffi::unload_panel(host);
    }

    /// `extra.jsonl` records as a JSON array.
    pub fn extension_data(&self, id: &QString) -> QString {
        let data = self.oynix_dir(id).map(|d| extensions::read_data(&d).unwrap_or_default()).unwrap_or_default();
        to_qjson(&Value::Array(data))
    }

    /// The `oynix` object for an extension's `extra.html`: its data and
    /// messaging with the extension's code.ts.
    pub fn extension_page_api(&self, id: &QString) -> QString {
        let data = self.extension_data(id);
        let id_js = serde_json::to_string(&s(id)).unwrap_or_default();
        qs(extensions::PAGE_API.replace("__ID__", &id_js).replace("__DATA__", &String::from(&data)))
    }

    pub fn webchannel_script(&self) -> QString {
        qs(crate::bridge::native::ffi::webchannel_js())
    }

    fn webext_manifest(&self, web_id: &str) -> Option<(&extensions::Entry, Value)> {
        let e = self.extensions.iter().find(|e| e.enabled && (e.web_id == web_id || e.id == web_id))?;
        let m = std::fs::read_to_string(std::path::Path::new(&e.path).join("manifest.json")).ok().and_then(|t| serde_json::from_str(&t).ok())?;
        Some((e, m))
    }

    pub fn webext_info(&self, web_id: &QString) -> QString {
        let Some((e, m)) = self.webext_manifest(&s(web_id)) else { return qs("null") };
        let list = |k: &str| m[k].as_array().cloned().unwrap_or_default();
        let mut perms: Vec<Value> = list("permissions");
        perms.extend(list("optional_permissions"));
        let (named, mut origins): (Vec<Value>, Vec<Value>) = perms.into_iter().partition(|p| p.as_str().is_some_and(|x| !x.contains("://") && x != "<all_urls>"));
        origins.extend(list("host_permissions"));
        to_qjson(&json!({
            "id": e.id, "name": e.name, "permissions": named, "origins": origins,
            "developer": self.settings.ext_developer_mode,
            "powerful": e.power || (self.settings.ext_developer_mode && self.settings.ext_powerful_apis),
            "power": e.power,
        }))
    }

    pub fn webext_commands(&self) -> QString {
        let mut out = vec![];
        for e in self.extensions.iter().filter(|e| e.enabled && e.is_webext() && !e.web_id.is_empty()) {
            let Some((_, m)) = self.webext_manifest(&e.web_id) else { continue };
            let Some(cmds) = m["commands"].as_object() else { continue };
            for (name, c) in cmds {
                let k = &c["suggested_key"];
                let key = k["linux"].as_str().or_else(|| k["default"].as_str()).unwrap_or("");
                if !key.is_empty() {
                    out.push(json!({ "ext": e.web_id, "name": name, "key": key.replace("Command", "Ctrl").replace("MacCtrl", "Ctrl"), "description": c["description"] }));
                }
            }
        }
        to_qjson(&Value::Array(out))
    }

    pub fn webext_omnibox(&self) -> QString {
        let mut out = vec![];
        for e in self.extensions.iter().filter(|e| e.enabled && e.is_webext() && !e.web_id.is_empty()) {
            if let Some((_, m)) = self.webext_manifest(&e.web_id) {
                if let Some(k) = m["omnibox"]["keyword"].as_str() {
                    out.push(json!({ "ext": e.web_id, "name": e.name, "keyword": k }));
                }
            }
        }
        to_qjson(&Value::Array(out))
    }

    pub fn webext_set_observing(&self, requests: bool, cookies: bool) {
        let on = (self.settings.ext_developer_mode && self.settings.ext_powerful_apis)
            || self.extensions.iter().any(|e| e.enabled && e.power && e.is_webext());
        crate::core::extevents::set_observing(requests && on, cookies && on);
    }

    pub fn file_data_url(&self, path: &QString, mime: &QString, consume: bool) -> QString {
        let p = std::path::PathBuf::from(s(path));
        let Ok(bytes) = std::fs::read(&p) else { return qs("") };
        if consume {
            let _ = std::fs::remove_file(&p);
        }
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = format!("data:{};base64,", s(mime));
        for c in bytes.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            out.push(T[(n >> 18) as usize & 63] as char);
            out.push(T[(n >> 12) as usize & 63] as char);
            out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
            out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
        }
        qs(out)
    }

    pub fn search_url(&self, text: &QString) -> QString {
        qs(omnibox::search_url(&self.settings.search_engine, &s(text)))
    }

    pub fn remove_download_file(&self, path: &QString) -> bool {
        let p = std::path::PathBuf::from(s(path));
        let dir = std::path::PathBuf::from(&self.settings.download_dir);
        match (p.canonicalize(), dir.canonicalize()) {
            (Ok(f), Ok(d)) if f.starts_with(&d) && f.is_file() => std::fs::remove_file(f).is_ok(),
            _ => false,
        }
    }

    pub fn webext_drain_events(&self) -> QString {
        to_qjson(&crate::core::extevents::drain())
    }

    /// # Safety
    /// `profile` must be a live WebEngineProfile from QML.
    pub unsafe fn webext_cookies(&self, web_id: &QString, profile: *mut cxx_qt::QObject) -> QString {
        if let Err(e) = crate::bridge::extbridge::powerful_allowed(&s(web_id), "cookies") {
            return to_qjson(&json!({ "error": e }));
        }
        let all: Value = serde_json::from_str(&crate::bridge::native::ffi::cookies_all(profile)).unwrap_or_default();
        to_qjson(&json!({ "value": all }))
    }

    /// # Safety
    /// `profile` must be a live WebEngineProfile from QML.
    pub unsafe fn webext_cookie_set(&self, web_id: &QString, profile: *mut cxx_qt::QObject, cookie: &QString) -> QString {
        if let Err(e) = crate::bridge::extbridge::powerful_allowed(&s(web_id), "cookies") {
            return qs(e);
        }
        if crate::bridge::native::ffi::cookie_set(profile, &s(cookie)) { qs("") } else { qs("Invalid cookie (it needs at least a valid url)") }
    }

    /// # Safety
    /// `profile` must be a live WebEngineProfile from QML.
    pub unsafe fn webext_cookie_remove(&self, web_id: &QString, profile: *mut cxx_qt::QObject, url: &QString, name: &QString) -> QString {
        if let Err(e) = crate::bridge::extbridge::powerful_allowed(&s(web_id), "cookies") {
            return qs(e);
        }
        crate::bridge::native::ffi::cookie_remove(profile, &s(url), &s(name));
        qs("")
    }

    pub fn extension_has_permission(&self, id: &QString, permission: &QString) -> bool {
        let (id, p) = (s(id), s(permission));
        self.extensions.iter().any(|e| e.id == id && e.enabled && e.permissions.contains(&p))
    }

    pub fn browser_style_json(&self) -> QString {
        let want = &self.settings.browser_style;
        let theme = self
            .extensions
            .iter()
            .find(|e| !want.is_empty() && &e.id == want && e.enabled && e.has_theme)
            .and_then(|e| extensions::read_theme(std::path::Path::new(&e.path)).ok().flatten());
        to_qjson(&theme.unwrap_or_else(|| json!({})))
    }

    pub fn hypertheme_json(&self, dark: bool) -> QString {
        to_qjson(&crate::core::hyper::resolved(&self.settings.hypertheme, dark))
    }

    pub fn hyperthemes(&self) -> QString {
        to_qjson(&crate::core::hyper::list())
    }

    pub fn install_hypertheme(&self, path: &QString) -> QString {
        // Prefer OyNIx's own profile directory. Only use StonedGem's original
        // shared directory when the normal OyNIx installation fails.
        let oynix_data = crate::core::paths::data_dir();
        let p = s(path);
        let p = p.trim().trim_start_matches("file://");
        let source = std::path::Path::new(p);

        let result = match stonedgem_hypertheme::install(source) {
            Ok(file) => Ok(file),
            Err(oynix_error) => (|| -> Result<String, String> {
                let theme = stonedgem_hypertheme::Hypertheme::open(source)?;

                // Match StonedGem's profile layout:
                // <XDG_DATA_HOME>/stonedgem/<profile>/hyperthemes
                let data_home = std::env::var_os("XDG_DATA_HOME")
                    .filter(|value| !value.is_empty())
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        std::env::var_os("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|| std::path::PathBuf::from("."))
                            .join(".local/share")
                    });
                let shared_dir = data_home
                    .join("stonedgem")
                    .join(stonedgem_profile::profile())
                    .join("hyperthemes");
                std::fs::create_dir_all(&shared_dir)
                    .map_err(|e| format!("{}: {e}", shared_dir.display()))?;

                // Use the same safe filename rule as stonedgem_hypertheme::install.
                let slug: String = theme.manifest.name.to_lowercase().chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                    .collect::<String>()
                    .trim_matches('-')
                    .to_string();
                let file = format!("{}.hypertheme", if slug.is_empty() { "theme".into() } else { slug });
                let shared_file = shared_dir.join(&file);
                theme.write(&shared_file)?;

                // OyNIx lists and loads from its own profile directory. Link the
                // shared fallback there so a successful fallback is still visible.
                let oynix_dir = oynix_data.join("hyperthemes");
                std::fs::create_dir_all(&oynix_dir)
                    .map_err(|e| format!("{}: {e}", oynix_dir.display()))?;
                let visible_file = oynix_dir.join(&file);
                if let Ok(meta) = std::fs::symlink_metadata(&visible_file) {
                    if meta.file_type().is_symlink() {
                        std::fs::remove_file(&visible_file)
                            .map_err(|e| format!("{}: {e}", visible_file.display()))?;
                    } else {
                        return Err(format!("{} already exists and is not a symlink", visible_file.display()));
                    }
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink(&shared_file, &visible_file)
                    .map_err(|e| format!("couldn't link StonedGem fallback into OyNIx: {e}"))?;
                #[cfg(windows)]
                std::os::windows::fs::symlink_file(&shared_file, &visible_file)
                    .map_err(|e| format!("couldn't link StonedGem fallback into OyNIx: {e}"))?;
                #[cfg(not(any(unix, windows)))]
                std::fs::copy(&shared_file, &visible_file)
                    .map_err(|e| format!("couldn't expose StonedGem fallback in OyNIx: {e}"))?;

                Ok(file)
            })().map_err(|fallback_error| {
                format!("OyNIx install failed: {oynix_error}; StonedGem shared fallback failed: {fallback_error}")
            }),
        };

        to_qjson(&match result {
            Ok(file) => json!({ "file": file }),
            Err(e) => json!({ "error": e }),
        })
    }

    pub fn theme_studio_url(&self) -> QString {
        let shim = crate::core::hyper::studio_shim(&crate::bridge::native::ffi::webchannel_js());
        qs(crate::core::hyper::studio_page(&shim).unwrap_or_default())
    }

    pub fn clear_favicons(&self) -> i32 {
        crate::core::favicons::clear() as i32
    }

    pub fn browser_styles(&self) -> QString {
        let list: Vec<Value> = self.extensions.iter().filter(|e| e.enabled && (e.has_theme || (e.power && e.has_shape))).map(|e| json!({ "id": e.id, "name": e.name })).collect();
        to_qjson(&Value::Array(list))
    }

    /// file:// URL of `extra.html`, or "".
    pub fn extension_page_url(&self, id: &QString) -> QString {
        let Some(dir) = self.oynix_dir(id) else { return qs("") };
        // extra.html, or (power extensions) extra.md rendered at install.
        let power = self.extensions.iter().any(|e| e.id == s(id) && e.power);
        for name in ["extra.html", "extra.md.html"] {
            let p = dir.join(name);
            if p.exists() && (name == "extra.html" || power) {
                return qs(format!("file://{}", p.to_string_lossy()));
            }
        }
        qs("")
    }

    pub fn theme_shape_url(&self) -> QString {
        let want = &self.settings.browser_style;
        self.extensions
            .iter()
            .find(|e| !want.is_empty() && &e.id == want && e.enabled && e.power && e.has_shape)
            .map(|e| qs(format!("file://{}", std::path::Path::new(&e.path).join("theme-shape.qml").to_string_lossy())))
            .unwrap_or_default()
    }

    /// # Safety
    /// `profile` must be a live WebEngineProfile handed over by QML.
    pub unsafe fn clear_cookies(&self, profile: *mut cxx_qt::QObject) {
        if !profile.is_null() {
            crate::bridge::native::ffi::clear_cookies(profile);
        }
    }

    /// Bumps each time a new rule set is installed (lists loaded/updated).
    pub fn blocker_generation(&self) -> i32 {
        blocker::generation() as i32
    }

    /// Tell Chromium (prefers-color-scheme) which scheme pages should use.
    /// With "dark websites" off, pages always get the light scheme while
    /// OyNIx is dark; with theme "auto" the system setting is followed.
    pub fn apply_color_scheme(&self, dark: bool) {
        let st = &self.settings;
        let scheme = match st.website_appearance.as_str() {
            "dark" => 2,
            "light" => 1,
            _ if st.theme == "auto" => 0,
            _ if dark => 2,
            _ => 1,
        };
        crate::bridge::native::ffi::set_color_scheme(scheme);
    }

    /// Script that hides elements matched by cosmetic rules on `host`, or ""
    /// when nothing applies. Idempotent; runs in the isolated world.
    pub fn cosmetic_script(&self, host: &QString) -> QString {
        let sels = blocker::cosmetic_selectors(&s(host));
        if sels.is_empty() {
            return qs("");
        }
        let list = serde_json::to_string(&sels).unwrap_or_else(|_| "[]".into());
        qs(format!("({})({});", scripts::COSMETIC, list))
    }

    /// # Safety
    /// `profile` must be a live QObject pointer handed over by QML.
    pub unsafe fn attach_profile(&self, profile: *mut cxx_qt::QObject) {
        if !profile.is_null() {
            crate::bridge::native::ffi::attach_interceptor(profile);
        }
    }

    // ── scripts ─────────────────────────────────────────────────────
    pub fn reader_script(&self) -> QString {
        qs(scripts::READER)
    }

    pub fn reader_probe_script(&self) -> QString {
        qs(scripts::READER_PROBE)
    }

    /// Scripts to inject into every page, as `[{name, source, world, subframes}]`.
    /// Includes privacy shims plus the user's `*.user.js` / `*.user.css`.
    pub fn page_scripts(&self) -> QString {
        let st = &self.settings;
        let mut v: Vec<Value> = Vec::new();
        if st.send_gpc {
            v.push(json!({ "name": "oynix-gpc", "source": scripts::GPC, "main": true }));
        }
        if st.fingerprint_protection {
            v.push(json!({ "name": "oynix-fp", "source": scripts::ANTI_FINGERPRINT, "main": true }));
        }
        if st.webusb {
            v.push(json!({ "name": "oynix-webusb", "source": scripts::WEBUSB_MAIN, "main": true, "allFrames": false }));
            let relay = format!("{}\n{}", crate::bridge::native::ffi::webchannel_js(), scripts::WEBUSB_RELAY);
            v.push(json!({ "name": "oynix-webusb-relay", "source": relay, "main": false, "start": true, "allFrames": false }));
        }
        v.push(json!({ "name": "oynix-gallery", "source": scripts::GALLERY_MARK.replace("__OYNIX_VERSION__", crate::VERSION), "main": false, "greasemonkey": true }));
        for e in self.extensions.iter().filter(|e| e.enabled && e.kind == "oynix") {
            if let Some(src) = extensions::oynix_script(e) {
                v.push(json!({ "name": format!("oynix-ext-{}", e.id), "source": src, "main": e.world == "main", "greasemonkey": true }));
            }
        }
        if let Ok(rd) = std::fs::read_dir(paths::userscripts_dir()) {
            let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
            files.sort();
            for p in files {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let Ok(src) = std::fs::read_to_string(&p) else { continue };
                if name.ends_with(".user.js") {
                    v.push(json!({ "name": name, "source": src, "main": false }));
                } else if name.ends_with(".user.css") {
                    let css = serde_json::to_string(&src).unwrap_or_default();
                    let js = format!(
                        "(function(){{var s=document.createElement('style');s.textContent={css};(document.head||document.documentElement).appendChild(s);}})();"
                    );
                    v.push(json!({ "name": name, "source": js, "main": false }));
                }
            }
        }
        to_qjson(&Value::Array(v))
    }

    // ── extensions ──────────────────────────────────────────────────
    fn bump_extensions(mut self: Pin<&mut Self>) {
        // Power extensions can add filter rules; rebuild the blocker when
        // the set of those changes (installed, updated, switched on or off).
        if blocker::extension_filters_changed(&extensions::filter_files(&self.extensions)) {
            blocker::init(active_lists(&self.settings));
        }
        let r = self.extensions_revision + 1;
        self.as_mut().set_extensions_revision(r);
    }

    pub fn extensions(&self) -> QString {
        to_qjson(&serde_json::to_value(&self.extensions).unwrap_or(Value::Null))
    }

    /// Returns `{ok, error, extension}` as JSON.
    pub fn install_extension(mut self: Pin<&mut Self>, path_or_url: &QString) -> QString {
        let p = to_path(&s(path_or_url));
        let mut list = self.extensions.clone();
        let r = extensions::install(&p, &mut list);
        // Packages downloaded from the web are only needed until installed.
        if p.is_file() && p.starts_with(paths::cache_dir().join("extension-downloads")) {
            let _ = std::fs::remove_file(&p);
        }
        self.as_mut().rust_mut().extensions = list;
        self.as_mut().bump_extensions();
        match r {
            Ok(e) => to_qjson(&json!({ "ok": true, "error": "", "extension": e })),
            Err(err) => to_qjson(&json!({ "ok": false, "error": err })),
        }
    }

    pub fn remove_extension(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let mut list = self.extensions.clone();
        let ok = extensions::remove(&s(id), &mut list);
        self.as_mut().rust_mut().extensions = list;
        self.bump_extensions();
        ok
    }

    pub fn is_extension_pinned(&self, id: &QString) -> bool {
        let id = s(id);
        self.settings.pinned_extensions.iter().any(|p| *p == id)
    }

    pub fn set_extension_pinned(mut self: Pin<&mut Self>, id: &QString, pinned: bool) {
        let id = s(id);
        {
            let mut r = self.as_mut().rust_mut();
            r.settings.pinned_extensions.retain(|p| *p != id);
            if pinned {
                r.settings.pinned_extensions.push(id);
            }
        }
        self.rust().settings.save();
        self.bump_extensions();
    }

    pub fn remember_favicon(&self, page_url: &QString, icon: &QString) {
        crate::core::favicons::remember(&s(page_url), &s(icon));
    }

    pub fn cached_favicon(&self, url: &QString) -> QString {
        qs(crate::core::favicons::cached(&s(url)).map(|p| format!("file://{}", p.to_string_lossy())).unwrap_or_default())
    }

    pub fn network_online(&self) -> bool {
        crate::core::paths::has_default_route()
    }

    pub fn extension_icon_for(&self, url: &QString) -> QString {
        let u = s(url);
        let Some(rest) = u.strip_prefix("chrome-extension://") else { return qs("") };
        let web_id = rest.split('/').next().unwrap_or("");
        qs(self.extensions.iter().find(|e| !web_id.is_empty() && e.web_id == web_id).map(|e| e.icon.as_str()).unwrap_or(""))
    }

    pub fn set_extension_enabled(mut self: Pin<&mut Self>, id: &QString, enabled: bool) {
        let id = s(id);
        if let Some(e) = self.as_mut().rust_mut().extensions.iter_mut().find(|e| e.id == id) {
            e.enabled = enabled;
        }
        extensions::save(&self.extensions);
        self.bump_extensions();
    }

    /// Returns the folder of the new extension, or "" on failure.
    pub fn create_extension(mut self: Pin<&mut Self>, name: &QString) -> QString {
        let mut list = self.extensions.clone();
        let r = extensions::create_template(&s(name), &mut list);
        self.as_mut().rust_mut().extensions = list;
        self.as_mut().bump_extensions();
        qs(r.map(|e| e.path).unwrap_or_default())
    }

    pub fn net_call(mut self: Pin<&mut Self>, ext: &QString, call_json: &QString) -> QString {
        use crate::core::netrules;
        let ext = s(ext);
        let c0: Value = serde_json::from_str(&s(call_json)).unwrap_or_default();
        // An extension stopped (off or removed): drop what it set, always allowed.
        if c0["op"] == "forget" {
            if netrules::forget(&ext) {
                blocker::init(active_lists(&self.settings));
            }
            return to_qjson(&json!({ "ok": true, "error": "" }));
        }
        if !self.extensions.iter().any(|e| e.id == ext && e.enabled && e.power && e.has_network) {
            return to_qjson(&json!({ "ok": false, "error": "not a running network.ts" }));
        }
        let c: Value = serde_json::from_str(&s(call_json)).unwrap_or_default();
        let r: Result<(), String> = match c["op"].as_str().unwrap_or("") {
            "block" => {
                let rules = match &c["rules"] {
                    Value::Array(a) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("\n"),
                    Value::String(t) => t.clone(),
                    _ => String::new(),
                };
                if netrules::set_block(&ext, &rules) {
                    blocker::init(active_lists(&self.settings));
                }
                Ok(())
            }
            "header" => netrules::add_header(&ext, c["pattern"].as_str().unwrap_or("*"), c["name"].as_str().unwrap_or(""), c["value"].as_str().unwrap_or("")),
            "clearHeaders" => {
                netrules::clear_headers(&ext);
                Ok(())
            }
            "observe" => {
                netrules::set_observing(&ext, c["on"].as_bool().unwrap_or(true));
                Ok(())
            }
            other => Err(format!("unknown call “{other}”")),
        };
        let _ = self.as_mut();
        to_qjson(&match r {
            Ok(()) => json!({ "ok": true, "error": "" }),
            Err(e) => json!({ "ok": false, "error": e }),
        })
    }

    pub fn net_drain(&self) -> QString {
        to_qjson(&crate::core::netrules::drain())
    }

    pub fn network_html(&self, id: &QString, token: &QString) -> QString {
        let id = s(id);
        let Some(e) = self.extensions.iter().find(|e| e.id == id && e.enabled && e.power && e.has_network) else { return qs("") };
        match extensions::network_html(e, &s(token)) {
            Ok(h) => qs(h),
            Err(err) => {
                eprintln!("oynix: network.ts of {}: {err}", e.name);
                qs("")
            }
        }
    }

    pub fn widget_html(&self, id: &QString, theme_css: &QString) -> QString {
        let id = s(id);
        let Some(e) = self.extensions.iter().find(|e| e.id == id && e.enabled && e.has_widget) else { return qs("") };
        match extensions::widget_html(e, &s(theme_css)) {
            Ok(h) => qs(h),
            Err(err) => qs(format!("<!doctype html><body style=\"font:13px sans-serif;color:#b00;padding:12px\">{}</body>", err.replace('<', "&lt;"))),
        }
    }

    pub fn extensions_dir(&self) -> QString {
        qs(extensions::root().to_string_lossy())
    }

    pub fn set_extension_web_id(mut self: Pin<&mut Self>, id: &QString, web_id: &QString) {
        let (id, w) = (s(id), s(web_id));
        let mut changed = false;
        if let Some(e) = self.as_mut().rust_mut().extensions.iter_mut().find(|e| e.id == id) {
            if e.web_id != w {
                e.web_id = w;
                changed = true;
            }
        }
        if changed {
            extensions::save(&self.extensions);
            self.bump_extensions();
        }
    }

    pub fn consume_first_run(mut self: Pin<&mut Self>, id: &QString) -> bool {
        let id = s(id);
        let mut was = false;
        if let Some(e) = self.as_mut().rust_mut().extensions.iter_mut().find(|e| e.id == id) {
            was = e.first_run;
            e.first_run = false;
        }
        if was {
            extensions::save(&self.extensions);
        }
        was
    }

    /// Scripts a web view needs for Chrome/Firefox content scripts: the
    /// WebChannel client, the OyNIx runtime, the view's tab id, then every
    /// enabled extension's content scripts. Empty when none are active.
    pub fn extension_scripts(&self, tab_id: i32) -> QString {
        let mut v: Vec<Value> = Vec::new();
        for e in self.extensions.iter().filter(|e| e.enabled && e.is_webext() && !e.web_id.is_empty()) {
            v.extend(extensions::content_scripts(e));
        }
        if v.is_empty() {
            return qs("[]");
        }
        let mut pre = vec![
            json!({ "name": "oynix-webchannel", "source": crate::bridge::native::ffi::webchannel_js(), "main": false, "start": true, "allFrames": true }),
            json!({ "name": "oynix-ext-runtime", "source": format!("var __oynixTabId = {tab_id};\n{}", crate::core::ext_runtime::RUNTIME), "main": false, "start": true, "allFrames": true }),
        ];
        pre.extend(v);
        to_qjson(&Value::Array(pre))
    }

    /// Scripts for extension pages (popup / options / background), which run
    /// in the page's main world.
    pub fn extension_page_scripts(&self, kind: &QString) -> QString {
        let ctx = extensions::page_contexts(&self.extensions, &s(kind));
        let boot = format!("{}\n__oynixExtPage({});", crate::core::ext_runtime::RUNTIME, ctx);
        to_qjson(&json!([
            { "name": "oynix-webchannel", "source": crate::bridge::native::ffi::webchannel_js(), "main": true, "start": true, "allFrames": false },
            { "name": "oynix-ext-page", "source": boot, "main": true, "start": true, "allFrames": false }
        ]))
    }

    pub fn extension_tab_page_scripts(&self, tab_id: i32) -> QString {
        if !self.extensions.iter().any(|e| e.enabled && e.is_webext() && !e.web_id.is_empty()) {
            return qs("[]");
        }
        let ctx = extensions::page_contexts(&self.extensions, "page-tab");
        let guard = "if (location.protocol === 'chrome-extension:')";
        let channel = format!("{guard} {{ (function () {{\n{}\nwindow.QWebChannel = QWebChannel;\n}})(); }}", crate::bridge::native::ffi::webchannel_js());
        let boot = format!("{guard} {{ window.__oynixTabId = {tab_id};\n{}\n__oynixExtPage({ctx}); }}", crate::core::ext_runtime::RUNTIME);
        to_qjson(&json!([
            { "name": "oynix-webchannel-page", "source": channel, "main": true, "start": true, "allFrames": false },
            { "name": "oynix-ext-tab-page", "source": boot, "main": true, "start": true, "allFrames": false }
        ]))
    }

    // ── system ──────────────────────────────────────────────────────
    pub fn profiles(&self) -> QString {
        to_qjson(&json!(paths::list_profiles()))
    }

    pub fn launch_profile(&self, name: &QString, private_window: bool) -> bool {
        let Ok(exe) = std::env::current_exe() else { return false };
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--profile").arg(s(name));
        if private_window {
            cmd.arg("--private");
        }
        if let Some(g) = crate::launch_env().gpu_arg.as_deref() {
            cmd.arg("--gpu").arg(g);
        }
        cmd.spawn().is_ok()
    }

    pub fn open_externally(&self, path_or_url: &QString) -> bool {
        std::process::Command::new("xdg-open").arg(s(path_or_url)).spawn().is_ok()
    }

    pub fn make_default_browser(&self) -> bool {
        std::process::Command::new("xdg-settings")
            .args(["set", "default-web-browser", "oynix.desktop"])
            .status()
            .map(|st| st.success())
            .unwrap_or(false)
    }

    pub fn config_path(&self, which: &QString) -> QString {
        let p = match s(which).as_str() {
            "filters" => paths::filters_dir(),
            "userscripts" => paths::userscripts_dir(),
            "data" => paths::data_dir(),
            "cache" => paths::cache_dir(),
            _ => paths::config_dir(),
        };
        qs(p.to_string_lossy())
    }

    pub fn format_bytes(&self, bytes: f64) -> QString {
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut b = bytes.max(0.0);
        let mut i = 0;
        while b >= 1024.0 && i < units.len() - 1 {
            b /= 1024.0;
            i += 1;
        }
        qs(if i == 0 { format!("{b:.0} {}", units[i]) } else { format!("{b:.1} {}", units[i]) })
    }

    pub fn local_path(&self, url: &QString) -> QString {
        qs(to_path(&s(url)).to_string_lossy())
    }
}

impl cxx_qt::Initialize for qobject::Core {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut().refresh_stats();
    }
}
