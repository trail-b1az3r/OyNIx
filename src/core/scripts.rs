//! JavaScript injected into pages: reader mode, privacy shims and helpers.
//! All of it is original code written for OyNIx.

/// Reader mode: score candidate containers by paragraph text density, keep
/// the best one, and re-render it in a clean typographic layout. Running the
/// script a second time restores the original page.
pub const READER: &str = r#"
(function () {
  if (window.__oynixReader) {
    document.documentElement.innerHTML = window.__oynixReader.html;
    window.scrollTo(0, window.__oynixReader.scroll);
    delete window.__oynixReader;
    return false;
  }
  function textLen(el) { return (el.innerText || '').trim().length; }
  var best = null, bestScore = 0;
  var candidates = document.querySelectorAll('article, main, [role=main], section, div');
  for (var i = 0; i < candidates.length; i++) {
    var el = candidates[i];
    var ps = el.querySelectorAll(':scope > p, :scope > div > p, :scope > section > p');
    if (ps.length < 2) continue;
    var score = 0;
    for (var j = 0; j < ps.length; j++) score += Math.min(textLen(ps[j]), 600);
    var links = el.querySelectorAll('a').length;
    score -= links * 12;
    if (/comment|sidebar|footer|nav|promo|related|share|advert/i.test(el.className + ' ' + el.id)) score *= 0.3;
    if (el.tagName === 'ARTICLE' || el.getAttribute('role') === 'main') score *= 1.4;
    if (score > bestScore) { bestScore = score; best = el; }
  }
  if (!best || bestScore < 400) return null;
  var title = (document.querySelector('h1') || {}).innerText || document.title;
  var byline = '';
  var meta = document.querySelector('meta[name=author]');
  if (meta) byline = meta.content;
  var clone = best.cloneNode(true);
  clone.querySelectorAll('script, style, iframe, form, button, input, nav, aside, footer, noscript, svg, [aria-hidden=true], .ad, .ads, .advert, .share, .social').forEach(function (n) { n.remove(); });
  clone.querySelectorAll('*').forEach(function (n) {
    for (var a = n.attributes.length - 1; a >= 0; a--) {
      var name = n.attributes[a].name;
      if (!/^(href|src|srcset|alt|title|colspan|rowspan)$/.test(name)) n.removeAttribute(name);
    }
  });
  var dupe = clone.querySelector('h1');
  if (dupe && dupe.textContent.trim() === String(title).trim()) dupe.remove();
  window.__oynixReader = { html: document.documentElement.innerHTML, scroll: window.scrollY };
  var dark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
  var css = 'html{background:' + (dark ? '#1c1c1e' : '#fbfaf7') + ';color:' + (dark ? '#e5e5ea' : '#1d1d1f') + '}' +
    'body{max-width:42em;margin:0 auto;padding:56px 28px 120px;font:20px/1.65 Charter,"Iowan Old Style",Georgia,serif;}' +
    'h1{font:700 2.0em/1.15 -apple-system,"Inter","Segoe UI",sans-serif;margin:0 0 .3em}' +
    '.oy-byline{font:15px -apple-system,"Inter",sans-serif;opacity:.6;margin-bottom:2.4em}' +
    'img,video,figure{max-width:100%;height:auto;border-radius:8px}figure{margin:1.5em 0}' +
    'figcaption{font-size:.75em;opacity:.65}a{color:' + (dark ? '#64a8ff' : '#0a5dd6') + '}' +
    'pre,code{font-size:.8em;background:rgba(127,127,127,.12);border-radius:6px}pre{padding:12px;overflow:auto}' +
    'blockquote{border-left:3px solid rgba(127,127,127,.4);margin-left:0;padding-left:1em;opacity:.85}';
  document.documentElement.innerHTML = '<head><meta charset="utf-8"><title></title><style>' + css + '</style></head><body></body>';
  document.title = title;
  var h = document.createElement('h1'); h.textContent = title; document.body.appendChild(h);
  var b = document.createElement('div'); b.className = 'oy-byline';
  b.textContent = [byline, location.hostname.replace(/^www\./, '')].filter(Boolean).join(' · ');
  document.body.appendChild(b);
  document.body.appendChild(clone);
  window.scrollTo(0, 0);
  return true;
})();
"#;

/// Is the current page likely to have a readable article?
pub const READER_PROBE: &str = r#"
(function () {
  var ps = document.querySelectorAll('article p, main p, p');
  var n = 0;
  for (var i = 0; i < ps.length && i < 400; i++) if ((ps[i].innerText || '').length > 140) n++;
  return n >= 4;
})();
"#;

/// Global Privacy Control signal (navigator.globalPrivacyControl).
pub const GPC: &str = r#"
try { Object.defineProperty(Navigator.prototype, 'globalPrivacyControl', { get: function () { return true; }, configurable: true }); } catch (e) {}
"#;

/// Reduce passive fingerprinting surface without breaking sites:
/// normalise high-entropy navigator values and add per-session noise to
/// canvas read-back.
pub const ANTI_FINGERPRINT: &str = r#"
(function () {
  try {
    var def = function (o, k, v) { Object.defineProperty(o, k, { get: function () { return v; }, configurable: true }); };
    def(Navigator.prototype, 'hardwareConcurrency', 4);
    if ('deviceMemory' in Navigator.prototype) def(Navigator.prototype, 'deviceMemory', 8);
    def(Navigator.prototype, 'plugins', []);
    var seed = Math.floor(Math.random() * 255);
    var toDataURL = HTMLCanvasElement.prototype.toDataURL;
    HTMLCanvasElement.prototype.toDataURL = function () {
      try {
        var ctx = this.getContext('2d');
        if (ctx && this.width && this.height) {
          var px = ctx.getImageData(0, 0, 1, 1);
          px.data[0] = (px.data[0] + seed) & 255;
          ctx.putImageData(px, 0, 0);
        }
      } catch (e) {}
      return toDataURL.apply(this, arguments);
    };
  } catch (e) {}
})();
"#;

/// Cosmetic filtering: a function taking an array of CSS selectors and
/// hiding matching elements through one stylesheet. Selectors are inserted
/// in chunks; a chunk with an invalid selector falls back to one-by-one so a
/// single bad rule cannot disable the rest.
pub const COSMETIC: &str = r#"function (sel) {
  if (window.__oynixCosmetic) return;
  window.__oynixCosmetic = true;
  function apply() {
    var root = document.documentElement;
    if (!root) return false;
    var st = document.createElement('style');
    st.setAttribute('data-oynix', 'cosmetic');
    root.appendChild(st);
    var sheet = st.sheet;
    if (!sheet) return true;
    var add = function (s) { sheet.insertRule(s + '{display:none!important}', sheet.cssRules.length); };
    for (var i = 0; i < sel.length; i += 200) {
      var chunk = sel.slice(i, i + 200);
      try { add(chunk.join(',')); }
      catch (e) { for (var j = 0; j < chunk.length; j++) { try { add(chunk[j]); } catch (e2) {} } }
    }
    return true;
  }
  if (!apply()) {
    var mo = new MutationObserver(function () { if (apply()) mo.disconnect(); });
    mo.observe(document, { childList: true });
  }
}"#;
