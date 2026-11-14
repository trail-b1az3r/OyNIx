// Scraper: collects text, images, links and tables from the page you're
// on, and reads text with OCR where there is no text to collect. The
// panel (graphics.qt) sends commands; results go back to it, and Save
// downloads them as a file.

declare const oynix: {
  post(message: unknown): Promise<void>;
  onMessage(listener: (message: any) => void): void;
  ocr(rect?: { x: number; y: number; width: number; height: number }): Promise<string>;
};

interface Result {
  title: string;     // shown above the output
  summary: string;   // one line: what was found
  text: string;      // what the panel shows (and copies)
  file: string;      // file name for Save
  mime: string;
  data: string;      // what Save writes
}

let last: Result | null = null;
const PREVIEW_LIMIT = 60000;

function send(r: Result): void {
  last = r;
  const text = r.text.length > PREVIEW_LIMIT ? r.text.slice(0, PREVIEW_LIMIT) + '\n\n… (shortened here; Save writes everything)' : r.text;
  oynix.post({ type: 'result', title: r.title, summary: r.summary, text, canSave: r.data.length > 0 });
}

function report(message: string): void {
  oynix.post({ type: 'status', message });
}

const host = location.hostname.replace(/^www\./, '') || 'page';
const stamp = new Date().toISOString().slice(0, 10);
const fileName = (what: string, ext: string) => `${host}-${what}-${stamp}.${ext}`;

function visible(el: Element): boolean {
  const s = getComputedStyle(el);
  if (s.display === 'none' || s.visibility === 'hidden' || s.opacity === '0') return false;
  const r = el.getBoundingClientRect();
  return r.width > 0 || r.height > 0;
}

// ── Text ──────────────────────────────────────────────────────────────

const SKIP = 'script, style, noscript, template, svg, canvas, nav, aside, footer, form, button, [aria-hidden="true"], [role="navigation"], [role="banner"], [role="contentinfo"]';

/** The element most likely to hold the page's main text. */
function mainElement(): Element {
  const marked = document.querySelector('article, main, [role="main"]');
  if (marked && (marked.textContent || '').trim().length > 400) return marked;
  let best: Element = document.body, bestScore = 0;
  document.querySelectorAll('div, section').forEach((el) => {
    let score = 0;
    el.querySelectorAll(':scope > p').forEach((p) => { score += Math.min((p.textContent || '').length, 500); });
    if (score > bestScore) { best = el; bestScore = score; }
  });
  return bestScore > 600 ? best : document.body;
}

/** Text with light structure: headings, lists and quotes keep a marker. */
function structuredText(root: Element): string {
  const out: string[] = [];
  const walk = (el: Element) => {
    if (el.matches(SKIP) || !visible(el)) return;
    const tag = el.tagName;
    const line = () => (el as HTMLElement).innerText.replace(/\s+\n/g, '\n').replace(/[ \t]+/g, ' ').trim();
    if (/^H[1-6]$/.test(tag)) { const t = line(); if (t) out.push('', '#'.repeat(Number(tag[1])) + ' ' + t, ''); return; }
    if (tag === 'P' || tag === 'FIGCAPTION' || tag === 'DT' || tag === 'DD') { const t = line(); if (t) out.push(t, ''); return; }
    if (tag === 'LI') { const t = line(); if (t) out.push('- ' + t.replace(/\n+/g, ' ')); return; }
    if (tag === 'BLOCKQUOTE') { const t = line(); if (t) out.push(t.split('\n').map((l) => '> ' + l).join('\n'), ''); return; }
    if (tag === 'PRE') { out.push('```', (el as HTMLElement).innerText, '```', ''); return; }
    if (tag === 'TABLE') { out.push('[table: use Tables to export it]', ''); return; }
    if (el.children.length === 0) { const t = line(); if (t) out.push(t, ''); return; }
    for (const c of Array.from(el.children)) walk(c);
    if (tag === 'UL' || tag === 'OL') out.push('');
  };
  walk(root);
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}

function scrapeText(): void {
  const root = mainElement();
  let text = structuredText(root);
  // Pages built from <div>s with no paragraphs: fall back to plain text.
  if (text.length < 200) text = (root as HTMLElement).innerText.trim();
  const words = text.split(/\s+/).filter(Boolean).length;
  const body = `${document.title}\n${location.href}\n\n${text}`;
  send({
    title: 'Text',
    summary: words ? `${words.toLocaleString()} words from the main part of the page` : 'This page has no text layer. Try Read Screen.',
    text: body, file: fileName('text', 'txt'), mime: 'text/plain', data: body,
  });
}

// ── Images ────────────────────────────────────────────────────────────

interface Img { url: string; alt: string; width: number; height: number; kind: string }

function collectImages(): Img[] {
  const seen = new Set<string>();
  const out: Img[] = [];
  const add = (url: string, alt: string, width: number, height: number, kind: string) => {
    if (!url || url.startsWith('data:') && url.length > 200000 || seen.has(url)) return;
    seen.add(url);
    out.push({ url, alt, width, height, kind });
  };
  document.querySelectorAll('img').forEach((img) => add(img.currentSrc || img.src, img.alt || '', img.naturalWidth, img.naturalHeight, 'img'));
  document.querySelectorAll('picture source[srcset]').forEach((s) => {
    const first = (s.getAttribute('srcset') || '').split(',')[0].trim().split(/\s+/)[0];
    if (first) add(new URL(first, location.href).href, '', 0, 0, 'source');
  });
  document.querySelectorAll('video[poster]').forEach((v) => add((v as HTMLVideoElement).poster, 'video poster', 0, 0, 'poster'));
  document.querySelectorAll('meta[property="og:image"]').forEach((m) => add((m as HTMLMetaElement).content, 'share image', 0, 0, 'og:image'));
  // Large elements painted with a CSS background image.
  document.querySelectorAll('body *').forEach((el) => {
    const r = el.getBoundingClientRect();
    if (r.width < 120 || r.height < 80) return;
    const bg = getComputedStyle(el).backgroundImage;
    const m = bg && bg.match(/url\(["']?([^"')]+)["']?\)/);
    if (m) add(new URL(m[1], location.href).href, el.getAttribute('aria-label') || '', Math.round(r.width), Math.round(r.height), 'background');
  });
  return out;
}

function scrapeImages(): void {
  const imgs = collectImages();
  const lines = imgs.map((i, n) => `${n + 1}. ${i.url}\n   ${[i.alt && `“${i.alt}”`, i.width ? `${i.width}×${i.height}` : '', i.kind].filter(Boolean).join(' · ')}`);
  send({
    title: 'Images',
    summary: `${imgs.length} images`,
    text: lines.join('\n') || 'No images found.',
    file: fileName('images', 'json'), mime: 'application/json', data: JSON.stringify({ page: location.href, images: imgs }, null, 2),
  });
}

// ── Links ─────────────────────────────────────────────────────────────

function scrapeLinks(): void {
  const seen = new Map<string, string>();
  document.querySelectorAll<HTMLAnchorElement>('a[href]').forEach((a) => {
    if (!/^https?:|^mailto:|^file:/.test(a.href)) return;
    const text = (a.innerText || a.getAttribute('aria-label') || a.title || '').replace(/\s+/g, ' ').trim();
    if (!seen.has(a.href) || (!seen.get(a.href) && text)) seen.set(a.href, text);
  });
  const links = Array.from(seen, ([url, text]) => ({ text, url }));
  const own = links.filter((l) => { try { return new URL(l.url).hostname === location.hostname; } catch { return false; } }).length;
  send({
    title: 'Links',
    summary: `${links.length} links · ${own} on this site, ${links.length - own} elsewhere`,
    text: links.map((l) => `${l.text || '(no text)'}\n  ${l.url}`).join('\n') || 'No links found.',
    file: fileName('links', 'csv'), mime: 'text/csv', data: toCsv([['text', 'url'], ...links.map((l) => [l.text, l.url])]),
  });
}

// ── Tables ────────────────────────────────────────────────────────────

function toCsv(rows: string[][]): string {
  return rows.map((r) => r.map((c) => /[",\n]/.test(c) ? `"${c.replace(/"/g, '""')}"` : c).join(',')).join('\n') + '\n';
}

function tableRows(t: HTMLTableElement): string[][] {
  // Expand colspans so columns line up.
  return Array.from(t.rows).map((row) => {
    const cells: string[] = [];
    Array.from(row.cells).forEach((c) => {
      const text = c.innerText.replace(/\s+/g, ' ').trim();
      for (let i = 0; i < Math.max(1, c.colSpan); i++) cells.push(i === 0 ? text : '');
    });
    return cells;
  });
}

function scrapeTables(): void {
  const tables = Array.from(document.querySelectorAll('table')).filter((t) => t.rows.length > 1 && visible(t));
  const parts = tables.map((t, i) => {
    const caption = t.caption?.innerText.trim() || `Table ${i + 1}`;
    return `# ${caption}\n${toCsv(tableRows(t))}`;
  });
  send({
    title: 'Tables',
    summary: tables.length ? `${tables.length} ${tables.length === 1 ? 'table' : 'tables'}, as CSV` : 'No tables on this page',
    text: parts.join('\n') || 'No tables found.',
    file: fileName('tables', 'csv'), mime: 'text/csv', data: parts.join('\n'),
  });
}

// ── Reading pixels (OCR) ──────────────────────────────────────────────

const nextFrame = () => new Promise<void>((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));
const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

async function ocrResult(title: string, text: string, what: string): Promise<void> {
  const clean = text.trim();
  send({
    title,
    summary: clean ? `${clean.split(/\s+/).length.toLocaleString()} words read from ${what}` : `No text found in ${what}`,
    text: clean || 'No text found. Text that is very small, blurry or in a non-Latin script may not be read.',
    file: fileName('ocr', 'txt'), mime: 'text/plain', data: clean,
  });
}

async function readScreen(): Promise<void> {
  report('Reading the visible part of the page…');
  await ocrResult('Read Screen', await oynix.ocr(), 'the visible page');
}

/** Let the user drag a rectangle, then read it. */
function readArea(): void {
  const shade = document.createElement('div');
  shade.className = 'oynix-scraper-shade';
  const box = document.createElement('div');
  box.className = 'oynix-scraper-box';
  const hint = document.createElement('div');
  hint.className = 'oynix-scraper-hint';
  hint.textContent = 'Drag over the text to read · Esc to cancel';
  shade.append(box, hint);
  document.documentElement.appendChild(shade);
  report('Drag over the part of the page to read.');

  let x0 = 0, y0 = 0, dragging = false;
  const rect = (e: MouseEvent) => ({ x: Math.min(x0, e.clientX), y: Math.min(y0, e.clientY), width: Math.abs(e.clientX - x0), height: Math.abs(e.clientY - y0) });
  const done = () => { shade.remove(); removeEventListener('keydown', onKey, true); };
  const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { done(); report('Cancelled.'); } };
  addEventListener('keydown', onKey, true);
  shade.addEventListener('mousedown', (e) => { dragging = true; x0 = e.clientX; y0 = e.clientY; e.preventDefault(); });
  shade.addEventListener('mousemove', (e) => {
    if (!dragging) return;
    const r = rect(e);
    Object.assign(box.style, { left: `${r.x}px`, top: `${r.y}px`, width: `${r.width}px`, height: `${r.height}px` });
  });
  shade.addEventListener('mouseup', async (e) => {
    if (!dragging) return;
    const r = rect(e);
    done();
    if (r.width < 8 || r.height < 8) { report('That area was too small.'); return; }
    await nextFrame(); // the shade must be gone from the captured picture
    report('Reading the selected area…');
    try {
      const text = await oynix.ocr(r);
      // Clicking the page closed the panel; bring it back with the result.
      await ocrResult('Read Area', text, 'the selected area');
      await oynix.openPanel();
    } catch (err) { report((err as Error).message); }
  });
}

/** Read the words inside pictures: each large image is scrolled into view and read. */
async function readImages(): Promise<void> {
  const imgs = Array.from(document.querySelectorAll('img, canvas')).filter((el) => {
    const r = el.getBoundingClientRect();
    return r.width >= 80 && r.height >= 24 && visible(el);
  }).slice(0, 25);
  if (!imgs.length) { report('No images or canvases large enough to read.'); return; }
  const startY = scrollY;
  const parts: string[] = [];
  for (let i = 0; i < imgs.length; i++) {
    const el = imgs[i];
    report(`Reading picture ${i + 1} of ${imgs.length}…`);
    el.scrollIntoView({ block: 'center', inline: 'nearest' });
    await sleep(250);
    const r = el.getBoundingClientRect();
    const x = Math.max(0, r.left), y = Math.max(0, r.top);
    const width = Math.min(innerWidth, r.right) - x, height = Math.min(innerHeight, r.bottom) - y;
    if (width < 8 || height < 8) continue;
    const text = (await oynix.ocr({ x, y, width, height })).trim();
    if (!text) continue;
    const label = el instanceof HTMLImageElement ? (el.alt || el.currentSrc.split('/').pop() || 'image') : 'canvas';
    parts.push(`[${label}]\n${text}`);
  }
  scrollTo(0, startY);
  await ocrResult('Read Pictures', parts.join('\n\n'), `${imgs.length} pictures`);
}

// ── Save ──────────────────────────────────────────────────────────────

function save(): void {
  if (!last || !last.data) return;
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([last.data], { type: `${last.mime};charset=utf-8` }));
  a.download = last.file;
  document.documentElement.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 10000);
  report(`Saved ${last.file} to Downloads.`);
}

oynix.onMessage(async (m: { cmd?: string }) => {
  try {
    switch (m.cmd) {
      case 'text': return scrapeText();
      case 'images': return scrapeImages();
      case 'links': return scrapeLinks();
      case 'tables': return scrapeTables();
      case 'ocr-screen': return await readScreen();
      case 'ocr-area': return readArea();
      case 'ocr-images': return await readImages();
      case 'save': return save();
      // A (re)opened panel: show the last result, if any.
      case 'hello': return last ? send(last) : report(`Ready on ${host}.`);
    }
  } catch (e) {
    report(`Something went wrong: ${(e as Error).message}`);
  }
});
