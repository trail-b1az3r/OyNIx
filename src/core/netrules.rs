//! What power extensions' `network.ts` asks of the network: blocker rules
//! added at runtime, extra request headers, and a feed of requests.
//!
//! `network.ts` runs in a hidden page per extension (NetworkHost.qml);
//! its calls arrive here through Core. The interceptor (cpp/native.cpp)
//! reads the header rules synchronously, so lookups are cheap and skipped
//! entirely while no rules exist.

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};

#[derive(Clone, Debug, PartialEq)]
pub struct HeaderRule {
    pub ext: String,
    pub pattern: String,
    pub name: String,
    pub value: String,
}

static BLOCK: LazyLock<RwLock<HashMap<String, String>>> = LazyLock::new(Default::default);
static HEADERS: RwLock<Vec<HeaderRule>> = RwLock::new(Vec::new());
static ANY_HEADERS: AtomicBool = AtomicBool::new(false);
static OBSERVERS: LazyLock<RwLock<Vec<String>>> = LazyLock::new(Default::default);
static OBSERVE: AtomicBool = AtomicBool::new(false);
static QUEUE: Mutex<VecDeque<Value>> = Mutex::new(VecDeque::new());
const MAX_QUEUED: usize = 2000;

/// Replace an extension's runtime blocker rules (Adblock/uBlock syntax).
/// Returns true when they changed (the blocker then needs rebuilding).
pub fn set_block(ext: &str, rules: &str) -> bool {
    let mut m = BLOCK.write().unwrap();
    let old = m.get(ext).cloned().unwrap_or_default();
    if old == rules {
        return false;
    }
    if rules.trim().is_empty() {
        m.remove(ext);
    } else {
        m.insert(ext.to_string(), rules.to_string());
    }
    true
}

/// Every extension's runtime rules, for the blocker.
pub fn block_text() -> String {
    BLOCK.read().unwrap().values().cloned().collect::<Vec<_>>().join("\n")
}

/// Add a header to requests whose URL matches `pattern` (`*` wildcards,
/// e.g. `https://api.example.com/*`). Headers browsers manage themselves
/// (Cookie, Host, Origin…) can't be set.
pub fn add_header(ext: &str, pattern: &str, name: &str, value: &str) -> Result<(), String> {
    let lower = name.trim().to_ascii_lowercase();
    if lower.is_empty() || !lower.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(format!("“{name}” isn't a valid header name"));
    }
    const FORBIDDEN: &[&str] = &["cookie", "host", "origin", "referer", "content-length", "connection", "transfer-encoding", "upgrade", "proxy-authorization", "sec-fetch-site", "sec-fetch-mode", "sec-fetch-dest"];
    if FORBIDDEN.contains(&lower.as_str()) {
        return Err(format!("the {name} header is managed by the browser"));
    }
    if value.contains(['\r', '\n']) {
        return Err("header values can't contain line breaks".into());
    }
    let mut h = HEADERS.write().unwrap();
    h.retain(|r| !(r.ext == ext && r.pattern == pattern && r.name.eq_ignore_ascii_case(name)));
    h.push(HeaderRule { ext: ext.into(), pattern: pattern.into(), name: name.trim().into(), value: value.into() });
    ANY_HEADERS.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn clear_headers(ext: &str) {
    let mut h = HEADERS.write().unwrap();
    h.retain(|r| r.ext != ext);
    ANY_HEADERS.store(!h.is_empty(), Ordering::Relaxed);
}

/// `*`-wildcard match over the whole string.
fn wildcard(pattern: &str, text: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pattern.chars().collect(), text.chars().collect());
    let (mut pi, mut ti, mut star, mut mark) = (0usize, 0usize, None::<usize>, 0usize);
    while ti < t.len() {
        if pi < p.len() && p[pi] != '*' && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// Headers to add to a request for `url`.
pub fn headers_for(url: &str) -> Vec<(String, String)> {
    if !ANY_HEADERS.load(Ordering::Relaxed) {
        return vec![];
    }
    HEADERS.read().unwrap().iter().filter(|r| wildcard(&r.pattern, url)).map(|r| (r.name.clone(), r.value.clone())).collect()
}

/// Start or stop feeding requests to an extension's onRequest listeners.
pub fn set_observing(ext: &str, on: bool) {
    let mut o = OBSERVERS.write().unwrap();
    o.retain(|x| x != ext);
    if on {
        o.push(ext.to_string());
    }
    OBSERVE.store(!o.is_empty(), Ordering::Relaxed);
    if o.is_empty() {
        QUEUE.lock().unwrap().clear();
    }
}

/// From the interceptor: one request.
pub fn observe(url: &str, first_party: &str, method: &str, kind: &str, blocked: bool) {
    if !OBSERVE.load(Ordering::Relaxed) {
        return;
    }
    let mut q = QUEUE.lock().unwrap();
    if q.len() >= MAX_QUEUED {
        q.pop_front();
    }
    q.push_back(json!({ "url": url, "firstParty": first_party, "method": method, "type": kind, "blocked": blocked, "time": super::library::now() }));
}

/// Take everything observed since the last call.
pub fn drain() -> Value {
    Value::Array(QUEUE.lock().unwrap().drain(..).collect())
}

/// An extension was turned off or removed: forget what it set.
pub fn forget(ext: &str) -> bool {
    clear_headers(ext);
    set_observing(ext, false);
    set_block(ext, "")
}

/// A fetch for network.ts without the page's cross-origin limits.
/// `req`: {url, method?, headers?: {name: value}, body?}. Blocks.
pub fn fetch(req: &Value) -> Value {
    let url = req["url"].as_str().unwrap_or("");
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return json!({ "error": "only http(s) addresses can be fetched" });
    }
    let method = req["method"].as_str().unwrap_or("GET").to_ascii_uppercase();
    let agent = super::updater::agent();
    let mut headers: Vec<(String, String)> = vec![];
    if let Some(h) = req["headers"].as_object() {
        for (k, v) in h {
            headers.push((k.clone(), v.as_str().unwrap_or("").to_string()));
        }
    }
    let body = req["body"].as_str().map(str::to_string);
    let result = match method.as_str() {
        "GET" | "HEAD" | "DELETE" => {
            let mut r = match method.as_str() {
                "HEAD" => agent.head(url),
                "DELETE" => agent.delete(url),
                _ => agent.get(url),
            };
            for (k, v) in &headers {
                r = r.header(k.as_str(), v.as_str());
            }
            r.call()
        }
        "POST" | "PUT" | "PATCH" => {
            let mut r = match method.as_str() {
                "PUT" => agent.put(url),
                "PATCH" => agent.patch(url),
                _ => agent.post(url),
            };
            for (k, v) in &headers {
                r = r.header(k.as_str(), v.as_str());
            }
            r.send(body.unwrap_or_default().as_bytes())
        }
        m => return json!({ "error": format!("unsupported method {m}") }),
    };
    match result {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let hdrs: serde_json::Map<String, Value> =
                resp.headers().iter().map(|(k, v)| (k.as_str().to_string(), json!(v.to_str().unwrap_or("")))).collect();
            let text = resp.body_mut().with_config().limit(16 << 20).read_to_string().unwrap_or_default();
            json!({ "status": status, "headers": hdrs, "body": text })
        }
        Err(ureq::Error::StatusCode(code)) => json!({ "status": code, "headers": {}, "body": "" }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_rules() {
        assert!(wildcard("https://api.example.com/*", "https://api.example.com/v1/x"));
        assert!(!wildcard("https://api.example.com/*", "https://evil.com/?https://api.example.com/"));
        assert!(wildcard("*://*.example.com/*", "https://a.example.com/"));
        add_header("t", "https://api.example.com/*", "X-Key", "1").unwrap();
        assert_eq!(headers_for("https://api.example.com/a"), vec![("X-Key".to_string(), "1".to_string())]);
        assert!(headers_for("https://other.example/").is_empty());
        assert!(add_header("t", "*", "Cookie", "x").is_err());
        assert!(add_header("t", "*", "X-A", "a\r\nb").is_err());
        assert!(set_block("t", "||ads.example^"));
        assert!(!set_block("t", "||ads.example^"));
        assert!(block_text().contains("ads.example"));
        assert!(forget("t"));
        assert!(headers_for("https://api.example.com/a").is_empty() && block_text().is_empty());
    }
}
