//! `AgentBridge` — connects external Claude clients (via `oynix --mcp`) and
//! the built-in Claude panel to the browser's tool executor in QML.
//!
//! The running browser listens on a Unix domain socket in the profile's data
//! directory (`agent.sock`, mode 0600, so only the same user can connect).
//! Each request is a JSON line `{"id", "tool", "args"}`; it is queued onto the
//! Qt thread as the `toolRequested` signal, QML executes it (applying site
//! permissions) and answers with `reply(id, json)`, which is written back to
//! the socket as `{"id", "content": [...], "isError"}`.

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
        #[qproperty(bool, listening)]
        #[qproperty(i32, connections)]
        type AgentBridge = super::AgentBridgeRust;

        /// A tool call arrived from an external client (source = "mcp").
        #[qsignal]
        fn tool_requested(self: Pin<&mut AgentBridge>, id: f64, name: QString, args: QString, source: QString);

        #[qinvokable]
        fn start(self: Pin<&mut AgentBridge>) -> bool;
        #[qinvokable]
        fn stop(self: Pin<&mut AgentBridge>);
        #[qinvokable]
        fn reply(self: Pin<&mut AgentBridge>, id: f64, result: &QString);
        #[qinvokable]
        fn tools(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn system_prompt(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn page_helper(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn socket_path(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn mcp_command(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn desktop_config(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn temp_image_path(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn take_image_base64(self: &AgentBridge, path: &QString) -> QString;
        #[qinvokable]
        fn network_requests(self: &AgentBridge, host: &QString, contains: &QString, limit: i32) -> QString;
        #[qinvokable]
        fn api_key_status(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn api_key(self: &AgentBridge) -> QString;
        #[qinvokable]
        fn set_api_key(self: &AgentBridge, key: &QString) -> bool;
    }

    impl cxx_qt::Threading for AgentBridge {}
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use crate::core::{agent, paths};

#[derive(Default)]
pub struct AgentBridgeRust {
    listening: bool,
    connections: i32,
    running: Option<Arc<AtomicBool>>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static PENDING: LazyLock<Mutex<HashMap<u64, mpsc::Sender<String>>>> = LazyLock::new(Default::default);

pub fn socket_path() -> PathBuf {
    paths::data_dir().join("agent.sock")
}

fn key_file() -> PathBuf {
    paths::config_dir().join("claude-api-key")
}

fn s(q: &QString) -> String {
    String::from(q)
}

fn qs(v: impl AsRef<str>) -> QString {
    QString::from(v.as_ref())
}

/// Convert the QML reply (`{text?, image?, error?, data?}`) to MCP content.
pub fn to_mcp_result(id: u64, reply: &str) -> Value {
    let v: Value = serde_json::from_str(reply).unwrap_or_else(|_| json!({ "error": "invalid reply from browser" }));
    let mut content = Vec::new();
    let is_error = v.get("error").map(|e| !e.is_null() && e != "").unwrap_or(false);
    if let Some(e) = v.get("error").and_then(Value::as_str).filter(|e| !e.is_empty()) {
        content.push(json!({ "type": "text", "text": e }));
    }
    if let Some(t) = v.get("text").and_then(Value::as_str) {
        content.push(json!({ "type": "text", "text": t }));
    }
    if let Some(p) = v.get("image").and_then(Value::as_str) {
        if let Ok(bytes) = std::fs::read(p) {
            let _ = std::fs::remove_file(p);
            let mime = if p.ends_with(".png") { "image/png" } else { "image/jpeg" };
            content.push(json!({ "type": "image", "data": b64(&bytes), "mimeType": mime }));
        }
    }
    if content.is_empty() {
        content.push(json!({ "type": "text", "text": "OK" }));
    }
    json!({ "id": id, "content": content, "isError": is_error })
}

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn handle_client(stream: UnixStream, thread: cxx_qt::CxxQtThread<qobject::AgentBridge>, running: Arc<AtomicBool>) {
    let Ok(mut writer) = stream.try_clone() else { return };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if !running.load(Ordering::Relaxed) {
            break;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        let client_id = req.get("id").cloned().unwrap_or(Value::Null);
        let name = req.get("tool").and_then(Value::as_str).unwrap_or("").to_string();
        let args = req.get("args").cloned().unwrap_or_else(|| json!({}));
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        PENDING.lock().unwrap().insert(id, tx);
        let (n, a) = (name.clone(), args.to_string());
        let queued = thread.queue(move |obj: Pin<&mut qobject::AgentBridge>| {
            obj.tool_requested(id as f64, qs(&n), qs(&a), qs("mcp"));
        });
        let reply = if queued.is_err() {
            json!({ "error": "the browser is shutting down" }).to_string()
        } else {
            // Permission prompts may wait for the user; allow plenty of time.
            rx.recv_timeout(Duration::from_secs(600)).unwrap_or_else(|_| json!({ "error": "timed out waiting for the browser" }).to_string())
        };
        PENDING.lock().unwrap().remove(&id);
        let mut out = to_mcp_result(id, &reply);
        out["id"] = client_id;
        if writeln!(writer, "{out}").is_err() {
            break;
        }
    }
}

impl qobject::AgentBridge {
    /// Start listening on the agent socket. Safe to call repeatedly.
    pub fn start(mut self: Pin<&mut Self>) -> bool {
        if self.listening {
            return true;
        }
        let path = socket_path();
        // A stale socket from a crashed run: remove it if nobody answers.
        if path.exists() && UnixStream::connect(&path).is_err() {
            let _ = std::fs::remove_file(&path);
        }
        let Ok(listener) = UnixListener::bind(&path) else { return false };
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        let running = Arc::new(AtomicBool::new(true));
        let thread = self.qt_thread();
        let r = running.clone();
        std::thread::Builder::new()
            .name("oynix-agent".into())
            .spawn(move || {
                for conn in listener.incoming() {
                    if !r.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(conn) = conn else { continue };
                    let (t, r2) = (thread.clone(), r.clone());
                    let _ = thread.queue(|mut o: Pin<&mut qobject::AgentBridge>| {
                        let c = o.connections + 1;
                        o.as_mut().set_connections(c);
                    });
                    std::thread::spawn(move || {
                        handle_client(conn, t.clone(), r2);
                        let _ = t.queue(|mut o: Pin<&mut qobject::AgentBridge>| {
                            let c = (o.connections - 1).max(0);
                            o.as_mut().set_connections(c);
                        });
                    });
                }
            })
            .ok();
        self.as_mut().rust_mut().running = Some(running);
        self.set_listening(true);
        true
    }

    pub fn stop(mut self: Pin<&mut Self>) {
        if let Some(r) = self.as_mut().rust_mut().running.take() {
            r.store(false, Ordering::Relaxed);
            // Wake the accept loop.
            let _ = UnixStream::connect(socket_path());
        }
        let _ = std::fs::remove_file(socket_path());
        for (_, tx) in PENDING.lock().unwrap().drain() {
            let _ = tx.send(json!({ "error": "Claude access was turned off in OyNIx" }).to_string());
        }
        self.set_listening(false);
    }

    pub fn reply(self: Pin<&mut Self>, id: f64, result: &QString) {
        if let Some(tx) = PENDING.lock().unwrap().remove(&(id as u64)) {
            let _ = tx.send(s(result));
        }
    }

    pub fn tools(&self) -> QString {
        qs(Value::Array(agent::tools()).to_string())
    }

    pub fn system_prompt(&self) -> QString {
        qs(agent::SYSTEM_PROMPT)
    }

    pub fn page_helper(&self) -> QString {
        qs(agent::PAGE_HELPER)
    }

    pub fn socket_path(&self) -> QString {
        qs(socket_path().to_string_lossy())
    }

    fn exe() -> String {
        // Prefer the stable launcher path when installed.
        for p in ["/usr/bin/oynix"] {
            if std::path::Path::new(p).exists() {
                return p.into();
            }
        }
        std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| "oynix".into())
    }

    fn profile_args() -> Vec<String> {
        let p = paths::profile();
        let mut v = vec!["--mcp".to_string()];
        if p != "default" {
            v.push("--profile".into());
            v.push(p.into());
        }
        v
    }

    /// Command for Claude Code: `claude mcp add …`.
    pub fn mcp_command(&self) -> QString {
        qs(format!("claude mcp add oynix -- {} {}", Self::exe(), Self::profile_args().join(" ")))
    }

    /// `mcpServers` entry for Claude Desktop's configuration file.
    pub fn desktop_config(&self) -> QString {
        let cfg = json!({ "mcpServers": { "oynix": { "command": Self::exe(), "args": Self::profile_args() } } });
        qs(serde_json::to_string_pretty(&cfg).unwrap_or_default())
    }

    pub fn temp_image_path(&self) -> QString {
        let dir = paths::cache_dir().join("agent");
        let _ = std::fs::create_dir_all(&dir);
        let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        qs(dir.join(format!("shot-{}-{n}.jpg", std::process::id())).to_string_lossy())
    }

    /// Read an image written by QML, return it base64-encoded and delete it.
    pub fn take_image_base64(&self, path: &QString) -> QString {
        let p = PathBuf::from(s(path));
        if !p.starts_with(paths::cache_dir()) {
            return qs("");
        }
        let data = std::fs::read(&p).unwrap_or_default();
        let _ = std::fs::remove_file(&p);
        qs(b64(&data))
    }

    pub fn network_requests(&self, host: &QString, contains: &QString, limit: i32) -> QString {
        qs(Value::Array(agent::network_for(&s(host), &s(contains), limit.max(1) as usize)).to_string())
    }

    /// `{configured, source, hint}` without revealing the key.
    pub fn api_key_status(&self) -> QString {
        let (key, source) = Self::load_key();
        let hint = if key.len() > 12 { format!("{}…{}", &key[..7], &key[key.len() - 4..]) } else { String::new() };
        qs(json!({ "configured": !key.is_empty(), "source": source, "hint": hint }).to_string())
    }

    fn load_key() -> (String, &'static str) {
        if let Ok(k) = std::fs::read_to_string(key_file()) {
            let k = k.trim().to_string();
            if !k.is_empty() {
                return (k, "settings");
            }
        }
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) if !k.trim().is_empty() => (k.trim().to_string(), "environment"),
            _ => (String::new(), ""),
        }
    }

    pub fn api_key(&self) -> QString {
        qs(Self::load_key().0)
    }

    /// Store (or, with "", remove) the API key in a 0600 file.
    pub fn set_api_key(&self, key: &QString) -> bool {
        let k = s(key).trim().to_string();
        let f = key_file();
        if k.is_empty() {
            return std::fs::remove_file(&f).is_ok() || !f.exists();
        }
        if std::fs::write(&f, &k).is_err() {
            return false;
        }
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o600)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_reference() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn reply_conversion() {
        let r = to_mcp_result(3, r#"{"text":"hi"}"#);
        assert_eq!(r["content"][0]["text"], "hi");
        assert_eq!(r["isError"], false);
        let r = to_mcp_result(3, r#"{"error":"denied"}"#);
        assert_eq!(r["isError"], true);
    }
}
