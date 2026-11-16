//! `OutlineService` — reads PDF outlines (core/outline.rs) off the UI
//! thread for tree tabs.

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
    }

    impl cxx_qt::Threading for OutlineService {}
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct OutlineRust {}

impl qobject::OutlineService {
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
