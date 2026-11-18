//! Themes: the accent's Material 3 scheme, adjusted for the mode (light,
//! dark, OLED, midnight) and style (normal, classic, neumorphic,
//! graphite) by a Z#++ program, `assets/themes.zsp`. Results are cached
//! per (accent, mode, style).

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde_json::Value;
use stonedgem_zsharp::{Program, Value as Zs};

const SOURCE: &str = include_str!("../../assets/themes.zsp");

pub const MODES: &[&str] = &["light", "dark", "oled", "midnight"];
pub const STYLES: &[&str] = &["normal", "classic", "neumorphic", "graphite"];

/// "#rrggbb" or "#aarrggbb" (Qt's order) → Z#++'s 0xRRGGBBAA.
fn parse(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    let v = u32::from_str_radix(h, 16).ok()?;
    match h.len() {
        6 => Some(v << 8 | 0xff),
        8 => Some((v << 8) | (v >> 24)),
        _ => None,
    }
}

/// Z#++ colour → a colour string QML reads ("#rrggbb", or "#aarrggbb").
fn qml(c: u32) -> String {
    let (r, g, b, a) = (c >> 24, (c >> 16) & 0xff, (c >> 8) & 0xff, c & 0xff);
    if a == 0xff {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{a:02x}{r:02x}{g:02x}{b:02x}")
    }
}

/// Run the theme program: the roles it changes, as QML colour strings.
fn overrides(base: &Value, mode: &str, style: &str) -> Result<BTreeMap<String, String>, String> {
    let mut prog = Program::from_source(SOURCE)?;
    let mut roles = BTreeMap::new();
    if let Value::Object(m) = base {
        for (k, v) in m {
            if let Some(c) = v.as_str().and_then(parse) {
                roles.insert(k.clone(), Zs::Color(c));
            }
        }
    }
    let r = prog.interp.call_global("theme", vec![Zs::str(mode), Zs::str(style), Zs::map(roles)])?;
    let Some(Zs::Map(m)) = r else { return Err("theme() didn't return a map".into()) };
    let out = m
        .borrow()
        .iter()
        .filter_map(|(k, v)| match v {
            Zs::Color(c) => Some((k.clone(), qml(*c))),
            _ => None,
        })
        .collect();
    Ok(out)
}

/// The colour roles for `seed` in `mode` and `style` (role → colour).
pub fn scheme(seed: &str, mode: &str, style: &str) -> Value {
    static CACHE: Mutex<Option<HashMap<(String, String, String), Value>>> = Mutex::new(None);
    let mode = if MODES.contains(&mode) { mode } else { "dark" };
    let style = if STYLES.contains(&style) { style } else { "normal" };
    let key = (seed.to_string(), mode.to_string(), style.to_string());
    if let Some(v) = CACHE.lock().unwrap().as_ref().and_then(|c| c.get(&key)) {
        return v.clone();
    }
    let mut s = super::material::scheme(seed, mode != "light");
    match overrides(&s, mode, style) {
        Ok(o) => {
            if let Value::Object(m) = &mut s {
                for (k, v) in o {
                    m.insert(k, Value::String(v));
                }
            }
        }
        Err(e) => eprintln!("oynix: assets/themes.zsp: {e}"),
    }
    CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, s.clone());
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(hex: &str) -> f64 {
        let c = parse(hex).unwrap();
        let ch = |v: u32| {
            let s = v as f64 / 255.0;
            if s <= 0.03928 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(c >> 24) + 0.7152 * ch((c >> 16) & 0xff) + 0.0722 * ch((c >> 8) & 0xff)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_mode_and_style_is_readable() {
        for seed in ["#3b82f6", "#e11d48", "#16a34a"] {
            for mode in MODES {
                for style in STYLES {
                    let s = scheme(seed, mode, style);
                    let g = |k: &str| s[k].as_str().unwrap_or_else(|| panic!("{mode}/{style}: no {k}")).to_string();
                    for bg in ["surface", "surfaceContainer", "surfaceContainerHigh"] {
                        let c = contrast(&g("onSurface"), &g(bg));
                        assert!(c >= 4.5, "{seed} {mode}/{style}: onSurface on {bg} is {c:.2}:1");
                        let c = contrast(&g("onSurfaceVariant"), &g(bg));
                        assert!(c >= 4.5, "{seed} {mode}/{style}: onSurfaceVariant on {bg} is {c:.2}:1");
                    }
                    let c = contrast(&g("onPrimary"), &g("primary"));
                    assert!(c >= 4.5, "{seed} {mode}/{style}: onPrimary on primary is {c:.2}:1");
                }
            }
        }
    }

    #[test]
    fn modes_look_as_named() {
        assert_eq!(scheme("#3b82f6", "oled", "normal")["surface"], "#000000");
        assert_eq!(scheme("#3b82f6", "midnight", "classic")["surface"], "#0b1022");
        assert_eq!(scheme("#3b82f6", "dark", "graphite")["surface"], "#2b2d31");
        let neu = scheme("#3b82f6", "dark", "neumorphic");
        assert!(neu["neuShadow"].as_str().unwrap().starts_with("#80"), "{}", neu["neuShadow"]);
        // OLED neumorphism: no dark shadow (nothing is darker than black).
        assert_eq!(scheme("#3b82f6", "oled", "graphite")["neuShadow"], "#00000000");
    }
}
