//! FFI between the Rust core and the small C++ glue layer in `cpp/`.

#[cxx::bridge(namespace = "oynix")]
pub mod ffi {
    extern "Rust" {
        fn should_block(url: &str, first_party: &str, resource_type: i32) -> bool;
        fn strip_tracking(url: &str) -> String;
        fn privacy_flags() -> u32;
    }

    unsafe extern "C++" {
        include!("oynix/cpp/native.h");

        #[namespace = ""]
        type QObject = cxx_qt::QObject;

        fn prepare_graphics(backend: i32);
        fn init_webengine();
        fn set_app_metadata(profile: &str);
        unsafe fn attach_interceptor(profile: *mut QObject);
        fn active_graphics_api() -> String;
        fn qt_runtime_version() -> String;
        fn webchannel_js() -> String;
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

fn should_block(url: &str, first_party: &str, resource_type: i32) -> bool {
    crate::core::blocker::should_block(url, first_party, resource_type)
}

fn strip_tracking(url: &str) -> String {
    crate::core::omnibox::strip_tracking(url)
}
