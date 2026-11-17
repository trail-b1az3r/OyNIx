//! Profile folders. Moved to StonedGem (`stonedgem-profile`); OyNIx keeps
//! its folder name, `oynix`, so existing profiles stay where they are:
//! `~/.config/oynix/<profile>`, `~/.local/share/oynix/<profile>`,
//! `~/.cache/oynix/<profile>`.

use std::path::PathBuf;

pub use stonedgem_profile::{has_default_route, profile, write_atomic};

fn oynix() {
    stonedgem_profile::set_app("oynix");
}

/// Select the active profile. Must be called once, before any path lookup.
pub fn set_profile(name: &str) {
    oynix();
    stonedgem_profile::set_profile(name);
}

pub fn config_dir() -> PathBuf {
    oynix();
    stonedgem_profile::config_dir()
}

pub fn data_dir() -> PathBuf {
    oynix();
    stonedgem_profile::data_dir()
}

pub fn cache_dir() -> PathBuf {
    oynix();
    stonedgem_profile::cache_dir()
}

pub fn filters_dir() -> PathBuf {
    oynix();
    stonedgem_profile::filters_dir()
}

pub fn userscripts_dir() -> PathBuf {
    oynix();
    stonedgem_profile::userscripts_dir()
}

pub fn downloads_dir() -> PathBuf {
    stonedgem_profile::downloads_dir()
}

pub fn list_profiles() -> Vec<String> {
    oynix();
    stonedgem_profile::list_profiles()
}
