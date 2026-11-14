// addons.mozilla.org hides its install button from browsers other than
// Firefox, so this adds one. It asks AMO's public API for the add-on's
// current .xpi and opens it; OyNIx recognises the download and asks
// before installing.

const BUTTON_ID = 'oynix-amo-add';

interface AmoAddon {
  type?: string;
  name?: string | Record<string, string>;
  current_version?: { file?: { url?: string } };
}

function slug(): string {
  const m = location.pathname.match(/\/addon\/([^\/?#]+)/);
  return m ? decodeURIComponent(m[1]) : '';
}

function localName(n: AmoAddon['name']): string {
  if (!n) return '';
  if (typeof n === 'string') return n;
  const lang = document.documentElement.lang || 'en-US';
  return n[lang] || n['en-US'] || n[Object.keys(n)[0]] || '';
}

function update(): void {
  const s = slug();
  const old = document.getElementById(BUTTON_ID);
  if (!s) { old?.remove(); return; }
  if (old && old.dataset.slug === s) return;
  old?.remove();

  const b = document.createElement('button');
  b.id = BUTTON_ID;
  b.dataset.slug = s;
  b.textContent = 'Add to OyNIx';
  b.addEventListener('click', async () => {
    b.disabled = true;
    b.textContent = 'Preparing…';
    try {
      const r = await fetch(`/api/v5/addons/addon/${encodeURIComponent(s)}/`);
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const d: AmoAddon = await r.json();
      const url = d.current_version?.file?.url;
      if (!url) throw new Error('No downloadable file for this add-on');
      if (d.type && d.type !== 'extension') throw new Error(`Only extensions can be installed (this is a ${d.type})`);
      b.textContent = `Downloading ${localName(d.name)}…`;
      location.href = url;
      setTimeout(() => { b.disabled = false; b.textContent = 'Add to OyNIx'; }, 4000);
    } catch (e) {
      b.disabled = false;
      b.textContent = `Add to OyNIx (${(e as Error).message})`;
    }
  });
  (document.body || document.documentElement).appendChild(b);
}

update();
// AMO is a single-page app: follow client-side navigation.
let last = location.href;
setInterval(() => { if (location.href !== last) { last = location.href; update(); } }, 700);
