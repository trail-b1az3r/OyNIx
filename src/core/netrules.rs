//! Runtime network rules from power extensions' `network.ts`. Moved to
//! StonedGem (`stonedgem-blocker`); the fetch stays here, with OyNIx's HTTP
//! client.

use serde_json::{json, Value};

pub use stonedgem_blocker::netrules::*;

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
