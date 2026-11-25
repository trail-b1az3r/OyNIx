//! Hyperthemes in OyNIx (5.4.2): whole-UI themes made of tokens, basic
//! Rust, Z#++, Zsharpend, TOML, CSS, TSX, QML and a shader (StonedGem's
//! `stonedgem-hypertheme`). `Theme.qml` reads the chosen one's tokens and
//! panels; Theme Studio, the node editor that makes them, runs as a page
//! whose Zsharpend script OyNIx runs itself (`bridge/studio.rs`).
//!
//! Installed themes live in `~/.local/share/oynix/<profile>/hyperthemes/`;
//! Neubrutalism is built in.

use serde_json::{json, Value};

/// The theme `file` resolved for light or dark, for QML: {} when there's
/// none or it doesn't load.
///
/// Besides tokens and panels: `power`, `global` (global.zxx's settings),
/// and file:// URLs of what the window loads: `background` (background.zsg
/// drawn, or the `background_image` picture global.zxx names), `overlay`
/// (global.zsg drawn), `globalQml`, `effectQml`, `formatQml` (power themes'
/// QML parts, unpacked), and `stylesJs` (styles.tsx as a function body).
pub fn resolved(file: &str, dark: bool) -> Value {
    if file.is_empty() {
        return json!({});
    }
    super::paths::data_dir(); // OyNIx's folders, not StonedGem's
    let h = match stonedgem_hypertheme::load(file) {
        Ok(h) => h,
        Err(e) => return json!({ "error": e }),
    };
    let r = h.resolve(dark);
    let mut out = json!({
        "file": file, "name": h.manifest.name, "power": h.is_power(),
        "tokens": r.tokens, "panels": r.panels, "warnings": r.warnings, "global": r.global,
    });
    let mut warnings: Vec<String> = vec![];
    match unpacked(file, &h) {
        Ok(dir) => {
            let url = |p: &std::path::Path| format!("file://{}", p.to_string_lossy());
            // The background: global.zxx's picture, else background.zsg drawn.
            let picture = r.global.get("background_image").and_then(Value::as_str).map(|p| dir.join(p.trim_start_matches("./"))).filter(|p| p.starts_with(&dir) && p.is_file());
            if let Some(p) = picture {
                out["background"] = json!(url(&p));
            } else if let Some(p) = drawn(&dir, "background", || stonedgem_hypertheme::background_png(&h, 960, 600), &mut warnings) {
                out["background"] = json!(url(&p));
            }
            if let Some(p) = drawn(&dir, "global", || stonedgem_hypertheme::global_png(&h, 960, 600), &mut warnings) {
                out["overlay"] = json!(url(&p));
            }
            if h.is_power() {
                for (key, f) in [("globalQml", "global.qml"), ("effectQml", "effect.qml"), ("formatQml", "format-panel.qt")] {
                    if dir.join(f).is_file() {
                        out[key] = json!(url(&dir.join(f)));
                    }
                }
                match h.styles_js() {
                    Some(Ok(js)) => out["stylesJs"] = json!(js),
                    Some(Err(e)) => warnings.push(format!("styles.tsx: {e}")),
                    None => {}
                }
            }
        }
        Err(e) => warnings.push(format!("couldn't unpack the theme: {e}")),
    }
    if let Some(w) = out["warnings"].as_array_mut() {
        w.extend(warnings.into_iter().map(Value::String));
    }
    out
}

/// Whether power theme `file` asks for a see-through window (global.zxx:
/// `window: { transparent: true }`), in light or dark mode.
pub fn wants_transparent_window(file: &str) -> bool {
    if file.is_empty() {
        return false;
    }
    super::paths::data_dir();
    let Ok(h) = stonedgem_hypertheme::load(file) else { return false };
    h.is_power() && [false, true].iter().any(|&dark| h.resolve(dark).global.get("window").and_then(|w| w.get("transparent")).and_then(Value::as_bool) == Some(true))
}

/// The theme unpacked in the cache (`hypertheme/<file>/`), once per version
/// of the installed file: QML parts and pictures are loaded from there.
fn unpacked(file: &str, h: &stonedgem_hypertheme::Hypertheme) -> Result<std::path::PathBuf, String> {
    use std::sync::Mutex;
    static DONE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
    let dir = super::paths::cache_dir().join("hypertheme").join(file);
    // What identifies this version: the installed file's time and size (built-ins: the build).
    let stamp = std::fs::metadata(stonedgem_hypertheme::dir().join(file))
        .map(|m| format!("{:?}-{}", m.modified().ok(), m.len()))
        .unwrap_or_else(|_| format!("builtin-{}", crate::VERSION));
    let mut done = DONE.lock().unwrap_or_else(|e| e.into_inner());
    let marker = dir.join(".stamp");
    let fresh = done.iter().any(|(f, s)| f == file && s == &stamp) || std::fs::read_to_string(&marker).is_ok_and(|s| s == stamp);
    if !fresh {
        h.materialize(&dir)?;
        let _ = std::fs::write(&marker, &stamp);
    }
    done.retain(|(f, _)| f != file);
    done.push((file.to_string(), stamp));
    Ok(dir)
}

/// A shader part drawn to `rendered/<name>.png` in the unpacked theme (once).
fn drawn(dir: &std::path::Path, name: &str, draw: impl FnOnce() -> Option<Result<Vec<u8>, String>>, warnings: &mut Vec<String>) -> Option<std::path::PathBuf> {
    let p = dir.join("rendered").join(format!("{name}.png"));
    if p.is_file() {
        return Some(p);
    }
    match draw()? {
        Ok(png) => {
            let _ = std::fs::create_dir_all(p.parent()?);
            std::fs::write(&p, png).ok()?;
            Some(p)
        }
        Err(e) => {
            warnings.push(format!("{name}.zsg: {e}"));
            None
        }
    }
}

/// Install a .hypertheme or .phypertheme file or folder. A power theme
/// needs `allow_power` (the user said yes): otherwise the answer is
/// `{ needsConfirm: true, name }`.
pub fn install(path: &str, allow_power: bool) -> Value {
    super::paths::data_dir();
    let p = path.trim().trim_start_matches("file://");
    let source = std::path::Path::new(p);
    match stonedgem_hypertheme::install_with(source, allow_power) {
        Ok(file) => json!({ "file": file }),
        Err(e) if e.starts_with(stonedgem_hypertheme::NEEDS_POWER_CONFIRM) => {
            let name = stonedgem_hypertheme::Hypertheme::open(source).map(|h| h.manifest.name).unwrap_or_default();
            json!({ "needsConfirm": true, "name": name, "error": e.trim_start_matches(stonedgem_hypertheme::NEEDS_POWER_CONFIRM).trim_start_matches(": ") })
        }
        Err(e) => json!({ "error": e }),
    }
}

/// Built-in and installed hyperthemes.
pub fn list() -> Value {
    super::paths::data_dir();
    let builtin: Vec<String> = stonedgem_hypertheme::builtins().into_iter().map(|(f, _)| f).collect();
    let v: Vec<Value> = stonedgem_hypertheme::available()
        .into_iter()
        .map(|(file, name)| {
            let description = stonedgem_hypertheme::load(&file).map(|h| h.manifest.description).unwrap_or_default();
            json!({ "file": file, "name": name, "description": description, "builtin": builtin.contains(&file), "power": file.ends_with(".phypertheme") })
        })
        .collect();
    Value::Array(v)
}

/// The `chrome.runtime` / `chrome.storage` the Studio page expects, over
/// Qt WebChannel to `StudioBridge` (registered as `oynixStudio`).
pub fn studio_shim(webchannel_js: &str) -> String {
    format!(
        r#"{webchannel_js}
(function () {{
  var ready = new Promise(function (resolve) {{
    function go() {{ new QWebChannel(qt.webChannelTransport, function (ch) {{ resolve(ch.objects.oynixStudio); }}); }}
    if (window.qt && qt.webChannelTransport) go(); else document.addEventListener("DOMContentLoaded", go);
  }});
  function call(method, args) {{
    return ready.then(function (o) {{ return new Promise(function (res) {{ o[method].apply(o, args.concat([res])); }}); }});
  }}
  window.chrome = {{
    runtime: {{
      sendMessage: function (msg) {{
        return call("message", [JSON.stringify(msg)]).then(function (t) {{
          var r = JSON.parse(t);
          if (r.error) throw new Error(r.error);
          return r.ok;
        }});
      }}
    }},
    storage: {{ local: {{
      get: function (key) {{ return call("storageGet", [String(key)]).then(function (t) {{ var o = {{}}; if (t) o[key] = t; return o; }}); }},
      set: function (items) {{ var k = Object.keys(items)[0]; return call("storageSet", [k, String(items[k])]); }}
    }} }}
  }};
}})();
"#
    )
}

/// The page's file name; `WebTab.qml` gives tabs showing it the Studio channel.
pub const STUDIO_PAGE: &str = "theme-studio.html";

/// The Studio page, opening `file` (an installed theme) if given.
pub fn studio_page_for(shim: &str, file: &str) -> Option<String> {
    let url = studio_page(shim)?;
    Some(if file.is_empty() { url } else { format!("{url}#open={}", url::form_urlencoded::byte_serialize(file.as_bytes()).collect::<String>()) })
}

/// Write the Studio page (with `shim` before its own script) to the cache;
/// its file:// URL.
pub fn studio_page(shim: &str) -> Option<String> {
    let html = stonedgem_hypertheme::studio_files().into_iter().find(|(f, _)| *f == "extra.html")?.1;
    let html = html.replacen("<script>", &format!("<script>\n{}\n</script>\n<script>", shim.replace("</script", "<\\/script")), 1);
    let dir = super::paths::cache_dir().join("theme-studio");
    std::fs::create_dir_all(&dir).ok()?;
    let file = dir.join(STUDIO_PAGE);
    super::paths::write_atomic(&file, html.as_bytes()).ok()?;
    Some(format!("file://{}", file.to_string_lossy()))
}

/// Where the Studio page keeps its graph between visits.
pub fn studio_store() -> std::path::PathBuf {
    super::paths::data_dir().join("theme-studio.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests point XDG_* at their own folders; one at a time.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn aurora_glass_power_theme() {
        let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("oynix-aurora-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", dir.join("data"));
        std::env::set_var("XDG_CACHE_HOME", dir.join("cache"));
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/themes/aurora-glass");
        // A power theme asks first.
        let r = install(src, false);
        assert_eq!(r["needsConfirm"], true, "{r}");
        assert_eq!(r["name"], "Aurora Glass");
        let r = install(src, true);
        assert_eq!(r["file"], "aurora-glass.phypertheme", "{r}");
        assert!(list().as_array().unwrap().iter().any(|t| t["file"] == "aurora-glass.phypertheme" && t["power"] == true));

        let dark = resolved("aurora-glass.phypertheme", true);
        assert_eq!(dark["power"], true);
        assert!(dark["warnings"].as_array().unwrap().is_empty(), "{}", dark["warnings"]);
        assert_eq!(dark["tokens"]["tertiary"], "#9b8cff", "theme.zxx imported modules/aurora.zxx");
        assert_eq!(dark["global"]["glass"]["blur"], 18);
        for key in ["background", "overlay", "globalQml", "effectQml", "formatQml"] {
            let url = dark[key].as_str().unwrap_or_else(|| panic!("{key} missing: {dark}"));
            assert!(std::path::Path::new(url.trim_start_matches("file://")).is_file(), "{key}: {url}");
        }
        assert!(dark["stylesJs"].as_str().unwrap().contains("exports.button"));
        let light = resolved("aurora-glass.phypertheme", false);
        assert_eq!(light["tokens"]["background"], "#eef2fb");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn builtin_neubrutalism_resolves() {
        let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("oynix-hyper-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", dir.join("data"));
        std::env::set_var("XDG_CACHE_HOME", dir.join("cache"));
        let l = list();
        assert!(l.as_array().unwrap().iter().any(|t| t["file"] == "neubrutalism.hypertheme" && t["builtin"] == true));
        let r = resolved("neubrutalism.hypertheme", false);
        assert_eq!(r["tokens"]["background"], "#fff4dc");
        assert_eq!(r["tokens"]["border_width"], 3.0);
        assert_eq!(r["panels"]["toolbar"]["fill"], "#ffde59");
        assert_eq!(resolved("neubrutalism.hypertheme", true)["tokens"]["background"], "#18181b");
        assert_eq!(resolved("", false), json!({}));
        assert!(resolved("missing.hypertheme", false)["error"].is_string());
        let url = studio_page(&studio_shim("/* qwebchannel */")).unwrap();
        let page = std::fs::read_to_string(url.trim_start_matches("file://")).unwrap();
        assert!(page.contains("ch.objects.oynixStudio") && page.contains("\"Neubrutalism\""));
        assert!(page.find("oynixStudio").unwrap() < page.find("const TEMPLATE").unwrap(), "the bridge comes first");
        let _ = std::fs::remove_dir_all(dir);
    }
}
