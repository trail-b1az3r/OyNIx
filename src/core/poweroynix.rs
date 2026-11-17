//! `.poweroynix` verification moved to StonedGem (`stonedgem-poweroynix`,
//! called by `stonedgem-extensions`). OyNIx supplies whom it trusts:
//! `packaging/poweroynix/trusted-keys.txt`, compiled in.

pub const TRUSTED_KEYS: &str = include_str!("../../packaging/poweroynix/trusted-keys.txt");
