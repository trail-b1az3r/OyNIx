//! `OcrService` — runs on-device text recognition (core/ocr.rs) for
//! extensions with the "ocr" permission. QML captures the tab, saves it,
//! and asks for a region of it to be read; work happens off the UI thread.

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
        /// Requests being worked on.
        #[qproperty(i32, pending)]
        type OcrService = super::OcrRust;

        #[qsignal]
        fn finished(self: Pin<&mut OcrService>, req: QString, text: QString, error: QString);

        /// Read the region (image pixels) of the PNG at `path`; deletes the file afterwards.
        #[qinvokable]
        fn recognize(self: Pin<&mut OcrService>, path: &QString, x: f64, y: f64, w: f64, h: f64, req: &QString);
        #[qinvokable]
        fn models_present(self: &OcrService) -> bool;
    }

    impl cxx_qt::Threading for OcrService {}
}

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct OcrRust {
    pending: i32,
}

impl qobject::OcrService {
    pub fn recognize(mut self: Pin<&mut Self>, path: &QString, x: f64, y: f64, w: f64, h: f64, req: &QString) {
        let path = std::path::PathBuf::from(String::from(path));
        let req = req.clone();
        let region = (x.max(0.0) as u32, y.max(0.0) as u32, w.max(1.0) as u32, h.max(1.0) as u32);
        let n = self.pending + 1;
        self.as_mut().set_pending(n);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let r = crate::core::ocr::recognize(&path, Some(region));
            let _ = std::fs::remove_file(&path);
            let _ = thread.queue(move |mut o: Pin<&mut qobject::OcrService>| {
                let n = (o.pending - 1).max(0);
                o.as_mut().set_pending(n);
                match r {
                    Ok(t) => o.finished(req, QString::from(&t), QString::default()),
                    Err(e) => o.finished(req, QString::default(), QString::from(&e)),
                }
            });
        });
    }

    pub fn models_present(&self) -> bool {
        crate::core::ocr::models_present()
    }
}
