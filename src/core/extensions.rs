//! Extension installer and registry.
//!
//! Three kinds of extension are supported:
//!
//! * **Chrome / Chromium WebExtensions** (`.crx` v2/v3, `.zip`, or an
//!   unpacked folder; Manifest V2 and V3) – loaded unchanged by Qt
//!   WebEngine's extension manager.
//! * **Firefox WebExtensions** (`.xpi`, `.zip` or folder) – normalised for
//!   Chromium: Gecko-only manifest keys are removed, MV3 event-page
//!   `background.scripts` become a service worker, and a small `browser.*`
//!   promise shim is added so `browser.tabs.query(...)` style code runs.
//! * **OyNIx extensions** – the lightweight native format: a folder, `.zip`
//!   or `.oynix` holding `oynix.json` + `code.ts` + `content.css`, plus
//!   optional `graphics.qt` (a QML panel), `extra.html` (a page) and
//!   `extra.jsonl` (data). `code.ts` is compiled to JavaScript on install; a
//!   package with only `content.js` is converted to `code.ts` first.
//!   OyNIx injects the result itself on matching pages.
//!
//! * **Power extensions** (`.poweroynix`, see `poweroynix.rs`) – an OyNIx
//!   extension that may also carry a Chrome/Firefox `manifest.json` (MV2 or
//!   MV3) and gets the powerful APIs without Developer mode. They must carry
//!   a valid signature from a trusted key; built-ins are exempt.
//!
//! `manifest_version` values above 3 do not exist yet; they are accepted and
//! treated as MV3 so forward-dated manifests still load, with a warning.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::io::Read;
use std::path::{Path, PathBuf};

use super::{library::now, paths};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Entry {
    /// Stable OyNIx id (folder name).
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    /// "chrome" | "firefox" | "oynix"
    pub kind: String,
    pub manifest_version: u32,
    /// Absolute path of the prepared, unpacked extension.
    pub path: String,
    pub enabled: bool,
    pub warnings: Vec<String>,
    /// Absolute path to the best icon, or "".
    pub icon: String,
    pub has_popup: bool,
    pub options_page: String,
    pub installed: i64,
    /// Chromium extension id assigned by Qt WebEngine once loaded.
    pub web_id: String,
    /// Page OyNIx runs as the background context (relative), or "".
    pub background_page: String,
    /// onInstalled has not fired yet.
    pub first_run: bool,
    // OyNIx-format only
    pub matches: Vec<String>,
    pub excludes: Vec<String>,
    pub run_at: String,
    pub world: String,
    /// Has `graphics.qt` (a QML panel shown from the toolbar).
    pub has_panel: bool,
    /// Has `extra.html` (a page shown from the toolbar when there's no panel).
    pub has_page: bool,
    /// Number of records in `extra.jsonl`.
    pub data_records: usize,
    /// Has a valid `theme.json` (can restyle the browser).
    pub has_theme: bool,
    /// Has `widget.tsx` (a card on the Start Page).
    pub has_widget: bool,
    /// Power only: has `theme-shape.qml` (draws the browser's shapes).
    pub has_shape: bool,
    /// Power only: has `network.ts` (runs in the background, see netrules.rs).
    pub has_network: bool,
    /// Power only: number of pictures in `images/`.
    pub images: usize,
    /// Extra abilities granted in oynix.json ("ocr").
    pub permissions: Vec<String>,
    /// Ships with OyNIx: can be turned off but not removed.
    pub builtin: bool,
    /// A verified power extension (`.poweroynix`): full API access.
    pub power: bool,
    /// Who verified it ("OyNIx" for built-ins).
    pub verified_by: String,
}

impl Entry {
    /// Has a Chrome/Firefox half loaded by Chromium's extension system.
    pub fn is_webext(&self) -> bool {
        self.kind != "oynix" || self.manifest_version > 0
    }
}

/// `filters.txt` of every enabled power extension with the "filters"
/// permission, for the content blocker.
pub fn filter_files(list: &[Entry]) -> Vec<PathBuf> {
    list.iter()
        .filter(|e| e.enabled && e.power && e.permissions.iter().any(|p| p == "filters"))
        .map(|e| Path::new(&e.path).join("filters.txt"))
        .filter(|p| p.exists())
        .collect()
}

pub fn root() -> PathBuf {
    let p = paths::data_dir().join("extensions");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn registry_file() -> PathBuf {
    paths::data_dir().join("extensions.json")
}

pub fn load() -> Vec<Entry> {
    let mut list: Vec<Entry> = std::fs::read(registry_file())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();

    // Older OyNIx releases stored MV2 manifests unchanged. Re-run the same
    // compatibility conversion used by the installer so existing extensions
    // (notably uBlock Origin) are migrated after a browser update too.
    let mut migrated = false;
    for entry in &mut list {
        let manifest_path = Path::new(&entry.path).join("manifest.json");
        let Ok(text) = std::fs::read_to_string(&manifest_path) else { continue };
        let Ok(manifest) = parse_json_lenient(&text) else { continue };
        if manifest.get("manifest_version").and_then(Value::as_u64) != Some(2) {
            continue;
        }
        let old_kind = entry.kind.clone();
        let old_enabled = entry.enabled;
        let old_builtin = entry.builtin;
        let old_power = entry.power;
        let old_verified_by = entry.verified_by.clone();
        let old_installed = entry.installed;
        let old_first_run = entry.first_run;
        let old_id = entry.id.clone();
        match prepare_webext(Path::new(&entry.path), entry, None, false) {
            Ok(()) => {
                entry.id = old_id;
                entry.kind = old_kind;
                entry.enabled = old_enabled;
                entry.builtin = old_builtin;
                entry.power = old_power;
                entry.verified_by = old_verified_by;
                entry.installed = old_installed;
                entry.first_run = old_first_run;
                migrated = true;
                eprintln!("oynix: migrated existing Manifest V2 extension {}", entry.name);
            }
            Err(err) => {
                eprintln!("oynix: could not migrate Manifest V2 extension {}: {err}", entry.name);
            }
        }
    }
    if migrated {
        save(&list);
    }
    list
}

pub fn save(list: &[Entry]) {
    if let Ok(b) = serde_json::to_vec_pretty(list) {
        let _ = paths::write_atomic(&registry_file(), &b);
    }
}

// ── Archive handling ───────────────────────────────────────────────────

/// Strip a CRX2/CRX3 header, returning the embedded zip.
fn crx_payload(bytes: &[u8]) -> Result<&[u8], String> {
    let u32le = |o: usize| -> Result<usize, String> {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .ok_or_else(|| "truncated CRX header".to_string())
    };
    match u32le(4)? {
        2 => {
            let start = 16 + u32le(8)? + u32le(12)?;
            bytes.get(start..).ok_or_else(|| "truncated CRX2 file".into())
        }
        3 => {
            let start = 12 + u32le(8)?;
            bytes.get(start..).ok_or_else(|| "truncated CRX3 file".into())
        }
        v => Err(format!("unsupported CRX version {v}")),
    }
}

/// The extension's public key from a CRX header. Chrome derives the
/// extension id from it; keeping it as the manifest "key" gives the
/// extension the same id in OyNIx as in the Chrome Web Store, which native
/// messaging hosts and some web services check.
pub fn crx_public_key(bytes: &[u8]) -> Option<Vec<u8>> {
    let u32le = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    if !bytes.starts_with(b"Cr24") {
        return None;
    }
    match u32le(4)? {
        2 => {
            let len = u32le(8)?;
            bytes.get(16..16 + len).map(|k| k.to_vec())
        }
        3 => {
            let header = bytes.get(12..12 + u32le(8)?)?;
            // CrxFileHeader: repeated AsymmetricKeyProof sha256_with_rsa = 2,
            // sha256_with_ecdsa = 3 (each {bytes public_key = 1}), and
            // signed_header_data = 10000 ({bytes crx_id = 1}).
            let mut keys = Vec::new();
            let mut crx_id = None;
            for (field, data) in proto_fields(header)? {
                match field {
                    2 | 3 => {
                        if let Some((_, k)) = proto_fields(data)?.into_iter().find(|(f, _)| *f == 1) {
                            keys.push(k.to_vec());
                        }
                    }
                    10000 => crx_id = proto_fields(data)?.into_iter().find(|(f, _)| *f == 1).map(|(_, v)| v.to_vec()),
                    _ => {}
                }
            }
            // The key whose hash is the crx_id is the extension's own (the
            // others are the store's signing keys).
            use sha2::{Digest, Sha256};
            match crx_id {
                Some(id) => keys.into_iter().find(|k| Sha256::digest(k)[..16] == id[..]),
                None => keys.into_iter().next(),
            }
        }
        _ => None,
    }
}

/// Length-delimited fields of a protobuf message: (field number, bytes).
/// Other wire types are skipped.
fn proto_fields(mut b: &[u8]) -> Option<Vec<(u64, &[u8])>> {
    fn varint(b: &mut &[u8]) -> Option<u64> {
        let mut v = 0u64;
        for i in 0..10 {
            let (&byte, rest) = b.split_first()?;
            *b = rest;
            v |= ((byte & 0x7f) as u64) << (7 * i);
            if byte & 0x80 == 0 {
                return Some(v);
            }
        }
        None
    }
    let mut out = Vec::new();
    while !b.is_empty() {
        let tag = varint(&mut b)?;
        match tag & 7 {
            0 => {
                varint(&mut b)?;
            }
            1 => b = b.get(8..)?,
            5 => b = b.get(4..)?,
            2 => {
                let len = varint(&mut b)? as usize;
                out.push((tag >> 3, b.get(..len)?));
                b = &b[len..];
            }
            _ => return None,
        }
    }
    Some(out)
}

fn b64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn unzip(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("not a valid archive: {e}"))?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name() rejects absolute paths and `..` traversal.
        let Some(rel) = f.enclosed_name() else { continue };
        let out = dest.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        std::fs::write(&out, buf).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let name = e.file_name();
        if name == ".git" || name == "node_modules" {
            continue;
        }
        let ty = e.file_type()?;
        if ty.is_dir() {
            copy_dir(&e.path(), &dest.join(&name))?;
        } else if ty.is_file() {
            std::fs::copy(e.path(), dest.join(&name))?;
        }
    }
    Ok(())
}

/// If the archive wrapped everything in one top-level folder, return it.
fn content_root(dir: &Path) -> PathBuf {
    if dir.join("manifest.json").exists() || dir.join("oynix.json").exists() {
        return dir.to_path_buf();
    }
    let entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| !p.file_name().map(|n| n == "__MACOSX").unwrap_or(false)).collect())
        .unwrap_or_default();
    if entries.len() == 1 && entries[0].is_dir() {
        return content_root(&entries[0]);
    }
    dir.to_path_buf()
}

// ── Manifest helpers ──────────────────────────────────────────────────

/// Parse JSON allowing a BOM and `//` line comments (Chrome accepts both).
fn parse_json_lenient(text: &str) -> Result<Value, String> {
    let text = text.trim_start_matches('\u{feff}');
    if let Ok(v) = serde_json::from_str(text) {
        return Ok(v);
    }
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let mut in_str = false;
        let mut prev = '\0';
        let mut cut = line.len();
        for (i, c) in line.char_indices() {
            if c == '"' && prev != '\\' {
                in_str = !in_str;
            }
            if !in_str && c == '/' && prev == '/' {
                cut = i - 1;
                break;
            }
            prev = c;
        }
        out.push_str(&line[..cut]);
        out.push('\n');
    }
    serde_json::from_str(&out).map_err(|e| format!("invalid manifest.json: {e}"))
}

/// Resolve `__MSG_key__` placeholders using `_locales/<default_locale>`.
fn localize(dir: &Path, manifest: &Value, s: &str) -> String {
    let Some(key) = s.strip_prefix("__MSG_").and_then(|k| k.strip_suffix("__")) else { return s.to_string() };
    let locale = manifest.get("default_locale").and_then(Value::as_str).unwrap_or("en");
    for loc in [locale, "en", "en_US"] {
        let f = dir.join("_locales").join(loc).join("messages.json");
        if let Ok(t) = std::fs::read_to_string(f) {
            if let Ok(v) = parse_json_lenient(&t) {
                if let Some(obj) = v.as_object() {
                    for (k, m) in obj {
                        if k.eq_ignore_ascii_case(key) {
                            if let Some(msg) = m.get("message").and_then(Value::as_str) {
                                return msg.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    key.to_string()
}

fn best_icon(dir: &Path, manifest: &Value) -> String {
    let mut candidates: Vec<(u32, String)> = Vec::new();
    let mut collect = |v: Option<&Value>| match v {
        Some(Value::Object(m)) => {
            for (size, p) in m {
                if let Some(p) = p.as_str() {
                    candidates.push((size.parse().unwrap_or(0), p.to_string()));
                }
            }
        }
        Some(Value::String(p)) => candidates.push((16, p.clone())),
        _ => {}
    };
    collect(manifest.get("icons"));
    for k in ["action", "browser_action", "page_action"] {
        collect(manifest.get(k).and_then(|a| a.get("default_icon")));
    }
    candidates.sort_by_key(|(s, _)| *s);
    // Prefer ~32–64px for toolbar use, otherwise the largest.
    let pick = candidates.iter().find(|(s, _)| *s >= 32).or(candidates.last());
    pick.map(|(_, p)| dir.join(p.trim_start_matches('/')))
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn slug(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() { "extension".into() } else { s.chars().take(40).collect() }
}

/// The `browser.*` namespace for Firefox extensions running on Chromium.
/// Wraps callback-style `chrome.*` APIs so they also return promises.
pub const BROWSER_SHIM: &str = r#"/* OyNIx: browser.* namespace for Firefox extensions on Chromium */
(function (g) {
  'use strict';
  if (typeof g.browser !== 'undefined' && g.browser && g.browser.runtime && g.browser.runtime.id) return;
  if (typeof g.chrome === 'undefined' || !g.chrome) return;
  var cache = new WeakMap();
  // Calls that take a listener or return synchronously must not be wrapped.
  var sync = /^(addListener|removeListener|hasListener|hasListeners|getURL|getManifest|getMessage|getUILanguage|connect|connectNative|getBackgroundPage|getViews|getExtensionTabs|create)$/;
  function wrap(target, path) {
    if (target === null || (typeof target !== 'object' && typeof target !== 'function')) return target;
    if (cache.has(target)) return cache.get(target);
    var proxy = new Proxy(target, {
      get: function (obj, key) {
        var value = obj[key];
        if (typeof value === 'function') {
          if (sync.test(String(key)) && !(path === 'contextMenus' && key === 'create')) return value.bind(obj);
          return function () {
            var args = Array.prototype.slice.call(arguments);
            if (args.length && typeof args[args.length - 1] === 'function') return value.apply(obj, args);
            return new Promise(function (resolve, reject) {
              try {
                var r = value.apply(obj, args.concat([function (result) {
                  var err = g.chrome.runtime && g.chrome.runtime.lastError;
                  if (err) reject(new Error(err.message)); else resolve(result);
                }]));
                if (r && typeof r.then === 'function') r.then(resolve, reject);
              } catch (e) {
                try { var r2 = value.apply(obj, args); if (r2 && typeof r2.then === 'function') r2.then(resolve, reject); else resolve(r2); }
                catch (e2) { reject(e2); }
              }
            });
          };
        }
        if (value && typeof value === 'object') return wrap(value, path ? path + '.' + String(key) : String(key));
        return value;
      }
    });
    cache.set(target, proxy);
    return proxy;
  }
  var ns = wrap(g.chrome, '');
  try {
    Object.defineProperty(g, 'browser', { value: ns, configurable: true, writable: true });
    if (g.chrome.contextMenus && !g.chrome.menus) g.chrome.menus = g.chrome.contextMenus;
    if (g.chrome.action && !g.chrome.browserAction) g.chrome.browserAction = g.chrome.action;
  } catch (e) { g.browser = ns; }
})(typeof globalThis !== 'undefined' ? globalThis : self);
"#;

const SHIM_FILE: &str = "oynix-browser-shim.js";
const BG_FILE: &str = "oynix-background.js";

/// Firefox-only manifest keys that Chromium rejects or ignores.
const GECKO_ONLY: &[&str] = &[
    "browser_specific_settings", "applications", "sidebar_action", "theme_experiment",
    "chrome_settings_overrides_firefox", "protocol_handlers", "user_scripts", "developer",
    "dictionaries", "experiment_apis", "incognito_firefox",
];

fn is_firefox(manifest: &Value, from_xpi: bool) -> bool {
    if from_xpi {
        return true;
    }
    if manifest.get("browser_specific_settings").is_some() || manifest.get("applications").is_some() {
        return true;
    }
    let mv = manifest.get("manifest_version").and_then(Value::as_u64).unwrap_or(2);
    let bg = manifest.get("background");
    mv >= 3 && bg.and_then(|b| b.get("scripts")).is_some() && bg.and_then(|b| b.get("service_worker")).is_none()
}

/// Rewrite a Firefox manifest (in place, on disk) so Chromium can load it.
fn convert_firefox(dir: &Path, m: &mut Map<String, Value>, warnings: &mut Vec<String>) -> Result<(), String> {
    for k in GECKO_ONLY {
        if m.remove(*k).is_some() && (*k == "sidebar_action" || *k == "protocol_handlers" || *k == "user_scripts") {
            warnings.push(format!("“{k}” is Firefox-only and was ignored"));
        }
    }
    std::fs::write(dir.join(SHIM_FILE), BROWSER_SHIM).map_err(|e| e.to_string())?;
    let mv = m.get("manifest_version").and_then(Value::as_u64).unwrap_or(2);

    // Permissions: Firefox aliases.
    for key in ["permissions", "optional_permissions"] {
        if let Some(Value::Array(perms)) = m.get_mut(key) {
            for p in perms.iter_mut() {
                if p.as_str() == Some("menus") {
                    *p = json!("contextMenus");
                }
            }
            perms.retain(|p| !matches!(p.as_str(), Some("menus.overrideContext") | Some("pkcs11") | Some("geckoProfiler") | Some("find") | Some("theme")));
            if mv >= 3 {
                // MV3 moved host patterns to host_permissions.
                let (hosts, rest): (Vec<Value>, Vec<Value>) =
                    perms.drain(..).partition(|p| p.as_str().map(|s| s.contains("://") || s == "<all_urls>").unwrap_or(false));
                *perms = rest;
                if !hosts.is_empty() && key == "permissions" {
                    let hp = m.entry("host_permissions").or_insert_with(|| json!([]));
                    if let Value::Array(a) = hp {
                        a.extend(hosts);
                    }
                }
            }
        }
    }

    // Action style hints Chromium doesn't know.
    for k in ["action", "browser_action", "page_action", "options_ui"] {
        if let Some(Value::Object(a)) = m.get_mut(k) {
            a.remove("browser_style");
            a.remove("theme_icons");
            a.remove("default_area");
        }
    }

    // Background.
    if let Some(Value::Object(bg)) = m.get_mut("background") {
        let scripts: Vec<String> = bg
            .get("scripts")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        let is_module = bg.get("type").and_then(Value::as_str) == Some("module");
        if mv >= 3 && !scripts.is_empty() && bg.get("service_worker").is_none() {
            let body = if is_module {
                std::iter::once(format!("import './{SHIM_FILE}';"))
                    .chain(scripts.iter().map(|s| format!("import './{}';", s.trim_start_matches('/'))))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                let list = std::iter::once(SHIM_FILE.to_string())
                    .chain(scripts.iter().map(|s| s.trim_start_matches('/').to_string()))
                    .map(|s| serde_json::to_string(&s).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("importScripts({list});")
            };
            std::fs::write(dir.join(BG_FILE), format!("// Generated by OyNIx from background.scripts\n{body}\n")).map_err(|e| e.to_string())?;
            bg.remove("scripts");
            bg.remove("persistent");
            bg.insert("service_worker".into(), json!(BG_FILE));
            warnings.push("Background page converted to a service worker (no DOM access in background)".into());
        } else if !scripts.is_empty() {
            let mut list = vec![json!(SHIM_FILE)];
            list.extend(scripts.into_iter().map(Value::String));
            bg.insert("scripts".into(), Value::Array(list));
        } else if mv >= 3 && bg.get("page").is_some() {
            warnings.push("MV3 background pages are not supported by Chromium; background was dropped".into());
            bg.remove("page");
        }
    }

    // Content scripts get the shim first.
    if let Some(Value::Array(cs)) = m.get_mut("content_scripts") {
        for c in cs.iter_mut() {
            if let Some(Value::Array(js)) = c.get_mut("js") {
                if !js.iter().any(|j| j.as_str() == Some(SHIM_FILE)) {
                    js.insert(0, json!(SHIM_FILE));
                }
            }
        }
    }
    Ok(())
}

/// Rewrite a Manifest V2 manifest so Qt WebEngine's Chromium (which no
/// longer loads V2) accepts it. OyNIx runs the background page itself and
/// supplies the chrome.* APIs, so Chromium only needs to serve the
/// extension's files and its action popup; V2 behaviour lives in OyNIx.
fn convert_mv2(m: &mut Map<String, Value>, warnings: &mut Vec<String>) {
    m.insert("manifest_version".into(), json!(3));
    // browser_action / page_action -> action
    let action = m.remove("browser_action");
    let page_action = m.remove("page_action");
    if !m.contains_key("action") {
        if let Some(a) = action.or(page_action) {
            m.insert("action".into(), a);
        }
    }
    // Host patterns moved to host_permissions in V3.
    for (from, to) in [("permissions", "host_permissions"), ("optional_permissions", "optional_host_permissions")] {
        let mut hosts = vec![];
        if let Some(Value::Array(perms)) = m.get_mut(from) {
            let (h, rest): (Vec<Value>, Vec<Value>) = perms.drain(..).partition(|p| p.as_str().is_some_and(|s| s.contains("://") || s == "<all_urls>"));
            *perms = rest;
            hosts = h;
        }
        if !hosts.is_empty() {
            if let Value::Array(a) = m.entry(to).or_insert_with(|| json!([])) {
                a.extend(hosts);
            }
        }
    }
    // The background page runs in OyNIx (from the original manifest).
    m.remove("background");
    // V2's CSP string isn't valid in V3; Chromium's default applies.
    if matches!(m.get("content_security_policy"), Some(Value::String(_))) {
        m.remove("content_security_policy");
        warnings.push("The extension's own content security policy was replaced by Chromium's default (eval isn't allowed)".into());
    }
    // web_accessible_resources: strings -> V3 objects.
    if let Some(Value::Array(war)) = m.get_mut("web_accessible_resources") {
        let names: Vec<Value> = war.iter().filter(|w| w.is_string()).cloned().collect();
        war.retain(|w| !w.is_string());
        if !names.is_empty() {
            war.push(json!({ "resources": names, "matches": ["<all_urls>"] }));
        }
    }
    if let Some(Value::Object(c)) = m.get_mut("commands") {
        for k in ["_execute_browser_action", "_execute_page_action"] {
            if let Some(v) = c.remove(k) {
                c.insert("_execute_action".into(), v);
            }
        }
    }
    warnings.push("Manifest V2: Chromium in Qt WebEngine no longer loads V2 extensions, so OyNIx runs this one as V3 and provides the V2 APIs itself. webRequest “blocking” answers can’t cancel or change requests".into());
}

/// Qt WebEngine's renderer is not an extension process, so Chromium only
/// serves files listed in `web_accessible_resources` — even to the
/// extension's own pages. Grant the extension (and only extensions, not web
/// pages, for MV3) access to all of its files.
fn expose_own_resources(m: &mut Map<String, Value>, mv: u32) {
    let war = m.entry("web_accessible_resources").or_insert_with(|| json!([]));
    let Value::Array(list) = war else { return };
    if mv >= 3 {
        let already = list.iter().any(|w| w.get("extension_ids").map(|x| x == &json!(["*"])).unwrap_or(false) && w.get("resources") == Some(&json!(["*"])));
        if !already {
            list.push(json!({ "resources": ["*"], "extension_ids": ["*"] }));
        }
    } else if !list.iter().any(|w| w.as_str() == Some("*")) {
        list.push(json!("*"));
    }
}

/// Qt WebEngine does not run extension service workers / background scripts
/// with a `chrome.*` API, so OyNIx hosts the background context itself in a
/// hidden page. Returns the page path relative to the extension root.
fn write_background_page(dir: &Path, manifest: &Value) -> Result<String, String> {
    let Some(bg) = manifest.get("background") else { return Ok(String::new()) };
    if let Some(page) = bg.get("page").and_then(Value::as_str) {
        return Ok(page.trim_start_matches('/').to_string());
    }
    let module = bg.get("type").and_then(Value::as_str) == Some("module");
    let mut scripts: Vec<String> = bg
        .get("scripts")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    if let Some(sw) = bg.get("service_worker").and_then(Value::as_str) {
        scripts.push(sw.to_string());
    }
    if scripts.is_empty() {
        return Ok(String::new());
    }
    let esc = |s: &str| s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
    let tags: String = scripts
        .iter()
        .map(|s| format!("<script{} src=\"{}\"></script>\n", if module { " type=\"module\"" } else { "" }, esc(&format!("/{}", s.trim_start_matches('/')))))
        .collect();
    let html = format!("<!doctype html>\n<!-- Generated by OyNIx: background context -->\n<meta charset=\"utf-8\"><title>background</title>\n{tags}");
    std::fs::write(dir.join(BG_PAGE), html).map_err(|e| e.to_string())?;
    Ok(BG_PAGE.to_string())
}

const BG_PAGE: &str = "oynix-background.html";

/// Messages for `chrome.i18n.getMessage` (default locale, falling back to en).
fn messages(dir: &Path, manifest: &Value) -> Value {
    let locale = manifest.get("default_locale").and_then(Value::as_str).unwrap_or("en");
    let mut out = Map::new();
    for loc in ["en", locale] {
        if let Ok(t) = std::fs::read_to_string(dir.join("_locales").join(loc).join("messages.json")) {
            if let Ok(Value::Object(m)) = parse_json_lenient(&t) {
                for (k, v) in m {
                    out.insert(k.to_lowercase(), v.clone());
                    out.insert(k, v);
                }
            }
        }
    }
    Value::Object(out)
}

fn read_manifest(e: &Entry) -> Option<Value> {
    std::fs::read_to_string(Path::new(&e.path).join("manifest.json")).ok().and_then(|t| parse_json_lenient(&t).ok())
}

/// Context object handed to the JS runtime for an extension.
pub fn runtime_context(e: &Entry, kind: &str) -> Value {
    let dir = Path::new(&e.path);
    let manifest = read_manifest(e).unwrap_or(Value::Null);
    json!({ "ext": e.web_id, "kind": kind, "manifest": manifest, "messages": messages(dir, &manifest), "firstRun": e.first_run })
}

/// Contexts for extension pages, keyed by Chromium extension id.
pub fn page_contexts(list: &[Entry], kind: &str) -> Value {
    let mut m = Map::new();
    for e in list.iter().filter(|e| e.enabled && e.is_webext() && !e.web_id.is_empty()) {
        m.insert(e.web_id.clone(), runtime_context(e, kind));
    }
    Value::Object(m)
}

fn gm_pattern(p: &str) -> String {
    if p == "<all_urls>" { "*://*/*".into() } else { p.to_string() }
}

/// Build injectable scripts for an extension's `content_scripts`.
/// Each entry becomes one Greasemonkey-headed script so Qt WebEngine applies
/// match patterns and timing natively; the code runs inside
/// `__oynixExt(ctx, function (chrome, browser) { … })`.
pub fn content_scripts(e: &Entry) -> Vec<Value> {
    let Some(manifest) = read_manifest(e) else { return vec![] };
    let dir = Path::new(&e.path);
    let ctx = serde_json::to_string(&runtime_context(e, "content")).unwrap_or_else(|_| "{}".into());
    let mut out = Vec::new();
    let Some(list) = manifest.get("content_scripts").and_then(Value::as_array) else { return out };
    for (i, cs) in list.iter().enumerate() {
        let strs = |k: &str| -> Vec<String> {
            cs.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        let matches = strs("matches");
        if matches.is_empty() {
            continue;
        }
        let run_at = match cs.get("run_at").and_then(Value::as_str).unwrap_or("document_idle") {
            "document_start" => "document-start",
            "document_end" => "document-end",
            _ => "document-idle",
        };
        let main_world = cs.get("world").and_then(Value::as_str) == Some("MAIN");
        let mut header = format!("// ==UserScript==\n// @name ext-{}-{i}\n", e.id);
        for m in &matches {
            header.push_str(&format!("// @match {}\n", gm_pattern(m)));
        }
        for x in strs("exclude_matches").iter().chain(strs("exclude_globs").iter()) {
            header.push_str(&format!("// @exclude {x}\n"));
        }
        for x in strs("include_globs") {
            header.push_str(&format!("// @include {x}\n"));
        }
        header.push_str(&format!("// @run-at {run_at}\n// ==/UserScript==\n"));

        let mut css = String::new();
        for f in strs("css") {
            if let Ok(t) = std::fs::read_to_string(dir.join(f.trim_start_matches('/'))) {
                css.push_str(&t);
                css.push('\n');
            }
        }
        let mut js = String::new();
        for f in strs("js") {
            if f.ends_with(SHIM_FILE) {
                continue; // the runtime provides `browser`
            }
            if let Ok(t) = std::fs::read_to_string(dir.join(f.trim_start_matches('/'))) {
                js.push_str(&format!("// ── {f}\n{t}\n;\n"));
            }
        }
        let css_inject = if css.trim().is_empty() {
            String::new()
        } else {
            format!(
                "(function(){{var s=document.createElement('style');s.dataset.oynixExtension={};s.textContent={};(document.head||document.documentElement).appendChild(s);}})();\n",
                serde_json::to_string(&e.id).unwrap(),
                serde_json::to_string(&css).unwrap()
            )
        };
        let source = if main_world {
            format!("{header}{css_inject}{js}")
        } else {
            format!("{header}__oynixExt({ctx}, function (chrome, browser) {{\n{css_inject}{js}}});\n")
        };
        out.push(json!({
            "name": format!("ext-{}-{i}", e.id),
            "source": source,
            "main": main_world,
            "greasemonkey": true,
            "allFrames": cs.get("all_frames").and_then(Value::as_bool).unwrap_or(false),
            "extension": true
        }));
    }
    out
}

// ── OyNIx native format ───────────────────────────────────────────────

/// Largest `extra.jsonl` OyNIx accepts (it's injected into every matching page).
const MAX_DATA_BYTES: u64 = 4 << 20;

const CONVERTED_HEADER: &str = "// Converted from content.js by OyNIx. JavaScript is valid TypeScript,\n// so nothing else changed; add types whenever you like.\n\n";

/// Make sure a folder has `code.ts`, converting a lone `content.js`.
/// Returns a warning when something was converted or ignored.
fn ensure_code_ts(dir: &Path) -> Result<Option<String>, String> {
    let ts = dir.join("code.ts");
    let js = dir.join("content.js");
    if ts.exists() {
        if js.exists() {
            let _ = std::fs::remove_file(&js);
            return Ok(Some("Both code.ts and content.js were present; content.js is ignored".into()));
        }
        return Ok(None);
    }
    if js.exists() {
        let src = std::fs::read_to_string(&js).map_err(|e| format!("content.js: {e}"))?;
        std::fs::write(&ts, format!("{CONVERTED_HEADER}{src}")).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&js);
        return Ok(Some("content.js was converted to code.ts".into()));
    }
    if dir.join("content.ts").exists() {
        std::fs::rename(dir.join("content.ts"), &ts).map_err(|e| e.to_string())?;
        return Ok(Some("content.ts was renamed to code.ts".into()));
    }
    Err("code.ts is missing (OyNIx extensions need oynix.json, code.ts and content.css)".into())
}

/// Where the compiled `code.ts` is kept inside an installed extension.
fn compiled_path(dir: &Path) -> PathBuf {
    dir.join(".build").join("code.js")
}

/// Compile `code.ts` if the cached JavaScript is missing or older.
pub fn compile_code(dir: &Path) -> Result<String, String> {
    ensure_code_ts(dir)?;
    let ts = dir.join("code.ts");
    let out = compiled_path(dir);
    let fresh = match (std::fs::metadata(&ts).and_then(|m| m.modified()), std::fs::metadata(&out).and_then(|m| m.modified())) {
        (Ok(a), Ok(b)) => b >= a,
        _ => false,
    };
    if fresh {
        if let Ok(js) = std::fs::read_to_string(&out) {
            return Ok(js);
        }
    }
    let src = std::fs::read_to_string(&ts).map_err(|e| format!("code.ts: {e}"))?;
    let js = super::tscompile::transpile(&src, "code.ts")?;
    let _ = std::fs::create_dir_all(out.parent().unwrap());
    let _ = std::fs::write(&out, &js);
    Ok(js)
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "avif"];
const MAX_IMAGE_BYTES: u64 = 2 << 20;

/// Pictures in a power extension's `images/` folder: (name, mime, path).
fn image_files(dir: &Path) -> Vec<(String, &'static str, PathBuf)> {
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(dir.join("images")) else { return out };
    for f in rd.flatten() {
        let p = f.path();
        let ext = p.extension().map(|x| x.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !p.is_file() || !IMAGE_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let mime = match ext.as_str() {
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "avif" => "image/avif",
            _ => "image/jpeg",
        };
        out.push((f.file_name().to_string_lossy().into_owned(), mime, p));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The extras only verified power extensions get: extra.md (rendered to a
/// page), images/, theme-shape.qml and network.ts.
fn prepare_power_extras(dir: &Path, e: &mut Entry) -> Result<(), String> {
    let imgs = image_files(dir);
    let total: u64 = imgs.iter().filter_map(|(_, _, p)| std::fs::metadata(p).ok()).map(|m| m.len()).sum();
    if total > MAX_IMAGE_BYTES {
        return Err(format!("images/ holds {} KB; the limit is {} KB", total >> 10, MAX_IMAGE_BYTES >> 10));
    }
    e.images = imgs.len();
    if dir.join("extra.md").exists() {
        let md = std::fs::read_to_string(dir.join("extra.md")).map_err(|err| format!("extra.md: {err}"))?;
        std::fs::write(dir.join("extra.md.html"), markdown_page(&e.name, &md)).map_err(|err| err.to_string())?;
        if dir.join("extra.html").exists() {
            e.warnings.push("extra.html and extra.md both exist; the page shows extra.html".into());
        }
        e.has_page = true;
    }
    if dir.join("theme-shape.qml").exists() {
        let qml = std::fs::read_to_string(dir.join("theme-shape.qml")).map_err(|err| format!("theme-shape.qml: {err}"))?;
        super::qmlcheck::check(&qml).map_err(|err| err.replace("graphics.qt", "theme-shape.qml").replace("in panels", "in theme shapes"))?;
        for prop in ["shape", "color"] {
            if !qml.contains(&format!("property string {prop}")) && !qml.contains(&format!("property color {prop}")) {
                return Err(format!("theme-shape.qml needs a “{prop}” property (see the .poweroynix docs)"));
            }
        }
        e.has_shape = true;
    }
    if dir.join("network.ts").exists() {
        compile_named(dir, "network.ts", "network.js")?;
        e.has_network = true;
    }
    Ok(())
}

/// The `oynix` object in network.ts. Every call is a message carrying the
/// page's token through ExtBridge; NetworkHost.qml checks it, does the
/// work and answers with `__oynixNet.result(...)`.
const NETWORK_API: &str = r#"var oynix = (function () {
  var id = __ID__, token = __TOKEN__, images = __IMAGES__, pending = {}, listeners = [], seq = 0;
  var ready = new Promise(function (resolve) {
    (function wait() {
      if (typeof qt === 'undefined' || !qt.webChannelTransport || typeof QWebChannel === 'undefined') return setTimeout(wait, 25);
      new QWebChannel(qt.webChannelTransport, function (ch) { resolve(ch.objects.oynixExtensions); });
    })();
  });
  function call(op, args) {
    return ready.then(function (b) {
      return new Promise(function (resolve, reject) {
        var req = 'r' + (++seq);
        pending[req] = { resolve: resolve, reject: reject };
        b.post(id, 'oynix:net', JSON.stringify(Object.assign({ token: token, op: op, req: req }, args || {})));
      });
    });
  }
  window.__oynixNet = {
    result: function (req, value, error) {
      var p = pending[req];
      if (!p) return;
      delete pending[req];
      if (error) p.reject(new Error(error)); else p.resolve(value);
    },
    requests: function (list) {
      list.forEach(function (r) { listeners.forEach(function (f) { try { f(r); } catch (e) { console.error(e); } }); });
    }
  };
  function response(r) {
    return { status: r.status, ok: r.status >= 200 && r.status < 300, headers: r.headers || {},
             text: function () { return Promise.resolve(r.body); },
             json: function () { return Promise.resolve(JSON.parse(r.body)); } };
  }
  return Object.freeze({
    id: id,
    data: Object.freeze(__DATA__),
    image: function (name) { return images[name] || ''; },
    network: Object.freeze({
      // Blocker rules in Adblock/uBlock syntax; replaces this extension's earlier list.
      block: function (rules) { return call('block', { rules: rules }); },
      // Add a header to requests whose address matches pattern (* wildcards).
      setRequestHeader: function (pattern, name, value) { return call('header', { pattern: pattern, name: name, value: String(value) }); },
      clearRequestHeaders: function () { return call('clearHeaders'); },
      // Called with {url, firstParty, method, type, blocked, time} for every request.
      onRequest: function (f) { if (typeof f !== 'function') return; listeners.push(f); if (listeners.length === 1) call('observe', { on: true }); },
      // fetch() without cross-origin limits. Resolves to {status, ok, headers, text(), json()}.
      fetch: function (url, init) { init = init || {}; return call('fetch', { url: String(url), method: init.method || 'GET', headers: init.headers || {}, body: init.body === undefined ? undefined : String(init.body) }).then(response); }
    })
  });
})();"#;

/// The hidden page a power extension's network.ts runs in.
pub fn network_html(e: &Entry, token: &str) -> Result<String, String> {
    let dir = Path::new(&e.path);
    let js = compile_named(dir, "network.ts", "network.js")?;
    let data = serde_json::to_string(&read_data(dir).unwrap_or_default()).unwrap_or_else(|_| "[]".into());
    let api = NETWORK_API
        .replace("__ID__", &serde_json::to_string(&e.id).unwrap_or_default())
        .replace("__TOKEN__", &serde_json::to_string(token).unwrap_or_default())
        .replace("__IMAGES__", &serde_json::to_string(&image_map(e)).unwrap_or_else(|_| "{}".into()))
        .replace("__DATA__", &data);
    let esc = |s: &str| s.replace("</", "<\\/");
    Ok(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{} network.ts</title></head><body><script>\n{}\n{}\n(function () {{\n{}\n}})();\n</script></body></html>",
        e.name.replace('<', "&lt;"),
        esc(&crate::bridge::native::ffi::webchannel_js()),
        esc(&api),
        esc(&js)
    ))
}

/// Markdown → a complete, readable page (light and dark).
pub fn markdown_page(title: &str, md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_FOOTNOTES | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut body = String::new();
    html::push_html(&mut body, Parser::new_ext(md, opts));
    let t = title.replace('&', "&amp;").replace('<', "&lt;");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{t}</title><style>\
:root{{color-scheme:light dark;--fg:#1d1b20;--muted:#49454f;--line:#cac4d0;--code:#f3edf7;--accent:#6750a4}}\
@media (prefers-color-scheme:dark){{:root{{--fg:#e6e0e9;--muted:#cac4d0;--line:#49454f;--code:#2b2930;--accent:#d0bcff}}}}\
body{{font:15px/1.6 system-ui,sans-serif;color:var(--fg);margin:0;padding:20px 22px;max-width:760px}}\
h1,h2,h3{{line-height:1.25;margin:1.2em 0 .4em}}h1{{font-size:1.6em}}a{{color:var(--accent)}}img{{max-width:100%;border-radius:10px}}\
code{{background:var(--code);padding:.1em .35em;border-radius:5px;font-size:.92em}}pre{{background:var(--code);padding:12px 14px;border-radius:10px;overflow:auto}}pre code{{padding:0;background:none}}\
table{{border-collapse:collapse}}td,th{{border:1px solid var(--line);padding:5px 9px}}blockquote{{margin:0;padding-left:14px;border-left:3px solid var(--line);color:var(--muted)}}\
</style></head><body>{body}</body></html>"
    )
}

fn compile_named(dir: &Path, src_name: &str, out_name: &str) -> Result<String, String> {
    let src_path = dir.join(src_name);
    let out = dir.join(".build").join(out_name);
    let fresh = match (std::fs::metadata(&src_path).and_then(|m| m.modified()), std::fs::metadata(&out).and_then(|m| m.modified())) {
        (Ok(a), Ok(b)) => b >= a,
        _ => false,
    };
    if fresh {
        if let Ok(js) = std::fs::read_to_string(&out) {
            return Ok(js);
        }
    }
    let src = std::fs::read_to_string(&src_path).map_err(|e| format!("{src_name}: {e}"))?;
    let js = super::tscompile::transpile(&src, src_name)?;
    let _ = std::fs::create_dir_all(out.parent().unwrap());
    let _ = std::fs::write(&out, &js);
    Ok(js)
}

/// Data URLs of a power extension's pictures, by file name, for
/// `oynix.image(name)` in code.ts and network.ts.
pub fn image_map(e: &Entry) -> serde_json::Map<String, Value> {
    use base64::Engine;
    let mut m = serde_json::Map::new();
    if !e.power {
        return m;
    }
    for (name, mime, p) in image_files(Path::new(&e.path)) {
        if let Ok(b) = std::fs::read(p) {
            m.insert(name, json!(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(b))));
        }
    }
    m
}

/// Compile `widget.tsx` (cached like code.ts).
pub fn compile_widget(dir: &Path) -> Result<String, String> {
    let src_path = dir.join("widget.tsx");
    let out = dir.join(".build").join("widget.js");
    let fresh = match (std::fs::metadata(&src_path).and_then(|m| m.modified()), std::fs::metadata(&out).and_then(|m| m.modified())) {
        (Ok(a), Ok(b)) => b >= a,
        _ => false,
    };
    if fresh {
        if let Ok(js) = std::fs::read_to_string(&out) {
            return Ok(js);
        }
    }
    let src = std::fs::read_to_string(&src_path).map_err(|e| format!("widget.tsx: {e}"))?;
    let js = super::tscompile::transpile_tsx(&src, "widget.tsx")?;
    let _ = std::fs::create_dir_all(out.parent().unwrap());
    let _ = std::fs::write(&out, &js);
    Ok(js)
}

/// The tiny React-like runtime widgets are written against: `h`,
/// `Fragment`, `useState`, `useEffect`, `render`, plus `oynix` (the
/// extension's id and extra.jsonl data). Re-rendering rebuilds the DOM;
/// widgets are small, so that's simple and fast enough.
const WIDGET_RUNTIME: &str = r#"
var Fragment = { fragment: true };
var __w = { root: null, el: null, states: [], effects: [], i: 0, e: 0, queued: false };
function h(type, props) {
  var kids = [];
  (function add(list) { for (var i = 0; i < list.length; i++) Array.isArray(list[i]) ? add(list[i]) : kids.push(list[i]); })([].slice.call(arguments, 2));
  return { type: type, props: props || {}, children: kids };
}
function useState(init) {
  var i = __w.i++;
  if (!(i in __w.states)) __w.states[i] = typeof init === 'function' ? init() : init;
  return [__w.states[i], function (v) {
    var next = typeof v === 'function' ? v(__w.states[i]) : v;
    if (next !== __w.states[i]) { __w.states[i] = next; schedule(); }
  }];
}
function useEffect(fn, deps) {
  var i = __w.e++, old = __w.effects[i];
  var changed = !old || !deps || !old.deps || deps.length !== old.deps.length || deps.some(function (d, k) { return d !== old.deps[k]; });
  if (changed) {
    if (old && typeof old.cleanup === 'function') old.cleanup();
    var slot = __w.effects[i] = { deps: deps, cleanup: null };
    setTimeout(function () { slot.cleanup = fn(); }, 0);
  }
}
function useRef(v) { var s = useState(function () { return { current: v }; }); return s[0]; }
function create(node) {
  if (node === null || node === undefined || node === false || node === true) return document.createTextNode('');
  if (typeof node !== 'object') return document.createTextNode(String(node));
  if (typeof node.type === 'function') return create(node.type(Object.assign({ children: node.children }, node.props)));
  if (node.type === Fragment) { var f = document.createDocumentFragment(); node.children.forEach(function (c) { f.appendChild(create(c)); }); return f; }
  var el = document.createElement(node.type);
  Object.keys(node.props).forEach(function (k) {
    var v = node.props[k];
    if (k === 'children' || v === undefined || v === null || v === false) return;
    if (k === 'ref') { if (v) v.current = el; return; }
    if (k === 'style' && typeof v === 'object') { Object.assign(el.style, v); return; }
    if (k.slice(0, 2) === 'on' && typeof v === 'function') { el.addEventListener(k.slice(2).toLowerCase(), v); return; }
    if (k === 'className') k = 'class';
    if (k === 'value' || k === 'checked') { el[k] = v; return; }
    el.setAttribute(k, v === true ? '' : v);
  });
  node.children.forEach(function (c) { el.appendChild(create(c)); });
  return el;
}
function schedule() { if (!__w.queued) { __w.queued = true; requestAnimationFrame(function () { __w.queued = false; draw(); }); } }
function draw() {
  __w.i = 0; __w.e = 0;
  var a = document.activeElement, id = a && a.id, pos = a && a.selectionStart;
  __w.el.replaceChildren(create(__w.root));
  if (id) { var n = document.getElementById(id); if (n) { n.focus(); try { n.setSelectionRange(pos, pos); } catch (e) {} } }
}
function render(node, el) { __w.root = node; __w.el = el || document.getElementById('root'); draw(); }
"#;

/// The page a widget runs in: the theme, the runtime, its data and code.
/// It's loaded with a made-up https origin per extension, so it can use
/// the network and its own localStorage, but no local files.
pub fn widget_html(e: &Entry, theme_css: &str) -> Result<String, String> {
    let dir = Path::new(&e.path);
    let js = compile_widget(dir)?;
    // Modules aren't available here: `export default X` becomes the
    // widget to render, and other exports are kept as plain declarations.
    let mut code = String::new();
    for line in js.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("export default ") {
            code.push_str("__oynixWidget = ");
            code.push_str(rest);
        } else if let Some(rest) = t.strip_prefix("export ") {
            if rest.starts_with('{') {
                continue;
            }
            code.push_str(rest);
        } else {
            code.push_str(line);
        }
        code.push('\n');
    }
    let data = serde_json::to_string(&read_data(dir).unwrap_or_default()).unwrap_or_else(|_| "[]".into());
    let id = serde_json::to_string(&e.id).unwrap_or_default();
    let esc = |s: &str| s.replace("</", "<\\/");
    Ok(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{theme}\nhtml,body{{margin:0;background:transparent;color:var(--fg);font:14px var(--font);}}#root{{padding:14px}}a{{color:var(--accent)}}button{{font:inherit;border:0;border-radius:999px;padding:6px 14px;background:var(--accent);color:var(--on-accent);cursor:pointer}}input{{font:inherit;border:0;border-radius:8px;padding:6px 10px;background:var(--field);color:var(--fg)}}</style></head><body><div id=\"root\"></div><script>\nvar oynix = Object.freeze({{ id: {id}, data: Object.freeze({data}) }});\nvar __oynixWidget = null;\n{runtime}\n{code}\nif (typeof __oynixWidget === 'function') render(h(__oynixWidget, null));\n</script></body></html>",
        theme = esc(theme_css),
        runtime = WIDGET_RUNTIME,
        code = esc(&code),
    ))
}

/// Parse `extra.jsonl`: one JSON value per non-empty line.
pub fn read_data(dir: &Path) -> Result<Vec<Value>, String> {
    let p = dir.join("extra.jsonl");
    let Ok(meta) = std::fs::metadata(&p) else { return Ok(vec![]) };
    if meta.len() > MAX_DATA_BYTES {
        return Err(format!("extra.jsonl is {} MB; the limit is 4 MB", meta.len() >> 20));
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("extra.jsonl: {e}"))?;
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(n, l)| serde_json::from_str(l).map_err(|e| format!("extra.jsonl:{}: {e}", n + 1)))
        .collect()
}

/// Read and validate `theme.json`: how the extension restyles the browser.
///
/// ```json
/// { "accent": "#0b8043",
///   "light": { "surface": "#fbfdf8" }, "dark": { "surface": "#101410" },
///   "font": "Inter", "displayFont": "Fraunces", "corners": 0.5 }
/// ```
/// `light` / `dark` override Material 3 colour roles by name; anything
/// left out is generated from `accent`. `corners` scales corner radii
/// (0 = square, 1 = default, 2 = very round).
pub fn read_theme(dir: &Path) -> Result<Option<Value>, String> {
    let p = dir.join("theme.json");
    let Ok(text) = std::fs::read_to_string(&p) else { return Ok(None) };
    let v = parse_json_lenient(&text).map_err(|e| format!("theme.json: {e}"))?;
    let Value::Object(m) = v else { return Err("theme.json must be an object".into()) };
    let hex = |s: &str| s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit());
    let roles: Vec<String> = super::material::scheme("#2f7cf6", false).as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
    let mut out = Map::new();
    for (k, val) in &m {
        match k.as_str() {
            "accent" => match val.as_str() {
                Some(c) if hex(c) => {
                    out.insert(k.clone(), json!(c.to_lowercase()));
                }
                _ => return Err("theme.json: accent must be a colour like \"#0b8043\"".into()),
            },
            "light" | "dark" => {
                let Value::Object(o) = val else { return Err(format!("theme.json: {k} must be an object of colour roles")) };
                let mut colours = Map::new();
                for (role, c) in o {
                    if !roles.contains(role) {
                        return Err(format!("theme.json: “{role}” isn’t a colour role (e.g. primary, surface, surfaceContainer, onSurface)"));
                    }
                    match c.as_str() {
                        Some(c) if hex(c) => {
                            colours.insert(role.clone(), json!(c.to_lowercase()));
                        }
                        _ => return Err(format!("theme.json: {k}.{role} must be a colour like \"#101410\"")),
                    }
                }
                out.insert(k.clone(), Value::Object(colours));
            }
            "font" | "displayFont" => match val.as_str() {
                Some(f) if !f.is_empty() && f.len() <= 64 && f.chars().all(|c| c.is_alphanumeric() || c == ' ' || c == '-') => {
                    out.insert(k.clone(), json!(f));
                }
                _ => return Err(format!("theme.json: {k} must be a font family name")),
            },
            "corners" => match val.as_f64() {
                Some(x) if (0.0..=2.0).contains(&x) => {
                    out.insert(k.clone(), json!(x));
                }
                _ => return Err("theme.json: corners must be a number from 0 to 2".into()),
            },
            other => return Err(format!("theme.json: unknown key “{other}” (accent, light, dark, font, displayFont, corners)")),
        }
    }
    Ok(Some(Value::Object(out)))
}

/// Validate an OyNIx extension folder: `oynix.json` + `code.ts` (or
/// `content.js`, converted) + `content.css`, and the optional extras.
fn prepare_oynix(dir: &Path, e: &mut Entry, power: bool) -> Result<(), String> {
    let meta = std::fs::read_to_string(dir.join("oynix.json")).map_err(|_| "oynix.json is missing".to_string())?;
    let v = parse_json_lenient(&meta)?;
    if let Some(w) = ensure_code_ts(dir)? {
        e.warnings.push(w);
    }
    if !dir.join("content.css").exists() {
        return Err("content.css is missing (it may be empty, but must exist)".into());
    }
    compile_code(dir)?;
    e.data_records = read_data(dir)?.len();
    e.has_panel = dir.join("graphics.qt").exists();
    if e.has_panel {
        let qml = std::fs::read_to_string(dir.join("graphics.qt")).map_err(|e| format!("graphics.qt: {e}"))?;
        super::qmlcheck::check(&qml)?;
    }
    e.has_page = dir.join("extra.html").exists();
    e.has_theme = read_theme(dir)?.is_some();
    e.has_widget = dir.join("widget.tsx").exists();
    if e.has_widget {
        compile_widget(dir)?;
    }
    e.permissions = match v.get("permissions") {
        None => vec![],
        Some(Value::Array(a)) => {
            let mut out = vec![];
            for p in a {
                match p.as_str() {
                    Some("ocr") => out.push("ocr".to_string()),
                    Some(x) if super::poweroynix::POWER_PERMISSIONS.contains(&x) => {
                        if !power {
                            return Err(format!("the “{x}” permission is only for verified .poweroynix extensions"));
                        }
                        out.push(x.to_string());
                    }
                    Some(other) => return Err(format!("unknown permission “{other}” (OyNIx extensions can ask for: ocr; power extensions also: filters)")),
                    None => return Err("permissions must be a list of strings".into()),
                }
            }
            out
        }
        Some(_) => return Err("permissions must be a list, e.g. [\"ocr\"]".into()),
    };
    if e.has_panel && e.has_page {
        e.warnings.push("graphics.qt and extra.html both exist; the toolbar shows graphics.qt".into());
    }
    let strs = |k: &str| -> Vec<String> {
        match v.get(k) {
            Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => vec![],
        }
    };
    e.name = v.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
    if e.name.is_empty() {
        return Err("oynix.json needs a \"name\"".into());
    }
    e.version = v.get("version").and_then(Value::as_str).unwrap_or("1.0").to_string();
    e.description = v.get("description").and_then(Value::as_str).unwrap_or("").to_string();
    e.matches = strs("matches");
    // "matches": [] means no pages (e.g. an extension that only restyles
    // the browser with theme.json); leaving the key out means every page.
    if e.matches.is_empty() && v.get("matches").is_none() {
        e.matches = vec!["*://*/*".into()];
        e.warnings.push("No \"matches\" given; runs on all http(s) pages".into());
    }
    for m in &e.matches {
        if !(m == "<all_urls>" || m.contains("://")) {
            return Err(format!("invalid match pattern “{m}” (expected e.g. *://*.example.com/*)"));
        }
    }
    e.excludes = strs("exclude");
    e.run_at = match v.get("run_at").and_then(Value::as_str).unwrap_or("document_end") {
        r @ ("document_start" | "document_end" | "document_idle") => r.to_string(),
        other => return Err(format!("run_at must be document_start, document_end or document_idle (got “{other}”)")),
    };
    e.world = match v.get("world").and_then(Value::as_str).unwrap_or("isolated") {
        w @ ("isolated" | "main") => w.to_string(),
        other => return Err(format!("world must be \"isolated\" or \"main\" (got “{other}”)")),
    };
    e.kind = "oynix".into();
    if e.permissions.iter().any(|p| p == "filters") && !dir.join("filters.txt").exists() {
        return Err("the “filters” permission needs a filters.txt".into());
    }
    for icon in ["icon.png", "icon.svg"] {
        if dir.join(icon).exists() {
            e.icon = dir.join(icon).to_string_lossy().into_owned();
            break;
        }
    }
    Ok(())
}

/// The `oynix` object `code.ts` sees: its id, the records from
/// `extra.jsonl`, and messaging with its panel (`graphics.qt` or
/// `extra.html`) through OyNIx's WebChannel bridge.
const OYNIX_API: &str = r#"var oynix = (function () {
  var id = __ID__, listeners = [], bridge = null, pending = {}, canOcr = __OCR__;
  function connect() {
    if (!__MESSAGING__) return Promise.reject(new Error('This extension has no panel to talk to'));
    if (!window.__oynixChannel) window.__oynixChannel = new Promise(function (r) {
      (function wait() {
        if (typeof qt === 'undefined' || !qt.webChannelTransport || typeof QWebChannel === 'undefined') return setTimeout(wait, 25);
        new QWebChannel(qt.webChannelTransport, r);
      })();
    });
    return window.__oynixChannel.then(function (ch) {
      if (!bridge) {
        bridge = ch.objects.oynixExtensions;
        bridge.message.connect(function (ext, target, payload) {
          if (ext !== id || target !== 'content') return;
          var msg; try { msg = JSON.parse(payload); } catch (e) { return; }
          // Answers to oynix.ocr() go to the page that asked.
          if (msg && msg.__oynix === 'ocr') {
            var p = pending[msg.req];
            if (p) { delete pending[msg.req]; if (msg.error) p.reject(new Error(msg.error)); else p.resolve(msg.text); }
            return;
          }
          // The panel talks to the tab you're looking at, not background tabs.
          if (document.visibilityState !== 'visible') return;
          listeners.forEach(function (f) { try { f(msg); } catch (e) { console.error(e); } });
        });
      }
      return bridge;
    });
  }
  var images = __IMAGES__;
  return Object.freeze({
    id: id,
    data: Object.freeze(__DATA__),
    // Power extensions: a picture from images/ as a data: URL ("" if missing).
    image: function (name) { return images[name] || ''; },
    post: function (msg) { return connect().then(function (b) { b.post(id, 'panel', JSON.stringify(msg === undefined ? null : msg)); }); },
    onMessage: function (f) { if (typeof f === 'function') { listeners.push(f); connect().catch(function () {}); } },
    openPanel: function () { return connect().then(function (b) { b.post(id, 'oynix:open', '{}'); }); },
    // Read text from what's on screen in this tab (CSS pixels relative to
    // the viewport; default: the whole visible area). Needs "permissions":
    // ["ocr"] in oynix.json. Works on scans, canvas and text in images.
    ocr: function (rect) {
      if (!canOcr) return Promise.reject(new Error('Add "permissions": ["ocr"] to oynix.json to use oynix.ocr()'));
      var r = rect || { x: 0, y: 0, width: innerWidth, height: innerHeight };
      var req = Math.random().toString(36).slice(2) + Date.now().toString(36);
      return connect().then(function (b) {
        return new Promise(function (resolve, reject) {
          pending[req] = { resolve: resolve, reject: reject };
          b.post(id, 'oynix:ocr', JSON.stringify({ req: req, url: location.href, x: r.x, y: r.y, width: r.width, height: r.height }));
        });
      });
    }
  });
})();"#;

/// The `oynix` object in an extension's `extra.html` (the mirror image of
/// OYNIX_API: it posts to code.ts and listens for messages from it).
pub const PAGE_API: &str = r#"window.oynix = (function () {
  var id = __ID__, listeners = [], ready = null;
  function connect() {
    if (!ready) ready = new Promise(function (resolve) {
      (function wait() {
        if (typeof qt === 'undefined' || !qt.webChannelTransport || typeof QWebChannel === 'undefined') return setTimeout(wait, 25);
        new QWebChannel(qt.webChannelTransport, function (ch) {
          var b = ch.objects.oynixExtensions;
          b.message.connect(function (ext, target, payload) {
            if (ext !== id || target !== 'panel') return;
            var msg; try { msg = JSON.parse(payload); } catch (e) { return; }
            listeners.forEach(function (f) { try { f(msg); } catch (e) { console.error(e); } });
          });
          resolve(b);
        });
      })();
    });
    return ready;
  }
  connect();
  return Object.freeze({
    id: id,
    data: Object.freeze(__DATA__),
    post: function (msg) { return connect().then(function (b) { b.post(id, 'content', JSON.stringify(msg === undefined ? null : msg)); }); },
    onMessage: function (f) { if (typeof f === 'function') listeners.push(f); }
  });
})();"#;

/// Build the injected script for an OyNIx extension: a Greasemonkey header
/// (Qt WebEngine applies @match / @exclude / @run-at natively), the CSS and
/// the JS.
pub fn oynix_script(e: &Entry) -> Option<String> {
    if e.matches.is_empty() {
        return None;
    }
    let dir = Path::new(&e.path);
    let js = match compile_code(dir) {
        Ok(js) => js,
        Err(err) => {
            eprintln!("oynix: extension {}: {err}", e.name);
            return None;
        }
    };
    let data = read_data(dir).unwrap_or_default();
    let css = std::fs::read_to_string(dir.join("content.css")).unwrap_or_default();
    let mut header = format!("// ==UserScript==\n// @name {}\n", e.name.replace('\n', " "));
    for m in &e.matches {
        let m = if m == "<all_urls>" { "*://*/*" } else { m.as_str() };
        header.push_str(&format!("// @match {m}\n"));
    }
    for x in &e.excludes {
        header.push_str(&format!("// @exclude {x}\n"));
    }
    let run_at = match e.run_at.as_str() {
        "document_start" => "document-start",
        "document_idle" => "document-idle",
        _ => "document-end",
    };
    header.push_str(&format!("// @run-at {run_at}\n// ==/UserScript==\n"));
    let css_js = serde_json::to_string(&css).unwrap_or_else(|_| "\"\"".into());
    let id_js = serde_json::to_string(&e.id).unwrap_or_default();
    let data_js = serde_json::to_string(&data).unwrap_or_else(|_| "[]".into());
    // Messaging with the panel needs the WebChannel client in this world.
    let ocr = e.permissions.iter().any(|p| p == "ocr");
    let channel = if (e.has_panel || e.has_page || ocr) && e.world != "main" { crate::bridge::native::ffi::webchannel_js() } else { String::new() };
    let api = OYNIX_API
        .replace("__ID__", &id_js)
        .replace("__DATA__", &data_js)
        .replace("__IMAGES__", &serde_json::to_string(&image_map(e)).unwrap_or_else(|_| "{}".into()))
        .replace("__MESSAGING__", if channel.is_empty() { "false" } else { "true" })
        .replace("__OCR__", if ocr && !channel.is_empty() { "true" } else { "false" });
    Some(format!(
        "{header}{channel}\n(function () {{\n  var css = {css_js};\n  if (!css.trim()) return;\n  var s = document.createElement('style');\n  s.dataset.oynixExtension = {id_js};\n  s.textContent = css;\n  // At document_start there may be no <html> element yet.\n  var add = function () {{ (document.head || document.documentElement).appendChild(s); }};\n  if (document.documentElement) add();\n  else new MutationObserver(function (_, o) {{ if (document.documentElement) {{ o.disconnect(); add(); }} }}).observe(document, {{ childList: true }});\n}})();\n(function () {{\n{api}\n{js}\n}})();\n"
    ))
}

// ── Install / remove ──────────────────────────────────────────────────

/// Install from a folder, `.crx`, `.xpi` or `.zip`. Re-installing an
/// extension with the same id replaces it (keeps its enabled state).
pub fn install(source: &Path, list: &mut Vec<Entry>) -> Result<Entry, String> {
    install_as(source, list, false)
}

/// Install an extension that ships inside OyNIx (exempt from .poweroynix
/// verification: it came with the browser).
pub fn install_builtin(source: &Path, list: &mut Vec<Entry>) -> Result<Entry, String> {
    install_as(source, list, true)
}

fn install_as(source: &Path, list: &mut Vec<Entry>, builtin: bool) -> Result<Entry, String> {
    let base = root();
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = base.join(format!(".staging-{}-{}-{n}", std::process::id(), now()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let result = install_inner(source, &staging, &base, list, builtin);
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn install_inner(source: &Path, staging: &Path, base: &Path, list: &mut Vec<Entry>, builtin: bool) -> Result<Entry, String> {
    let ext = source.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let from_xpi = ext == "xpi";
    let mut crx_key: Option<Vec<u8>> = None;
    if source.is_dir() {
        copy_dir(source, staging).map_err(|e| e.to_string())?;
    } else if source.file_name().map(|n| n == "oynix.json" || n == "manifest.json").unwrap_or(false) {
        copy_dir(source.parent().unwrap_or(Path::new(".")), staging).map_err(|e| e.to_string())?;
    } else {
        let bytes = std::fs::read(source).map_err(|e| format!("cannot read {}: {e}", source.display()))?;
        let zip = if bytes.starts_with(b"Cr24") { crx_payload(&bytes)? } else { &bytes[..] };
        unzip(zip, staging)?;
        crx_key = crx_public_key(&bytes);
    }
    let dir = content_root(staging);

    let mut e = Entry { enabled: true, installed: now(), ..Default::default() };
    let power = ext == "poweroynix" || super::poweroynix::is_power_dir(&dir);
    if power {
        // Checked before anything below rewrites a file.
        e.verified_by = if builtin { "OyNIx".into() } else { super::poweroynix::verify(&dir)? };
        if !dir.join("oynix.json").exists() {
            return Err("a .poweroynix needs oynix.json, code.ts and content.css, like any OyNIx extension".into());
        }
        if dir.join("manifest.json").exists() {
            prepare_webext(&dir, &mut e, crx_key.as_deref(), from_xpi)?;
        }
        let icon = std::mem::take(&mut e.icon);
        prepare_oynix(&dir, &mut e, true)?;
        prepare_power_extras(&dir, &mut e)?;
        if e.icon.is_empty() {
            e.icon = icon;
        }
        e.power = true;
    } else if dir.join("oynix.json").exists() && !dir.join("manifest.json").exists() {
        prepare_oynix(&dir, &mut e, false)?;
        for f in ["network.ts", "theme-shape.qml", "extra.md"] {
            if dir.join(f).exists() {
                e.warnings.push(format!("{f} is ignored: it's only for verified .poweroynix extensions"));
            }
        }
    } else {
        prepare_webext(&dir, &mut e, crx_key.as_deref(), from_xpi)?;
    }
    finish_install(e, &dir, base, list)
}

/// The Chrome/Firefox half: normalise manifest.json and fill in `e`.
fn prepare_webext(dir: &Path, e: &mut Entry, crx_key: Option<&[u8]>, from_xpi: bool) -> Result<(), String> {
    let dir = dir.to_path_buf();
    {
        let text = std::fs::read_to_string(dir.join("manifest.json")).map_err(|_| "no manifest.json or oynix.json found".to_string())?;
        let mut manifest = parse_json_lenient(&text)?;
        let Value::Object(ref mut m) = manifest else { return Err("manifest.json is not an object".into()) };
        if let (Some(k), false) = (crx_key, m.contains_key("key")) {
            m.insert("key".into(), json!(b64_encode(k)));
        }
        let mv = m.get("manifest_version").and_then(Value::as_u64).unwrap_or(0);
        match mv {
            2 | 3 => {}
            v if v > 3 => {
                e.warnings.push(format!("Manifest V{v} is newer than any browser implements; loading it as Manifest V3"));
                m.insert("manifest_version".into(), json!(3));
            }
            _ => return Err("manifest_version must be 2 or 3".into()),
        }
        let mv = m.get("manifest_version").and_then(Value::as_u64).unwrap_or(3) as u32;
        let firefox = is_firefox(&Value::Object(m.clone()), from_xpi);
        if firefox {
            convert_firefox(&dir, m, &mut e.warnings)?;
        }
        // The background page comes from the manifest as written.
        let original = Value::Object(m.clone());
        if mv == 2 {
            convert_mv2(m, &mut e.warnings);
        }
        expose_own_resources(m, 3.max(mv));
        std::fs::write(dir.join("manifest.json"), serde_json::to_vec_pretty(&manifest).unwrap()).map_err(|e| e.to_string())?;
        e.background_page = write_background_page(&dir, &original)?;
        e.first_run = true;

        let raw_name = manifest.get("name").and_then(Value::as_str).unwrap_or("Extension").to_string();
        e.name = localize(&dir, &manifest, &raw_name);
        e.description = localize(&dir, &manifest, manifest.get("description").and_then(Value::as_str).unwrap_or(""));
        e.version = manifest.get("version").and_then(Value::as_str).unwrap_or("").to_string();
        e.kind = if firefox { "firefox".into() } else { "chrome".into() };
        e.manifest_version = mv;
        e.has_popup = ["action", "browser_action", "page_action"]
            .iter()
            .any(|k| manifest.get(*k).and_then(|a| a.get("default_popup")).and_then(Value::as_str).map(|s| !s.is_empty()).unwrap_or(false));
        e.options_page = manifest
            .get("options_ui")
            .and_then(|o| o.get("page"))
            .or_else(|| manifest.get("options_page"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        e.icon = best_icon(&dir, &manifest);
    }
    Ok(())
}

fn finish_install(mut e: Entry, dir: &Path, base: &Path, list: &mut Vec<Entry>) -> Result<Entry, String> {
    let dir = dir.to_path_buf();
    e.id = slug(&e.name);
    let target = base.join(&e.id);
    if let Some(old) = list.iter().find(|x| x.id == e.id) {
        e.enabled = old.enabled;
        e.builtin = old.builtin;
        e.first_run = false;
    }
    let _ = std::fs::remove_dir_all(&target);
    std::fs::rename(&dir, &target).or_else(|_| copy_dir(&dir, &target)).map_err(|err| format!("cannot install: {err}"))?;
    e.path = target.to_string_lossy().into_owned();
    if !e.icon.is_empty() {
        e.icon = e.icon.replace(&dir.to_string_lossy().into_owned(), &e.path);
    }
    list.retain(|x| x.id != e.id);
    list.push(e.clone());
    save(list);
    Ok(e)
}

pub fn remove(id: &str, list: &mut Vec<Entry>) -> bool {
    let Some(pos) = list.iter().position(|e| e.id == id) else { return false };
    if list[pos].builtin {
        return false; // ships with OyNIx; turn it off instead
    }
    let e = list.remove(pos);
    // Only delete folders we own.
    let p = PathBuf::from(&e.path);
    if p.starts_with(root()) {
        let _ = std::fs::remove_dir_all(p);
    }
    save(list);
    true
}

/// Create a ready-to-edit OyNIx extension skeleton and register it.
const TEMPLATE_TS: &str = r#"// Runs on every page matched by oynix.json "matches".
// OyNIx compiles this TypeScript when the extension is installed or edited.

// What OyNIx provides (types only; erased when compiled).
declare const oynix: {
  readonly id: string;
  /** Records from extra.jsonl, one per line. */
  readonly data: readonly unknown[];
  /** Send a message to graphics.qt / extra.html. */
  post(message: unknown): Promise<void>;
  onMessage(listener: (message: unknown) => void): void;
  /** Open this extension's panel from the toolbar. */
  openPanel(): Promise<void>;
};

const host: string = location.hostname;
console.log(`[${oynix.id}] loaded on ${host}`);
"#;

pub fn create_template(name: &str, list: &mut Vec<Entry>) -> Result<Entry, String> {
    let src = std::env::temp_dir().join(format!("oynix-template-{}", now()));
    std::fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    let meta = json!({
        "name": name,
        "version": "1.0",
        "description": "My OyNIx extension",
        "matches": ["*://*/*"],
        "exclude": [],
        "run_at": "document_end",
        "world": "isolated"
    });
    std::fs::write(src.join("oynix.json"), serde_json::to_vec_pretty(&meta).unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(src.join("code.ts"), TEMPLATE_TS).map_err(|e| e.to_string())?;
    std::fs::write(src.join("content.css"), "/* Styles injected into matched pages. */\n").map_err(|e| e.to_string())?;
    let r = install(&src, list);
    let _ = std::fs::remove_dir_all(&src);
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("oynix-ext-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn with_data_home() {
        std::env::set_var("XDG_DATA_HOME", std::env::temp_dir().join(format!("oynix-ext-data-{}", std::process::id())));
    }

    #[test]
    fn firefox_mv3_is_converted() {
        with_data_home();
        let d = tmp("ff");
        std::fs::write(
            d.join("manifest.json"),
            r#"{ "manifest_version": 3, "name": "Fox Tool", "version": "1.2",
                 "browser_specific_settings": { "gecko": { "id": "fox@example.org" } },
                 "permissions": ["menus", "storage", "https://example.org/*"],
                 "background": { "scripts": ["bg.js"] },
                 "content_scripts": [{ "matches": ["<all_urls>"], "js": ["cs.js"] }],
                 "action": { "default_popup": "popup.html", "browser_style": true } }"#,
        )
        .unwrap();
        std::fs::write(d.join("bg.js"), "browser.runtime.onInstalled.addListener(()=>{})").unwrap();
        std::fs::write(d.join("cs.js"), "").unwrap();
        let mut list = vec![];
        let e = install(&d, &mut list).unwrap();
        assert_eq!(e.kind, "firefox");
        assert!(e.has_popup);
        let m: Value = serde_json::from_slice(&std::fs::read(Path::new(&e.path).join("manifest.json")).unwrap()).unwrap();
        assert!(m.get("browser_specific_settings").is_none());
        assert_eq!(m["background"]["service_worker"], BG_FILE);
        assert_eq!(m["content_scripts"][0]["js"][0], SHIM_FILE);
        assert_eq!(m["permissions"], json!(["contextMenus", "storage"]));
        assert_eq!(m["host_permissions"], json!(["https://example.org/*"]));
        assert!(m["action"].get("browser_style").is_none());
        assert_eq!(m["web_accessible_resources"], json!([{ "resources": ["*"], "extension_ids": ["*"] }]));
        let bg = std::fs::read_to_string(Path::new(&e.path).join(BG_FILE)).unwrap();
        assert!(bg.contains("importScripts(\"oynix-browser-shim.js\", \"bg.js\")"));
        assert!(remove(&e.id, &mut list));
    }

    #[test]
    fn crx3_and_future_manifest() {
        with_data_home();
        let d = tmp("crx");
        let mut zipbuf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zipbuf));
            let o = zip::write::SimpleFileOptions::default();
            w.start_file("manifest.json", o).unwrap();
            w.write_all(br#"{"manifest_version": 4, "name": "__MSG_n__", "version": "2", "default_locale": "en"}"#).unwrap();
            w.start_file("_locales/en/messages.json", o).unwrap();
            w.write_all(br#"{"n": {"message": "Shiny Chrome Ext"}}"#).unwrap();
            w.finish().unwrap();
        }
        let header = b"hdr!";
        let mut crx = b"Cr24".to_vec();
        crx.extend_from_slice(&3u32.to_le_bytes());
        crx.extend_from_slice(&(header.len() as u32).to_le_bytes());
        crx.extend_from_slice(header);
        crx.extend_from_slice(&zipbuf);
        let f = d.join("x.crx");
        std::fs::write(&f, crx).unwrap();
        let mut list = vec![];
        let e = install(&f, &mut list).unwrap();
        assert_eq!(e.kind, "chrome");
        assert_eq!(e.name, "Shiny Chrome Ext");
        assert_eq!(e.manifest_version, 3);
        assert!(e.warnings.iter().any(|w| w.contains("V4")));
        remove(&e.id, &mut list);
    }

    #[test]
    fn oynix_format_requires_all_files() {
        with_data_home();
        let d = tmp("oy");
        std::fs::write(d.join("oynix.json"), r#"{"name": "Dark Docs", "matches": ["*://docs.example.org/*"], "run_at": "document_start"}"#).unwrap();
        let mut list = vec![];
        let err = install(&d, &mut list).unwrap_err();
        assert!(err.contains("code.ts"), "{err}");
        std::fs::write(d.join("code.ts"), "enum Mark { Done = 1 }\nconst t: string = ' ✓' + Mark.Done;\ndocument.title += t;").unwrap();
        let err = install(&d, &mut list).unwrap_err();
        assert!(err.contains("content.css"), "{err}");
        std::fs::write(d.join("content.css"), "body{background:#111}").unwrap();
        std::fs::write(d.join("extra.jsonl"), "{\"word\": \"alpha\"}\n\n{\"word\": \"beta\"}\n").unwrap();
        std::fs::write(d.join("graphics.qt"), "import QtQuick\nItem {}\n").unwrap();
        let e = install(&d, &mut list).unwrap();
        assert_eq!(e.kind, "oynix");
        assert_eq!(e.data_records, 2);
        assert!(e.has_panel && !e.has_page);
        let s = oynix_script(&e).unwrap();
        assert!(s.contains("// @match *://docs.example.org/*"));
        assert!(s.contains("// @run-at document-start"));
        assert!(s.contains("body{background:#111}"));
        assert!(s.contains("Mark[Mark[\"Done\"] = 1]") && !s.contains(": string"));
        assert!(s.contains(r#"[{"word":"alpha"},{"word":"beta"}]"#));
        remove(&e.id, &mut list);
    }

    #[test]
    fn oynix_converts_javascript_and_rejects_bad_input() {
        with_data_home();
        let d = tmp("oyjs");
        std::fs::write(d.join("oynix.json"), r#"{"name": "Plain JS"}"#).unwrap();
        std::fs::write(d.join("content.js"), "document.body.dataset.x = '1';").unwrap();
        std::fs::write(d.join("content.css"), "").unwrap();
        let mut list = vec![];
        let e = install(&d, &mut list).unwrap();
        assert!(e.warnings.iter().any(|w| w.contains("converted to code.ts")), "{:?}", e.warnings);
        let installed = Path::new(&e.path);
        assert!(installed.join("code.ts").exists() && !installed.join("content.js").exists());
        assert!(d.join("content.js").exists(), "the source folder is left alone");
        assert!(oynix_script(&e).unwrap().contains("dataset.x = \"1\""));
        remove(&e.id, &mut list);

        let b = tmp("oybad");
        std::fs::write(b.join("oynix.json"), r#"{"name": "Broken"}"#).unwrap();
        std::fs::write(b.join("code.ts"), "const x: = 1;").unwrap();
        std::fs::write(b.join("content.css"), "").unwrap();
        let err = install(&b, &mut list).unwrap_err();
        assert!(err.starts_with("code.ts:1:"), "{err}");
        std::fs::write(b.join("code.ts"), "export {};").unwrap();
        std::fs::write(b.join("extra.jsonl"), "{\"ok\": 1}\nnot json\n").unwrap();
        let err = install(&b, &mut list).unwrap_err();
        assert!(err.starts_with("extra.jsonl:2:"), "{err}");
    }

    #[test]
    fn widgets_compile_and_render() {
        with_data_home();
        let d = tmp("widget");
        std::fs::write(d.join("oynix.json"), r#"{"name": "Clock Widget", "matches": []}"#).unwrap();
        std::fs::write(d.join("code.ts"), "export {};").unwrap();
        std::fs::write(d.join("content.css"), "").unwrap();
        std::fs::write(d.join("extra.jsonl"), "{\"city\": \"Oslo\"}\n").unwrap();
        std::fs::write(
            d.join("widget.tsx"),
            "export function Label(p: { t: string }) { return <b>{p.t}</b>; }\nexport default function Clock() {\n  const [n, setN] = useState(0);\n  return <div onClick={() => setN(n + 1)}><Label t={oynix.data[0].city} /> {n}</div>;\n}\n",
        )
        .unwrap();
        let mut list = vec![];
        let e = install(&d, &mut list).unwrap();
        assert!(e.has_widget);
        let html = widget_html(&e, ":root{--fg:#000}").unwrap();
        assert!(html.contains("__oynixWidget = function Clock()") && html.contains("function Label(") && !html.contains("export "), "{html}");
        assert!(html.contains("\"city\":\"Oslo\"") && html.contains("render(h(__oynixWidget"));
        // A widget that doesn't compile is refused at install.
        std::fs::write(d.join("widget.tsx"), "export default () => <div>").unwrap();
        assert!(install(&d, &mut list).unwrap_err().starts_with("widget.tsx:"));
        remove(&e.id, &mut list);
    }

    #[test]
    fn power_extensions_need_verification() {
        with_data_home();
        let d = tmp("power");
        std::fs::write(d.join("oynix.json"), r#"{"name": "Power Test", "power": true, "permissions": ["filters"], "matches": []}"#).unwrap();
        std::fs::write(d.join("code.ts"), "export {};").unwrap();
        std::fs::write(d.join("content.css"), "").unwrap();
        std::fs::write(d.join("filters.txt"), "||tracker.example^\n").unwrap();
        std::fs::write(
            d.join("manifest.json"),
            r#"{"manifest_version": 2, "name": "Power Test", "version": "1.0", "permissions": ["cookies", "nativeMessaging"], "background": {"scripts": ["bg.js"]}}"#,
        )
        .unwrap();
        std::fs::write(d.join("bg.js"), "chrome.runtime.onInstalled.addListener(() => {});").unwrap();
        let mut list = vec![];
        // Unsigned: refused, whether it says "power" or arrives as .poweroynix.
        let err = install(&d, &mut list).unwrap_err();
        assert!(err.contains("has to be verified"), "{err}");
        assert!(list.is_empty());

        // The "filters" permission is power-only.
        let plain = tmp("notpower");
        std::fs::write(plain.join("oynix.json"), r#"{"name": "Not Power", "permissions": ["filters"]}"#).unwrap();
        std::fs::write(plain.join("code.ts"), "export {};").unwrap();
        std::fs::write(plain.join("content.css"), "").unwrap();
        let err = install(&plain, &mut list).unwrap_err();
        assert!(err.contains("only for verified .poweroynix"), "{err}");

        // Built-ins are trusted: both halves install as one extension.
        let e = install_builtin(&d, &mut list).unwrap();
        assert!(e.power && e.kind == "oynix" && e.is_webext());
        assert_eq!(e.manifest_version, 2);
        assert_eq!(e.verified_by, "OyNIx");
        assert!(!e.background_page.is_empty());
        assert_eq!(filter_files(&list), vec![Path::new(&e.path).join("filters.txt")]);
        remove(&e.id, &mut list);
    }
}

#[cfg(test)]
mod template_tests {
    #[test]
    fn template_compiles() {
        let js = crate::core::tscompile::transpile(super::TEMPLATE_TS, "code.ts").unwrap();
        assert!(js.contains("location.hostname") && !js.contains("declare"));
    }
}

#[cfg(test)]
mod crx_tests {
    #[test]
    fn crx3_key_matches_crx_id() {
        use sha2::{Digest, Sha256};
        // Build a minimal CRX3: two key proofs, the second is the extension's.
        let store_key = b"store-signing-key".to_vec();
        let own_key = b"extension-own-public-key".to_vec();
        let id = Sha256::digest(&own_key)[..16].to_vec();
        let ld = |field: u64, data: &[u8]| -> Vec<u8> {
            let mut v = vec![];
            let mut tag = field << 3 | 2;
            loop { let b = (tag & 0x7f) as u8; tag >>= 7; if tag == 0 { v.push(b); break } v.push(b | 0x80); }
            let mut len = data.len() as u64;
            loop { let b = (len & 0x7f) as u8; len >>= 7; if len == 0 { v.push(b); break } v.push(b | 0x80); }
            v.extend_from_slice(data);
            v
        };
        let mut header = ld(2, &ld(1, &store_key));
        header.extend(ld(2, &ld(1, &own_key)));
        header.extend(ld(10000, &ld(1, &id)));
        let mut crx = b"Cr24".to_vec();
        crx.extend(3u32.to_le_bytes());
        crx.extend((header.len() as u32).to_le_bytes());
        crx.extend(&header);
        crx.extend(b"PK...");
        assert_eq!(super::crx_public_key(&crx).unwrap(), own_key);
        assert_eq!(crate::core::nativemsg::chrome_id_from_key(&own_key).len(), 32);
    }
}
