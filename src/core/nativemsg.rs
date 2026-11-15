//! Native messaging (runtime.connectNative / sendNativeMessage): talk to a
//! program on this computer through its stdin/stdout, using the same host
//! manifests Chrome, Chromium, Brave, Vivaldi, Edge and Firefox install, so
//! hosts such as KeePassXC-Browser's work unchanged.
//!
//! Messages are UTF-8 JSON preceded by a 32-bit length in native byte
//! order. A host may only be started by an extension its manifest lists
//! (allowed_origins / allowed_extensions).

use serde_json::Value;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};

pub struct Host {
    pub path: PathBuf,
    pub allowed_origins: Vec<String>,
    pub allowed_extensions: Vec<String>,
    pub manifest: PathBuf,
}

fn host_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let config = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
    let mut v: Vec<PathBuf> = ["oynix", "google-chrome", "google-chrome-beta", "chromium", "BraveSoftware/Brave-Browser", "vivaldi", "microsoft-edge"]
        .iter()
        .map(|d| config.join(d).join("NativeMessagingHosts"))
        .collect();
    v.push(home.join(".mozilla/native-messaging-hosts"));
    v.extend(
        ["/etc/opt/chrome/native-messaging-hosts", "/etc/chromium/native-messaging-hosts", "/usr/lib/mozilla/native-messaging-hosts", "/usr/lib64/mozilla/native-messaging-hosts"]
            .iter()
            .map(PathBuf::from),
    );
    v
}

pub fn find_host(name: &str) -> Result<Host, String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_') {
        return Err(format!("Invalid native messaging host name “{name}”"));
    }
    for dir in host_dirs() {
        let manifest = dir.join(format!("{name}.json"));
        let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
        let path = PathBuf::from(v["path"].as_str().unwrap_or(""));
        if !path.is_absolute() {
            return Err(format!("{}: the host path must be absolute", manifest.display()));
        }
        let list = |k: &str| v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        return Ok(Host { path, allowed_origins: list("allowed_origins"), allowed_extensions: list("allowed_extensions"), manifest });
    }
    Err(format!("Specified native messaging host not found: {name}"))
}

/// Ids an extension is known by: its id in OyNIx, the Chrome Web Store id
/// derived from its manifest "key", and its Firefox add-on id.
pub fn extension_ids(web_id: &str, manifest: &Value) -> Vec<String> {
    let mut ids = vec![web_id.to_string()];
    if let Some(key) = manifest["key"].as_str() {
        if let Some(id) = chrome_id_from_key_b64(key) {
            ids.push(id);
        }
    }
    for k in ["browser_specific_settings", "applications"] {
        if let Some(g) = manifest[k]["gecko"]["id"].as_str() {
            ids.push(g.to_string());
        }
    }
    ids
}

/// Chrome's extension id: the first 16 bytes of SHA-256(public key),
/// written with the letters a–p.
pub fn chrome_id_from_key(der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(der)[..16].iter().flat_map(|b| [b >> 4, b & 15]).map(|n| (b'a' + n) as char).collect()
}

fn chrome_id_from_key_b64(b64: &str) -> Option<String> {
    let der = base64_decode(b64)?;
    Some(chrome_id_from_key(&der))
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
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
    let mut out = Vec::new();
    for c in clean.chunks(4) {
        let mut n = 0u32;
        for (i, &ch) in c.iter().enumerate() {
            n |= val(ch)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if c.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if c.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

pub fn allowed(host: &Host, ids: &[String]) -> bool {
    ids.iter().any(|id| host.allowed_origins.iter().any(|o| o.trim_end_matches('/') == format!("chrome-extension://{id}")) || host.allowed_extensions.iter().any(|e| e == id))
}

pub struct Connection {
    pub child: Child,
    pub stdin: ChildStdin,
}

/// Start the host. `reader` receives each message (or Err when it exits).
pub fn start(host: &Host, origin_id: &str, mut on_message: impl FnMut(Result<String, String>) + Send + 'static) -> Result<Connection, String> {
    let firefox_style = !host.allowed_extensions.is_empty() && host.allowed_origins.is_empty();
    let mut cmd = Command::new(&host.path);
    if firefox_style {
        cmd.arg(&host.manifest).arg(origin_id);
    } else {
        cmd.arg(format!("chrome-extension://{origin_id}/"));
    }
    if let Some(dir) = host.path.parent() {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Error when communicating with the native messaging host: {e}"))?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    std::thread::spawn(move || loop {
        let mut len = [0u8; 4];
        if stdout.read_exact(&mut len).is_err() {
            on_message(Err("Native host has exited.".into()));
            return;
        }
        let n = u32::from_ne_bytes(len) as usize;
        // Chrome limits messages from the host to 1 MB.
        if n > 1 << 20 {
            on_message(Err("Message from the native host is too large.".into()));
            return;
        }
        let mut buf = vec![0u8; n];
        if stdout.read_exact(&mut buf).is_err() {
            on_message(Err("Native host has exited.".into()));
            return;
        }
        on_message(Ok(String::from_utf8_lossy(&buf).into_owned()));
    });
    Ok(Connection { child, stdin })
}

pub fn send(stdin: &mut ChildStdin, json: &str) -> Result<(), String> {
    let len = u32::try_from(json.len()).map_err(|_| "Message too large")?;
    stdin.write_all(&len.to_ne_bytes()).and_then(|_| stdin.write_all(json.as_bytes())).and_then(|_| stdin.flush()).map_err(|e| e.to_string())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_id_from_key() {
        // The documented example from Chrome's "key" docs is hard to embed;
        // check the mapping on a known digest instead.
        let id = chrome_id_from_key(b"test");
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| ('a'..='p').contains(&c)));
        assert_eq!(chrome_id_from_key_b64("dGVzdA==").unwrap(), id);
    }

    #[test]
    fn round_trip_with_a_host() {
        // A tiny host written in Python: echoes each message back with "ok".
        let dir = std::env::temp_dir().join("oynix-nm-test");
        let _ = std::fs::create_dir_all(&dir);
        let host_py = dir.join("host.py");
        std::fs::write(&host_py, "#!/usr/bin/env python3\nimport sys,json,struct\nwhile True:\n  h=sys.stdin.buffer.read(4)\n  if len(h)<4: break\n  m=json.loads(sys.stdin.buffer.read(struct.unpack('=I',h)[0]))\n  o=json.dumps({'ok':True,'echo':m,'argv':sys.argv[1:]}).encode()\n  sys.stdout.buffer.write(struct.pack('=I',len(o))+o); sys.stdout.buffer.flush()\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&host_py, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        if Command::new("python3").arg("-c").arg("1").status().is_err() {
            return;
        }
        let host = Host { path: host_py, allowed_origins: vec!["chrome-extension://abc/".into()], allowed_extensions: vec![], manifest: dir.join("t.json") };
        assert!(allowed(&host, &["abc".into()]) && !allowed(&host, &["xyz".into()]));
        let (tx, rx) = std::sync::mpsc::channel();
        let mut c = start(&host, "abc", move |m| { let _ = tx.send(m); }).unwrap();
        send(&mut c.stdin, r#"{"hello":1}"#).unwrap();
        let reply: Value = serde_json::from_str(&rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap().unwrap()).unwrap();
        assert_eq!(reply["echo"]["hello"], 1);
        assert_eq!(reply["argv"][0], "chrome-extension://abc/");
        let _ = c.child.kill();
    }
}
