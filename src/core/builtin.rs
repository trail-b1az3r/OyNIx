//! Extensions that ship with OyNIx, in the ordinary .oynix format (sources
//! in builtin/). They're compiled into the binary, installed on first run
//! and updated when OyNIx updates. They can be turned off in Settings but
//! not removed.

use super::extensions::{self, Entry};

macro_rules! files {
    ($dir:literal: $($f:literal),+) => {
        &[$(($f, include_str!(concat!("../../builtin/", $dir, "/", $f)))),+]
    };
}

/// (folder, files)
pub const BUILTINS: &[(&str, &[(&str, &str)])] = &[
    ("firefox-addons", files!("firefox-addons": "oynix.json", "code.ts", "content.css")),
    ("scraper", files!("scraper": "oynix.json", "code.ts", "content.css", "graphics.qt")),
    ("graphite", files!("graphite": "oynix.json", "code.ts", "content.css", "theme.json")),
];

fn version_of(files: &[(&str, &str)]) -> String {
    files
        .iter()
        .find(|(n, _)| *n == "oynix.json")
        .and_then(|(_, t)| serde_json::from_str::<serde_json::Value>(t).ok())
        .and_then(|v| v["version"].as_str().map(String::from))
        .unwrap_or_default()
}

fn name_of(files: &[(&str, &str)]) -> String {
    files
        .iter()
        .find(|(n, _)| *n == "oynix.json")
        .and_then(|(_, t)| serde_json::from_str::<serde_json::Value>(t).ok())
        .and_then(|v| v["name"].as_str().map(String::from))
        .unwrap_or_default()
}

/// Install missing or outdated built-ins. Keeps each one's on/off state.
/// Returns true when the list changed.
pub fn sync(list: &mut Vec<Entry>) -> bool {
    let mut changed = false;
    for (dir, files) in BUILTINS {
        let version = version_of(files);
        let name = name_of(files);
        let current = list.iter().find(|e| e.builtin && e.name == name);
        if current.is_some_and(|e| e.version == version && std::path::Path::new(&e.path).join("oynix.json").exists()) {
            continue;
        }
        let tmp = std::env::temp_dir().join(format!("oynix-builtin-{dir}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        if std::fs::create_dir_all(&tmp).is_err() {
            continue;
        }
        for (f, text) in files.iter() {
            let _ = std::fs::write(tmp.join(f), text);
        }
        match extensions::install(&tmp, list) {
            Ok(e) => {
                if let Some(x) = list.iter_mut().find(|x| x.id == e.id) {
                    x.builtin = true;
                }
                changed = true;
            }
            Err(err) => eprintln!("oynix: built-in extension {dir}: {err}"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
    if changed {
        extensions::save(list);
    }
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtins_are_valid() {
        for (dir, files) in super::BUILTINS {
            let tmp = std::env::temp_dir().join(format!("oynix-builtin-test-{dir}"));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp).unwrap();
            for (f, t) in files.iter() {
                std::fs::write(tmp.join(f), t).unwrap();
            }
            crate::core::extensions::compile_code(&tmp).unwrap_or_else(|e| panic!("{dir}: {e}"));
            if let Some((_, qml)) = files.iter().find(|(f, _)| *f == "graphics.qt") {
                crate::core::qmlcheck::check(qml).unwrap_or_else(|e| panic!("{dir}: {e}"));
            }
            if files.iter().any(|(f, _)| *f == "theme.json") {
                assert!(crate::core::extensions::read_theme(&tmp).unwrap().is_some());
            }
        }
    }
}
