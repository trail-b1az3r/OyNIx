//! Native content blocker.
//!
//! Every network request goes through the Qt WebEngine URL request
//! interceptor (`cpp/native.cpp`), which calls [`should_block`] on the IO
//! thread. Rules come from:
//!
//!   * a small built-in list of ad / tracker hosts (works offline, first run);
//!   * the subscribed filter lists (EasyList, EasyPrivacy, uBlock, AdGuard,
//!     Fanboy… see `filterlists.rs`), downloaded and refreshed automatically;
//!   * any `*.txt` in `~/.config/oynix/<profile>/filters/`.
//!
//! Supported syntax (Adblock Plus / uBlock Origin): `||host^`, `|https://`,
//! wildcards `*`, separators `^`, end anchors, `@@` exceptions, hosts-file
//! lines, and the options `third-party`/`1p`/`3p`, resource types (`script`,
//! `image`, `stylesheet`, `xhr`, `subdocument`, `media`, `font`, `ping`,
//! `websocket`, `object`, `other`, negated with `~`), `domain=`/`from=`,
//! `important`, `document`/`elemhide`/`generichide` exceptions. Cosmetic
//! rules (`##`, `domain##`, `#@#`) hide page elements via injected CSS.
//! Rules that need features OyNIx does not implement (redirects, `$csp`,
//! `$removeparam`, scriptlets, procedural selectors, regex) are skipped.
//!
//! Matching uses a host-suffix index for `||host^` rules and an
//! Aho–Corasick automaton over each rule's longest literal for the rest, so
//! a request costs a handful of hash lookups plus one linear scan of the URL.

use aho_corasick::{AhoCorasick, MatchKind};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use super::filterlists::Category;
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Ad,
    Tracker,
}

// ── Resource types ─────────────────────────────────────────────────────
const T_SCRIPT: u32 = 1;
const T_IMAGE: u32 = 1 << 1;
const T_STYLE: u32 = 1 << 2;
const T_OBJECT: u32 = 1 << 3;
const T_XHR: u32 = 1 << 4;
const T_SUBDOC: u32 = 1 << 5;
const T_PING: u32 = 1 << 6;
const T_MEDIA: u32 = 1 << 7;
const T_FONT: u32 = 1 << 8;
const T_WEBSOCKET: u32 = 1 << 9;
const T_OTHER: u32 = 1 << 10;
const T_DOCUMENT: u32 = 1 << 11;
const T_ALL_SUB: u32 = T_DOCUMENT - 1;

/// Map `QWebEngineUrlRequestInfo::ResourceType` to a type bit.
fn type_bit(rt: i32) -> u32 {
    match rt {
        0 | 19 => T_DOCUMENT,
        1 | 20 => T_SUBDOC,
        2 => T_STYLE,
        3 | 9 | 10 | 15 => T_SCRIPT,
        4 | 12 => T_IMAGE,
        5 => T_FONT,
        7 | 17 => T_OBJECT,
        8 => T_MEDIA,
        13 | 21 => T_XHR,
        14 | 16 => T_PING,
        254 => T_WEBSOCKET,
        _ => T_OTHER,
    }
}

// ── Network rules ──────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Anchor {
    None,
    Start,
    Host,
}

#[derive(Clone, Debug)]
struct Rule {
    pat: Box<str>,
    anchor: Anchor,
    end: bool,
    third: Option<bool>,
    /// 0 = every sub-resource type (never the main document).
    types: u32,
    incl: Box<[Box<str>]>,
    excl: Box<[Box<str>]>,
    important: bool,
    kind: Kind,
}

struct Req<'a> {
    url: &'a str,
    host: &'a str,
    fp: &'a str,
    third: bool,
    ty: u32,
}

fn is_sep(c: u8) -> bool {
    !(c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.' | b'%'))
}

/// Wildcard match supporting `*` and `^` (separator or end of input).
fn glob(p: &[u8], t: &[u8], lead_star: bool, end: bool) -> bool {
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = if lead_star { Some((0, 0)) } else { None };
    loop {
        if pi < p.len() {
            match p[pi] {
                b'*' => {
                    star = Some((pi + 1, ti));
                    pi += 1;
                    continue;
                }
                b'^' => {
                    if ti < t.len() && is_sep(t[ti]) {
                        pi += 1;
                        ti += 1;
                        continue;
                    } else if ti == t.len() {
                        pi += 1;
                        continue;
                    }
                }
                c => {
                    if ti < t.len() && t[ti] == c {
                        pi += 1;
                        ti += 1;
                        continue;
                    }
                }
            }
        } else if !end || ti == t.len() {
            return true;
        }
        match star {
            Some((sp, st)) if st < t.len() => {
                star = Some((sp, st + 1));
                pi = sp;
                ti = st + 1;
            }
            _ => return false,
        }
    }
}

fn host_range(url: &str) -> Option<(usize, usize)> {
    let s = url.find("://")? + 3;
    let rest = &url[s..];
    let rest_start = rest.find('@').map(|i| i + 1).filter(|&i| i < rest.find('/').unwrap_or(rest.len())).unwrap_or(0);
    let hs = s + rest_start;
    let he = url[hs..].find(['/', '?', '#', ':']).map(|i| hs + i).unwrap_or(url.len());
    Some((hs, he))
}

fn pattern_matches(r: &Rule, url: &str) -> bool {
    let p = r.pat.as_bytes();
    let t = url.as_bytes();
    match r.anchor {
        Anchor::Start => glob(p, t, false, r.end),
        Anchor::None => glob(p, t, true, r.end),
        Anchor::Host => {
            let Some((hs, he)) = host_range(url) else { return false };
            let mut i = hs;
            loop {
                if glob(p, &t[i..], false, r.end) {
                    return true;
                }
                match url[i..he].find('.') {
                    Some(d) => i += d + 1,
                    None => return false,
                }
            }
        }
    }
}

fn dom_match(host: &str, d: &str) -> bool {
    host == d || (host.len() > d.len() && host.ends_with(d) && host.as_bytes()[host.len() - d.len() - 1] == b'.')
}

fn applies(r: &Rule, q: &Req) -> bool {
    if let Some(t) = r.third {
        if t != q.third {
            return false;
        }
    }
    if r.types == 0 {
        if q.ty == T_DOCUMENT {
            return false;
        }
    } else if r.types & q.ty == 0 {
        return false;
    }
    if !r.incl.is_empty() && !r.incl.iter().any(|d| dom_match(q.fp, d)) {
        return false;
    }
    !r.excl.iter().any(|d| dom_match(q.fp, d))
}

#[derive(Default)]
struct Engine {
    rules: Vec<Rule>,
    host_index: HashMap<Box<str>, Vec<u32>>,
    token_map: HashMap<Box<str>, usize>,
    tokens: Vec<Box<str>>,
    token_rules: Vec<Vec<u32>>,
    ac: Option<AhoCorasick>,
    fallback: Vec<u32>,
}

fn best_token(p: &str) -> Option<&str> {
    p.split(['*', '^']).filter(|s| s.len() >= 3).max_by_key(|s| s.len())
}

impl Engine {
    fn add(&mut self, r: Rule) {
        let id = self.rules.len() as u32;
        let pure_host = r.anchor == Anchor::Host
            && r.pat.ends_with('^')
            && r.pat[..r.pat.len() - 1].bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_');
        if pure_host {
            self.host_index.entry(r.pat[..r.pat.len() - 1].into()).or_default().push(id);
        } else if let Some(tok) = best_token(&r.pat) {
            let idx = match self.token_map.get(tok) {
                Some(&i) => i,
                None => {
                    let i = self.tokens.len();
                    self.tokens.push(tok.into());
                    self.token_rules.push(Vec::new());
                    self.token_map.insert(tok.into(), i);
                    i
                }
            };
            self.token_rules[idx].push(id);
        } else {
            self.fallback.push(id);
        }
        self.rules.push(r);
    }

    fn finish(&mut self) {
        if !self.tokens.is_empty() {
            self.ac = AhoCorasick::builder().match_kind(MatchKind::Standard).build(self.tokens.iter().map(|t| t.as_bytes())).ok();
        }
        self.token_map = HashMap::new();
    }

    fn find(&self, q: &Req) -> Option<&Rule> {
        self.find_where(q, |_| true)
    }

    fn find_where(&self, q: &Req, keep: impl Fn(&Rule) -> bool) -> Option<&Rule> {
        let mut h = q.host;
        loop {
            if let Some(ids) = self.host_index.get(h) {
                for &id in ids {
                    let r = &self.rules[id as usize];
                    if keep(r) && applies(r, q) {
                        return Some(r);
                    }
                }
            }
            match h.find('.') {
                Some(i) => h = &h[i + 1..],
                None => break,
            }
        }
        if let Some(ac) = &self.ac {
            for m in ac.find_overlapping_iter(q.url) {
                for &id in &self.token_rules[m.pattern().as_usize()] {
                    let r = &self.rules[id as usize];
                    if keep(r) && applies(r, q) && pattern_matches(r, q.url) {
                        return Some(r);
                    }
                }
            }
        }
        for &id in &self.fallback {
            let r = &self.rules[id as usize];
            if keep(r) && applies(r, q) && pattern_matches(r, q.url) {
                return Some(r);
            }
        }
        None
    }

    fn len(&self) -> usize {
        self.rules.len()
    }
}

// ── Cosmetic rules ─────────────────────────────────────────────────────
#[derive(Default)]
struct Cosmetic {
    generic: Vec<Box<str>>,
    generic_seen: HashSet<Box<str>>,
    global_unhide: HashSet<Box<str>>,
    specific: HashMap<Box<str>, Vec<Box<str>>>,
    unhide: HashMap<Box<str>, HashSet<Box<str>>>,
}

const PROCEDURAL: &[&str] = &[
    ":-abp-", ":has-text(", ":contains(", ":xpath(", ":upward(", ":remove(", ":style(", ":matches-css",
    ":min-text-length", ":watch-attr", ":matches-path", ":others(", ":if(", ":if-not(", ":nth-ancestor(",
    ":matches-attr", ":remove-attr", ":remove-class", ":matches-media", ":matches-prop", ":shadow-dom-",
];

impl Cosmetic {
    fn add(&mut self, domains: &str, sel: &str, exception: bool) {
        let sel = sel.trim();
        if sel.is_empty() || sel.starts_with("+js(") || sel.starts_with('^') || PROCEDURAL.iter().any(|p| sel.contains(p)) {
            return;
        }
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        for d in domains.split(',').map(str::trim).filter(|d| !d.is_empty()) {
            if d.ends_with(".*") || d.contains('/') {
                continue;
            }
            match d.strip_prefix('~') {
                Some(n) => neg.push(n.to_lowercase()),
                None => pos.push(d.to_lowercase()),
            }
        }
        if !domains.trim().is_empty() && pos.is_empty() && neg.is_empty() {
            return; // only unsupported domain forms
        }
        let s: Box<str> = sel.into();
        if exception {
            if pos.is_empty() {
                self.global_unhide.insert(s);
            } else {
                for d in pos {
                    self.unhide.entry(d.into()).or_default().insert(s.clone());
                }
            }
            return;
        }
        if pos.is_empty() {
            if self.generic_seen.insert(s.clone()) {
                self.generic.push(s.clone());
            }
        } else {
            for d in pos {
                self.specific.entry(d.into()).or_default().push(s.clone());
            }
        }
        for d in neg {
            self.unhide.entry(d.into()).or_default().insert(s.clone());
        }
    }

    fn finish(&mut self) {
        if !self.global_unhide.is_empty() {
            let gu = std::mem::take(&mut self.global_unhide);
            self.generic.retain(|s| !gu.contains(s));
            self.global_unhide = gu;
        }
        self.generic_seen = HashSet::new();
    }

    fn len(&self) -> usize {
        self.generic.len() + self.specific.values().map(Vec::len).sum::<usize>()
    }
}

// ── Rule set ───────────────────────────────────────────────────────────
#[derive(Default)]
struct Rules {
    block: Engine,
    allow: Engine,
    doc_allow: HashSet<String>,
    elemhide: HashSet<String>,
    generichide: HashSet<String>,
    cosmetic: Cosmetic,
    lists_loaded: usize,
    skipped: usize,
}

fn suffix_in(set: &HashSet<String>, host: &str) -> bool {
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

struct Parsed {
    rule: Rule,
    special: Option<&'static str>,
}

/// Parse one network rule. Returns None for comments, unsupported or
/// malformed rules.
fn parse_network(line: &str, kind: Kind) -> Option<(bool, Parsed)> {
    let (exception, body) = match line.strip_prefix("@@") {
        Some(b) => (true, b),
        None => (false, line),
    };
    if body.starts_with('/') && body.len() > 2 && (body.ends_with('/') || body.contains("/$")) {
        return None; // regex rule
    }
    let (pattern, options) = match body.rfind('$') {
        Some(i) if !body[i + 1..].contains('/') || body[i + 1..].starts_with("domain=") => (&body[..i], &body[i + 1..]),
        _ => (body, ""),
    };
    let mut third = None;
    let mut types = 0u32;
    let mut neg_types = 0u32;
    let mut incl: Vec<Box<str>> = Vec::new();
    let mut excl: Vec<Box<str>> = Vec::new();
    let mut important = false;
    let mut special = None;
    for opt in options.split(',').map(str::trim).filter(|o| !o.is_empty()) {
        let (neg, name) = match opt.strip_prefix('~') {
            Some(n) => (true, n),
            None => (false, opt),
        };
        let (key, value) = match name.split_once('=') {
            Some((k, v)) => (k, Some(v)),
            None => (name, None),
        };
        let bit = match key {
            "script" => T_SCRIPT,
            "image" => T_IMAGE,
            "stylesheet" | "css" => T_STYLE,
            "object" | "object-subrequest" => T_OBJECT,
            "xmlhttprequest" | "xhr" => T_XHR,
            "subdocument" | "frame" => T_SUBDOC,
            "ping" | "beacon" => T_PING,
            "media" => T_MEDIA,
            "font" => T_FONT,
            "websocket" => T_WEBSOCKET,
            "other" => T_OTHER,
            _ => 0,
        };
        if bit != 0 {
            if neg { neg_types |= bit } else { types |= bit }
            continue;
        }
        match key {
            "third-party" | "3p" => third = Some(!neg),
            "first-party" | "1p" => third = Some(neg),
            "domain" | "from" => {
                for d in value.unwrap_or("").split('|').map(str::trim).filter(|d| !d.is_empty()) {
                    if d.ends_with(".*") || d.starts_with('/') {
                        continue;
                    }
                    match d.strip_prefix('~') {
                        Some(n) => excl.push(n.to_lowercase().into()),
                        None => incl.push(d.to_lowercase().into()),
                    }
                }
                if incl.is_empty() && excl.is_empty() {
                    return None;
                }
            }
            "important" => important = true,
            "match-case" | "all" | "ehide" | "shide" | "specifichide" => {}
            "document" | "doc" => {
                if exception { special = Some("document") } else { return None }
            }
            "elemhide" => special = Some("elemhide"),
            "generichide" | "ghide" => special = Some("generichide"),
            "popup" | "popunder" => {
                if options.split(',').count() == 1 || !exception { return None }
            }
            _ => return None, // redirect, csp, removeparam, badfilter, denyallow, …
        }
    }
    if special.is_some() && !exception {
        return None;
    }
    if neg_types != 0 {
        types = if types == 0 { T_ALL_SUB & !neg_types } else { types & !neg_types };
        if types == 0 {
            return None;
        }
    }
    let mut p = pattern.to_ascii_lowercase();
    let anchor = if let Some(rest) = p.strip_prefix("||") {
        p = rest.to_string();
        Anchor::Host
    } else if let Some(rest) = p.strip_prefix('|') {
        p = rest.to_string();
        Anchor::Start
    } else {
        Anchor::None
    };
    let mut end = false;
    if p.ends_with('|') {
        p.pop();
        end = true;
    }
    while p.ends_with("**") {
        p.pop();
    }
    let trimmed = p.trim_matches('*');
    if trimmed.is_empty() && special.is_none() {
        // Matches everything – only valid when restricted by domain.
        if incl.is_empty() {
            return None;
        }
    }
    if anchor == Anchor::None && special.is_none() {
        p = trimmed.to_string();
        if p.len() < 3 && incl.is_empty() {
            return None; // too short to be meaningful
        }
    }
    Some((
        exception,
        Parsed {
            rule: Rule { pat: p.into(), anchor, end, third, types, incl: incl.into(), excl: excl.into(), important, kind },
            special,
        },
    ))
}

fn hosts_of(r: &Rule) -> Vec<String> {
    if r.anchor == Anchor::Host {
        let h = r.pat.trim_end_matches('^').trim_end_matches('*');
        if !h.is_empty() && !h.contains('/') && !h.contains('*') {
            return vec![h.to_string()];
        }
    }
    r.incl.iter().map(|d| d.to_string()).collect()
}

fn parse_list(txt: &str, category: Category, r: &mut Rules) {
    let kind = if category.is_tracking() { Kind::Tracker } else { Kind::Ad };
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
            continue;
        }
        // hosts-file format
        if let Some(rest) = line.strip_prefix("0.0.0.0 ").or_else(|| line.strip_prefix("127.0.0.1 ")) {
            let h = rest.split_whitespace().next().unwrap_or("").to_lowercase();
            if !h.is_empty() && h != "localhost" && h != "0.0.0.0" && !h.starts_with('#') {
                r.block.add(Rule { pat: format!("{h}^").into(), anchor: Anchor::Host, end: false, third: None, types: 0, incl: Box::new([]), excl: Box::new([]), important: false, kind });
            }
            continue;
        }
        if line.starts_with('#') && !line.starts_with("##") && !line.starts_with("#@#") {
            continue; // comment in hosts files
        }
        // Cosmetic rules
        if let Some(i) = line.find("#@#") {
            if !line.contains("#@?#") && !line.contains("#@$#") {
                r.cosmetic.add(&line[..i], &line[i + 3..], true);
            }
            continue;
        }
        if line.contains("#?#") || line.contains("#$#") || line.contains("#%#") || line.contains("#$?#") || line.contains("#@?#") {
            r.skipped += 1;
            continue;
        }
        if let Some(i) = line.find("##") {
            r.cosmetic.add(&line[..i], &line[i + 2..], false);
            continue;
        }
        match parse_network(line, kind) {
            Some((false, p)) => r.block.add(p.rule),
            Some((true, p)) => match p.special {
                Some("document") => r.doc_allow.extend(hosts_of(&p.rule)),
                Some("elemhide") => r.elemhide.extend(hosts_of(&p.rule)),
                Some("generichide") => r.generichide.extend(hosts_of(&p.rule)),
                _ => r.allow.add(p.rule),
            },
            None => r.skipped += 1,
        }
    }
}

fn builtin(r: &mut Rules) {
    let mut add = |h: &str, kind: Kind| {
        let line = match h.split_once('/') {
            Some((host, path)) => format!("||{host}/{path}"),
            None => format!("||{h}^$third-party"),
        };
        if let Some((false, p)) = parse_network(&line, kind) {
            r.block.add(p.rule);
        }
    };
    for h in ADS {
        add(h, Kind::Ad);
    }
    for h in TRACKERS {
        add(h, Kind::Tracker);
    }
    for p in URL_PATTERNS {
        if let Some((false, p)) = parse_network(&format!("{p}$third-party"), Kind::Tracker) {
            r.block.add(p.rule);
        }
    }
}

// ── State ──────────────────────────────────────────────────────────────
static RULES: LazyLock<RwLock<Arc<Rules>>> = LazyLock::new(|| RwLock::new(Arc::new(Rules::default())));
static GENERATION: AtomicU64 = AtomicU64::new(0);
static LOADING: AtomicBool = AtomicBool::new(false);

static ENABLED: AtomicBool = AtomicBool::new(true);
static BLOCK_ADS: AtomicBool = AtomicBool::new(true);
static BLOCK_TRACKERS: AtomicBool = AtomicBool::new(true);
static COSMETIC: AtomicBool = AtomicBool::new(true);
static TOTAL_BLOCKED: AtomicU64 = AtomicU64::new(0);
static TOTAL_ADS: AtomicU64 = AtomicU64::new(0);
static TOTAL_TRACKERS: AtomicU64 = AtomicU64::new(0);

/// Hosts (top-level page) where the user disabled blocking.
static ALLOWLIST: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(Default::default);

/// first-party host → (blocked count, set of blocked third-party domains)
static PER_SITE: LazyLock<Mutex<HashMap<String, (u64, HashSet<String>)>>> = LazyLock::new(Default::default);

fn rules() -> Arc<Rules> {
    RULES.read().unwrap().clone()
}

/// Build a rule set from the built-in list, the given subscriptions
/// (id, category) and the user's filters folder.
fn build(lists: &[(String, Category)]) -> Rules {
    let mut r = Rules::default();
    builtin(&mut r);
    for (id, cat) in lists {
        if let Ok(txt) = std::fs::read_to_string(super::filterlists::file(id)) {
            parse_list(&txt, *cat, &mut r);
            r.lists_loaded += 1;
        }
    }
    if let Ok(rd) = std::fs::read_dir(paths::filters_dir()) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "txt").unwrap_or(false) {
                if let Ok(txt) = std::fs::read_to_string(e.path()) {
                    parse_list(&txt, Category::Custom, &mut r);
                    r.lists_loaded += 1;
                }
            }
        }
    }
    r.block.finish();
    r.allow.finish();
    r.cosmetic.finish();
    r
}

/// Install the built-in rules immediately, then parse the subscribed lists
/// on a background thread and swap them in when ready.
pub fn init(lists: Vec<(String, Category)>) {
    if GENERATION.load(Ordering::Relaxed) == 0 {
        let mut r = Rules::default();
        builtin(&mut r);
        r.block.finish();
        *RULES.write().unwrap() = Arc::new(r);
        load_stats();
    }
    LOADING.store(true, Ordering::Relaxed);
    std::thread::Builder::new()
        .name("oynix-filters".into())
        .spawn(move || {
            let r = build(&lists);
            *RULES.write().unwrap() = Arc::new(r);
            GENERATION.fetch_add(1, Ordering::Relaxed);
            LOADING.store(false, Ordering::Relaxed);
        })
        .ok();
}

/// Synchronous variant (tests and tools).
#[cfg(test)]
fn install(r: Rules) {
    *RULES.write().unwrap() = Arc::new(r);
}

pub fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

pub fn loading() -> bool {
    LOADING.load(Ordering::Relaxed)
}

/// Decide whether a request should be blocked.
pub fn classify(url: &str, first_party: &str, resource_type: i32) -> Option<Kind> {
    if !ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    let ty = type_bit(resource_type);
    if ty == T_DOCUMENT {
        return None; // never block top-level navigations
    }
    let lower;
    let url = if url.bytes().any(|b| b.is_ascii_uppercase()) {
        lower = url.to_ascii_lowercase();
        lower.as_str()
    } else {
        url
    };
    if !(url.starts_with("http") || url.starts_with("ws")) {
        return None;
    }
    let (hs, he) = host_range(url)?;
    let host = &url[hs..he];
    if host.is_empty() {
        return None;
    }
    let fp_host = omnibox::host_of(first_party).to_ascii_lowercase();
    if !fp_host.is_empty() && suffix_in(&ALLOWLIST.read().unwrap(), &fp_host) {
        return None;
    }
    let r = rules();
    if !fp_host.is_empty() && suffix_in(&r.doc_allow, &fp_host) {
        return None;
    }
    let third = fp_host.is_empty() || omnibox::base_domain(&fp_host) != omnibox::base_domain(host);
    let q = Req { url, host, fp: &fp_host, third, ty };
    let ads = BLOCK_ADS.load(Ordering::Relaxed);
    let trk = BLOCK_TRACKERS.load(Ordering::Relaxed);
    let wanted = |rule: &Rule| (rule.kind == Kind::Ad && ads) || (rule.kind == Kind::Tracker && trk);
    let hit = r.block.find_where(&q, wanted)?;
    if !hit.important && r.allow.find(&q).is_some() {
        return None;
    }
    Some(hit.kind)
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

/// CSS selectors to hide on pages of `host` (cosmetic filtering).
pub fn cosmetic_selectors(host: &str) -> Vec<String> {
    if !ENABLED.load(Ordering::Relaxed) || !COSMETIC.load(Ordering::Relaxed) || host.is_empty() {
        return vec![];
    }
    let host = host.to_ascii_lowercase();
    if suffix_in(&ALLOWLIST.read().unwrap(), &host) {
        return vec![];
    }
    let r = rules();
    if suffix_in(&r.elemhide, &host) || suffix_in(&r.doc_allow, &host) {
        return vec![];
    }
    let c = &r.cosmetic;
    let mut excluded: HashSet<&str> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    let mut h = host.as_str();
    loop {
        if let Some(s) = c.unhide.get(h) {
            excluded.extend(s.iter().map(|x| &**x));
        }
        match h.find('.') {
            Some(i) => h = &h[i + 1..],
            None => break,
        }
    }
    let mut h = host.as_str();
    loop {
        if let Some(sels) = c.specific.get(h) {
            out.extend(sels.iter().filter(|s| !excluded.contains(&***s)).map(|s| s.to_string()));
        }
        match h.find('.') {
            Some(i) => h = &h[i + 1..],
            None => break,
        }
    }
    if !suffix_in(&r.generichide, &host) {
        out.extend(c.generic.iter().filter(|s| !excluded.contains(&***s)).map(|s| s.to_string()));
    }
    out
}

pub fn configure(enabled: bool, ads: bool, trackers: bool, cosmetic: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
    BLOCK_ADS.store(ads, Ordering::Relaxed);
    BLOCK_TRACKERS.store(trackers, Ordering::Relaxed);
    COSMETIC.store(cosmetic, Ordering::Relaxed);
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

/// (network rules, cosmetic rules, lists loaded)
pub fn rule_count() -> (usize, usize, usize) {
    let r = rules();
    (r.block.len() + r.allow.len(), r.cosmetic.len(), r.lists_loaded)
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

    fn rs(list: &str) -> Rules {
        let mut r = Rules::default();
        parse_list(list, Category::Ads, &mut r);
        parse_list("||tracker.example^\n", Category::Privacy, &mut r);
        r.block.finish();
        r.allow.finish();
        r.cosmetic.finish();
        r
    }

    #[test]
    fn glob_semantics() {
        assert!(glob(b"ads^", b"ads/x", false, false));
        assert!(glob(b"ads^", b"ads", false, false));
        assert!(!glob(b"ads^", b"adsx", false, false));
        assert!(glob(b"/banner/*/img", b"https://x.com/banner/a/b/img.png", true, false));
        assert!(glob(b"x.js", b"a/x.js", true, true));
        assert!(!glob(b"x.js", b"a/x.js?1", true, true));
    }

    #[test]
    fn network_syntax_and_cosmetics() {
        let list = "\
||ads.example.com^
||cdn.example.net/ads/*$script,third-party
/banner/*/img^
@@||ads.example.com/allowed^
||strict.example^$important
@@||strict.example^
||only.example^$domain=news.site|~sports.news.site
@@||goodsite.org^$document
@@||nohide.org^$elemhide
###ad-box
news.site##.sponsored
sports.news.site#@#.sponsored
~nohide2.org##.promo
example.org#?#div:has-text(Ad)
||redirect.example^$redirect=noop.js
";
        install(rs(list));
        configure(true, true, true, true);
        let c = |u: &str, fp: &str, t: i32| classify(u, fp, t);
        assert_eq!(c("https://ads.example.com/x.js", "https://news.site/", 3), Some(Kind::Ad));
        assert_eq!(c("https://sub.ads.example.com/x.js", "https://news.site/", 3), Some(Kind::Ad));
        assert_eq!(c("https://ads.example.com/allowed/1", "https://news.site/", 3), None);
        assert_eq!(c("https://cdn.example.net/ads/a.js", "https://news.site/", 3), Some(Kind::Ad));
        assert_eq!(c("https://cdn.example.net/ads/a.png", "https://news.site/", 4), None, "type restricted");
        assert_eq!(c("https://cdn.example.net/ads/a.js", "https://www.example.net/", 3), None, "first party");
        assert_eq!(c("https://img.host/banner/x/img/1.gif", "https://a.b/", 4), Some(Kind::Ad));
        assert_eq!(c("https://img.host/banner/x/img.gif", "https://a.b/", 4), None, "^ does not match .");
        assert_eq!(c("https://strict.example/a", "https://a.b/", 3), Some(Kind::Ad), "important beats exception");
        assert_eq!(c("https://only.example/a", "https://news.site/", 3), Some(Kind::Ad));
        assert_eq!(c("https://only.example/a", "https://sports.news.site/", 3), None);
        assert_eq!(c("https://only.example/a", "https://other.site/", 3), None);
        assert_eq!(c("https://ads.example.com/x", "https://goodsite.org/", 3), None, "$document allow");
        assert_eq!(c("https://tracker.example/p", "https://a.b/", 4), Some(Kind::Tracker));
        assert_eq!(c("https://ads.example.com/", "", 0), None, "main frame");
        assert_eq!(c("https://redirect.example/x", "https://a.b/", 3), None, "unsupported option skipped");

        let sel = |h: &str| cosmetic_selectors(h);
        assert!(sel("news.site").contains(&".sponsored".to_string()));
        assert!(sel("news.site").contains(&"#ad-box".to_string()));
        assert!(!sel("sports.news.site").contains(&".sponsored".to_string()));
        assert!(sel("nohide.org").is_empty());
        assert!(sel("x.org").contains(&".promo".to_string()));
        assert!(!sel("nohide2.org").contains(&".promo".to_string()));
        assert!(!sel("example.org").iter().any(|s| s.contains("has-text")));
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// `OYNIX_BENCH_DIR=/path/with/lists cargo test --release bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_lists() {
        let Ok(dir) = std::env::var("OYNIX_BENCH_DIR") else { return };
        let t = std::time::Instant::now();
        let mut r = Rules::default();
        builtin(&mut r);
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let cat = if e.file_name().to_string_lossy().contains("priv") { Category::Privacy } else { Category::Ads };
            parse_list(&std::fs::read_to_string(e.path()).unwrap(), cat, &mut r);
        }
        r.block.finish();
        r.allow.finish();
        r.cosmetic.finish();
        println!("parsed in {:?}: {} block, {} allow, {} cosmetic ({} generic), {} skipped",
            t.elapsed(), r.block.len(), r.allow.len(), r.cosmetic.len(), r.cosmetic.generic.len(), r.skipped);
        install(r);
        configure(true, true, true, true);
        let urls = [
            ("https://www.googletagmanager.com/gtm.js?id=GTM-X", "https://news.example/", 3),
            ("https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js", "https://blog.example/", 3),
            ("https://cdn.jsdelivr.net/npm/vue@3/dist/vue.global.js", "https://app.example/", 3),
            ("https://www.wikipedia.org/static/images/logo.png", "https://www.wikipedia.org/", 4),
            ("https://connect.facebook.net/en_US/fbevents.js", "https://shop.example/", 3),
            ("https://example.com/assets/app.css", "https://example.com/", 2),
        ];
        for (u, fp, t) in urls {
            println!("{:?}  {u}", classify(u, fp, t));
        }
        let n = 20_000;
        let t = std::time::Instant::now();
        for i in 0..n {
            let (u, fp, ty) = urls[i % urls.len()];
            std::hint::black_box(classify(u, fp, ty));
        }
        println!("{:.2} µs per request", t.elapsed().as_secs_f64() * 1e6 / n as f64);
        let t = std::time::Instant::now();
        let s = cosmetic_selectors("www.theguardian.com");
        println!("cosmetic for theguardian: {} selectors in {:?}", s.len(), t.elapsed());
    }
}
