//! Site icons, scraped and kept small (5.4.2).
//!
//! Qt WebEngine only hands out icons for pages open right now, so
//! bookmarks, history and tabs that haven't loaded yet would show letter
//! tiles. When a page finishes loading (or shows an icon), OyNIx:
//!
//! 1. scrapes the page's `<link rel="icon">`, `apple-touch-icon`,
//!    `shortcut icon` and `mask-icon` tags, picks the best (largest raster
//!    icon, 64 px or more preferred), then tries the icon Qt reported and
//!    `/favicon.ico`;
//! 2. downloads it next to the cache as `<site>.orig`, decodes it (PNG,
//!    ICO, JPEG, GIF, WebP, BMP) and scales it to **64×64** PNG
//!    (`<site>.png`, centred on transparency if it isn't square);
//! 3. deletes the downloaded original.
//!
//! It's dynamic: an icon is fetched again after a day, or at once when the
//! site starts showing a different icon (`<site>.src` remembers which one
//! it came from). Settings › Privacy › Clear Data Cache removes them all
//! ([`clear`]). Files live in `~/.cache/oynix/<profile>/favicons/`; private
//! windows never add to it.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use image::{imageops::FilterType, DynamicImage, GenericImageView, RgbaImage};

/// Icons are stored at this size.
pub const SIZE: u32 = 64;
const MAX_PAGE: u64 = 768 * 1024;
const MAX_ICON: u64 = 1024 * 1024;
const REFRESH: Duration = Duration::from_secs(24 * 3600);

fn dir() -> PathBuf {
    super::paths::cache_dir().join("favicons")
}

/// Cache key for a page URL: its host without "www.", made file-name safe.
fn key(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    let host = u.host_str()?.trim_start_matches("www.").to_ascii_lowercase();
    let safe: String = host.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')).collect();
    (!safe.is_empty() && !safe.starts_with('.')).then_some(safe)
}

/// "en.wikipedia.org" -> ["en.wikipedia.org", "wikipedia.org"].
fn and_parents(k: &str) -> Vec<String> {
    let parts: Vec<&str> = k.split('.').collect();
    (0..parts.len().saturating_sub(1).max(1)).map(|i| parts[i..].join(".")).collect()
}

fn png_of(k: &str) -> PathBuf {
    dir().join(format!("{k}.png"))
}

/// The cached 64×64 icon for the site of `url` (or a parent domain), if any.
pub fn cached(url: &str) -> Option<PathBuf> {
    and_parents(&key(url)?).into_iter().map(|k| png_of(&k)).find(|p| p.is_file())
}

/// Remove every saved icon (and anything left over from older versions).
/// Returns how many icons there were.
pub fn clear() -> usize {
    let n = std::fs::read_dir(dir()).map(|rd| rd.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "png")).count()).unwrap_or(0);
    let _ = std::fs::remove_dir_all(dir());
    n
}

// ── Scraping ─────────────────────────────────────────────────────────

/// An icon a page links to.
#[derive(Debug, Clone, PartialEq)]
struct Link {
    href: String,
    /// The largest side it declares (`sizes="180x180"`); 0 if unknown,
    /// u32::MAX for `sizes="any"`.
    size: u32,
    rel: String,
    kind: String,
}

/// The value of attribute `name` in the inside of a tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let at = from + i;
        from = at + name.len();
        // A whole attribute name: preceded by space, followed by `=`.
        if at > 0 && !lower.as_bytes()[at - 1].is_ascii_whitespace() {
            continue;
        }
        let rest = tag[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else { continue };
        let rest = rest.trim_start();
        let v = match rest.chars().next() {
            Some(q @ ('"' | '\'')) => rest[1..].split(q).next().unwrap_or(""),
            _ => rest.split(|c: char| c.is_ascii_whitespace() || c == '>').next().unwrap_or(""),
        };
        return Some(v.replace("&amp;", "&"));
    }
    None
}

/// Icon links in `html`, in page order.
fn links(html: &str) -> Vec<Link> {
    let lower = html.to_ascii_lowercase();
    let mut out = vec![];
    let mut from = 0;
    while let Some(i) = lower[from..].find("<link") {
        let start = from + i + 5;
        let Some(end) = lower[start..].find('>') else { break };
        let tag = &html[start..start + end];
        from = start + end;
        let rel = attr(tag, "rel").unwrap_or_default().to_ascii_lowercase();
        let is_icon = rel.split_whitespace().any(|r| matches!(r, "icon" | "apple-touch-icon" | "apple-touch-icon-precomposed" | "mask-icon"));
        let Some(href) = attr(tag, "href").filter(|h| !h.trim().is_empty()) else { continue };
        if !is_icon {
            continue;
        }
        let sizes = attr(tag, "sizes").unwrap_or_default().to_ascii_lowercase();
        let size = if sizes.contains("any") {
            u32::MAX
        } else {
            sizes.split_whitespace().filter_map(|s| s.split('x').next()?.parse::<u32>().ok()).max().unwrap_or(0)
        };
        out.push(Link { href: href.trim().to_string(), size, rel, kind: attr(tag, "type").unwrap_or_default().to_ascii_lowercase() });
        if out.len() >= 32 {
            break;
        }
    }
    out
}

/// Candidate icon URLs for a page, best first: raster icons that are 64 px
/// or larger (smallest of those, so less to download), then smaller ones
/// (largest first), apple-touch-icons, the icon Qt reported, then
/// /favicon.ico. SVG-only icons go last (they can't be scaled here).
fn candidates(page: &url::Url, html: &str, reported: Option<&str>) -> Vec<String> {
    let mut ls = links(html);
    let svg = |l: &Link| l.kind.contains("svg") || l.href.to_ascii_lowercase().split('?').next().unwrap_or("").ends_with(".svg") || l.rel.contains("mask-icon");
    let size_of = |l: &Link| if l.size == 0 && l.rel.contains("apple-touch-icon") { 180 } else { l.size };
    ls.sort_by_key(|l| {
        let s = size_of(l);
        let band = if svg(l) { 3 } else if s >= SIZE && s != u32::MAX { 0 } else if s == 0 { 2 } else { 1 };
        // Within a band: ≥64 smallest first; smaller ones largest first.
        let within = if band == 0 { s as i64 } else { -(s.min(1 << 20) as i64) };
        (band, within)
    });
    let mut out: Vec<String> = vec![];
    let mut push = |u: Option<url::Url>| {
        if let Some(u) = u.filter(|u| matches!(u.scheme(), "http" | "https" | "data")) {
            let s = u.to_string();
            if !out.contains(&s) {
                out.push(s);
            }
        }
    };
    let (good, rest): (Vec<&Link>, Vec<&Link>) = ls.iter().partition(|l| !svg(l));
    for l in &good {
        push(page.join(&l.href).ok());
    }
    if let Some(r) = reported {
        push(url::Url::parse(r).ok());
    }
    push(page.join("/favicon.ico").ok());
    for l in rest {
        push(page.join(&l.href).ok());
    }
    out
}

// ── Decoding and scaling ─────────────────────────────────────────────

/// Decode an icon file and scale it to `SIZE`×`SIZE` PNG bytes. ICO files
/// give their largest image. Not square: centred on transparency.
pub fn to_png64(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("not an image this can read: {e}"))?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    let scale = SIZE as f32 / w.max(h) as f32;
    let (nw, nh) = (((w as f32 * scale).round() as u32).clamp(1, SIZE), ((h as f32 * scale).round() as u32).clamp(1, SIZE));
    let filter = if w.max(h) > SIZE { FilterType::Lanczos3 } else { FilterType::CatmullRom };
    let scaled = img.resize_exact(nw, nh, filter).to_rgba8();
    let mut canvas = RgbaImage::new(SIZE, SIZE);
    image::imageops::overlay(&mut canvas, &scaled, ((SIZE - nw) / 2) as i64, ((SIZE - nh) / 2) as i64);
    let mut out = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(canvas).write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

fn data_url(u: &str) -> Option<Vec<u8>> {
    let rest = u.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    if meta.ends_with(";base64") {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()
    } else {
        Some(data.as_bytes().to_vec())
    }
}

fn get(agent: &ureq::Agent, url: &str, limit: u64) -> Option<Vec<u8>> {
    if url.starts_with("data:") {
        return data_url(url).filter(|b| b.len() as u64 <= limit);
    }
    let mut resp = agent.get(url).header("Accept", "image/avif,image/webp,image/png,image/*;q=0.8,*/*;q=0.5").call().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.body_mut().with_config().limit(limit).read_to_vec().ok().filter(|b| !b.is_empty())
}

/// Download the first candidate that decodes; write `<k>.png` (64×64) and
/// `<k>.src`, deleting the downloaded original. The icon URL used.
fn fetch_into(agent: &ureq::Agent, cands: &[String], k: &str, d: &Path) -> Option<String> {
    let orig = d.join(format!("{k}.orig"));
    for c in cands.iter().take(8) {
        let Some(bytes) = get(agent, c, MAX_ICON) else { continue };
        // Keep the original only while it's being converted.
        if super::paths::write_atomic(&orig, &bytes).is_err() {
            return None;
        }
        let png = std::fs::read(&orig).ok().and_then(|b| to_png64(&b).ok());
        let _ = std::fs::remove_file(&orig);
        if let Some(png) = png {
            if super::paths::write_atomic(&d.join(format!("{k}.png")), &png).is_ok() {
                let _ = super::paths::write_atomic(&d.join(format!("{k}.src")), c.as_bytes());
                return Some(c.clone());
            }
        }
    }
    None
}

fn age(p: &Path) -> Option<Duration> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().map(|t| SystemTime::now().duration_since(t).unwrap_or_default())
}

/// Whether the icon for `k` should be fetched (again): missing, older than
/// a day, or the page now shows a different icon than the one saved.
fn stale(k: &str, reported: Option<&str>) -> bool {
    let d = dir();
    let png = d.join(format!("{k}.png"));
    match age(&png) {
        None => true,
        Some(a) if a >= REFRESH => true,
        Some(_) => {
            let src = std::fs::read_to_string(d.join(format!("{k}.src"))).unwrap_or_default();
            // Qt reports `image://favicon/<url>`; a new URL means a new icon.
            reported.is_some_and(|r| !r.is_empty() && !src.is_empty() && r != src && !src.starts_with("data:"))
        }
    }
}

/// Remove pre-5.4.2 icons (full-size originals saved without an extension).
fn migrate(d: &Path) {
    if let Ok(rd) = std::fs::read_dir(d) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            // Old icons are named after the site ("github.com"), so keep
            // only this version's files.
            let ours = p.extension().is_some_and(|x| x == "png" || x == "src");
            if p.is_file() && !ours {
                let _ = std::fs::remove_file(p);
            }
        }
    }
}

/// Keep the site's icon for a page that loaded (`icon` as Qt reports it,
/// usually `image://favicon/https://…/favicon.ico`, or "" before it has
/// one). Runs in the background; does nothing while the saved one is fresh.
pub fn remember(page_url: &str, icon: &str) {
    let Some(k) = key(page_url) else { return };
    let Ok(page) = url::Url::parse(page_url) else { return };
    let reported = icon.strip_prefix("image://favicon/").unwrap_or(icon).to_string();
    let reported = (reported.starts_with("https://") || reported.starts_with("http://")).then_some(reported);
    if !stale(&k, reported.as_deref()) {
        return;
    }
    // One fetch per site at a time.
    static BUSY: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    {
        let mut b = BUSY.lock().unwrap_or_else(|e| e.into_inner());
        if b.contains(&k) {
            return;
        }
        b.push(k.clone());
    }
    let d = dir();
    std::thread::Builder::new()
        .name("oynix-favicon".into())
        .spawn(move || {
            let _ = std::fs::create_dir_all(&d);
            migrate(&d);
            let agent = super::updater::agent();
            let html = get(&agent, page.as_str(), MAX_PAGE).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            // Only the <head> matters.
            let head_end = html.to_ascii_lowercase().find("</head").unwrap_or(html.len());
            let cands = candidates(&page, &html[..head_end], reported.as_deref());
            if let Some(_used) = fetch_into(&agent, &cands, &k, &d) {
                // Also stand in for the parent domain (a bookmark to
                // wikipedia.org gets en.wikipedia.org's icon) until it has its own.
                for parent in and_parents(&k).into_iter().skip(1) {
                    let p = d.join(format!("{parent}.png"));
                    if !p.exists() {
                        let _ = std::fs::copy(d.join(format!("{k}.png")), p);
                    }
                }
            }
            BUSY.lock().unwrap_or_else(|e| e.into_inner()).retain(|x| x != &k);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys() {
        assert_eq!(key("https://www.GitHub.com/x?y").as_deref(), Some("github.com"));
        assert_eq!(key("oynix://start"), None);
        assert_eq!(key("file:///tmp/a"), None);
        assert_eq!(and_parents("en.wikipedia.org"), ["en.wikipedia.org", "wikipedia.org"]);
        assert_eq!(and_parents("localhost"), ["localhost"]);
    }

    #[test]
    fn scrapes_and_ranks_icon_links() {
        let html = r##"<html><head>
            <link rel="stylesheet" href="/a.css">
            <link rel="icon" href="/favicon-16.png" sizes="16x16" type="image/png">
            <LINK REL='shortcut icon' HREF='/legacy.ico'>
            <link rel="apple-touch-icon" href="/touch.png">
            <link rel="icon" href="/favicon-96.png" sizes="96x96">
            <link rel="icon" href="/favicon-192.png" sizes="192x192">
            <link rel="icon" href="/icon.svg" type="image/svg+xml" sizes="any">
            <link rel="mask-icon" href="/mask.svg" color="#000">
            <link rel=icon href=/bare.png>
        </head><body><link rel="icon" href="/body.png"></body></html>"##;
        let page = url::Url::parse("https://www.example.com/news/today").unwrap();
        let c = candidates(&page, &html[..html.find("</head").unwrap()], Some("https://cdn.example.com/qt.ico"));
        assert_eq!(
            c,
            [
                "https://www.example.com/favicon-96.png",   // ≥ 64, smallest first
                "https://www.example.com/touch.png",        // apple-touch-icon (180)
                "https://www.example.com/favicon-192.png",
                "https://www.example.com/favicon-16.png",   // smaller, largest first
                "https://www.example.com/legacy.ico",       // unknown size
                "https://www.example.com/bare.png",
                "https://cdn.example.com/qt.ico",           // what Qt reported
                "https://www.example.com/favicon.ico",
                "https://www.example.com/icon.svg",         // SVG last
                "https://www.example.com/mask.svg",
            ]
        );
        assert_eq!(attr(r#" data-href="x" href="/y.png""#, "href").as_deref(), Some("/y.png"));
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, _| if x < w / 2 { image::Rgba([255, 0, 0, 255]) } else { image::Rgba([0, 0, 255, 255]) });
        let mut out = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn scales_every_icon_to_64() {
        for (w, h) in [(16, 16), (180, 180), (512, 256), (32, 48)] {
            let out = image::load_from_memory(&to_png64(&png(w, h)).unwrap()).unwrap();
            assert_eq!(out.dimensions(), (64, 64), "{w}x{h}");
        }
        // Wide icons are centred on transparency.
        let wide = image::load_from_memory(&to_png64(&png(512, 256)).unwrap()).unwrap().to_rgba8();
        assert_eq!(wide.get_pixel(32, 2)[3], 0);
        assert_eq!(wide.get_pixel(8, 32)[0], 255);
        // An ICO with a 32 px PNG inside.
        let inner = png(32, 32);
        let mut ico = vec![0, 0, 1, 0, 1, 0, 32, 32, 0, 0, 1, 0, 32, 0];
        ico.extend((inner.len() as u32).to_le_bytes());
        ico.extend(22u32.to_le_bytes());
        ico.extend(&inner);
        assert_eq!(image::load_from_memory(&to_png64(&ico).unwrap()).unwrap().dimensions(), (64, 64));
        assert!(to_png64(b"<html>not found</html>").is_err());
    }

    #[test]
    fn fetch_keeps_only_the_64px_copy() {
        let d = std::env::temp_dir().join(format!("oynix-favicons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // A leftover from 5.4.1 (full size, no extension) goes.
        std::fs::write(d.join("old.example"), b"x").unwrap();
        std::fs::write(d.join("example.org.orig"), b"x").unwrap();
        migrate(&d);
        assert!(!d.join("old.example").exists() && !d.join("example.org.orig").exists());
        use base64::Engine;
        let u = format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png(180, 180)));
        let agent = super::super::updater::agent();
        let used = fetch_into(&agent, &["data:text/plain,not-an-icon".into(), u.clone()], "example.com", &d);
        assert_eq!(used.as_deref(), Some(u.as_str()));
        let names: Vec<String> = std::fs::read_dir(&d).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        assert!(names.contains(&"example.com.png".into()) && names.contains(&"example.com.src".into()), "{names:?}");
        assert!(!names.iter().any(|n| n.ends_with(".orig")), "the original is removed: {names:?}");
        let saved = image::load_from_memory(&std::fs::read(d.join("example.com.png")).unwrap()).unwrap();
        assert_eq!(saved.dimensions(), (64, 64));
        let _ = std::fs::remove_dir_all(&d);
    }
}
