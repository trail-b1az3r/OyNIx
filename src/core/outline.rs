//! Tables of contents for tree tabs.
//!
//! PDFs: the document's own outline (bookmarks), read with lopdf. Chromium's
//! PDF viewer keeps the file to itself, so OyNIx reads it again (local
//! files directly; web PDFs with one download), and caches the result in
//! `~/.cache/oynix/<profile>/outlines/` so that happens once per document:
//! local files until they change, web PDFs for a day.
//!
//! Markdown outlines come from the page itself (see `scripts::OUTLINE`).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub level: usize,
    pub title: String,
    /// 1-based page number.
    pub page: usize,
}

const MAX_BYTES: u64 = 100 << 20;
const MAX_ITEMS: usize = 500;
const WEB_TTL: Duration = Duration::from_secs(24 * 3600);

fn cache_file(url: &str) -> PathBuf {
    let h: String = Sha256::digest(url.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect();
    super::paths::cache_dir().join("outlines").join(format!("{h}.json"))
}

fn local_path(url: &str) -> Option<PathBuf> {
    url::Url::parse(url).ok().filter(|u| u.scheme() == "file").and_then(|u| u.to_file_path().ok())
}

fn cached(url: &str) -> Option<Vec<Item>> {
    let f = cache_file(url);
    let made = std::fs::metadata(&f).and_then(|m| m.modified()).ok()?;
    let fresh = match local_path(url) {
        Some(p) => std::fs::metadata(p).and_then(|m| m.modified()).is_ok_and(|src| src <= made),
        None => SystemTime::now().duration_since(made).unwrap_or_default() < WEB_TTL,
    };
    if !fresh {
        return None;
    }
    serde_json::from_slice(&std::fs::read(f).ok()?).ok()
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    if let Some(p) = local_path(url) {
        let len = std::fs::metadata(&p).map_err(|e| e.to_string())?.len();
        if len > MAX_BYTES {
            return Err("the PDF is too large to read its outline".into());
        }
        return std::fs::read(p).map_err(|e| e.to_string());
    }
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("unsupported address".into());
    }
    let mut resp = super::updater::agent().get(url).call().map_err(|e| e.to_string())?;
    resp.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(|e| e.to_string())
}

/// Read the outline from PDF bytes. Never panics on malformed files.
pub fn parse(bytes: &[u8]) -> Vec<Item> {
    let r = std::panic::catch_unwind(|| {
        let doc = lopdf::Document::load_mem(bytes).ok()?;
        let toc = doc.get_toc().ok()?;
        Some(
            toc.toc
                .into_iter()
                .take(MAX_ITEMS)
                .map(|t| Item { level: t.level.max(1), title: t.title.trim().to_string(), page: t.page.max(1) })
                .filter(|i| !i.title.is_empty())
                .collect::<Vec<_>>(),
        )
    });
    r.ok().flatten().unwrap_or_default()
}

/// The outline of the PDF at `url` (file:// or http(s)://), from the cache
/// when it's still valid. Blocks: call it off the UI thread.
pub fn pdf(url: &str) -> Result<Vec<Item>, String> {
    let url = url.split('#').next().unwrap_or(url);
    if let Some(items) = cached(url) {
        return Ok(items);
    }
    let bytes = fetch(url)?;
    if !bytes.starts_with(b"%PDF") {
        return Err("not a PDF".into());
    }
    let items = parse(&bytes);
    let f = cache_file(url);
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let _ = super::paths::write_atomic(&f, &serde_json::to_vec(&items).unwrap_or_default());
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-page PDF with a nested outline, built with lopdf.
    fn sample() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object, Stream};
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids = vec![];
        for _ in 0..2 {
            let content = doc.add_object(Stream::new(dictionary! {}, b"".to_vec()));
            kids.push(Object::Reference(doc.add_object(dictionary! {
                "Type" => "Page", "Parent" => pages_id, "Contents" => content,
                "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
            })));
        }
        let (p1, p2) = (kids[0].clone(), kids[1].clone());
        doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }));
        let outlines_id = doc.new_object_id();
        let intro_id = doc.new_object_id();
        let part_id = doc.new_object_id();
        let sub_id = doc.new_object_id();
        let s = |t: &str| Object::string_literal(t);
        doc.objects.insert(sub_id, Object::Dictionary(dictionary! {
            "Title" => s("Details"), "Parent" => part_id, "Dest" => vec![p2.clone(), "Fit".into()],
        }));
        doc.objects.insert(part_id, Object::Dictionary(dictionary! {
            "Title" => s("Part Two"), "Parent" => outlines_id, "Prev" => intro_id, "Dest" => vec![p2, "Fit".into()],
            "First" => sub_id, "Last" => sub_id, "Count" => 1,
        }));
        doc.objects.insert(intro_id, Object::Dictionary(dictionary! {
            "Title" => s("Introduction"), "Parent" => outlines_id, "Next" => part_id, "Dest" => vec![p1, "Fit".into()],
        }));
        doc.objects.insert(outlines_id, Object::Dictionary(dictionary! { "Type" => "Outlines", "First" => intro_id, "Last" => part_id, "Count" => 3 }));
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id, "Outlines" => outlines_id });
        doc.trailer.set("Root", catalog);
        let mut out = vec![];
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn reads_nested_outline_and_caches_it() {
        std::env::set_var("XDG_CACHE_HOME", std::env::temp_dir().join(format!("oynix-outline-{}", std::process::id())));
        let items = parse(&sample());
        let got: Vec<(usize, &str, usize)> = items.iter().map(|i| (i.level, i.title.as_str(), i.page)).collect();
        assert_eq!(got, [(1, "Introduction", 1), (1, "Part Two", 2), (2, "Details", 2)]);
        assert!(parse(b"%PDF-1.4 garbage").is_empty());

        let f = std::env::temp_dir().join(format!("oynix-outline-{}.pdf", std::process::id()));
        std::fs::write(&f, sample()).unwrap();
        let url = url::Url::from_file_path(&f).unwrap().to_string();
        assert_eq!(pdf(&url).unwrap(), items);
        assert!(cache_file(&url).exists());
        // Served from the cache while the file is unchanged.
        assert_eq!(cached(&url).unwrap(), items);
    }
}
