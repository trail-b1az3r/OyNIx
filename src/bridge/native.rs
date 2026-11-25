//! FFI between the Rust core and the small C++ glue layer in `cpp/`.

#[cxx::bridge(namespace = "oynix")]
pub mod ffi {
    extern "Rust" {
        fn should_block(url: &str, first_party: &str, resource_type: i32) -> bool;
        fn strip_tracking(url: &str) -> String;
        fn privacy_flags() -> u32;
        fn record_freeze_recovery();
        fn extra_request_headers(url: &str) -> String;
        fn observe_request(url: &str, first_party: &str, method: &str, resource_type: i32, headers_json: &str, blocked: bool);
        fn cookie_changed(cookie_json: &str, removed: bool);
    }

    unsafe extern "C++" {
        include!("oynix/cpp/native.h");

        #[namespace = ""]
        type QObject = cxx_qt::QObject;

        fn prepare_graphics(backend: i32);
        fn enable_window_alpha();
        fn window_alpha() -> bool;
        fn init_webengine();
        fn start_expose_guard();
        fn set_app_metadata(profile: &str);
        unsafe fn attach_interceptor(profile: *mut QObject);
        unsafe fn clear_cookies(profile: *mut QObject);
        unsafe fn load_panel(host: *mut QObject, qml_path: &str, ext_dir: &str, api: *mut QObject) -> String;
        unsafe fn unload_panel(host: *mut QObject);
        unsafe fn cookies_all(profile: *mut QObject) -> String;
        unsafe fn cookie_set(profile: *mut QObject, cookie_json: &str) -> bool;
        unsafe fn cookie_remove(profile: *mut QObject, url: &str, name: &str) -> bool;
        fn active_graphics_api() -> String;
        fn qt_runtime_version() -> String;
        fn webchannel_js() -> String;
        fn set_color_scheme(scheme: i32);
        unsafe fn send_mouse(item: *mut QObject, x: f64, y: f64, button: i32, clicks: i32, modifiers: i32, kind: i32);
        unsafe fn send_key(item: *mut QObject, key: i32, modifiers: i32, text: &str);
        unsafe fn send_text(item: *mut QObject, text: &str);
        unsafe fn send_wheel(item: *mut QObject, x: f64, y: f64, dx: f64, dy: f64);
    }
}

use std::sync::atomic::{AtomicU32, Ordering};

/// bit 0 = DNT, bit 1 = GPC, bit 2 = strip tracking params, bit 3 = block third-party cookies
static FLAGS: AtomicU32 = AtomicU32::new(0b1110);

pub fn set_privacy_flags(dnt: bool, gpc: bool, strip: bool, third_party_cookies: bool) {
    FLAGS.store((dnt as u32) | ((gpc as u32) << 1) | ((strip as u32) << 2) | ((third_party_cookies as u32) << 3), Ordering::Relaxed);
}

fn privacy_flags() -> u32 {
    FLAGS.load(Ordering::Relaxed)
}

fn record_freeze_recovery() {
    crate::core::watchdog::record_recovery();
}

/// Headers network.ts asked for, as "name\nvalue\n…" ("" almost always).
fn extra_request_headers(url: &str) -> String {
    let mut out = String::new();
    for (k, v) in crate::core::netrules::headers_for(url) {
        out.push_str(&k);
        out.push('\n');
        out.push_str(&v);
        out.push('\n');
    }
    out
}

fn observe_request(url: &str, first_party: &str, method: &str, resource_type: i32, headers_json: &str, blocked: bool) {
    crate::core::netrules::observe(url, first_party, method, crate::core::extevents::type_name(resource_type), blocked);
    let headers = serde_json::from_str(headers_json).unwrap_or(serde_json::Value::Array(vec![]));
    crate::core::extevents::request(url, first_party, method, resource_type, headers, blocked);
}

fn cookie_changed(cookie_json: &str, removed: bool) {
    if let Ok(v) = serde_json::from_str(cookie_json) {
        crate::core::extevents::cookie(v, removed);
    }
}

fn should_block(url: &str, first_party: &str, resource_type: i32) -> bool {
    let blocked = crate::core::blocker::should_block(url, first_party, resource_type);
    crate::core::agent::log_request(url, first_party, resource_type, blocked);
    blocked
}

fn strip_tracking(url: &str) -> String {
    // Called for every main-frame navigation; record it for network_requests.
    crate::core::agent::log_request(url, url, 0, false);
    crate::core::omnibox::strip_tracking(url)
}
