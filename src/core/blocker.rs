//! Native content blocker.
//!
//! Every network request made by any page goes through the Qt WebEngine URL
//! request interceptor (see `cpp/native.cpp`), which calls
//! [`should_block`] synchronously on the IO thread. Matching is a hash-set
//! walk over the host's parent domains plus a small list of URL substring
//! rules, so the cost per request is a handful of hash lookups.
//!
//! Rule sources:
//!   * a built-in list of well-known ad / tracker / telemetry hosts;
//!   * any `*.txt` file in `~/.config/oynix/<profile>/filters/` written in the
//!     common Adblock-Plus / uBlock syntax subset (`||host^`, `|https://…`,
//!     `@@||host^` exceptions, plain substrings, `0.0.0.0 host` hosts files).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, RwLock};

use super::{omnibox, paths};

/// Ad networks.
const ADS: &[&str] = &[
    "doubleclick.net", "googlesyndication.com", "googleadservices.com", "adservice.google.com",
    "pagead2.googlesyndication.com", "adnxs.com", "adsrvr.org", "advertising.com", "taboola.com",
    "outbrain.com", "criteo.com", "criteo.net", "pubmatic.com", "rubiconproject.com", "openx.net",
    "casalemedia.com", "amazon-adsystem.com", "media.net", "adform.net", "smartadserver.com",
    "yieldmo.com", "sharethrough.com", "33across.com", "indexww.com", "teads.tv", "moatads.com",
    "zedo.com", "revcontent.com", "mgid.com", "propellerads.com", "popads.net", "popcash.net",
    "adcolony.com", "applovin.com", "unityads.unity3d.com", "inmobi.com", "bidswitch.net",
    "contextweb.com", "lijit.com", "sovrn.com", "spotxchange.com", "springserve.com",
    "tremorhub.com", "undertone.com", "yieldlab.net", "adroll.com", "adition.com", "serving-sys.com",
    "flashtalking.com", "eyeota.net", "adsafeprotected.com", "doubleverify.com", "3lift.com",
    "triplelift.com", "gumgum.com", "kargo.com", "nativo.com", "adskeeper.co.uk", "zergnet.com",
    "exoclick.com", "juicyads.com", "trafficjunky.net", "adsterra.com", "admixer.net",
    "ads.yahoo.com", "ads.twitter.com", "ads.linkedin.com", "ads.pinterest.com", "ads.tiktok.com",
];

/// Analytics, fingerprinting, session-replay and telemetry endpoints.
const TRACKERS: &[&str] = &[
    "google-analytics.com", "analytics.google.com", "googletagmanager.com", "googletagservices.com",
    "ssl.google-analytics.com", "stats.g.doubleclick.net", "connect.facebook.net", "pixel.facebook.com",
    "graph.facebook.com/tr", "analytics.twitter.com", "static.ads-twitter.com", "t.co/i/adsct",
    "bat.bing.com", "clarity.ms", "scorecardresearch.com", "quantserve.com", "quantcount.com",
    "hotjar.com", "hotjar.io", "mouseflow.com", "fullstory.com", "crazyegg.com", "luckyorange.com",
    "inspectlet.com", "smartlook.com", "logrocket.com", "logrocket.io", "mixpanel.com", "segment.com",
    "segment.io", "cdn.segment.com", "amplitude.com", "heap.io", "heapanalytics.com", "kissmetrics.com",
    "chartbeat.com", "chartbeat.net", "newrelic.com", "nr-data.net", "parsely.com", "parse.ly",
    "omtrdc.net", "demdex.net", "everesttech.net", "2o7.net", "krxd.net", "bluekai.com",
    "exelator.com", "rlcdn.com", "agkn.com", "adsymptotic.com", "tapad.com", "liveramp.com",
    "pippio.com", "crwdcntrl.net", "addthis.com", "addthisedge.com", "sharethis.com",
    "branch.io", "app-measurement.com", "appsflyer.com", "adjust.com", "kochava.com",
    "braze.com", "iterable.com", "optimizely.com", "cdn.optimizely.com", "vwo.com",
    "visualwebsiteoptimizer.com", "snap.licdn.com", "px.ads.linkedin.com", "tr.snapchat.com",
    "sc-static.net", "analytics.tiktok.com", "ct.pinterest.com", "pixel.wp.com", "stats.wp.com",
    "matomo.cloud", "statcounter.com", "histats.com", "yandex.ru/metrika", "mc.yandex.ru",
    "top-fwz1.mail.ru", "counter.yadro.ru", "omniture.com", "sentry-cdn.com", "bugsnag.com",
    "fingerprintjs.com", "fpjs.io", "iovation.com", "threatmetrix.com", "siftscience.com",
    "onesignal.com", "pushcrew.com", "pushengage.com", "cdn.mxpnl.com", "insight.adsrvr.org",
    "tags.tiqcdn.com", "tealiumiq.com", "ensighten.com", "trustarc.com", "evidon.com",
    "permutive.com", "permutive.app", "zeotap.com", "id5-sync.com", "lotame.com", "33across.com",
    "adobedtm.com", "assets.adobedtm.com", "pardot.com", "marketo.net", "hubspot.com/__ptq.gif",
    "hs-analytics.net", "hs-banner.com", "hsadspixel.net", "usefathom.com/script.js",
];

/// URL path fragments that are almost always tracking beacons.
const URL_PATTERNS: &[&str] = &[
    "/pagead/", "/adserver/", "/ads/ga-audiences", "/collect?v=", "/g/collect", "/j/collect",
    "/r/collect", "/tr?id=", "/pixel.gif?", "/beacon.js", "/__utm.gif", "/analytics.js",
    "/gtag/js", "/gtm.js", "/fbevents.js", "/piwik.js", "/matomo.js", "/adsbygoogle.js",
];

#[derive(Default)]
struct Rules {
    ads: HashSet<String>,
    trackers: HashSet<String>,
    /// Domains from user filter lists (counted as ads+trackers).
    custom: HashSet<String>,
    exceptions: HashSet<String>,
    substrings: Vec<String>,
    path_rules: Vec<(String, String)>,
    lists_loaded: usize,
}

static RULES: LazyLock<RwLock<Rules>> = LazyLock::new(|| RwLock::new(Rules::default()));

static ENABLED: AtomicBool = AtomicBool::new(true);
static BLOCK_ADS: AtomicBool = AtomicBool::new(true);
static BLOCK_TRACKERS: AtomicBool = AtomicBool::new(true);
static TOTAL_BLOCKED: AtomicU64 = AtomicU64::new(0);
static TOTAL_ADS: AtomicU64 = AtomicU64::new(0);
static TOTAL_TRACKERS: AtomicU64 = AtomicU64::new(0);

/// Hosts (top-level page) where the user disabled blocking.
static ALLOWLIST: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(Default::default);

/// first-party host → (blocked count, set of blocked third-party domains)
static PER_SITE: LazyLock<Mutex<HashMap<String, (u64, HashSet<String>)>>> = LazyLock::new(Default::default);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ad,
    Tracker,
}

pub fn init() {
    let mut r = Rules::default();
    r.ads = ADS.iter().map(|s| s.to_string()).collect();
    for t in TRACKERS {
        if let Some((h, p)) = t.split_once('/') {
            r.path_rules.push((h.to_string(), format!("/{p}")));
        } else {
            r.trackers.insert(t.to_string());
        }
    }
    r.substrings = URL_PATTERNS.iter().map(|s| s.to_string()).collect();
    if let Ok(rd) = std::fs::read_dir(paths::filters_dir()) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "txt").unwrap_or(false) {
                if let Ok(txt) = std::fs::read_to_string(e.path()) {
                    parse_list(&txt, &mut r);
                    r.lists_loaded += 1;
                }
            }
        }
    }
    *RULES.write().unwrap() = r;
    load_stats();
}

/// Parse the subset of Adblock Plus syntax that maps cleanly onto network
/// blocking. Cosmetic (`##`) rules and rules with options we cannot honour
/// are skipped.
fn parse_list(txt: &str, r: &mut Rules) {
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') || line.starts_with('#') {
            continue;
        }
        if line.contains("##") || line.contains("#@#") || line.contains("#?#") || line.contains("#$#") {
            continue;
        }
        // hosts-file format
        if let Some(rest) = line.strip_prefix("0.0.0.0 ").or_else(|| line.strip_prefix("127.0.0.1 ")) {
            let h = rest.split_whitespace().next().unwrap_or("");
            if !h.is_empty() && h != "localhost" && h != "0.0.0.0" {
                r.custom.insert(h.to_lowercase());
            }
            continue;
        }
        let (exception, body) = match line.strip_prefix("@@") {
            Some(b) => (true, b),
            None => (false, line),
        };
        let (pattern, options) = match body.split_once('$') {
            Some((p, o)) => (p, o),
            None => (body, ""),
        };
        // Skip rules restricted to specific first parties – we can't
        // evaluate `domain=` cheaply and would over-block.
        if options.contains("domain=") {
            continue;
        }
        if let Some(rest) = pattern.strip_prefix("||") {
            let end = rest.find(['^', '/', '*', '|']).unwrap_or(rest.len());
            let host = rest[..end].to_lowercase();
            let tail = &rest[end..];
            if host.is_empty() || host.contains('*') {
                continue;
            }
            if tail.is_empty() || tail == "^" || tail == "^|" {
                if exception {
                    r.exceptions.insert(host);
                } else {
                    r.custom.insert(host);
                }
            } else if !exception {
                let path = tail.trim_start_matches('^').trim_end_matches(['^', '|', '*']);
                if !path.is_empty() && !path.contains('*') {
                    r.path_rules.push((host, path.to_string()));
                }
            }
        } else if !exception {
            let p = pattern.trim_start_matches('|').trim_end_matches('|').trim_matches('*');
            if p.len() >= 6 && !p.contains('*') && !p.contains('^') {
                r.substrings.push(p.to_string());
            }
        }
    }
}

fn host_matches(set: &HashSet<String>, host: &str) -> bool {
    let mut h = host;
    loop {
        if set.contains(h) {
            return true;
        }
        match h.find('.') {
            Some(i) => h = &h[i + 1..],
            None => return false,
        }
    }
}

/// Decide whether a request should be blocked.
///
/// `resource_type` follows `QWebEngineUrlRequestInfo::ResourceType`;
/// `0` (main frame) is never blocked.
pub fn classify(url: &str, first_party: &str, resource_type: i32) -> Option<Kind> {
    if resource_type == 0 || !ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    let lower;
    let url = if url.bytes().any(|b| b.is_ascii_uppercase()) {
        lower = url.to_ascii_lowercase();
        lower.as_str()
    } else {
        url
    };
    let host = match url.split_once("://") {
        Some((_, rest)) => rest.split(['/', '?', '#', ':']).next().unwrap_or(""),
        None => return None,
    };
    if host.is_empty() {
        return None;
    }
    let fp_host = omnibox::host_of(first_party);
    if !fp_host.is_empty() && ALLOWLIST.read().unwrap().contains(&fp_host) {
        return None;
    }
    // First-party requests are never treated as third-party trackers.
    let same_site = !fp_host.is_empty() && omnibox::base_domain(&fp_host) == omnibox::base_domain(host);

    let r = RULES.read().unwrap();
    if host_matches(&r.exceptions, host) {
        return None;
    }
    let ads = BLOCK_ADS.load(Ordering::Relaxed);
    let trk = BLOCK_TRACKERS.load(Ordering::Relaxed);
    if !same_site {
        if ads && host_matches(&r.ads, host) {
            return Some(Kind::Ad);
        }
        if trk && host_matches(&r.trackers, host) {
            return Some(Kind::Tracker);
        }
        if (ads || trk) && host_matches(&r.custom, host) {
            return Some(Kind::Ad);
        }
    }
    if trk {
        let path = &url[url.find(host).map(|i| i + host.len()).unwrap_or(0)..];
        for (h, p) in &r.path_rules {
            if (host == h || host.ends_with(&format!(".{h}"))) && path.starts_with(p.as_str()) {
                return Some(Kind::Tracker);
            }
        }
        if !same_site && r.substrings.iter().any(|s| url.contains(s.as_str())) {
            return Some(Kind::Tracker);
        }
    }
    None
}

/// Entry point used by the interceptor. Records statistics.
pub fn should_block(url: &str, first_party: &str, resource_type: i32) -> bool {
    let Some(kind) = classify(url, first_party, resource_type) else { return false };
    TOTAL_BLOCKED.fetch_add(1, Ordering::Relaxed);
    match kind {
        Kind::Ad => TOTAL_ADS.fetch_add(1, Ordering::Relaxed),
        Kind::Tracker => TOTAL_TRACKERS.fetch_add(1, Ordering::Relaxed),
    };
    let fp = omnibox::host_of(first_party);
    let blocked_host = omnibox::host_of(url);
    if let Ok(mut m) = PER_SITE.lock() {
        let e = m.entry(fp).or_default();
        e.0 += 1;
        if e.1.len() < 64 {
            e.1.insert(omnibox::base_domain(&blocked_host).to_string());
        }
    }
    true
}

pub fn configure(enabled: bool, ads: bool, trackers: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
    BLOCK_ADS.store(ads, Ordering::Relaxed);
    BLOCK_TRACKERS.store(trackers, Ordering::Relaxed);
}

pub fn set_allowlist<I: IntoIterator<Item = String>>(hosts: I) {
    *ALLOWLIST.write().unwrap() = hosts.into_iter().collect();
}

pub fn site_report(host: &str) -> (u64, Vec<String>) {
    let m = PER_SITE.lock().unwrap();
    match m.get(host) {
        Some((n, set)) => {
            let mut v: Vec<String> = set.iter().cloned().collect();
            v.sort();
            (*n, v)
        }
        None => (0, vec![]),
    }
}

pub fn totals() -> (u64, u64, u64) {
    (
        TOTAL_BLOCKED.load(Ordering::Relaxed),
        TOTAL_ADS.load(Ordering::Relaxed),
        TOTAL_TRACKERS.load(Ordering::Relaxed),
    )
}

pub fn rule_count() -> (usize, usize) {
    let r = RULES.read().unwrap();
    (r.ads.len() + r.trackers.len() + r.custom.len() + r.path_rules.len() + r.substrings.len(), r.lists_loaded)
}

/// Top tracker domains across all sites, for the privacy report.
pub fn top_trackers(limit: usize) -> Vec<(String, usize)> {
    let m = PER_SITE.lock().unwrap();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (_, (_, set)) in m.iter() {
        for d in set {
            *counts.entry(d.as_str()).or_default() += 1;
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(limit);
    v
}

fn stats_file() -> std::path::PathBuf {
    paths::data_dir().join("privacy-stats.json")
}

fn load_stats() {
    if let Ok(b) = std::fs::read(stats_file()) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&b) {
            let g = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
            TOTAL_BLOCKED.store(g("blocked"), Ordering::Relaxed);
            TOTAL_ADS.store(g("ads"), Ordering::Relaxed);
            TOTAL_TRACKERS.store(g("trackers"), Ordering::Relaxed);
        }
    }
}

pub fn save_stats() {
    let (b, a, t) = totals();
    let v = serde_json::json!({ "blocked": b, "ads": a, "trackers": t });
    let _ = paths::write_atomic(&stats_file(), v.to_string().as_bytes());
}

pub fn reset_stats() {
    TOTAL_BLOCKED.store(0, Ordering::Relaxed);
    TOTAL_ADS.store(0, Ordering::Relaxed);
    TOTAL_TRACKERS.store(0, Ordering::Relaxed);
    PER_SITE.lock().unwrap().clear();
    save_stats();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_third_party_trackers_only() {
        let mut r = Rules::default();
        r.trackers.insert("google-analytics.com".into());
        r.ads.insert("doubleclick.net".into());
        parse_list("||evil.example^\n@@||ok.evil.example^\n||cdn.x.com/track/pixel\n", &mut r);
        *RULES.write().unwrap() = r;
        configure(true, true, true);
        assert_eq!(classify("https://www.google-analytics.com/a.js", "https://news.site/", 10), Some(Kind::Tracker));
        assert_eq!(classify("https://ad.doubleclick.net/x", "https://news.site/", 10), Some(Kind::Ad));
        assert_eq!(classify("https://a.evil.example/x", "https://news.site/", 10), Some(Kind::Ad));
        assert_eq!(classify("https://ok.evil.example/x", "https://news.site/", 10), None);
        assert_eq!(classify("https://cdn.x.com/track/pixel?1", "https://news.site/", 10), Some(Kind::Tracker));
        // main frame never blocked
        assert_eq!(classify("https://ad.doubleclick.net/x", "", 0), None);
        // first party is allowed
        assert_eq!(classify("https://doubleclick.net/x", "https://www.doubleclick.net/", 10), None);
    }
}
