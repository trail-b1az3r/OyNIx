//! History, bookmarks (with folders and Favourites), reading list and
//! recently-closed tabs. Everything is kept in memory and flushed to JSON
//! files in the profile's data directory.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::paths;

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

const HISTORY_LIMIT: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visits: u32,
    pub last_visit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Bookmark {
    pub id: u64,
    pub url: String,
    pub title: String,
    /// "" = root, "favorites" = Favourites bar/start page, otherwise folder name.
    pub folder: String,
    pub added: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReadingItem {
    pub url: String,
    pub title: String,
    pub added: i64,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ClosedTab {
    pub url: String,
    pub title: String,
    pub closed: i64,
}

#[derive(Default)]
pub struct Library {
    pub history: Vec<HistoryEntry>,
    pub bookmarks: Vec<Bookmark>,
    pub reading: Vec<ReadingItem>,
    pub closed: Vec<ClosedTab>,
    next_id: u64,
    history_dirty: bool,
}

fn load<T: for<'de> Deserialize<'de> + Default>(name: &str) -> T {
    std::fs::read(paths::data_dir().join(name))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn store<T: Serialize>(name: &str, v: &T) {
    if let Ok(b) = serde_json::to_vec(v) {
        let _ = paths::write_atomic(&paths::data_dir().join(name), &b);
    }
}

fn recordable(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://")
}

impl Library {
    pub fn load() -> Self {
        let mut l = Library {
            history: load("history.json"),
            bookmarks: load("bookmarks.json"),
            reading: load("reading-list.json"),
            closed: Vec::new(),
            next_id: 1,
            history_dirty: false,
        };
        if l.bookmarks.is_empty() && !paths::data_dir().join("bookmarks.json").exists() {
            for (t, u) in [
                ("Wikipedia", "https://www.wikipedia.org/"),
                ("GitHub", "https://github.com/"),
                ("Rust", "https://www.rust-lang.org/"),
                ("Qt", "https://www.qt.io/"),
                ("OpenStreetMap", "https://www.openstreetmap.org/"),
                ("Hacker News", "https://news.ycombinator.com/"),
            ] {
                l.add_bookmark(u, t, "favorites");
            }
        }
        l.next_id = l.bookmarks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        l
    }

    // ── History ────────────────────────────────────────────────────
    pub fn record_visit(&mut self, url: &str, title: &str) {
        if !recordable(url) {
            return;
        }
        if let Some(pos) = self.history.iter().position(|h| h.url == url) {
            let mut e = self.history.remove(pos);
            e.visits += 1;
            e.last_visit = now();
            if !title.is_empty() {
                e.title = title.to_string();
            }
            self.history.insert(0, e);
        } else {
            self.history.insert(0, HistoryEntry { url: url.into(), title: title.into(), visits: 1, last_visit: now() });
            self.history.truncate(HISTORY_LIMIT);
        }
        self.history_dirty = true;
    }

    pub fn update_title(&mut self, url: &str, title: &str) {
        if title.is_empty() {
            return;
        }
        if let Some(e) = self.history.iter_mut().take(50).find(|h| h.url == url) {
            if e.title != title {
                e.title = title.to_string();
                self.history_dirty = true;
            }
        }
    }

    pub fn search_history(&self, q: &str, limit: usize) -> Vec<&HistoryEntry> {
        let q = q.to_lowercase();
        self.history
            .iter()
            .filter(|h| q.is_empty() || h.url.to_lowercase().contains(&q) || h.title.to_lowercase().contains(&q))
            .take(limit)
            .collect()
    }

    pub fn remove_history(&mut self, url: &str) {
        self.history.retain(|h| h.url != url);
        self.history_dirty = true;
    }

    /// Clear history newer than `since` (unix seconds). `0` clears all.
    pub fn clear_history(&mut self, since: i64) {
        self.history.retain(|h| since > 0 && h.last_visit < since);
        self.history_dirty = true;
        self.flush();
    }

    pub fn most_visited(&self, limit: usize) -> Vec<&HistoryEntry> {
        let mut v: Vec<&HistoryEntry> = self.history.iter().take(2000).collect();
        v.sort_by(|a, b| b.visits.cmp(&a.visits).then(b.last_visit.cmp(&a.last_visit)));
        let mut seen = std::collections::HashSet::new();
        v.into_iter()
            .filter(|h| seen.insert(super::omnibox::host_of(&h.url)))
            .take(limit)
            .collect()
    }

    /// Completion candidates for the address field: every visited host
    /// (weighted by total visits) plus full pages (weighted by their own visits).
    pub fn completion_candidates(&self) -> Vec<(String, f64)> {
        let mut hosts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut out = Vec::new();
        for (i, h) in self.history.iter().take(3000).enumerate() {
            let recency = 1.0 / (1.0 + i as f64 / 200.0);
            let p = super::omnibox::pretty(&h.url);
            if p.is_empty() {
                continue;
            }
            let bare = p.strip_prefix("http://").unwrap_or(&p);
            let host_part = bare.split(['/', '?', '#']).next().unwrap_or("").to_string();
            *hosts.entry(host_part).or_default() += h.visits as f64 * recency + 0.5;
            out.push((p, h.visits as f64 * recency * 0.5));
        }
        for b in &self.bookmarks {
            let p = super::omnibox::pretty(&b.url);
            let bare = p.strip_prefix("http://").unwrap_or(&p);
            let host_part = bare.split(['/', '?', '#']).next().unwrap_or("").to_string();
            *hosts.entry(host_part).or_default() += 3.0;
        }
        out.extend(hosts);
        out
    }

    pub fn flush(&mut self) {
        if self.history_dirty {
            store("history.json", &self.history);
            self.history_dirty = false;
        }
    }

    // ── Bookmarks ──────────────────────────────────────────────────
    pub fn is_bookmarked(&self, url: &str) -> bool {
        self.bookmarks.iter().any(|b| b.url == url)
    }

    pub fn add_bookmark(&mut self, url: &str, title: &str, folder: &str) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        self.bookmarks.push(Bookmark {
            id,
            url: url.into(),
            title: if title.is_empty() { url.into() } else { title.into() },
            folder: folder.into(),
            added: now(),
        });
        store("bookmarks.json", &self.bookmarks);
        id
    }

    pub fn remove_bookmark_url(&mut self, url: &str) {
        self.bookmarks.retain(|b| b.url != url);
        store("bookmarks.json", &self.bookmarks);
    }

    pub fn remove_bookmark(&mut self, id: u64) {
        self.bookmarks.retain(|b| b.id != id);
        store("bookmarks.json", &self.bookmarks);
    }

    pub fn edit_bookmark(&mut self, id: u64, title: &str, url: &str, folder: &str) {
        if let Some(b) = self.bookmarks.iter_mut().find(|b| b.id == id) {
            b.title = title.into();
            b.url = url.into();
            b.folder = folder.into();
        }
        store("bookmarks.json", &self.bookmarks);
    }

    pub fn move_bookmark(&mut self, id: u64, to_index: usize) {
        if let Some(pos) = self.bookmarks.iter().position(|b| b.id == id) {
            let b = self.bookmarks.remove(pos);
            let i = to_index.min(self.bookmarks.len());
            self.bookmarks.insert(i, b);
            store("bookmarks.json", &self.bookmarks);
        }
    }

    pub fn folders(&self) -> Vec<String> {
        let mut f: Vec<String> = self
            .bookmarks
            .iter()
            .map(|b| b.folder.clone())
            .filter(|f| !f.is_empty() && f != "favorites")
            .collect();
        f.sort();
        f.dedup();
        f
    }

    /// Import bookmarks from a Netscape-format HTML export (Firefox, Chrome,
    /// Safari and most browsers can produce one).
    pub fn import_html(&mut self, html: &str) -> usize {
        let mut n = 0;
        let mut folder = String::new();
        for line in html.lines() {
            let l = line.trim();
            if let Some(i) = l.find("<H3") {
                if let Some(s) = l[i..].find('>') {
                    let rest = &l[i + s + 1..];
                    folder = rest.split("</H3>").next().unwrap_or("").to_string();
                }
            } else if l.starts_with("</DL>") {
                folder.clear();
            } else if let Some(i) = l.find("HREF=\"") {
                let rest = &l[i + 6..];
                let url = rest.split('"').next().unwrap_or("");
                let title = rest
                    .find('>')
                    .map(|gt| &rest[gt + 1..])
                    .map(|t| t.split("</A>").next().unwrap_or(t))
                    .filter(|t| !t.is_empty())
                    .unwrap_or(url);
                if recordable(url) && !self.is_bookmarked(url) {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.bookmarks.push(Bookmark {
                        id,
                        url: url.into(),
                        title: html_unescape(title),
                        folder: html_unescape(&folder),
                        added: now(),
                    });
                    n += 1;
                }
            }
        }
        store("bookmarks.json", &self.bookmarks);
        n
    }

    pub fn export_html(&self) -> String {
        let mut s = String::from("<!DOCTYPE NETSCAPE-Bookmark-file-1>\n<META HTTP-EQUIV=\"Content-Type\" CONTENT=\"text/html; charset=UTF-8\">\n<TITLE>Bookmarks</TITLE>\n<H1>Bookmarks</H1>\n<DL><p>\n");
        let mut folders: Vec<String> = vec![String::new(), "favorites".into()];
        folders.extend(self.folders());
        for f in folders {
            let items: Vec<&Bookmark> = self.bookmarks.iter().filter(|b| b.folder == f).collect();
            if items.is_empty() {
                continue;
            }
            if !f.is_empty() {
                s.push_str(&format!("  <DT><H3>{}</H3>\n  <DL><p>\n", html_escape(if f == "favorites" { "Favorites" } else { &f })));
            }
            for b in items {
                s.push_str(&format!("    <DT><A HREF=\"{}\" ADD_DATE=\"{}\">{}</A>\n", html_escape(&b.url), b.added, html_escape(&b.title)));
            }
            if !f.is_empty() {
                s.push_str("  </DL><p>\n");
            }
        }
        s.push_str("</DL><p>\n");
        s
    }

    // ── Reading list ───────────────────────────────────────────────
    pub fn add_reading(&mut self, url: &str, title: &str) {
        if !self.reading.iter().any(|r| r.url == url) {
            self.reading.insert(0, ReadingItem { url: url.into(), title: title.into(), added: now(), read: false });
            store("reading-list.json", &self.reading);
        }
    }

    pub fn set_read(&mut self, url: &str, read: bool) {
        if let Some(r) = self.reading.iter_mut().find(|r| r.url == url) {
            r.read = read;
        }
        store("reading-list.json", &self.reading);
    }

    pub fn remove_reading(&mut self, url: &str) {
        self.reading.retain(|r| r.url != url);
        store("reading-list.json", &self.reading);
    }

    // ── Recently closed ────────────────────────────────────────────
    pub fn push_closed(&mut self, url: &str, title: &str) {
        if url.is_empty() || url.starts_with("oynix://start") {
            return;
        }
        self.closed.insert(0, ClosedTab { url: url.into(), title: title.into(), closed: now() });
        self.closed.truncate(25);
    }

    pub fn pop_closed(&mut self) -> Option<ClosedTab> {
        if self.closed.is_empty() {
            None
        } else {
            Some(self.closed.remove(0))
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn html_unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_dedupes_and_orders() {
        let mut l = Library::default();
        l.record_visit("https://a.com/", "A");
        l.record_visit("https://b.com/", "B");
        l.record_visit("https://a.com/", "A2");
        assert_eq!(l.history.len(), 2);
        assert_eq!(l.history[0].url, "https://a.com/");
        assert_eq!(l.history[0].visits, 2);
        assert_eq!(l.history[0].title, "A2");
        l.record_visit("oynix://start", "x");
        assert_eq!(l.history.len(), 2);
    }

    #[test]
    fn bookmark_html_roundtrip() {
        let mut a = Library::default();
        a.bookmarks.push(Bookmark { id: 1, url: "https://x.com/".into(), title: "X & Y".into(), folder: "Dev".into(), added: 1 });
        let html = a.export_html();
        let mut b = Library::default();
        // import writes to disk – point XDG at a temp dir
        std::env::set_var("XDG_DATA_HOME", std::env::temp_dir().join("oynix-test"));
        assert_eq!(b.import_html(&html), 1);
        assert_eq!(b.bookmarks[0].title, "X & Y");
        assert_eq!(b.bookmarks[0].folder, "Dev");
    }
}
