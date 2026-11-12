//! Address-bar logic: turning what the user typed into a URL, search
//! engines, `!bang` shortcuts, tracking-parameter stripping and display
//! formatting of URLs.

use url::Url;

pub struct SearchEngine {
    pub id: &'static str,
    pub name: &'static str,
    pub search: &'static str,
    /// Suggestion endpoint returning OpenSearch JSON (`["q", ["s1", ...]]`).
    pub suggest: &'static str,
}

pub const ENGINES: &[SearchEngine] = &[
    SearchEngine { id: "duckduckgo", name: "DuckDuckGo", search: "https://duckduckgo.com/?q={}", suggest: "https://duckduckgo.com/ac/?type=list&q={}" },
    SearchEngine { id: "kagi", name: "Kagi", search: "https://kagi.com/search?q={}", suggest: "https://kagi.com/api/autosuggest?q={}" },
    SearchEngine { id: "google", name: "Google", search: "https://www.google.com/search?q={}", suggest: "https://suggestqueries.google.com/complete/search?client=firefox&q={}" },
    SearchEngine { id: "bing", name: "Bing", search: "https://www.bing.com/search?q={}", suggest: "https://api.bing.com/osjson.aspx?query={}" },
    SearchEngine { id: "brave", name: "Brave Search", search: "https://search.brave.com/search?q={}", suggest: "https://search.brave.com/api/suggest?q={}" },
    SearchEngine { id: "startpage", name: "Startpage", search: "https://www.startpage.com/do/search?q={}", suggest: "" },
    SearchEngine { id: "ecosia", name: "Ecosia", search: "https://www.ecosia.org/search?q={}", suggest: "https://ac.ecosia.org/autocomplete?type=list&q={}" },
    SearchEngine { id: "qwant", name: "Qwant", search: "https://www.qwant.com/?q={}", suggest: "" },
];

/// Built-in bang shortcuts (`!w rust` → Wikipedia search for "rust").
pub const BANGS: &[(&str, &str, &str)] = &[
    ("g", "Google", "https://www.google.com/search?q={}"),
    ("ddg", "DuckDuckGo", "https://duckduckgo.com/?q={}"),
    ("k", "Kagi", "https://kagi.com/search?q={}"),
    ("b", "Bing", "https://www.bing.com/search?q={}"),
    ("brave", "Brave Search", "https://search.brave.com/search?q={}"),
    ("w", "Wikipedia", "https://en.wikipedia.org/w/index.php?search={}"),
    ("yt", "YouTube", "https://www.youtube.com/results?search_query={}"),
    ("gh", "GitHub", "https://github.com/search?q={}"),
    ("so", "Stack Overflow", "https://stackoverflow.com/search?q={}"),
    ("r", "Reddit", "https://www.reddit.com/search/?q={}"),
    ("a", "Amazon", "https://www.amazon.com/s?k={}"),
    ("m", "Maps", "https://www.openstreetmap.org/search?query={}"),
    ("osm", "OpenStreetMap", "https://www.openstreetmap.org/search?query={}"),
    ("i", "Images", "https://duckduckgo.com/?ia=images&iax=images&q={}"),
    ("n", "News", "https://duckduckgo.com/?ia=news&iar=news&q={}"),
    ("rs", "docs.rs", "https://docs.rs/releases/search?query={}"),
    ("crates", "crates.io", "https://crates.io/search?q={}"),
    ("mdn", "MDN", "https://developer.mozilla.org/en-US/search?q={}"),
    ("aw", "ArchWiki", "https://wiki.archlinux.org/index.php?search={}"),
    ("hn", "Hacker News", "https://hn.algolia.com/?q={}"),
    ("imdb", "IMDb", "https://www.imdb.com/find?q={}"),
    ("wa", "Wolfram|Alpha", "https://www.wolframalpha.com/input?i={}"),
    ("tr", "Translate", "https://translate.google.com/?sl=auto&tl=en&text={}"),
    ("pypi", "PyPI", "https://pypi.org/search/?q={}"),
    ("npm", "npm", "https://www.npmjs.com/search?q={}"),
];

/// Query parameters that exist purely to track users across sites.
const TRACKING_PARAMS: &[&str] = &[
    "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", "utm_id",
    "utm_name", "utm_reader", "utm_social", "utm_social-type", "fbclid", "gclid", "gclsrc",
    "dclid", "gbraid", "wbraid", "msclkid", "mc_cid", "mc_eid", "yclid", "_openstat",
    "igshid", "twclid", "ttclid", "li_fat_id", "mkt_tok", "oly_anon_id", "oly_enc_id",
    "_hsenc", "_hsmi", "vero_id", "rb_clickid", "s_cid", "ml_subscriber", "ml_subscriber_hash",
    "si", "ref_src", "ref_url",
];

const INTERNAL_SCHEME: &str = "oynix";

pub fn engine(id: &str) -> &'static SearchEngine {
    ENGINES.iter().find(|e| e.id == id).unwrap_or(&ENGINES[0])
}

fn encode(q: &str) -> String {
    url::form_urlencoded::byte_serialize(q.as_bytes()).collect()
}

pub fn search_url(engine_id: &str, query: &str) -> String {
    engine(engine_id).search.replace("{}", &encode(query))
}

pub fn suggest_url(engine_id: &str, query: &str) -> String {
    let e = engine(engine_id);
    if e.suggest.is_empty() {
        return String::new();
    }
    e.suggest.replace("{}", &encode(query))
}

/// Expand a `!bang query` / `query !bang` into its target URL.
pub fn expand_bang(input: &str) -> Option<String> {
    let words: Vec<&str> = input.split_whitespace().collect();
    let pos = words.iter().position(|w| w.len() > 1 && w.starts_with('!'))?;
    let key = words[pos][1..].to_lowercase();
    let (_, _, tpl) = BANGS.iter().find(|(k, _, _)| *k == key)?;
    let rest: Vec<&str> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != pos)
        .map(|(_, w)| *w)
        .collect();
    let q = rest.join(" ");
    if q.is_empty() {
        // `!gh` alone → site root
        let u = Url::parse(&tpl.replace("{}", "")).ok()?;
        return Some(format!("{}://{}/", u.scheme(), u.host_str()?));
    }
    Some(tpl.replace("{}", &encode(&q)))
}

fn looks_like_host(s: &str) -> bool {
    if s.contains(char::is_whitespace) || s.is_empty() {
        return false;
    }
    let host = s.split(['/', '?', '#']).next().unwrap_or("");
    let host_no_port = host.rsplit_once(':').map(|(h, p)| if p.chars().all(|c| c.is_ascii_digit()) { h } else { host }).unwrap_or(host);
    if host_no_port.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if host_no_port.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    if host_no_port.starts_with('[') && host_no_port.ends_with(']') {
        return true;
    }
    let labels: Vec<&str> = host_no_port.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().unwrap();
    tld.len() >= 2
        && tld.chars().all(|c| c.is_ascii_alphabetic())
        && labels.iter().all(|l| !l.is_empty() && l.chars().all(|c| c.is_alphanumeric() || c == '-'))
}

/// Resolve raw address-bar text into a navigable URL.
pub fn resolve(input: &str, engine_id: &str, bangs: bool, https_upgrade: bool) -> String {
    let text = input.trim();
    if text.is_empty() {
        return "oynix://start".into();
    }
    if bangs {
        if let Some(u) = expand_bang(text) {
            return u;
        }
    }
    // Explicit scheme.
    if let Ok(u) = Url::parse(text) {
        match u.scheme() {
            "http" | "https" | "file" | "about" | "data" | "view-source" | "ftp" | INTERNAL_SCHEME => {
                return u.to_string();
            }
            "chrome" | "qrc" => return u.to_string(),
            _ => {}
        }
    }
    // Local absolute path.
    if text.starts_with('/') || text.starts_with("~/") {
        let p = if let Some(rest) = text.strip_prefix("~/") {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(rest)
        } else {
            std::path::PathBuf::from(text)
        };
        if p.exists() {
            if let Ok(u) = Url::from_file_path(&p) {
                return u.to_string();
            }
        }
    }
    if looks_like_host(text) {
        let local = text.starts_with("localhost")
            || text.split(['/', ':']).next().map(|h| h.parse::<std::net::Ipv4Addr>().is_ok()).unwrap_or(false);
        let scheme = if https_upgrade && !local { "https" } else { "http" };
        if let Ok(u) = Url::parse(&format!("{scheme}://{text}")) {
            return u.to_string();
        }
    }
    search_url(engine_id, text)
}

/// Remove known tracking parameters. Returns the input unchanged if nothing
/// was stripped (so callers can cheaply detect "no change").
pub fn strip_tracking(raw: &str) -> String {
    let Ok(mut u) = Url::parse(raw) else { return raw.to_string() };
    if u.query().is_none() {
        return raw.to_string();
    }
    let pairs: Vec<(String, String)> = u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let kept: Vec<&(String, String)> = pairs
        .iter()
        .filter(|(k, _)| !TRACKING_PARAMS.contains(&k.to_ascii_lowercase().as_str()))
        .collect();
    if kept.len() == pairs.len() {
        return raw.to_string();
    }
    if kept.is_empty() {
        u.set_query(None);
    } else {
        u.query_pairs_mut().clear().extend_pairs(kept.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    }
    u.to_string()
}

pub fn host_of(raw: &str) -> String {
    Url::parse(raw).ok().and_then(|u| u.host_str().map(str::to_string)).unwrap_or_default()
}

/// Short, human friendly form shown in the address bar when it is not
/// focused: `https://www.example.com/path` → `example.com`.
pub fn display(raw: &str, full: bool) -> String {
    let Ok(u) = Url::parse(raw) else { return raw.to_string() };
    match u.scheme() {
        INTERNAL_SCHEME => return String::new(),
        "http" | "https" => {}
        _ => return raw.to_string(),
    }
    if full {
        return raw.to_string();
    }
    let h = u.host_str().unwrap_or("");
    h.strip_prefix("www.").unwrap_or(h).to_string()
}

pub fn is_secure(raw: &str) -> bool {
    Url::parse(raw).map(|u| matches!(u.scheme(), "https" | "oynix" | "file" | "qrc")).unwrap_or(false)
}

/// Registrable-ish domain (last two labels, three for common 2nd-level ccTLDs).
pub fn base_domain(host: &str) -> &str {
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() <= 2 {
        return host;
    }
    let sld = labels[labels.len() - 2];
    let take = if matches!(sld, "co" | "com" | "org" | "net" | "ac" | "gov" | "edu") && labels.last().map(|t| t.len() == 2).unwrap_or(false) {
        3
    } else {
        2
    };
    let skip: usize = labels[..labels.len() - take].iter().map(|l| l.len() + 1).sum();
    &host[skip..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_hosts_and_searches() {
        assert_eq!(resolve("example.com", "duckduckgo", true, true), "https://example.com/");
        assert_eq!(resolve("localhost:8080", "duckduckgo", true, true), "http://localhost:8080/");
        assert_eq!(resolve("https://a.b/c", "duckduckgo", true, true), "https://a.b/c");
        assert!(resolve("hello world", "duckduckgo", true, true).starts_with("https://duckduckgo.com/?q=hello+world"));
        assert!(resolve("rust", "kagi", true, true).starts_with("https://kagi.com/search?q=rust"));
        assert_eq!(resolve("oynix://settings", "kagi", true, true), "oynix://settings");
    }

    #[test]
    fn bangs() {
        assert_eq!(expand_bang("!w rust lang").unwrap(), "https://en.wikipedia.org/w/index.php?search=rust+lang");
        assert_eq!(expand_bang("rust !gh").unwrap(), "https://github.com/search?q=rust");
        assert_eq!(expand_bang("!gh").unwrap(), "https://github.com/");
        assert!(expand_bang("!nope x").is_none());
    }

    #[test]
    fn strips_trackers() {
        assert_eq!(strip_tracking("https://a.com/?utm_source=x&id=3&fbclid=y"), "https://a.com/?id=3");
        assert_eq!(strip_tracking("https://a.com/?utm_source=x"), "https://a.com/");
        assert_eq!(strip_tracking("https://a.com/?q=1"), "https://a.com/?q=1");
    }

    #[test]
    fn display_and_domains() {
        assert_eq!(display("https://www.example.com/x", false), "example.com");
        assert_eq!(base_domain("a.b.example.co.uk"), "example.co.uk");
        assert_eq!(base_domain("cdn.example.com"), "example.com");
    }
}
