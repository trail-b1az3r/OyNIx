//! `oynix --mcp`: a Model Context Protocol server on stdio.
//!
//! Claude Desktop / Claude Code start this process; it lists the browser
//! tools and relays each call to the running OyNIx instance over the
//! profile's agent socket (starting OyNIx if it is not running). All
//! permission checks, the on-screen indicator and the activity log live in
//! the browser — this process is only a relay.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use crate::bridge::agentbridge::socket_path;
use crate::core::agent;

struct Link {
    stream: Option<(UnixStream, BufReader<UnixStream>)>,
    next: u64,
}

impl Link {
    fn connect(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }
        let path = socket_path();
        let mut started = false;
        let deadline = Instant::now() + Duration::from_secs(25);
        loop {
            match UnixStream::connect(&path) {
                Ok(s) => {
                    let r = BufReader::new(s.try_clone().map_err(|e| e.to_string())?);
                    self.stream = Some((s, r));
                    return Ok(());
                }
                Err(_) if Instant::now() < deadline => {
                    if !started {
                        started = true;
                        launch_browser();
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
                Err(_) => {
                    return Err("OyNIx is not running, or Claude access is turned off. Open OyNIx and enable \
                                Settings › Extensions › Claude for OyNIx."
                        .into())
                }
            }
        }
    }

    fn call(&mut self, name: &str, args: &Value) -> Value {
        for attempt in 0..2 {
            if let Err(e) = self.connect() {
                return json!({ "content": [{ "type": "text", "text": e }], "isError": true });
            }
            self.next += 1;
            let req = json!({ "id": self.next, "tool": name, "args": args });
            let (w, r) = self.stream.as_mut().unwrap();
            let mut line = String::new();
            let ok = writeln!(w, "{req}").is_ok() && r.read_line(&mut line).map(|n| n > 0).unwrap_or(false);
            if ok {
                if let Ok(mut v) = serde_json::from_str::<Value>(&line) {
                    if let Some(o) = v.as_object_mut() {
                        o.remove("id");
                    }
                    return v;
                }
            }
            self.stream = None; // reconnect once
            if attempt == 1 {
                break;
            }
        }
        json!({ "content": [{ "type": "text", "text": "Lost the connection to OyNIx." }], "isError": true })
    }
}

fn launch_browser() {
    let Ok(exe) = std::env::current_exe() else { return };
    let mut cmd = std::process::Command::new(exe);
    let profile = crate::core::paths::profile();
    if profile != "default" {
        cmd.arg("--profile").arg(profile);
    }
    cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let _ = cmd.spawn();
}

fn tool_list() -> Value {
    let tools: Vec<Value> = agent::tools()
        .into_iter()
        .map(|t| json!({ "name": t["name"], "description": t["description"], "inputSchema": t["input_schema"] }))
        .collect();
    json!({ "tools": tools })
}

fn respond(out: &mut impl Write, id: &Value, result: Result<Value, (i64, String)>) {
    let msg = match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, m)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": m } }),
    };
    let _ = writeln!(out, "{msg}");
    let _ = out.flush();
}

pub fn run() -> i32 {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    let mut link = Link { stream: None, next: 0 };
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            respond(&mut out, &Value::Null, Err((-32700, "parse error".into())));
            continue;
        };
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let Some(id) = msg.get("id").cloned() else { continue }; // notifications
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).unwrap_or("2025-06-18"),
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "oynix", "title": "OyNIx browser", "version": crate::VERSION },
                "instructions": agent::SYSTEM_PROMPT
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tool_list()),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                if !agent::tools().iter().any(|t| t["name"] == name) {
                    Err((-32602, format!("unknown tool: {name}")))
                } else {
                    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                    Ok(link.call(name, &args))
                }
            }
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            _ => Err((-32601, format!("method not found: {method}"))),
        };
        respond(&mut out, &id, result);
    }
    0
}
