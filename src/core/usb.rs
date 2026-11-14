//! WebUSB backend: USB device enumeration, WebUSB-shaped descriptors,
//! `requestDevice()` filter matching, per-origin permissions and the
//! device operations themselves (libusb through `rusb`).
//!
//! Pages never talk to this directly: OyNIx's injected `navigator.usb`
//! forwards calls through the `UsbBridge` QObject, which checks the origin's
//! permission for every call.

use rusb::{Context, Device, DeviceHandle, Direction, TransferType, UsbContext};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use super::paths;

/// Interface classes WebUSB refuses to claim (same protected set browsers
/// use): audio, HID, mass storage, smart card, video, audio/video and
/// wireless controllers. Those devices have their own, safer APIs.
pub const PROTECTED_CLASSES: [u8; 7] = [0x01, 0x03, 0x08, 0x0B, 0x0E, 0x10, 0xE0];

/// How long a blocking libusb call waits before checking whether the device
/// was closed. Transfers retry until they complete or the device closes.
const SLICE: Duration = Duration::from_millis(1500);

// ── Device descriptions ───────────────────────────────────────────────

/// Stable identity for a device across plug cycles: vendor, product and
/// serial number (or the physical port when the device has no serial).
pub fn device_key(vid: u16, pid: u16, serial: &str, bus: u8, ports: &[u8]) -> String {
    if serial.is_empty() {
        let p: Vec<String> = ports.iter().map(|x| x.to_string()).collect();
        format!("{vid:04x}:{pid:04x}@{bus}-{}", p.join("."))
    } else {
        format!("{vid:04x}:{pid:04x}#{serial}")
    }
}

/// Read a string attribute from sysfs (works without opening the device, so
/// the chooser can show names before permission is granted).
fn sysfs(bus: u8, ports: &[u8], attr: &str) -> String {
    if ports.is_empty() {
        return String::new();
    }
    let p: Vec<String> = ports.iter().map(|x| x.to_string()).collect();
    let path = PathBuf::from("/sys/bus/usb/devices").join(format!("{bus}-{}", p.join("."))).join(attr);
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn read_string(h: Option<&DeviceHandle<Context>>, idx: Option<u8>) -> String {
    match (h, idx) {
        (Some(h), Some(i)) => h.read_string_descriptor_ascii(i).unwrap_or_default(),
        _ => String::new(),
    }
}

/// Describe a device as a WebUSB `USBDevice` (without methods). `handle`
/// is used for string descriptors when the device is already open.
pub fn describe(dev: &Device<Context>, handle: Option<&DeviceHandle<Context>>) -> Option<Value> {
    let d = dev.device_descriptor().ok()?;
    let bus = dev.bus_number();
    let ports = dev.port_numbers().unwrap_or_default();
    let mut manufacturer = read_string(handle, d.manufacturer_string_index());
    let mut product = read_string(handle, d.product_string_index());
    let mut serial = read_string(handle, d.serial_number_string_index());
    if manufacturer.is_empty() {
        manufacturer = sysfs(bus, &ports, "manufacturer");
    }
    if product.is_empty() {
        product = sysfs(bus, &ports, "product");
    }
    if serial.is_empty() {
        serial = sysfs(bus, &ports, "serial");
    }
    let usb = d.usb_version();
    let ver = d.device_version();
    let active = handle
        .and_then(|h| h.active_configuration().ok())
        .or_else(|| dev.active_config_descriptor().ok().map(|c| c.number()))
        .unwrap_or(0);

    let mut configs = Vec::new();
    for i in 0..d.num_configurations() {
        let Ok(c) = dev.config_descriptor(i) else { continue };
        let mut ifaces = Vec::new();
        for iface in c.interfaces() {
            let mut alts = Vec::new();
            for alt in iface.descriptors() {
                let eps: Vec<Value> = alt
                    .endpoint_descriptors()
                    .map(|e| {
                        json!({
                            "endpointNumber": e.number(),
                            "direction": if e.direction() == Direction::In { "in" } else { "out" },
                            "type": match e.transfer_type() {
                                TransferType::Bulk => "bulk",
                                TransferType::Interrupt => "interrupt",
                                TransferType::Isochronous => "isochronous",
                                TransferType::Control => "control",
                            },
                            "packetSize": e.max_packet_size(),
                        })
                    })
                    .collect();
                alts.push(json!({
                    "alternateSetting": alt.setting_number(),
                    "interfaceClass": alt.class_code(),
                    "interfaceSubclass": alt.sub_class_code(),
                    "interfaceProtocol": alt.protocol_code(),
                    "interfaceName": read_string(handle, alt.description_string_index()),
                    "endpoints": eps,
                }));
            }
            ifaces.push(json!({ "interfaceNumber": iface.number(), "alternates": alts }));
        }
        configs.push(json!({
            "configurationValue": c.number(),
            "configurationName": read_string(handle, c.description_string_index()),
            "interfaces": ifaces,
        }));
    }

    Some(json!({
        "key": device_key(d.vendor_id(), d.product_id(), &serial, bus, &ports),
        "usbVersionMajor": usb.major(), "usbVersionMinor": usb.minor(), "usbVersionSubminor": usb.sub_minor(),
        "deviceClass": d.class_code(), "deviceSubclass": d.sub_class_code(), "deviceProtocol": d.protocol_code(),
        "vendorId": d.vendor_id(), "productId": d.product_id(),
        "deviceVersionMajor": ver.major(), "deviceVersionMinor": ver.minor(), "deviceVersionSubminor": ver.sub_minor(),
        "manufacturerName": manufacturer, "productName": product, "serialNumber": serial,
        "activeConfiguration": active,
        "configurations": configs,
    }))
}

/// The libusb context, or None when USB is unavailable (no USB bus, or a
/// sandbox without /dev/bus/usb). Never panics, unlike rusb's global one.
fn context() -> Option<&'static Context> {
    static CTX: OnceLock<Option<Context>> = OnceLock::new();
    CTX.get_or_init(|| Context::new().ok()).as_ref()
}

/// Every connected device (hubs excluded), described.
pub fn enumerate() -> Vec<(Device<Context>, Value)> {
    let Some(ctx) = context() else { return vec![] };
    let Ok(list) = ctx.devices() else { return vec![] };
    list.iter()
        .filter_map(|d| {
            let v = describe(&d, None)?;
            (v["deviceClass"] != 9).then_some((d, v))
        })
        .collect()
}

/// Short label for choosers and settings: "Product (Manufacturer)" or the IDs.
pub fn display_name(info: &Value) -> String {
    let p = info["productName"].as_str().unwrap_or("");
    let m = info["manufacturerName"].as_str().unwrap_or("");
    let ids = format!("{:04x}:{:04x}", info["vendorId"].as_u64().unwrap_or(0), info["productId"].as_u64().unwrap_or(0));
    match (p.is_empty(), m.is_empty()) {
        (false, false) => format!("{p} ({m})"),
        (false, true) => p.to_string(),
        (true, false) => format!("{m} device {ids}"),
        (true, true) => format!("USB device {ids}"),
    }
}

// ── Filters (navigator.usb.requestDevice({filters})) ─────────────────

/// Does `info` satisfy one `USBDeviceFilter`? Class/subclass/protocol match
/// either the device itself or any interface alternate, as in the spec.
pub fn filter_matches(f: &Value, info: &Value) -> bool {
    let eq = |k: &str, v: &Value| f.get(k).map_or(true, |want| want.as_u64() == v.as_u64());
    if !eq("vendorId", &info["vendorId"]) || !eq("productId", &info["productId"]) {
        return false;
    }
    if let Some(sn) = f.get("serialNumber").and_then(|v| v.as_str()) {
        if info["serialNumber"].as_str() != Some(sn) {
            return false;
        }
    }
    // (subclass/protocol without a class are rejected by validate_options)
    if f.get("classCode").is_none() {
        return true;
    }
    let triple = |c: &Value, s: &Value, p: &Value| eq("classCode", c) && eq("subclassCode", s) && eq("protocolCode", p);
    if triple(&info["deviceClass"], &info["deviceSubclass"], &info["deviceProtocol"]) {
        return true;
    }
    info["configurations"].as_array().into_iter().flatten().any(|c| {
        c["interfaces"].as_array().into_iter().flatten().any(|i| {
            i["alternates"].as_array().into_iter().flatten().any(|a| triple(&a["interfaceClass"], &a["interfaceSubclass"], &a["interfaceProtocol"]))
        })
    })
}

/// Apply `requestDevice` options: an empty filter list matches everything;
/// `exclusionFilters` remove matches.
pub fn options_match(options: &Value, info: &Value) -> bool {
    let filters = options["filters"].as_array().cloned().unwrap_or_default();
    let included = filters.is_empty() || filters.iter().any(|f| filter_matches(f, info));
    let excluded = options["exclusionFilters"].as_array().is_some_and(|ex| ex.iter().any(|f| filter_matches(f, info)));
    included && !excluded
}

/// Check filters for invalid combinations (the spec rejects a productId
/// without a vendorId, and a subclass/protocol without its parent).
pub fn validate_options(options: &Value) -> Result<(), String> {
    for list in ["filters", "exclusionFilters"] {
        for f in options[list].as_array().into_iter().flatten() {
            if f.get("productId").is_some() && f.get("vendorId").is_none() {
                return Err("A filter with productId must also have vendorId".into());
            }
            if f.get("subclassCode").is_some() && f.get("classCode").is_none() {
                return Err("A filter with subclassCode must also have classCode".into());
            }
            if f.get("protocolCode").is_some() && f.get("subclassCode").is_none() {
                return Err("A filter with protocolCode must also have subclassCode".into());
            }
        }
    }
    Ok(())
}

// ── Permissions ──────────────────────────────────────────────────────

/// origin -> device key -> display name
#[derive(Default, Clone)]
pub struct Permissions(pub BTreeMap<String, BTreeMap<String, String>>);

fn permissions_path() -> PathBuf {
    paths::data_dir().join("usb-permissions.json")
}

impl Permissions {
    pub fn load() -> Self {
        let txt = std::fs::read_to_string(permissions_path()).unwrap_or_default();
        Permissions(serde_json::from_str(&txt).unwrap_or_default())
    }
    pub fn save(&self) {
        let _ = std::fs::create_dir_all(paths::data_dir());
        let _ = std::fs::write(permissions_path(), serde_json::to_string_pretty(&self.0).unwrap_or_default());
    }
    pub fn allowed(&self, origin: &str, key: &str) -> bool {
        self.0.get(origin).is_some_and(|m| m.contains_key(key))
    }
    pub fn grant(&mut self, origin: &str, key: &str, name: &str) {
        self.0.entry(origin.to_string()).or_default().insert(key.to_string(), name.to_string());
    }
    pub fn revoke(&mut self, origin: &str, key: &str) {
        if let Some(m) = self.0.get_mut(origin) {
            m.remove(key);
            if m.is_empty() {
                self.0.remove(origin);
            }
        }
    }
    pub fn to_json(&self) -> Value {
        let list: Vec<Value> = self
            .0
            .iter()
            .flat_map(|(o, m)| m.iter().map(move |(k, n)| json!({ "origin": o, "key": k, "name": n })))
            .collect();
        Value::Array(list)
    }
}

// ── Open devices and operations ──────────────────────────────────────

pub struct OpenDevice {
    pub handle: DeviceHandle<Context>,
    pub closed: AtomicBool,
    pub claimed: Mutex<HashSet<u8>>,
}

pub type Shared = Arc<OpenDevice>;

pub fn open(dev: &Device<Context>) -> Result<Shared, String> {
    let handle = dev.open().map_err(|e| match e {
        rusb::Error::Access => "Access denied to the USB device. On Linux, add a udev rule granting your user access (see the OyNIx README).".to_string(),
        other => format!("Could not open the USB device: {other}"),
    })?;
    // Linux: let libusb detach (and later re-attach) a kernel driver bound
    // to an interface we claim.
    let _ = handle.set_auto_detach_kernel_driver(true);
    Ok(Arc::new(OpenDevice { handle, closed: AtomicBool::new(false), claimed: Mutex::new(HashSet::new()) }))
}

pub fn close(d: &OpenDevice) {
    d.closed.store(true, Ordering::SeqCst);
    let claimed: Vec<u8> = d.claimed.lock().map(|mut c| c.drain().collect()).unwrap_or_default();
    for i in claimed {
        let _ = d.handle.release_interface(i);
    }
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn unb64(s: &str) -> Result<Vec<u8>, String> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for c in clean.chunks(4) {
        let mut n = 0u32;
        for (i, &ch) in c.iter().enumerate() {
            n |= val(ch).ok_or("invalid base64 data")? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if c.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if c.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// Endpoint transfer type within the currently selected configuration.
fn endpoint_type(d: &OpenDevice, address: u8) -> Option<TransferType> {
    let dev = d.handle.device();
    let cfg = dev.active_config_descriptor().ok()?;
    cfg.interfaces()
        .flat_map(|i| i.descriptors().collect::<Vec<_>>())
        .flat_map(|a| a.endpoint_descriptors().collect::<Vec<_>>())
        .find(|e| e.address() == address)
        .map(|e| e.transfer_type())
}

fn interface_class(d: &OpenDevice, number: u8) -> Option<u8> {
    let cfg = d.handle.device().active_config_descriptor().ok()?;
    let class = cfg.interfaces().find(|i| i.number() == number)?.descriptors().next()?.class_code();
    Some(class)
}

fn setup(args: &Value) -> Result<(u8, u8, u16, u16), String> {
    let s = &args["setup"];
    let rt = match s["requestType"].as_str() {
        Some("standard") => rusb::RequestType::Standard,
        Some("class") => rusb::RequestType::Class,
        Some("vendor") => rusb::RequestType::Vendor,
        _ => return Err("TypeError: setup.requestType must be standard, class or vendor".into()),
    };
    let rc = match s["recipient"].as_str() {
        Some("device") => rusb::Recipient::Device,
        Some("interface") => rusb::Recipient::Interface,
        Some("endpoint") => rusb::Recipient::Endpoint,
        Some("other") => rusb::Recipient::Other,
        _ => return Err("TypeError: setup.recipient must be device, interface, endpoint or other".into()),
    };
    let num = |k: &str| s[k].as_u64().map(|v| v as u16).ok_or(format!("TypeError: setup.{k} is required"));
    Ok((rusb::request_type(Direction::In, rt, rc), num("request")? as u8, num("value")?, num("index")?))
}

fn transfer_status(e: &rusb::Error) -> Option<&'static str> {
    match e {
        rusb::Error::Pipe => Some("stall"),
        rusb::Error::Overflow => Some("babble"),
        _ => None,
    }
}

fn net_err(e: rusb::Error) -> String {
    match e {
        rusb::Error::NoDevice => "NotFoundError: The device was disconnected.".into(),
        rusb::Error::Access => "SecurityError: Access denied.".into(),
        rusb::Error::Busy => "NetworkError: The interface is in use by another program.".into(),
        other => format!("NetworkError: {other}"),
    }
}

/// Repeat a blocking libusb call in short slices until it finishes or the
/// device is closed.
fn sliced<T>(d: &OpenDevice, mut f: impl FnMut(Duration) -> rusb::Result<T>) -> rusb::Result<T> {
    loop {
        match f(SLICE) {
            Err(rusb::Error::Timeout) if !d.closed.load(Ordering::SeqCst) => continue,
            Err(rusb::Error::Timeout) => return Err(rusb::Error::Interrupted),
            r => return r,
        }
    }
}

/// Run one `USBDevice` method. Returns the JSON result (or an error string
/// "DOMExceptionName: message" for the page).
pub fn run(d: &OpenDevice, op: &str, args: &Value) -> Result<Value, String> {
    if d.closed.load(Ordering::SeqCst) {
        return Err("InvalidStateError: The device must be opened first.".into());
    }
    let h = &d.handle;
    let u8arg = |k: &str| args[k].as_u64().map(|v| v as u8).ok_or(format!("TypeError: {k} is required"));
    let claimed = |i: u8| d.claimed.lock().map(|c| c.contains(&i)).unwrap_or(false);
    match op {
        "selectConfiguration" => {
            let v = u8arg("configurationValue")?;
            if h.active_configuration().ok() != Some(v) {
                h.set_active_configuration(v).map_err(net_err)?;
            }
            Ok(json!({ "activeConfiguration": v }))
        }
        "claimInterface" => {
            let i = u8arg("interfaceNumber")?;
            if let Some(class) = interface_class(d, i) {
                if PROTECTED_CLASSES.contains(&class) {
                    return Err(format!("SecurityError: Interface {i} implements a protected class (0x{class:02x}) and cannot be claimed."));
                }
            } else {
                return Err(format!("NotFoundError: The configuration has no interface {i}."));
            }
            h.claim_interface(i).map_err(net_err)?;
            if let Ok(mut c) = d.claimed.lock() {
                c.insert(i);
            }
            Ok(json!({ "claimed": true }))
        }
        "releaseInterface" => {
            let i = u8arg("interfaceNumber")?;
            h.release_interface(i).map_err(net_err)?;
            if let Ok(mut c) = d.claimed.lock() {
                c.remove(&i);
            }
            Ok(json!({ "claimed": false }))
        }
        "selectAlternateInterface" => {
            let i = u8arg("interfaceNumber")?;
            if !claimed(i) {
                return Err("InvalidStateError: The interface must be claimed first.".into());
            }
            h.set_alternate_setting(i, u8arg("alternateSetting")?).map_err(net_err)?;
            Ok(json!({}))
        }
        "controlTransferIn" => {
            let (rt, req, value, index) = setup(args)?;
            let mut buf = vec![0u8; args["length"].as_u64().unwrap_or(0).min(65535) as usize];
            match sliced(d, |t| h.read_control(rt, req, value, index, &mut buf, t)) {
                Ok(n) => Ok(json!({ "status": "ok", "data": b64(&buf[..n]) })),
                Err(e) => transfer_status(&e).map(|s| json!({ "status": s, "data": "" })).ok_or_else(|| net_err(e)),
            }
        }
        "controlTransferOut" => {
            let (rt, req, value, index) = setup(args)?;
            let data = unb64(args["data"].as_str().unwrap_or(""))?;
            let rt = rt & !0x80; // host-to-device
            match sliced(d, |t| h.write_control(rt, req, value, index, &data, t)) {
                Ok(n) => Ok(json!({ "status": "ok", "bytesWritten": n })),
                Err(e) => transfer_status(&e).map(|s| json!({ "status": s, "bytesWritten": 0 })).ok_or_else(|| net_err(e)),
            }
        }
        "transferIn" | "transferOut" => {
            let n = u8arg("endpointNumber")? & 0x0f;
            let addr = if op == "transferIn" { 0x80 | n } else { n };
            let kind = endpoint_type(d, addr).ok_or(format!("NotFoundError: Endpoint {n} not found in the active configuration."))?;
            let res = if op == "transferIn" {
                let mut buf = vec![0u8; args["length"].as_u64().unwrap_or(0).min(16 << 20) as usize];
                let r = match kind {
                    TransferType::Bulk => sliced(d, |t| h.read_bulk(addr, &mut buf, t)),
                    TransferType::Interrupt => sliced(d, |t| h.read_interrupt(addr, &mut buf, t)),
                    _ => return Err("NotSupportedError: Isochronous transfers are not supported.".into()),
                };
                r.map(|k| json!({ "status": "ok", "data": b64(&buf[..k]) }))
            } else {
                let data = unb64(args["data"].as_str().unwrap_or(""))?;
                let r = match kind {
                    TransferType::Bulk => sliced(d, |t| h.write_bulk(addr, &data, t)),
                    TransferType::Interrupt => sliced(d, |t| h.write_interrupt(addr, &data, t)),
                    _ => return Err("NotSupportedError: Isochronous transfers are not supported.".into()),
                };
                r.map(|k| json!({ "status": "ok", "bytesWritten": k }))
            };
            res.or_else(|e| {
                let empty = if op == "transferIn" { json!({ "data": "" }) } else { json!({ "bytesWritten": 0 }) };
                transfer_status(&e)
                    .map(|s| {
                        let mut m: Map<String, Value> = empty.as_object().cloned().unwrap_or_default();
                        m.insert("status".into(), json!(s));
                        Value::Object(m)
                    })
                    .ok_or_else(|| net_err(e))
            })
        }
        "clearHalt" => {
            let n = u8arg("endpointNumber")? & 0x0f;
            let addr = if args["direction"] == "in" { 0x80 | n } else { n };
            h.clear_halt(addr).map_err(net_err)?;
            Ok(json!({}))
        }
        "reset" => {
            h.reset().map_err(net_err)?;
            Ok(json!({}))
        }
        "isochronousTransferIn" | "isochronousTransferOut" => Err("NotSupportedError: Isochronous transfers are not supported.".into()),
        other => Err(format!("NotSupportedError: Unknown operation {other}")),
    }
}

/// Keys of the devices connected right now (for connect/disconnect events).
pub fn connected_keys() -> Vec<(String, Value)> {
    enumerate().into_iter().map(|(_, v)| (v["key"].as_str().unwrap_or("").to_string(), v)).collect()
}

/// Find a connected device by key.
pub fn find(key: &str) -> Option<(Device<Context>, Value)> {
    enumerate().into_iter().find(|(_, v)| v["key"] == key)
}

/// Is libusb usable on this system?
pub fn available() -> bool {
    context().is_some_and(|c| c.devices().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev() -> Value {
        json!({
            "vendorId": 0x2341, "productId": 0x8036, "serialNumber": "ABC",
            "deviceClass": 0xEF, "deviceSubclass": 2, "deviceProtocol": 1,
            "configurations": [{ "interfaces": [
                { "alternates": [{ "interfaceClass": 2, "interfaceSubclass": 2, "interfaceProtocol": 0 }] },
                { "alternates": [{ "interfaceClass": 0xFF, "interfaceSubclass": 0, "interfaceProtocol": 0 }] }
            ]}]
        })
    }

    #[test]
    fn filters() {
        let d = dev();
        assert!(filter_matches(&json!({}), &d));
        assert!(filter_matches(&json!({ "vendorId": 0x2341 }), &d));
        assert!(!filter_matches(&json!({ "vendorId": 0x1234 }), &d));
        assert!(filter_matches(&json!({ "vendorId": 0x2341, "productId": 0x8036 }), &d));
        assert!(!filter_matches(&json!({ "vendorId": 0x2341, "productId": 1 }), &d));
        assert!(filter_matches(&json!({ "classCode": 0xFF }), &d)); // interface class
        assert!(filter_matches(&json!({ "classCode": 0xEF, "subclassCode": 2 }), &d)); // device class
        assert!(!filter_matches(&json!({ "classCode": 0xFF, "subclassCode": 9 }), &d));
        assert!(filter_matches(&json!({ "serialNumber": "ABC" }), &d));
        assert!(!filter_matches(&json!({ "serialNumber": "XYZ" }), &d));
        assert!(options_match(&json!({ "filters": [] }), &d));
        assert!(!options_match(&json!({ "filters": [{ "vendorId": 1 }] }), &d));
        assert!(!options_match(&json!({ "filters": [], "exclusionFilters": [{ "vendorId": 0x2341 }] }), &d));
        assert!(validate_options(&json!({ "filters": [{ "productId": 1 }] })).is_err());
        assert!(validate_options(&json!({ "filters": [{ "classCode": 1, "protocolCode": 1 }] })).is_err());
        assert!(validate_options(&json!({ "filters": [{ "vendorId": 1, "productId": 1 }] })).is_ok());
    }

    #[test]
    fn keys_and_names() {
        assert_eq!(device_key(0x2341, 0x8036, "ABC", 1, &[2, 3]), "2341:8036#ABC");
        assert_eq!(device_key(0x2341, 0x8036, "", 1, &[2, 3]), "2341:8036@1-2.3");
        assert_eq!(display_name(&json!({ "productName": "Uno", "manufacturerName": "Arduino", "vendorId": 1, "productId": 2 })), "Uno (Arduino)");
        assert_eq!(display_name(&json!({ "productName": "", "manufacturerName": "", "vendorId": 1, "productId": 2 })), "USB device 0001:0002");
    }

    #[test]
    fn base64_roundtrip() {
        for n in 0..20 {
            let v: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(unb64(&b64(&v)).unwrap(), v);
        }
        assert_eq!(b64(b"hello"), "aGVsbG8=");
    }

    #[test]
    fn permissions() {
        let mut p = Permissions::default();
        p.grant("https://a.example", "k1", "Dev");
        assert!(p.allowed("https://a.example", "k1"));
        assert!(!p.allowed("https://b.example", "k1"));
        p.revoke("https://a.example", "k1");
        assert!(p.0.is_empty());
    }
}
