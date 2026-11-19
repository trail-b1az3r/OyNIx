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
pub fn resolved(file: &str, dark: bool) -> Value {
    if file.is_empty() {
        return json!({});
    }
    super::paths::data_dir(); // OyNIx's folders, not StonedGem's
    match stonedgem_hypertheme::load(file) {
        Ok(h) => {
            let r = h.resolve(dark);
            json!({ "file": file, "name": h.manifest.name, "tokens": r.tokens, "panels": r.panels, "warnings": r.warnings })
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
            json!({ "file": file, "name": name, "description": description, "builtin": builtin.contains(&file) })
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

    #[test]
    fn builtin_neubrutalism_resolves() {
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
