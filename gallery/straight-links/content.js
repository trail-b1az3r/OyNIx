// Straight Links: unwrap known redirect URLs to their destination.
// Each rule: [host test, path test, query parameter holding the target].
var RULES = [
  [/(^|\.)google\.[a-z.]+$/, /^\/url$/, ['q', 'url']],
  [/(^|\.)bing\.com$/, /^\/ck\/a$/, ['u']],
  [/(^|\.)duckduckgo\.com$/, /^\/l\/$/, ['uddg']],
  [/(^|\.)youtube\.com$/, /^\/redirect$/, ['q']],
  [/^l\.facebook\.com$|^lm\.facebook\.com$/, /^\/l\.php$/, ['u']],
  [/^l\.instagram\.com$/, /^\/$/, ['u']],
  [/^out\.reddit\.com$/, /./, ['url']],
  [/^steamcommunity\.com$/, /^\/linkfilter\/$/, ['url', 'u']],
  [/(^|\.)linkedin\.com$/, /^\/redir\/redirect/, ['url']],
  [/^slack-redir\.net$/, /^\/link$/, ['url']],
  [/safelinks\.protection\.outlook\.com$/, /./, ['url']]
];

function target(href) {
  var u;
  try { u = new URL(href, location.href); } catch (e) { return null; }
  for (var i = 0; i < RULES.length; i++) {
    var r = RULES[i];
    if (!r[0].test(u.hostname) || !r[1].test(u.pathname)) continue;
    for (var j = 0; j < r[2].length; j++) {
      var v = u.searchParams.get(r[2][j]);
      if (v) {
        // Bing encodes the target as "a1" + base64.
        if (r[2][j] === 'u' && /^a1/.test(v)) {
          try { v = atob(v.slice(2).replace(/-/g, '+').replace(/_/g, '/')); } catch (e) { continue; }
        }
        if (/^https?:\/\//i.test(v)) return v;
      }
    }
  }
  return null;
}

function fix(a) {
  if (a.dataset.oynixStraight) return;
  a.dataset.oynixStraight = '1';
  a.removeAttribute('ping');
  var t = target(a.href);
  if (t) a.href = t;
}

// Fix links when they are about to be used, so links added later by the
// page are covered without scanning the whole document repeatedly.
['pointerover', 'focusin', 'mousedown', 'touchstart'].forEach(function (type) {
  document.addEventListener(type, function (e) {
    var a = e.target.closest && e.target.closest('a[href]');
    if (a) fix(a);
  }, { capture: true, passive: true });
});
document.querySelectorAll('a[href]').forEach(fix);
