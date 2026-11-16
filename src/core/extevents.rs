//! Events that start outside QML (the network interceptor, the cookie
//! store) and are delivered to extensions: queued here, drained by QML
//! (Core.drainExtensionEvents) and dispatched through ExtBridge.
//!
//! Recording only happens while an extension that may receive the events
//! is running with powerful APIs allowed.

use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

static OBSERVE_REQUESTS: AtomicBool = AtomicBool::new(false);
static OBSERVE_COOKIES: AtomicBool = AtomicBool::new(false);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static QUEUE: Mutex<VecDeque<(String, Value)>> = Mutex::new(VecDeque::new());
const MAX_QUEUED: usize = 4000;

pub fn set_observing(requests: bool, cookies: bool) {
    OBSERVE_REQUESTS.store(requests, Ordering::Relaxed);
    OBSERVE_COOKIES.store(cookies, Ordering::Relaxed);
    if !requests && !cookies {
        if let Ok(mut q) = QUEUE.lock() {
            q.clear();
        }
    }
}

fn push(name: &str, args: Value) {
    if let Ok(mut q) = QUEUE.lock() {
        if q.len() >= MAX_QUEUED {
            q.pop_front();
        }
        q.push_back((name.to_string(), args));
    }
}

/// webRequest type names for Qt's resource types.
pub fn type_name(t: i32) -> &'static str {
    match t {
        0 => "main_frame",
        1 => "sub_frame",
        2 => "stylesheet",
        3 => "script",
        4 => "image",
        5 => "font",
        6 => "other",
        7 => "media",
        8 | 9 | 17 => "other",
        10 => "xmlhttprequest",
        11 => "ping",
        12 | 13 => "other",
        14 => "csp_report",
        15 => "other",
        16 => "xmlhttprequest",
        18 => "object",
        _ => "other",
    }
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0)
}

/// Called by the network interceptor for every request.
pub fn request(url: &str, first_party: &str, method: &str, resource_type: i32, headers: Value, blocked: bool) {
    if !OBSERVE_REQUESTS.load(Ordering::Relaxed) || url.starts_with("chrome-extension:") {
        return;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    let details = json!({
        "requestId": id, "url": url, "method": method, "type": type_name(resource_type),
        "frameId": if resource_type == 0 { 0 } else { -1 }, "parentFrameId": -1, "tabId": -1,
        "initiator": first_party, "originUrl": first_party, "documentUrl": first_party, "timeStamp": now_ms(),
    });
    push("webRequest.onBeforeRequest", json!([details.clone()]));
    let mut with_headers = details;
    with_headers["requestHeaders"] = headers;
    push("webRequest.onBeforeSendHeaders", json!([with_headers.clone()]));
    push("webRequest.onSendHeaders", json!([with_headers.clone()]));
    if blocked {
        let mut e = with_headers;
        e["error"] = json!("net::ERR_BLOCKED_BY_CLIENT");
        e["fromCache"] = json!(false);
        push("webRequest.onErrorOccurred", json!([e]));
    }
}

/// Called by the cookie store when a cookie is added or removed.
pub fn cookie(cookie: Value, removed: bool) {
    if !OBSERVE_COOKIES.load(Ordering::Relaxed) {
        return;
    }
    push("cookies.onChanged", json!([{ "removed": removed, "cookie": cookie, "cause": if removed { "explicit" } else { "explicit" } }]));
}

/// Take everything queued so far: [[name, args], ...]
pub fn drain() -> Value {
    let items: Vec<Value> = QUEUE.lock().map(|mut q| q.drain(..).map(|(n, a)| json!([n, a])).collect()).unwrap_or_default();
    Value::Array(items)
}
