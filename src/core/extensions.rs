//! Extension installer and registry. Moved to StonedGem
//! (`stonedgem-extensions`); OyNIx supplies its name, trusted keys, the QML
//! panel checker and Qt's WebChannel script (see [`init_host`]).

pub use stonedgem_extensions::registry::*;

/// Tell the extension crate what OyNIx provides. Called once at start-up,
/// after the profile is chosen; later calls do nothing.
pub fn init_host() {
    stonedgem_extensions::host::set(stonedgem_extensions::Host {
        name: "OyNIx",
        trusted_keys: super::poweroynix::TRUSTED_KEYS,
        check_qml: Some(super::qmlcheck::check),
        webchannel_js: Some(crate::bridge::native::ffi::webchannel_js),
    });
}
