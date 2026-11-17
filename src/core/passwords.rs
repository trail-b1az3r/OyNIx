//! Saved passwords. The store moved to StonedGem (`stonedgem-vault`); OyNIx
//! keeps it in the profile's data folder.

pub use stonedgem_vault::passwords::{origin_of, parse_csv, Store};

pub fn load() -> Store {
    stonedgem_vault::passwords::load(&super::paths::data_dir())
}

pub fn save(store: &Store) -> Result<(), String> {
    stonedgem_vault::passwords::save(&super::paths::data_dir(), store)
}
