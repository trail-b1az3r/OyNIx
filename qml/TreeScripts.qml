pragma Singleton
import QtQuick

// Page scripts for tree tabs (run in the isolated world, so pages can't see
// or interfere with them).
QtObject {
    // What kind of document this is, and its outline.
    // Returns {kind: "pdf"} | {kind: "md", raw, items: [{level, title, line|index}]} | {kind: ""}.
    readonly property string outline: `(function () {
  var ct = (document.contentType || '').toLowerCase();
  var path = location.pathname.toLowerCase();
  if (ct === 'application/pdf' || /\\.pdf$/.test(path) && document.querySelector('embed[type=\"application/pdf\"]')) return { kind: 'pdf' };
  var md = /markdown/.test(ct) || /\\.(md|markdown|mdown|mkd|mkdn)$/.test(path);
  if (!md) return { kind: '' };
  var items = [];
  var body = document.body;
  var pre = body && body.children.length === 1 && body.firstElementChild.tagName === 'PRE' ? body.firstElementChild : null;
  var headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
  if (pre || (ct.indexOf('text/') === 0 && ct !== 'text/html' && !headings.length)) {
    // Raw Markdown shown as text: read # headings (and underlined ones).
    var lines = (pre || body).textContent.split('\\n'), fence = false;
    for (var i = 0; i < lines.length && items.length < 500; i++) {
      var l = lines[i];
      if (/^\\s{0,3}(\\x60{3}|~~~)/.test(l)) { fence = !fence; continue; }
      if (fence) continue;
      var m = /^\\s{0,3}(#{1,6})\\s+(.+?)\\s*#*\\s*$/.exec(l);
      if (m) { items.push({ level: m[1].length, title: m[2], line: i }); continue; }
      var next = lines[i + 1] || '';
      if (l.trim() && !/^\\s*[-*>|]/.test(l) && /^\\s{0,3}(=+|-+)\\s*$/.test(next) && next.trim().length >= 2)
        items.push({ level: next.trim()[0] === '=' ? 1 : 2, title: l.trim(), line: i });
    }
    return { kind: 'md', raw: true, items: items };
  }
  for (var j = 0; j < headings.length && items.length < 500; j++) {
    var t = headings[j].textContent.replace(/\\s+/g, ' ').trim();
    if (t) items.push({ level: +headings[j].tagName[1], title: t, index: j });
  }
  return { kind: 'md', raw: false, items: items };
})()`

    // Shared by scrollTo and active: where a heading is on the page.
    readonly property string locate: `function __oynixLocate(t) {
  if (!t.raw) {
    var h = document.querySelectorAll('h1, h2, h3, h4, h5, h6')[t.index];
    return h ? h.getBoundingClientRect().top : null;
  }
  var body = document.body, root = body.children.length === 1 ? body.firstElementChild : body;
  var text = root.textContent, offset = 0;
  for (var n = 0, k = 0; n < t.line; n++) { k = text.indexOf('\\n', offset); if (k < 0) return null; offset = k + 1; }
  var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT), node, seen = 0;
  while ((node = walker.nextNode())) {
    if (seen + node.length > offset) {
      var r = document.createRange();
      r.setStart(node, offset - seen);
      r.setEnd(node, Math.min(node.length, offset - seen + 1));
      var rect = r.getClientRects()[0] || r.getBoundingClientRect();
      return rect.top;
    }
    seen += node.length;
  }
  return null;
}`

    // Scroll so heading `t` ({raw, line|index}) is at the top.
    function scrollTo(t) {
        return "(function () {" + locate + "\nvar top = __oynixLocate(" + JSON.stringify(t) + ");"
            + "if (top !== null) window.scrollTo({ top: top + window.scrollY - 12, behavior: 'smooth' }); })()"
    }

    // Index (into items) of the section being read, or -1.
    function active(items, raw) {
        var targets = items.map(function (it) { return { raw: raw, line: it.line, index: it.index } })
        return "(function () {" + locate + "\nvar ts = " + JSON.stringify(targets) + ", cur = -1;"
            + "for (var i = 0; i < ts.length; i++) { var top = __oynixLocate(ts[i]); if (top === null) continue; if (top <= 80) cur = i; else break; }"
            + "return cur; })()"
    }
}
