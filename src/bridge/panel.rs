//! `ExtensionPanel` — the `extension` object an OyNIx extension's panel
//! (`graphics.qt`) sees. It holds the extension's id and `extra.jsonl`
//! data, and carries messages between the panel and the extension's
//! `code.ts` (QML wires `posted` to ExtBridge and feeds `deliver`).

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
        #[qproperty(QString, id)]
        #[qproperty(QString, data)]
        type ExtensionPanel = super::PanelRust;

        /// A message from code.ts (JSON text).
        #[qsignal]
        fn message(self: Pin<&mut ExtensionPanel>, payload: QString);
        /// The panel sent a message to code.ts (JSON text).
        #[qsignal]
        fn posted(self: Pin<&mut ExtensionPanel>, payload: QString);
        /// The panel asked to close.
        #[qsignal]
        fn close_requested(self: Pin<&mut ExtensionPanel>);

        /// Send JSON text to the extension's code.ts.
        #[qinvokable]
        fn post(self: Pin<&mut ExtensionPanel>, payload: &QString);
        #[qinvokable]
        fn deliver(self: Pin<&mut ExtensionPanel>, payload: &QString);
        #[qinvokable]
        fn close(self: Pin<&mut ExtensionPanel>);
    }
}

use core::pin::Pin;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct PanelRust {
    id: QString,
    data: QString,
}

impl qobject::ExtensionPanel {
    pub fn post(self: Pin<&mut Self>, payload: &QString) {
        // Only well-formed JSON crosses over, so code.ts can always parse it.
        if serde_json::from_str::<serde_json::Value>(&String::from(payload)).is_ok() {
            self.posted(payload.clone());
        }
    }

    pub fn deliver(self: Pin<&mut Self>, payload: &QString) {
        self.message(payload.clone());
    }

    pub fn close(self: Pin<&mut Self>) {
        self.close_requested();
    }
}
