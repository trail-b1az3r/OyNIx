//! `StudioBridge` — what Theme Studio's page reaches over Qt WebChannel
//! (registered as `oynixStudio`, on a channel only tabs showing the Studio
//! page get). It runs the Studio's Zsharpend script (shared with StonedGem:
//! `stonedgem_hypertheme::studio`) and keeps the page's graph.

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
        type StudioBridge = super::StudioRust;

        /// The script asked for this hypertheme (file name) to be used.
        #[qsignal]
        fn applied(self: Pin<&mut StudioBridge>, file: QString);
        /// A short message for the user.
        #[qsignal]
        fn notified(self: Pin<&mut StudioBridge>, text: QString);

        /// The page's chrome.runtime.sendMessage: { ok: reply } or { error }.
        #[qinvokable]
        fn message(self: Pin<&mut StudioBridge>, json: &QString) -> QString;
        #[qinvokable]
        fn storage_get(self: &StudioBridge, key: &QString) -> QString;
        #[qinvokable]
        fn storage_set(self: &StudioBridge, key: &QString, value: &QString) -> bool;
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::{json, Map, Value};
use stonedgem_hypertheme::studio::{Event, Studio};

#[derive(Default)]
pub struct StudioRust {
    studio: Option<Studio>,
}

impl qobject::StudioBridge {
    pub fn message(mut self: Pin<&mut Self>, json: &QString) -> QString {
        crate::core::paths::data_dir(); // themes go into OyNIx's profile
        let msg: Value = serde_json::from_str(&String::from(json)).unwrap_or(Value::Null);
        if self.rust().studio.is_none() {
            match Studio::new() {
                Ok(st) => self.as_mut().rust_mut().studio = Some(st),
                Err(e) => return QString::from(json!({ "error": e }).to_string().as_str()),
            }
        }
        let (r, events) = self.as_mut().rust_mut().studio.as_mut().map(|st| st.message(msg)).unwrap_or((Err("no studio".into()), vec![]));
        for e in events {
            match e {
                Event::Apply(f) => self.as_mut().applied(QString::from(f.as_str())),
                Event::Notify(t) => self.as_mut().notified(QString::from(t.as_str())),
            }
        }
        let out = match r {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "error": e }),
        };
        QString::from(out.to_string().as_str())
    }

    pub fn storage_get(&self, key: &QString) -> QString {
        let m: Map<String, Value> = std::fs::read(crate::core::hyper::studio_store()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        QString::from(m.get(&String::from(key)).and_then(Value::as_str).unwrap_or(""))
    }

    pub fn storage_set(&self, key: &QString, value: &QString) -> bool {
        let path = crate::core::hyper::studio_store();
        let mut m: Map<String, Value> = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let v = String::from(value);
        if v.len() > 4 << 20 {
            return false;
        }
        m.insert(String::from(key), Value::String(v));
        crate::core::paths::write_atomic(&path, serde_json::to_vec(&m).unwrap_or_default().as_slice()).is_ok()
    }
}
