//! `.poweroynix` verification. The checking moved to StonedGem
//! (`stonedgem-poweroynix`); OyNIx supplies whom it trusts:
//! `packaging/poweroynix/trusted-keys.txt`, compiled in.

use std::path::Path;

pub use stonedgem_poweroynix::{is_power_dir, POWER_PERMISSIONS};

const TRUSTED_KEYS: &str = include_str!("../../packaging/poweroynix/trusted-keys.txt");

/// Check `dir`'s `oynix.sig` against OyNIx's trusted keys. Returns who verified it.
pub fn verify(dir: &Path) -> Result<String, String> {
    stonedgem_poweroynix::verify(dir, TRUSTED_KEYS)
}
