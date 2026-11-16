//! `OutlineService` — background work kept off the UI thread: PDF
//! outlines (core/outline.rs) for tree tabs, and network.ts fetches
//! (core/netrules.rs). Not on the WebChannel, so pages never see it.

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
        type OutlineService = super::OutlineRust;

        /// `items` is a JSON array of {level, title, page}; `error` is "" on success.
        #[qsignal]
        fn pdf_ready(self: Pin<&mut OutlineService>, url: QString, items: QString, error: QString);

        #[qinvokable]
        fn request_pdf(self: &OutlineService, url: &QString);

        /// A network.ts fetch finished: `result` is {status, headers, body} or {error}.
        #[qsignal]
        fn net_fetched(self: Pin<&mut OutlineService>, ext: QString, req: QString, result: QString);
        /// network.ts: fetch without cross-origin limits (power extensions
        /// with network.ts only; QML checks the caller's token first).
        #[qinvokable]
        fn net_fetch(self: &OutlineService, ext: &QString, req: &QString, request_json: &QString);
    }

    impl cxx_qt::Threading for OutlineService {}
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct OutlineRust {}

impl qobject::OutlineService {
    pub fn net_fetch(&self, ext: &QString, req: &QString, request_json: &QString) {
        let (ext, req) = (ext.clone(), req.clone());
        let allowed = crate::core::extensions::load().iter().any(|e| e.id == String::from(&ext) && e.enabled && e.power && e.has_network);
        let request: serde_json::Value = serde_json::from_str(&String::from(request_json)).unwrap_or_default();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = if allowed { crate::core::netrules::fetch(&request) } else { serde_json::json!({ "error": "not allowed" }) };
            let _ = thread.queue(move |o: Pin<&mut qobject::OutlineService>| {
                o.net_fetched(ext, req, QString::from(&result.to_string()));
            });
        });
    }

    pub fn request_pdf(&self, url: &QString) {
        let url = String::from(url);
        let thread = self.qt_thread();
        std::thread::Builder::new()
            .name("oynix-outline".into())
            .spawn(move || {
                let r = crate::core::outline::pdf(&url);
                let _ = thread.queue(move |o: Pin<&mut qobject::OutlineService>| {
                    let (items, err) = match r {
                        Ok(v) => (serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()), String::new()),
                        Err(e) => ("[]".to_string(), e),
                    };
                    o.pdf_ready(QString::from(&url), QString::from(&items), QString::from(&err));
                });
            })
            .ok();
    }
}
