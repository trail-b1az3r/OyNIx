#!/usr/bin/env python3
"""Build the OyNIx website and Extension Gallery into _site/ (GitHub Pages).

    python3 scripts/build-site.py [--out _site]

Sources
  site/pages/*.html        page bodies with a small header comment
  site/pages/docs/*.html   documentation chapters (order from the header)
  site/assets/             CSS, JS, screenshots (copied as they are)
  gallery/<id>/            OyNIx extensions: oynix.json + code.ts + content.css,
                           optionally graphics.qt, extra.html, extra.jsonl
  qml/Icon.qml, qml/MShape.qml   icon paths and shape geometry, so the site
                           uses exactly the browser's icons and shapes

Page bodies may use:
  {{root}}                 relative path to the site root
  {{version}}              OyNIx version from Cargo.toml
  {{icon:name}}            inline SVG icon from the browser's icon set
  {{shape:name:icon}}      an expressive shape with an icon in it
  {{shot:file|alt}}        a screenshot in a window frame (uses file-dark.png
                           in dark mode when it exists)
Standard library only.
"""

import argparse, hashlib, html, json, math, os, re, shutil, sys, zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
GALLERY = ROOT / "gallery"
REPO = "https://github.com/trail-b1az3r/OyNIx"


def version():
    m = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M)
    return m.group(1) if m else "0.0.0"


# ── Browser icons and shapes ─────────────────────────────────────────────

def load_icons():
    src = (ROOT / "qml/Icon.qml").read_text()
    return dict(re.findall(r'^\s*"([A-Za-z]+)":\s*"([^"]+)"', src, re.M))


def load_shapes():
    src = (ROOT / "qml/MShape.qml").read_text()
    return {k: (int(n), float(a)) for k, n, a in re.findall(r'"([A-Za-z0-9]+)":\s*\[(\d+),\s*([\d.]+)\]', src)}


def shape_path(n, a, size=100.0, steps=160):
    """Same formula as MShape.pathFor(): r = R(1 + a·cos(nθ))."""
    R = size / 2 / (1 + abs(a))
    c = size / 2
    pts = []
    for i in range(steps):
        th = i / steps * math.pi * 2
        r = R * (1 + a * math.cos(n * th))
        pts.append(f"{c + r * math.cos(th):.2f} {c + r * math.sin(th):.2f}")
    return "M" + "L".join(pts) + "Z"


def svg_data_uri(svg):
    return "url(\"data:image/svg+xml," + svg.replace('"', "'").replace("#", "%23").replace("<", "%3C").replace(">", "%3E") + "\")"


def shapes_css(shapes, icons):
    lines = [":root {"]
    for name, (n, a) in shapes.items():
        d = shape_path(n, a) if n else "M50 0a50 50 0 1 0 .01 0Z"
        lines.append(f"  --{name}: {svg_data_uri(f'<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><path d=\"{d}\"/></svg>')};")
    check = icons.get("check", "M5 12.5l4.5 4.5L19 7.5")
    lines.append(f"  --check: {svg_data_uri(f'<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"black\" stroke-width=\"2.4\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"{check}\"/></svg>')};")
    lines.append("}")
    for name in shapes:
        lines.append(f".s-{name} {{ --mask: var(--{name}); }}")
    return "\n".join(lines) + "\n"


# ── Templating ───────────────────────────────────────────────────────────

class Site:
    def __init__(self, out):
        self.out = out
        self.version = version()
        self.icons = load_icons()
        self.shapes = load_shapes()

    def icon(self, name, cls="icon"):
        d = self.icons.get(name)
        if d is None:
            sys.exit(f"unknown icon '{name}'")
        return f'<svg class="{cls}" viewBox="0 0 24 24" aria-hidden="true"><path d="{d}"/></svg>'

    def shape(self, name, inner="", extra="", style=""):
        if name not in self.shapes:
            sys.exit(f"unknown shape '{name}'")
        st = f' style="{style}"' if style else ""
        return f'<span class="shape s-{name} {extra}"{st}>{inner}</span>'

    def shot(self, spec, root):
        file, _, alt = spec.partition("|")
        base = SITE / "assets/shots" / file
        if not base.exists():
            sys.exit(f"missing screenshot {base}")
        dark = base.with_name(base.stem + "-dark" + base.suffix)
        src = f"{root}assets/shots/{file}"
        w, h = image_size(base)
        img = f'<img src="{src}" alt="{html.escape(alt)}" width="{w // 2}" height="{h // 2}" loading="lazy" decoding="async">'
        if dark.exists():
            img = f'<picture><source srcset="{root}assets/shots/{dark.name}" media="(prefers-color-scheme: dark)">{img}</picture>'
        return f'<figure class="window" style="margin:0">{img}</figure>'

    def expand(self, text, root):
        def rep(m):
            kind, _, arg = m.group(1).partition(":")
            if kind == "root":
                return root
            if kind == "version":
                return self.version
            if kind == "repo":
                return REPO
            if kind == "icon":
                return self.icon(arg)
            if kind == "shape":
                name, _, ic = arg.partition(":")
                return self.shape(name, self.icon(ic) if ic else "")
            if kind == "shot":
                return self.shot(arg, root)
            sys.exit(f"unknown placeholder {m.group(0)}")
        return re.sub(r"\{\{([a-z]+(?::[^}]*)?)\}\}", rep, text)

    def layout(self, path, title, body, description, current, addr, root=None):
        depth = path.count("/")
        root = "../" * depth if root is None else root
        nav = [("", "Overview"), ("docs/", "Docs"), ("extensions/", "Extensions"), ("#download", "Download")]
        links = []
        for href, label in nav:
            target = (root + href) if not href.startswith("#") else (root + "index.html" + href if depth else href)
            cur = ' aria-current="page"' if current == href else ""
            links.append(f'<a href="{target or "./"}"{cur}>{label}</a>')
        full_title = "OyNIx" if title == "OyNIx" else f"{title} · OyNIx"
        page = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(full_title)}</title>
<meta name="description" content="{html.escape(description)}">
<meta name="color-scheme" content="light dark">
<meta name="theme-color" content="#ededf5" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#1e1f25" media="(prefers-color-scheme: dark)">
<meta property="og:title" content="{html.escape(full_title)}">
<meta property="og:description" content="{html.escape(description)}">
<meta property="og:image" content="{root}assets/og.png">
<link rel="icon" href="{root}assets/icon.svg" type="image/svg+xml">
<link rel="preload" href="{root}assets/fonts/RobotoFlex-latin-100-1000.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="{root}assets/fonts.css">
<link rel="stylesheet" href="{root}assets/site.css">
<link rel="stylesheet" href="{root}assets/shapes.css">
</head>
<body>
<header class="topbar">
  <div class="topbar-inner">
    <a class="brand" href="{root or './'}"><img src="{root}assets/icon.svg" alt="" width="32" height="32">OyNIx</a>
    <div class="addr" aria-hidden="true">{self.icon("lock")}{addr}</div>
    <button class="menu-btn" aria-label="Menu" aria-expanded="false">{self.icon("list")}</button>
    <nav class="tabs" aria-label="Site">{"".join(links)}</nav>
  </div>
</header>
<main>
{self.expand(body, root)}
</main>
<footer>
  <div class="wrap">
    <span>OyNIx {self.version}</span>
    <a href="{REPO}">Source code</a>
    <a href="{REPO}/releases">Releases</a>
    <a href="{REPO}/issues">Report a problem</a>
    <span class="spacer"></span>
    <span>No cookies, no analytics. This site doesn’t track you either.</span>
  </div>
</footer>
<script src="{root}assets/site.js" defer></script>
</body>
</html>
"""
        dest = self.out / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(page)


def image_size(p):
    """Pixel size of a PNG or WebP file."""
    b = Path(p).read_bytes()[:40]
    if b[:8] == b"\x89PNG\r\n\x1a\n":
        return int.from_bytes(b[16:20], "big"), int.from_bytes(b[20:24], "big")
    if b[:4] == b"RIFF" and b[8:12] == b"WEBP":
        kind = b[12:16]
        if kind == b"VP8X":
            return 1 + int.from_bytes(b[24:27], "little"), 1 + int.from_bytes(b[27:30], "little")
        if kind == b"VP8 ":
            return int.from_bytes(b[26:28], "little") & 0x3FFF, int.from_bytes(b[28:30], "little") & 0x3FFF
        if kind == b"VP8L":
            n = int.from_bytes(b[21:25], "little")
            return (n & 0x3FFF) + 1, ((n >> 14) & 0x3FFF) + 1
    sys.exit(f"unsupported image {p}")


def parse_page(p):
    text = p.read_text()
    m = re.match(r"<!--(.*?)-->\s*", text, re.S)
    meta = {}
    if m:
        for line in m.group(1).strip().splitlines():
            k, _, v = line.partition(":")
            meta[k.strip()] = v.strip()
        text = text[m.end():]
    return meta, text


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", re.sub(r"<[^>]+>", "", s).lower()).strip("-")


# ── Docs ─────────────────────────────────────────────────────────────────

def build_docs(site):
    chapters = []
    for p in sorted((SITE / "pages/docs").glob("*.html")):
        meta, body = parse_page(p)
        chapters.append((int(meta.get("order", 99)), p.stem, meta, body))
    chapters.sort()
    for i, (_, stem, meta, body) in enumerate(chapters):
        path = "docs/index.html" if stem == "index" else f"docs/{stem}/index.html"
        root = "../" * path.count("/")
        heads = []

        def add_id(m):
            title = m.group(2)
            hid = slug(title)
            heads.append((hid, re.sub(r"<[^>]+>", "", title)))
            return f'<h2{m.group(1)} id="{hid}">{title}</h2>'
        body = re.sub(r"<h2([^>]*)>(.*?)</h2>", add_id, body)

        rail = []
        for _, s2, m2, _ in chapters:
            href = root + ("docs/" if s2 == "index" else f"docs/{s2}/")
            cur = ' aria-current="page"' if s2 == stem else ""
            rail.append(f'<a href="{href}"{cur}><span class="pill">{site.icon(m2.get("icon", "list"))}</span>{html.escape(m2.get("short", m2["title"]))}</a>')
        toc = "".join(f'<a href="#{h}">{html.escape(t)}</a>' for h, t in heads)
        nxt = []
        if i > 0:
            _, s0, m0, _ = chapters[i - 1]
            nxt.append(f'<a href="{root}docs/{"" if s0 == "index" else s0 + "/"}"><small>Previous</small>{html.escape(m0["title"])}</a>')
        if i + 1 < len(chapters):
            _, s1, m1, _ = chapters[i + 1]
            nxt.append(f'<a class="fwd" href="{root}docs/{s1}/"><small>Next</small>{html.escape(m1["title"])}</a>')
        page = f"""<div class="docs">
<nav class="rail" aria-label="Documentation">{"".join(rail)}</nav>
<article class="doc">
<h1>{html.escape(meta["title"])}</h1>
<p class="lede">{meta.get("lede", "")}</p>
{body}
<nav class="next">{"".join(nxt)}</nav>
</article>
<aside class="toc">{"<b>On this page</b>" + toc if heads else ""}</aside>
</div>"""
        addr = "<b>oynix</b> / docs" + ("" if stem == "index" else f" / {stem}")
        site.layout(path, meta["title"], page, meta.get("description", meta.get("lede", "")), "docs/", addr)
    return len(chapters)


# ── Extension Gallery ────────────────────────────────────────────────────

def zip_dir(src, dest):
    """Deterministic zip: fixed timestamps and order, so unchanged
    extensions keep the same checksum between builds."""
    with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as z:
        for f in sorted(src.iterdir()):
            if f.is_file():
                info = zipfile.ZipInfo(f.name, date_time=(2026, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = 0o644 << 16
                z.writestr(info, f.read_bytes())


def build_gallery(site):
    out = site.out / "extensions"
    (out / "packages").mkdir(parents=True, exist_ok=True)
    items = []
    for d in sorted(p for p in GALLERY.iterdir() if p.is_dir()):
        meta = json.loads((d / "oynix.json").read_text())
        for f in ("code.ts", "content.css"):
            if not (d / f).exists():
                sys.exit(f"{d.name}: {f} is missing")
        allowed = {"oynix.json", "code.ts", "content.css", "graphics.qt", "extra.html", "extra.jsonl", "icon.png", "icon.svg"}
        extra_files = sorted(f.name for f in d.iterdir() if f.is_file() and f.name not in allowed)
        if extra_files:
            sys.exit(f"{d.name}: unexpected files {extra_files} (allowed: {sorted(allowed)})")
        if (d / "extra.jsonl").exists():
            for n, line in enumerate((d / "extra.jsonl").read_text().splitlines(), 1):
                if line.strip():
                    try:
                        json.loads(line)
                    except ValueError as e:
                        sys.exit(f"{d.name}: extra.jsonl:{n}: {e}")
        g = meta.get("gallery", {})
        file = f'{meta["name"].replace(" ", "-")}-{meta["version"]}.oynix'
        pkg = out / "packages" / file
        zip_dir(d, pkg)
        data = pkg.read_bytes()
        items.append({
            "id": d.name, "name": meta["name"], "version": meta["version"],
            "description": meta.get("description", ""), "summary": g.get("summary", ""),
            "about": g.get("about", ""), "category": g.get("category", "Other"),
            "author": g.get("author", ""), "shape": g.get("shape", "cookie9"),
            "matches": meta.get("matches", []), "run_at": meta.get("run_at", "document_end"),
            "world": meta.get("world", "isolated"),
            "package": f"packages/{file}", "size": len(data), "sha256": hashlib.sha256(data).hexdigest(),
            "ts": (d / "code.ts").read_text(), "css": (d / "content.css").read_text(),
            "extras": {f: (d / f).read_text() for f in ("graphics.qt", "extra.html", "extra.jsonl") if (d / f).exists()},
        })

    tones = [("var(--primary-container)", "var(--on-primary-container)"), ("var(--tertiary-container)", "var(--on-tertiary-container)"),
             ("var(--secondary-container)", "var(--on-secondary-container)"), ("var(--surface-highest)", "var(--on-surface)")]

    def tone(i):
        f, k = tones[i % len(tones)]
        return f"--fill:{f};--ink:{k}"

    # Machine-readable index (for tools, and future in-browser updates).
    index = [{k: v for k, v in it.items() if k not in ("ts", "css", "about", "extras")} | {"files": ["oynix.json", "code.ts", "content.css", *it["extras"]]} for it in items]
    (out / "index.json").write_text(json.dumps({"version": 1, "extensions": index}, indent=2))

    cats = sorted({it["category"] for it in items})
    cards = []
    for i, it in enumerate(items):
        text = html.escape(" ".join([it["name"], it["summary"], it["description"], it["category"]]).lower())
        cards.append(f'''<a class="ext" href="{it["id"]}/" data-category="{html.escape(it["category"])}" data-text="{text}">
  {site.shape(it["shape"], f'<span class="monogram">{html.escape(it["name"][0])}</span>', style=tone(i))}
  <div><b>{html.escape(it["name"])}</b><span>{html.escape(it["description"])}</span><small>{html.escape(it["category"])} · v{html.escape(it["version"])}</small></div>
</a>''')
    chips = "".join(f'<button class="chip" type="button" aria-pressed="false" data-category="{html.escape(c)}">{html.escape(c)}</button>' for c in cats)
    body = f'''<section class="wrap" style="padding-top:56px">
  <div class="gallery-head">
    <div>
      <span class="eyebrow">Extension Gallery</span>
      <h1 style="font-size:clamp(40px,5.4vw,68px)">Small tools,<br>short enough to read.</h1>
      <p class="lede" style="margin-top:22px">Every extension here is a few dozen lines of TypeScript and CSS that you can read on its own page before installing. They run only on the pages they say they run on, and never send anything anywhere.</p>
    </div>
    <label class="search">{site.icon("search")}<input type="search" placeholder="Search extensions" aria-label="Search extensions"></label>
  </div>
  <div class="chips" role="group" aria-label="Category">{chips}</div>
  <div class="grid" data-filter>{"".join(cards)}</div>
  <p class="empty" hidden>Nothing matches. Try a shorter word, or clear the category.</p>
  <div class="chapter tone-plain" style="margin-top:72px">
    <div class="chapter-text">
      <h2>Looking for something bigger?</h2>
      <p>OyNIx also runs Chrome and Firefox extensions. Open <a href="https://addons.mozilla.org/firefox/extensions/">addons.mozilla.org</a> in OyNIx and every add-on page gets an <b>Add to OyNIx</b> button; Chrome Web Store downloads and unpacked folders install from Settings › Extensions.</p>
      <a class="btn text" href="../docs/extensions/">How extensions work in OyNIx {site.icon("chevronRight")}</a>
    </div>
    <div class="chapter-text">
      <h2>Write your own</h2>
      <p>An OyNIx extension is three files, <code>oynix.json</code>, <code>code.ts</code> and <code>content.css</code>, plus an optional panel or page and data. Settings › Extensions › New Extension makes a template; when it works, send a pull request adding it to <a href="{REPO}/tree/main/gallery"><code>gallery/</code></a>.</p>
      <a class="btn text" href="../docs/build-an-extension/">Build an extension {site.icon("chevronRight")}</a>
    </div>
  </div>
</section>'''
    site.layout("extensions/index.html", "Extension Gallery", body,
                "Small, readable OyNIx extensions you can install with one click.", "extensions/", "<b>oynix</b> / extensions")

    for i, it in enumerate(items):
        about = "".join(f"<p>{html.escape(p)}</p>" for p in it["about"].split("\n\n") if p)
        matches = ", ".join(html.escape(m) for m in it["matches"]) or "—"
        when = {"document_start": "Before the page loads", "document_end": "When the page has loaded", "document_idle": "Once the page is idle"}.get(it["run_at"], it["run_at"])
        ts_lines = it["ts"].count("\n") + 1
        extras_html = "".join(f'<details class="source"><summary>{html.escape(f)}</summary><pre><code>{html.escape(t)}</code></pre></details>' for f, t in it["extras"].items())
        panel = "graphics.qt (panel)" if "graphics.qt" in it["extras"] else ("extra.html (page)" if "extra.html" in it["extras"] else "None")
        all_sites = any(m in ("*://*/*", "<all_urls>") for m in it["matches"])
        body = f'''<div class="wrap ext-page">
  <article>
    <div class="ext-hero">
      {site.shape(it["shape"], f'<span class="monogram">{html.escape(it["name"][0])}</span>', style=tone(i))}
      <div><h1>{html.escape(it["name"])}</h1><p>{html.escape(it["summary"])}</p></div>
    </div>
    <div class="doc" style="max-width:none">
      {about}
      <h2 style="font-size:26px;margin:44px 0 14px">Read the code</h2>
      <p>This is the whole extension. OyNIx shows it to the page exactly as written here.</p>
      <details class="source" open><summary>code.ts · {ts_lines} lines</summary><pre><code>{html.escape(it["ts"])}</code></pre></details>
      <details class="source"><summary>content.css</summary><pre><code>{html.escape(it["css"])}</code></pre></details>
      {extras_html}
    </div>
  </article>
  <aside class="facts">
    <a class="btn only-oynix" href="../{it["package"]}" download>{site.icon("plus")}Add to OyNIx</a>
    <a class="btn not-oynix" href="../{it["package"]}" download>{site.icon("download")}Download</a>
    <p class="hint not-oynix">Open this page in OyNIx to install with one click, or run<br><code>oynix --install-extension {html.escape(it["package"].split("/")[-1])}</code></p>
    <p class="hint only-oynix">OyNIx will ask before installing. Remove it any time in Settings › Extensions.</p>
    <dl>
      <dt>Version</dt><dd>{html.escape(it["version"])}</dd>
      <dt>Category</dt><dd>{html.escape(it["category"])}</dd>
      <dt>Runs on</dt><dd>{"Every website" if all_sites else matches}</dd>
      <dt>Starts</dt><dd>{when}</dd>
      <dt>Network</dt><dd>None</dd>
      <dt>Toolbar</dt><dd>{panel}</dd>
      <dt>Size</dt><dd>{it["size"]:,} bytes</dd>
      <dt>SHA-256</dt><dd><code style="font-size:11px">{it["sha256"]}</code></dd>
    </dl>
  </aside>
</div>'''
        site.layout(f'extensions/{it["id"]}/index.html', it["name"], body, it["description"], "extensions/",
                    f'<b>oynix</b> / extensions / {it["id"]}')
    return len(items)


# ── Main ─────────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "_site"))
    out = Path(ap.parse_args().out)
    if out.exists():
        shutil.rmtree(out)
    site = Site(out)
    shutil.copytree(SITE / "assets", out / "assets")
    shutil.copy(ROOT / "assets/icon.svg", out / "assets/icon.svg")
    (out / "assets/shapes.css").write_text(shapes_css(site.shapes, site.icons))

    for p in sorted((SITE / "pages").glob("*.html")):
        meta, body = parse_page(p)
        path = "index.html" if p.stem == "index" else f"{p.stem}/index.html"
        site.layout(path, meta.get("title", "OyNIx"), body, meta.get("description", ""), meta.get("nav", ""), meta.get("addr", "<b>oynix</b>"))
    n_docs = build_docs(site)
    n_ext = build_gallery(site)
    (out / ".nojekyll").write_text("")
    # GitHub Pages serves 404.html at any depth, so it links absolutely.
    site.layout("404.html", "Page not found", """<section class="wrap" style="padding:96px 0 64px">
  {{shape:cookie4:globe}}
  <h1 style="margin:28px 0 20px">Can’t find this page</h1>
  <p class="lede">The address may be mistyped, or the page moved when the docs were reorganised.</p>
  <div class="btn-row" style="margin-top:28px"><a class="btn" href="{{root}}">Go to the start</a><a class="btn tonal" href="{{root}}docs/">Documentation</a></div>
</section>""", "Page not found.", "", "<b>oynix</b> / ?", root="/OyNIx/")
    print(f"built {out}: {n_docs} docs pages, {n_ext} extensions, version {site.version}")


if __name__ == "__main__":
    main()
