//! Material 3 colour scheme generation.
//!
//! Builds tonal palettes from a seed colour and maps them to the Material 3
//! colour roles for light and dark themes. Tones are CIE L* (as in HCT); hue
//! and chroma are taken in CIE LCh(ab) and reduced as needed to stay inside
//! sRGB, which closely approximates the HCT-based reference implementation.

use serde_json::{json, Map, Value};

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

const WHITE: (f64, f64, f64) = (0.95047, 1.0, 1.08883);

fn f_lab(t: f64) -> f64 {
    if t > 216.0 / 24389.0 { t.cbrt() } else { (24389.0 / 27.0 * t + 16.0) / 116.0 }
}

fn f_lab_inv(t: f64) -> f64 {
    let t3 = t * t * t;
    if t3 > 216.0 / 24389.0 { t3 } else { (116.0 * t - 16.0) / (24389.0 / 27.0) }
}

/// sRGB (0–255) → (L*, C, h°)
pub fn to_lch(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let (r, g, b) = (srgb_to_linear(r as f64 / 255.0), srgb_to_linear(g as f64 / 255.0), srgb_to_linear(b as f64 / 255.0));
    let x = 0.4124 * r + 0.3576 * g + 0.1805 * b;
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let z = 0.0193 * r + 0.1192 * g + 0.9505 * b;
    let (fx, fy, fz) = (f_lab(x / WHITE.0), f_lab(y / WHITE.1), f_lab(z / WHITE.2));
    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let bb = 200.0 * (fy - fz);
    (l, (a * a + bb * bb).sqrt(), bb.atan2(a).to_degrees().rem_euclid(360.0))
}

/// (L*, C, h°) → linear-light RGB (may be out of gamut).
fn lch_to_linear(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let (x, y, z) = (f_lab_inv(fx) * WHITE.0, f_lab_inv(fy) * WHITE.1, f_lab_inv(fz) * WHITE.2);
    (
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    )
}

fn in_gamut(c: (f64, f64, f64)) -> bool {
    let e = 1e-4;
    (-e..=1.0 + e).contains(&c.0) && (-e..=1.0 + e).contains(&c.1) && (-e..=1.0 + e).contains(&c.2)
}

/// Colour at tone `t` (L*) with the requested hue/chroma, reducing chroma
/// until it fits in sRGB. Returns `#rrggbb`.
pub fn tone(h: f64, c: f64, t: f64) -> String {
    let t = t.clamp(0.0, 100.0);
    let (mut lo, mut hi) = (0.0, c);
    let mut rgb = lch_to_linear(t, 0.0, h);
    if in_gamut(lch_to_linear(t, c, h)) {
        rgb = lch_to_linear(t, c, h);
    } else {
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            let v = lch_to_linear(t, mid, h);
            if in_gamut(v) {
                lo = mid;
                rgb = v;
            } else {
                hi = mid;
            }
        }
    }
    let to8 = |v: f64| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to8(rgb.0), to8(rgb.1), to8(rgb.2))
}

fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Material 3 colour roles for `seed`. `contrast` 0 = standard.
pub fn scheme(seed: &str, dark: bool) -> Value {
    let (r, g, b) = parse_hex(seed).unwrap_or((0x2f, 0x7c, 0xf6));
    let (_, c0, h) = to_lch(r, g, b);
    // Expressive palette: vivid primary, tertiary rotated for contrast.
    let pc = c0.max(48.0);
    let p = |t: f64| tone(h, pc, t);
    let s = |t: f64| tone(h, (c0 / 3.0).clamp(12.0, 24.0), t);
    let t3 = |t: f64| tone((h + 60.0).rem_euclid(360.0), (c0 / 1.5).clamp(24.0, 40.0), t);
    let n = |t: f64| tone(h, 4.0, t);
    let nv = |t: f64| tone(h, 9.0, t);
    let e = |t: f64| tone(25.0, 80.0, t);

    let mut m = Map::new();
    let mut put = |k: &str, v: String| {
        m.insert(k.to_string(), Value::String(v));
    };
    if !dark {
        put("primary", p(40.0)); put("onPrimary", p(100.0)); put("primaryContainer", p(90.0)); put("onPrimaryContainer", p(10.0));
        put("secondary", s(40.0)); put("onSecondary", s(100.0)); put("secondaryContainer", s(90.0)); put("onSecondaryContainer", s(10.0));
        put("tertiary", t3(40.0)); put("onTertiary", t3(100.0)); put("tertiaryContainer", t3(90.0)); put("onTertiaryContainer", t3(10.0));
        put("error", e(40.0)); put("onError", e(100.0)); put("errorContainer", e(90.0)); put("onErrorContainer", e(10.0));
        put("surface", n(98.0)); put("surfaceDim", n(87.0)); put("surfaceBright", n(98.0));
        put("surfaceContainerLowest", n(100.0)); put("surfaceContainerLow", n(96.0)); put("surfaceContainer", n(94.0));
        put("surfaceContainerHigh", n(92.0)); put("surfaceContainerHighest", n(90.0));
        put("onSurface", n(10.0)); put("onSurfaceVariant", nv(30.0)); put("outline", nv(50.0)); put("outlineVariant", nv(80.0));
        put("inverseSurface", n(20.0)); put("inverseOnSurface", n(95.0)); put("inversePrimary", p(80.0));
    } else {
        put("primary", p(80.0)); put("onPrimary", p(20.0)); put("primaryContainer", p(30.0)); put("onPrimaryContainer", p(90.0));
        put("secondary", s(80.0)); put("onSecondary", s(20.0)); put("secondaryContainer", s(30.0)); put("onSecondaryContainer", s(90.0));
        put("tertiary", t3(80.0)); put("onTertiary", t3(20.0)); put("tertiaryContainer", t3(30.0)); put("onTertiaryContainer", t3(90.0));
        put("error", e(80.0)); put("onError", e(20.0)); put("errorContainer", e(30.0)); put("onErrorContainer", e(90.0));
        put("surface", n(6.0)); put("surfaceDim", n(6.0)); put("surfaceBright", n(24.0));
        put("surfaceContainerLowest", n(4.0)); put("surfaceContainerLow", n(10.0)); put("surfaceContainer", n(12.0));
        put("surfaceContainerHigh", n(17.0)); put("surfaceContainerHighest", n(22.0));
        put("onSurface", n(90.0)); put("onSurfaceVariant", nv(80.0)); put("outline", nv(60.0)); put("outlineVariant", nv(30.0));
        put("inverseSurface", n(90.0)); put("inverseOnSurface", n(20.0)); put("inversePrimary", p(40.0));
    }
    put("shadow", "#000000".into());
    put("scrim", "#000000".into());
    json!(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(hex: &str) -> f64 {
        let (r, g, b) = parse_hex(hex).unwrap();
        to_lch(r, g, b).0
    }

    #[test]
    fn tones_hit_their_lightness() {
        for t in [10.0, 40.0, 80.0, 90.0, 98.0] {
            let c = tone(260.0, 60.0, t);
            assert!((lum(&c) - t).abs() < 1.5, "{c} vs {t}");
        }
        assert_eq!(tone(0.0, 0.0, 100.0), "#ffffff");
        assert_eq!(tone(0.0, 0.0, 0.0), "#000000");
    }

    #[test]
    fn roles_have_contrast() {
        for dark in [false, true] {
            let s = scheme("#2f7cf6", dark);
            let d = (lum(s["primary"].as_str().unwrap()) - lum(s["onPrimary"].as_str().unwrap())).abs();
            assert!(d > 45.0, "primary/onPrimary contrast {d}");
            let d2 = (lum(s["surface"].as_str().unwrap()) - lum(s["onSurface"].as_str().unwrap())).abs();
            assert!(d2 > 80.0);
        }
    }
}
