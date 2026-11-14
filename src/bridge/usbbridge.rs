//! `UsbBridge` — WebUSB for web pages. OyNIx's injected `navigator.usb`
//! reaches it over Qt WebChannel (registered as `oynixUsb`). The bridge
//! enforces per-origin permissions on every call, asks QML to show the
//! device chooser, and runs blocking USB transfers on worker threads.

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
        type UsbBridge = super::UsbBridgeRust;

        /// QML shows the chooser and answers with `choose(id, key)`.
        #[qsignal]
        fn chooser_requested(self: Pin<&mut UsbBridge>, id: QString, origin: QString, devices: QString);
        /// Answer to a request: {"ok":true,...} or {"ok":false,"error":"Name: msg"}.
        #[qsignal]
        fn result(self: Pin<&mut UsbBridge>, id: QString, payload: QString);
        /// "connect" / "disconnect" with the device and the origins allowed to see it.
        #[qsignal]
        fn device_event(self: Pin<&mut UsbBridge>, kind: QString, payload: QString);
        #[qsignal]
        fn permissions_changed(self: Pin<&mut UsbBridge>);

        #[qinvokable]
        fn available(self: &UsbBridge) -> bool;
        #[qinvokable]
        fn request_device(self: Pin<&mut UsbBridge>, origin: &QString, options: &QString, id: &QString);
        #[qinvokable]
        fn choose(self: Pin<&mut UsbBridge>, id: &QString, key: &QString);
        #[qinvokable]
        fn get_devices(self: Pin<&mut UsbBridge>, origin: &QString, id: &QString);
        #[qinvokable]
        fn call(self: Pin<&mut UsbBridge>, origin: &QString, id: &QString, key: &QString, op: &QString, args: &QString);
        #[qinvokable]
        fn poll(self: Pin<&mut UsbBridge>);
        #[qinvokable]
        fn permissions(self: &UsbBridge) -> QString;
        #[qinvokable]
        fn revoke(self: Pin<&mut UsbBridge>, origin: &QString, key: &QString);
    }

    impl cxx_qt::Threading for UsbBridge {}
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::core::usb::{self, Permissions, Shared};

pub struct UsbBridgeRust {
    perms: Permissions,
    /// chooser id -> (origin, devices offered)
    pending: HashMap<String, (String, Vec<Value>)>,
    /// "origin|key" -> open device
    open: HashMap<String, Shared>,
    /// Devices seen at the last poll (for connect/disconnect events).
    seen: Option<HashMap<String, Value>>,
}

impl Default for UsbBridgeRust {
    fn default() -> Self {
        Self { perms: Permissions::load(), pending: HashMap::new(), open: HashMap::new(), seen: None }
    }
}

fn s(q: &QString) -> String {
    String::from(q)
}
fn q(v: Value) -> QString {
    QString::from(&v.to_string())
}
fn err(msg: &str) -> Value {
    json!({ "ok": false, "error": msg })
}

/// Only secure origins may use WebUSB (http://localhost counts as secure).
fn secure_origin(origin: &str) -> bool {
    origin.starts_with("https://")
        || origin.starts_with("chrome-extension://")
        || ["http://localhost", "http://127.0.0.1", "http://[::1]"].iter().any(|p| origin == *p || origin.starts_with(&format!("{p}:")))
}

impl qobject::UsbBridge {
    pub fn available(&self) -> bool {
        usb::available()
    }

    fn reply(self: Pin<&mut Self>, id: &str, v: Value) {
        self.result(QString::from(id), q(v));
    }

    pub fn request_device(mut self: Pin<&mut Self>, origin: &QString, options: &QString, id: &QString) {
        let (origin, id) = (s(origin), s(id));
        if !secure_origin(&origin) {
            return self.reply(&id, err("SecurityError: WebUSB is only available on secure (https) pages."));
        }
        let opts: Value = serde_json::from_str(&s(options)).unwrap_or(json!({}));
        if let Err(e) = usb::validate_options(&opts) {
            return self.reply(&id, err(&format!("TypeError: {e}")));
        }
        let devices: Vec<Value> = usb::enumerate().into_iter().map(|(_, v)| v).filter(|v| usb::options_match(&opts, v)).collect();
        let list: Vec<Value> = devices
            .iter()
            .map(|v| {
                json!({
                    "key": v["key"], "name": usb::display_name(v),
                    "ids": format!("{:04x}:{:04x}", v["vendorId"].as_u64().unwrap_or(0), v["productId"].as_u64().unwrap_or(0)),
                    "paired": self.perms.allowed(&origin, v["key"].as_str().unwrap_or("")),
                })
            })
            .collect();
        self.as_mut().rust_mut().pending.insert(id.clone(), (origin.clone(), devices));
        self.chooser_requested(QString::from(&id), QString::from(&origin), q(Value::Array(list)));
    }

    pub fn choose(mut self: Pin<&mut Self>, id: &QString, key: &QString) {
        let (id, key) = (s(id), s(key));
        let Some((origin, devices)) = self.as_mut().rust_mut().pending.remove(&id) else { return };
        let Some(info) = devices.into_iter().find(|v| v["key"] == key.as_str()) else {
            return self.reply(&id, err("NotFoundError: No device selected."));
        };
        self.as_mut().rust_mut().perms.grant(&origin, &key, &usb::display_name(&info));
        self.perms.save();
        self.as_mut().permissions_changed();
        self.reply(&id, json!({ "ok": true, "device": info }));
    }

    pub fn get_devices(self: Pin<&mut Self>, origin: &QString, id: &QString) {
        let origin = s(origin);
        let list: Vec<Value> = usb::enumerate()
            .into_iter()
            .map(|(_, v)| v)
            .filter(|v| self.perms.allowed(&origin, v["key"].as_str().unwrap_or("")))
            .collect();
        self.reply(&s(id), json!({ "ok": true, "devices": list }));
    }

    pub fn call(mut self: Pin<&mut Self>, origin: &QString, id: &QString, key: &QString, op: &QString, args: &QString) {
        let (origin, id, key, op) = (s(origin), s(id), s(key), s(op));
        let slot = format!("{origin}|{key}");
        if !self.perms.allowed(&origin, &key) {
            return self.reply(&id, err("SecurityError: Access to this device was not granted or has been revoked."));
        }
        match op.as_str() {
            "open" => {
                if self.open.contains_key(&slot) {
                    return self.reply(&id, json!({ "ok": true }));
                }
                let Some((dev, _)) = usb::find(&key) else {
                    return self.reply(&id, err("NotFoundError: The device is not connected."));
                };
                match usb::open(&dev) {
                    Ok(h) => {
                        // Descriptor strings are readable now; refresh them.
                        let info = usb::describe(&dev, Some(&h.handle));
                        self.as_mut().rust_mut().open.insert(slot, h);
                        self.reply(&id, json!({ "ok": true, "device": info }))
                    }
                    Err(e) => self.reply(&id, err(&format!("SecurityError: {e}"))),
                }
            }
            "close" | "forget" => {
                if let Some(h) = self.as_mut().rust_mut().open.remove(&slot) {
                    usb::close(&h);
                }
                if op == "forget" {
                    self.as_mut().rust_mut().perms.revoke(&origin, &key);
                    self.perms.save();
                    self.as_mut().permissions_changed();
                }
                self.reply(&id, json!({ "ok": true }))
            }
            _ => {
                let Some(h) = self.open.get(&slot).cloned() else {
                    return self.reply(&id, err("InvalidStateError: The device must be opened first."));
                };
                let args: Value = serde_json::from_str(&s(args)).unwrap_or(json!({}));
                let thread = self.qt_thread();
                std::thread::spawn(move || {
                    let v = match usb::run(&h, &op, &args) {
                        Ok(mut r) => {
                            r["ok"] = json!(true);
                            r
                        }
                        Err(e) => err(&e),
                    };
                    let _ = thread.queue(move |o: Pin<&mut qobject::UsbBridge>| o.result(QString::from(&id), q(v)));
                });
            }
        }
    }

    /// Detect plugged / unplugged devices. Called periodically from QML
    /// while pages use WebUSB.
    pub fn poll(mut self: Pin<&mut Self>) {
        let now: HashMap<String, Value> = usb::connected_keys().into_iter().collect();
        let Some(before) = self.as_mut().rust_mut().seen.replace(now.clone()) else { return };
        let origins = |p: &Permissions, key: &str| -> Vec<String> { p.0.iter().filter(|(_, m)| m.contains_key(key)).map(|(o, _)| o.clone()).collect() };
        for (k, v) in &now {
            if !before.contains_key(k) {
                let o = origins(&self.perms, k);
                if !o.is_empty() {
                    self.as_mut().device_event(QString::from("connect"), q(json!({ "device": v, "origins": o })));
                }
            }
        }
        for (k, v) in &before {
            if !now.contains_key(k) {
                // Drop handles to devices that are gone.
                let gone: Vec<String> = self.open.keys().filter(|s| s.ends_with(&format!("|{k}"))).cloned().collect();
                for slot in gone {
                    if let Some(h) = self.as_mut().rust_mut().open.remove(&slot) {
                        usb::close(&h);
                    }
                }
                let o = origins(&self.perms, k);
                if !o.is_empty() {
                    self.as_mut().device_event(QString::from("disconnect"), q(json!({ "device": v, "origins": o })));
                }
            }
        }
    }

    pub fn permissions(&self) -> QString {
        q(self.perms.to_json())
    }

    pub fn revoke(mut self: Pin<&mut Self>, origin: &QString, key: &QString) {
        let (origin, key) = (s(origin), s(key));
        if let Some(h) = self.as_mut().rust_mut().open.remove(&format!("{origin}|{key}")) {
            usb::close(&h);
        }
        self.as_mut().rust_mut().perms.revoke(&origin, &key);
        self.perms.save();
        self.permissions_changed();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn secure_origins() {
        assert!(super::secure_origin("https://example.com"));
        assert!(super::secure_origin("http://localhost:8080"));
        assert!(super::secure_origin("http://127.0.0.1"));
        assert!(!super::secure_origin("http://example.com"));
        assert!(!super::secure_origin("http://localhost.evil.com"));
    }
}
