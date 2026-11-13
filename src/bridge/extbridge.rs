//! `ExtBridge` — the object extension contexts reach over Qt WebChannel
//! (registered as `oynixExtensions`). It stores `chrome.storage` data and
//! routes messages between content scripts, popups, options pages and
//! background contexts. Tab operations are forwarded to QML, which owns the
//! tabs, via the `tabsRequest` signal.

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
        type ExtBridge = super::ExtBridgeRust;

        #[qsignal]
        fn message(self: Pin<&mut ExtBridge>, ext: QString, target: QString, payload: QString);
        #[qsignal]
        fn replied(self: Pin<&mut ExtBridge>, ext: QString, id: QString, response: QString);
        #[qsignal]
        fn storage_changed(self: Pin<&mut ExtBridge>, ext: QString, area: QString, changes: QString);
        #[qsignal]
        fn tabs_request(self: Pin<&mut ExtBridge>, ext: QString, id: QString, op: QString, args: QString);
        #[qsignal]
        fn tabs_result(self: Pin<&mut ExtBridge>, id: QString, result: QString);
        #[qsignal]
        fn ext_event(self: Pin<&mut ExtBridge>, name: QString, args: QString);
        #[qsignal]
        fn badge_changed(self: Pin<&mut ExtBridge>, ext: QString, text: QString, color: QString, title: QString);
        #[qsignal]
        fn options_requested(self: Pin<&mut ExtBridge>, ext: QString);

        #[qinvokable]
        fn post(self: Pin<&mut ExtBridge>, ext: &QString, target: &QString, payload: &QString);
        #[qinvokable]
        fn reply(self: Pin<&mut ExtBridge>, ext: &QString, id: &QString, response: &QString);
        #[qinvokable]
        fn storage_get(self: Pin<&mut ExtBridge>, ext: &QString, area: &QString, keys: &QString) -> QString;
        #[qinvokable]
        fn storage_set(self: Pin<&mut ExtBridge>, ext: &QString, area: &QString, items: &QString) -> bool;
        #[qinvokable]
        fn storage_remove(self: Pin<&mut ExtBridge>, ext: &QString, area: &QString, keys: &QString) -> bool;
        #[qinvokable]
        fn storage_clear(self: Pin<&mut ExtBridge>, ext: &QString, area: &QString) -> bool;
        #[qinvokable]
        fn tabs(self: Pin<&mut ExtBridge>, ext: &QString, id: &QString, op: &QString, args: &QString);
        #[qinvokable]
        fn tabs_reply(self: Pin<&mut ExtBridge>, id: &QString, result: &QString);
        #[qinvokable]
        fn dispatch(self: Pin<&mut ExtBridge>, name: &QString, args: &QString);
        #[qinvokable]
        fn set_badge(self: Pin<&mut ExtBridge>, ext: &QString, text: &QString, color: &QString, title: &QString) -> bool;
        #[qinvokable]
        fn open_options(self: Pin<&mut ExtBridge>, ext: &QString) -> bool;
        #[qinvokable]
        fn forget(self: Pin<&mut ExtBridge>, ext: &QString);
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::core::paths;

#[derive(Default)]
pub struct ExtBridgeRust {
    /// (extension id, area) -> items
    stores: HashMap<(String, String), Map<String, Value>>,
}

fn s(q: &QString) -> String {
    String::from(q)
}

fn qs(v: impl AsRef<str>) -> QString {
    QString::from(v.as_ref())
}

/// Extension ids are 32 chars a–p; anything else is rejected so a page can
/// never make us touch arbitrary files.
fn valid_ext(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric())
}

fn valid_area(a: &str) -> bool {
    matches!(a, "local" | "sync" | "session" | "managed")
}

fn store_file(ext: &str, area: &str) -> std::path::PathBuf {
    let dir = paths::data_dir().join("extension-storage");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{ext}.{area}.json"))
}

impl ExtBridgeRust {
    fn store(&mut self, ext: &str, area: &str) -> &mut Map<String, Value> {
        self.stores.entry((ext.to_string(), area.to_string())).or_insert_with(|| {
            if area == "session" || area == "managed" {
                return Map::new();
            }
            std::fs::read(store_file(ext, area))
                .ok()
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .and_then(|v| if let Value::Object(m) = v { Some(m) } else { None })
                .unwrap_or_default()
        })
    }

    fn persist(&self, ext: &str, area: &str) {
        if area == "session" || area == "managed" {
            return;
        }
        if let Some(m) = self.stores.get(&(ext.to_string(), area.to_string())) {
            let _ = paths::write_atomic(&store_file(ext, area), Value::Object(m.clone()).to_string().as_bytes());
        }
    }
}

/// `chrome.storage.*.get` key semantics: null → all, string, array, or
/// object of defaults.
pub fn select(store: &Map<String, Value>, keys: &Value) -> Map<String, Value> {
    match keys {
        Value::Null => store.clone(),
        Value::String(k) => store.get(k).map(|v| Map::from_iter([(k.clone(), v.clone())])).unwrap_or_default(),
        Value::Array(ks) => ks
            .iter()
            .filter_map(|k| k.as_str())
            .filter_map(|k| store.get(k).map(|v| (k.to_string(), v.clone())))
            .collect(),
        Value::Object(defaults) => defaults
            .iter()
            .map(|(k, d)| (k.clone(), store.get(k).cloned().unwrap_or_else(|| d.clone())))
            .collect(),
        _ => Map::new(),
    }
}

fn key_list(keys: &Value) -> Vec<String> {
    match keys {
        Value::String(k) => vec![k.clone()],
        Value::Array(a) => a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect(),
        _ => vec![],
    }
}

impl qobject::ExtBridge {
    pub fn post(self: Pin<&mut Self>, ext: &QString, target: &QString, payload: &QString) {
        if valid_ext(&s(ext)) {
            self.message(ext.clone(), target.clone(), payload.clone());
        }
    }

    pub fn reply(self: Pin<&mut Self>, ext: &QString, id: &QString, response: &QString) {
        self.replied(ext.clone(), id.clone(), response.clone());
    }

    pub fn storage_get(mut self: Pin<&mut Self>, ext: &QString, area: &QString, keys: &QString) -> QString {
        let (ext, area) = (s(ext), s(area));
        if !valid_ext(&ext) || !valid_area(&area) {
            return qs("{}");
        }
        let keys: Value = serde_json::from_str(&s(keys)).unwrap_or(Value::Null);
        let store = self.as_mut().rust_mut().get_mut().store(&ext, &area).clone();
        qs(Value::Object(select(&store, &keys)).to_string())
    }

    pub fn storage_set(mut self: Pin<&mut Self>, ext: &QString, area: &QString, items: &QString) -> bool {
        let (e, a) = (s(ext), s(area));
        if !valid_ext(&e) || !valid_area(&a) || a == "managed" {
            return false;
        }
        let Ok(Value::Object(items)) = serde_json::from_str::<Value>(&s(items)) else { return false };
        let mut changes = Map::new();
        {
            let mut rust = self.as_mut().rust_mut();
            let store = rust.store(&e, &a);
            for (k, v) in items {
                let old = store.insert(k.clone(), v.clone());
                if old.as_ref() != Some(&v) {
                    let mut c = Map::new();
                    if let Some(o) = old {
                        c.insert("oldValue".into(), o);
                    }
                    c.insert("newValue".into(), v);
                    changes.insert(k, Value::Object(c));
                }
            }
            rust.persist(&e, &a);
        }
        if !changes.is_empty() {
            self.storage_changed(ext.clone(), area.clone(), qs(Value::Object(changes).to_string()));
        }
        true
    }

    pub fn storage_remove(mut self: Pin<&mut Self>, ext: &QString, area: &QString, keys: &QString) -> bool {
        let (e, a) = (s(ext), s(area));
        if !valid_ext(&e) || !valid_area(&a) {
            return false;
        }
        let keys = key_list(&serde_json::from_str(&s(keys)).unwrap_or(Value::Null));
        let mut changes = Map::new();
        {
            let mut rust = self.as_mut().rust_mut();
            let store = rust.store(&e, &a);
            for k in keys {
                if let Some(old) = store.remove(&k) {
                    changes.insert(k, json!({ "oldValue": old }));
                }
            }
            rust.persist(&e, &a);
        }
        if !changes.is_empty() {
            self.storage_changed(ext.clone(), area.clone(), qs(Value::Object(changes).to_string()));
        }
        true
    }

    pub fn storage_clear(mut self: Pin<&mut Self>, ext: &QString, area: &QString) -> bool {
        let (e, a) = (s(ext), s(area));
        if !valid_ext(&e) || !valid_area(&a) {
            return false;
        }
        let changes: Map<String, Value> = {
            let mut rust = self.as_mut().rust_mut();
            let store = rust.store(&e, &a);
            let c = store.iter().map(|(k, v)| (k.clone(), json!({ "oldValue": v }))).collect();
            store.clear();
            rust.persist(&e, &a);
            c
        };
        if !changes.is_empty() {
            self.storage_changed(ext.clone(), area.clone(), qs(Value::Object(changes).to_string()));
        }
        true
    }

    pub fn tabs(self: Pin<&mut Self>, ext: &QString, id: &QString, op: &QString, args: &QString) {
        if valid_ext(&s(ext)) {
            self.tabs_request(ext.clone(), id.clone(), op.clone(), args.clone());
        }
    }

    pub fn tabs_reply(self: Pin<&mut Self>, id: &QString, result: &QString) {
        self.tabs_result(id.clone(), result.clone());
    }

    pub fn dispatch(self: Pin<&mut Self>, name: &QString, args: &QString) {
        self.ext_event(name.clone(), args.clone());
    }

    pub fn set_badge(self: Pin<&mut Self>, ext: &QString, text: &QString, color: &QString, title: &QString) -> bool {
        if !valid_ext(&s(ext)) {
            return false;
        }
        let t: String = s(text).chars().take(4).collect();
        self.badge_changed(ext.clone(), qs(t), color.clone(), title.clone());
        true
    }

    pub fn open_options(self: Pin<&mut Self>, ext: &QString) -> bool {
        if !valid_ext(&s(ext)) {
            return false;
        }
        self.options_requested(ext.clone());
        true
    }

    /// Drop an uninstalled extension's storage.
    pub fn forget(mut self: Pin<&mut Self>, ext: &QString) {
        let e = s(ext);
        if !valid_ext(&e) {
            return;
        }
        self.as_mut().rust_mut().stores.retain(|(x, _), _| *x != e);
        for a in ["local", "sync"] {
            let _ = std::fs::remove_file(store_file(&e, a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_get_semantics() {
        let store: Map<String, Value> = serde_json::from_str(r#"{"a":1,"b":"x"}"#).unwrap();
        assert_eq!(Value::Object(select(&store, &Value::Null)), json!({"a":1,"b":"x"}));
        assert_eq!(Value::Object(select(&store, &json!("a"))), json!({"a":1}));
        assert_eq!(Value::Object(select(&store, &json!(["b","zz"]))), json!({"b":"x"}));
        assert_eq!(Value::Object(select(&store, &json!({"a":0,"c":true}))), json!({"a":1,"c":true}));
        assert!(valid_ext("nedkmelhfnmdmhenoafgjafplfcclogb"));
        assert!(!valid_ext("../etc"));
    }
}
