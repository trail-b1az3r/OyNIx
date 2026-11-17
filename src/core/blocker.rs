//! The content blocker. Moved to StonedGem (`stonedgem-blocker`); OyNIx
//! supplies the filter files of its enabled power extensions.

pub use stonedgem_blocker::blocker::*;

use super::filterlists::Category;

/// Rebuild the rules from the built-ins, `lists`, the user's filters folder,
/// enabled extensions' filters.txt and network.ts rules.
pub fn init(lists: Vec<(String, Category)>) {
    super::paths::filters_dir(); // the folder name is OyNIx's
    set_extension_filters(super::extensions::filter_files(&super::extensions::load()));
    stonedgem_blocker::blocker::init(lists);
}
