// Wallbreaker: gets you past "please turn off your ad blocker" walls.
//
// Runs in the page's own JavaScript world at document start, before the
// page's scripts, in four layers:
//   1. filters.txt (added to OyNIx's content blocker) stops the known
//      anti-adblock services and detector scripts from loading at all;
//   2. Admiral's loader is stopped before it runs, and BlockAdBlock-style
//      detectors are replaced with ones that always answer "no ad blocker";
//   3. the checks detectors make are answered the way a browser without an
//      ad blocker would answer them: bait elements have a size, and blocked
//      "is there an ad blocker?" test requests look like they loaded;
//   4. any wall that still appears is removed, and the page can scroll again.
// Nothing is sent anywhere, and pages that don't check for ad blockers are
// left alone.

(() => {
  const w = window as any;
  if (w.__oynixWallbreaker) return;
  Object.defineProperty(w, '__oynixWallbreaker', { value: true });

  // Detectors check whether functions are native; wrappers made here
  // answer like the originals.
  const disguised = new WeakMap<Function, Function>();
  const fnToString = Function.prototype.toString;
  const toStringProxy = new Proxy(fnToString, {
    apply(target, self, args) {
      return Reflect.apply(target, disguised.get(self) || self, args);
    },
  });
  disguised.set(toStringProxy, fnToString);
  Function.prototype.toString = toStringProxy;

  function wrap(owner: any, name: string, make: (orig: any) => any): void {
    const orig = owner && owner[name];
    if (typeof orig !== 'function') return;
    const fn = make(orig);
    disguised.set(fn, orig);
    try {
      Object.defineProperty(fn, 'name', { value: orig.name });
      Object.defineProperty(fn, 'length', { value: orig.length });
    } catch (e) { /* not configurable */ }
    owner[name] = fn;
  }

  function wrapGetter(proto: any, name: string, make: (get: () => any) => () => any): void {
    const d = Object.getOwnPropertyDescriptor(proto, name);
    if (!d || !d.get) return;
    const get = make(d.get);
    disguised.set(get, d.get);
    Object.defineProperty(proto, name, { ...d, get });
  }

  // A global the page can't replace or remove.
  function pin(name: string, value: unknown): void {
    try {
      Object.defineProperty(w, name, { configurable: false, enumerable: false, get: () => value, set: () => {} });
    } catch (e) { /* the page got there first (it can't, at document start) */ }
  }

  // ── 2a. Admiral ──────────────────────────────────────────────────────
  // Admiral's loader is an inline script that spells its own name with
  // URL escapes (decodeURI("%61%64%6d%69%72%61%6c") = "admiral", sometimes
  // escaped twice) and then injects its detector from a random domain.
  // Stop that script the first time it calls one of these functions.
  const ADMIRAL = /%(?:25)?61%(?:25)?64%(?:25)?6d%(?:25)?69%(?:25)?72%(?:25)?61%(?:25)?6c|getadmiral\.com/i;
  const checkedScripts = new WeakMap<Element, boolean>();
  function abortIfAdmiral(): void {
    const s = document.currentScript as HTMLScriptElement | null;
    if (!s || s.src) return;
    let bad = checkedScripts.get(s);
    if (bad === undefined) {
      const text = s.textContent || '';
      bad = text.length < 200000 && ADMIRAL.test(text);
      checkedScripts.set(s, bad);
    }
    if (bad) throw new ReferenceError('admiral is not defined');
  }
  for (const name of ['decodeURI', 'decodeURIComponent', 'atob']) {
    wrap(w, name, (orig) => function (this: unknown, ...args: unknown[]) {
      abortIfAdmiral();
      return Reflect.apply(orig, this, args);
    });
  }
  wrap(Document.prototype, 'createElement', (orig) => function (this: Document, ...args: unknown[]) {
    abortIfAdmiral();
    return Reflect.apply(orig, this, args);
  });
  // Whatever still calls window.admiral(...) gets a function that does nothing.
  const admiralStub: any = function () {};
  admiralStub.q = [];
  pin('admiral', admiralStub);

  // ── 2b. BlockAdBlock, FuckAdBlock, SniffAdBlock and friends ───────────
  class NoAdBlock {
    setOption() { return this; }
    check() { return false; }
    emitEvent() { return this; }
    clearEvent() {}
    on(detected: boolean, fn: () => void) { if (!detected && typeof fn === 'function') setTimeout(fn, 1); return this; }
    onDetected() { return this; }
    onNotDetected(fn: () => void) { return this.on(false, fn); }
  }
  for (const name of ['blockAdBlock', 'fuckAdBlock', 'sniffAdBlock']) pin(name, new NoAdBlock());
  for (const name of ['BlockAdBlock', 'FuckAdBlock', 'SniffAdBlock']) pin(name, NoAdBlock);
  // The classic test: an "ads.js" sets canRunAds = true unless it's blocked.
  pin('canRunAds', true);
  pin('isAdBlockActive', false);

  // ── 3a. Bait elements ─────────────────────────────────────────────────
  // Detectors add an empty <div class="adsbox ad-banner"> and check whether
  // it has a size. OyNIx's cosmetic filters hide it, so report the size it
  // would have had.
  const BAIT = /(^|[\s_-])(ad|ads|adsbox|ad-?banner|ad-?placeholder|ad-?slot|ad-?unit|adsbygoogle|banner-?ad|banner_ad|text-?ad|textads?|pub_300x250m?|pub_728x90|sponsor-?ad|ad-?container|doubleclick|advert|advertisement)([\s_-]|$)/i;
  // Called from hot paths (offsetHeight, getComputedStyle): cheap tests first.
  function isBait(el: Element): boolean {
    if (!(el instanceof HTMLElement)) return false;
    if (!BAIT.test(typeof el.className === 'string' ? el.className : '') && !BAIT.test(el.id || '')) return false;
    return el.childElementCount <= 2 && (el.textContent || '').trim().length <= 20;
  }
  const SIZE = { offsetHeight: 90, offsetWidth: 300, clientHeight: 90, clientWidth: 300 } as Record<string, number>;
  for (const name of Object.keys(SIZE)) {
    wrapGetter(HTMLElement.prototype, name, (get) => function (this: HTMLElement) {
      const v = get.call(this);
      return v === 0 && this.isConnected && isBait(this) ? SIZE[name] : v;
    });
  }
  wrapGetter(HTMLElement.prototype, 'offsetParent', (get) => function (this: HTMLElement) {
    const v = get.call(this);
    return v === null && this.isConnected && isBait(this) ? document.body : v;
  });
  wrap(Element.prototype, 'getBoundingClientRect', (orig) => function (this: Element) {
    const r: DOMRect = orig.call(this);
    if (r.width === 0 && r.height === 0 && this.isConnected && isBait(this)) {
      return new DOMRect(r.x, r.y, SIZE.offsetWidth, SIZE.offsetHeight);
    }
    return r;
  });
  wrap(w, 'getComputedStyle', (orig) => function (this: unknown, el: Element, ...rest: unknown[]) {
    const style: CSSStyleDeclaration = Reflect.apply(orig, this, [el, ...rest]);
    if (!el || !isBait(el) || (style.display !== 'none' && style.visibility !== 'hidden')) return style;
    const fake: Record<string, string> = { display: 'block', visibility: 'visible', height: SIZE.offsetHeight + 'px', width: SIZE.offsetWidth + 'px' };
    return new Proxy(style, {
      get(t, k) {
        if (typeof k === 'string' && k in fake) return fake[k];
        if (k === 'getPropertyValue') return (n: string) => (n in fake ? fake[n] : t.getPropertyValue(n));
        const v = Reflect.get(t, k, t);
        return typeof v === 'function' ? v.bind(t) : v;
      },
    });
  });

  // ── 3b. Test requests ─────────────────────────────────────────────────
  // Detectors also try to load an ad script or call an ad server and treat
  // a failure as "ad blocker". When such a request is blocked, make it look
  // like it worked. (When it isn't blocked, nothing changes.)
  const BAIT_URL = /(?:pagead2\.googlesyndication\.com|googlesyndication\.com\/pagead|doubleclick\.net|adservice\.google\.|\/(?:ads|advert|adverts|advertisement|adframe|adsbygoogle|show_ads(?:_impl)?|prebid|banner_?ads?|ad-?script|ads-?prebid)(?:[._-][\w.-]*)?\.js(?:[?#]|$))/i;

  // <script src> that failed to load: answer "load" instead of "error".
  window.addEventListener('error', (ev) => {
    const t = ev.target as HTMLScriptElement | null;
    if (!t || t.tagName !== 'SCRIPT' || !BAIT_URL.test(t.src || '')) return;
    ev.stopImmediatePropagation();
    ev.preventDefault();
    setTimeout(() => t.dispatchEvent(new Event('load')), 0);
  }, true);

  // fetch(): a blocked bait request resolves with an empty response.
  wrap(w, 'fetch', (orig) => function (this: unknown, input: any, ...rest: unknown[]) {
    const url = String(input && typeof input === 'object' && 'url' in input ? input.url : input);
    const p: Promise<Response> = Reflect.apply(orig, this, [input, ...rest]);
    if (!BAIT_URL.test(url)) return p;
    return p.catch(() => new Response('', { status: 200, headers: { 'content-type': 'text/javascript' } }));
  });

  // XMLHttpRequest: same, by turning its error into a successful load.
  const xhrUrls = new WeakMap<XMLHttpRequest, string>();
  wrap(XMLHttpRequest.prototype, 'open', (orig) => function (this: XMLHttpRequest, ...args: any[]) {
    const url = String(args[1]);
    if (BAIT_URL.test(url) && !xhrUrls.has(this)) {
      xhrUrls.set(this, url);
      // Added before the page's own handlers, so it runs first.
      this.addEventListener('error', (ev) => {
        ev.stopImmediatePropagation();
        for (const [k, v] of [['readyState', 4], ['status', 200], ['statusText', 'OK'], ['responseText', ''], ['response', '']] as [string, unknown][]) {
          Object.defineProperty(this, k, { configurable: true, get: () => v });
        }
        for (const type of ['readystatechange', 'load', 'loadend']) this.dispatchEvent(new ProgressEvent(type));
      });
    }
    return Reflect.apply(orig, this, args);
  });

  // ── 4. Walls that still appear ────────────────────────────────────────
  // A fixed overlay that covers a good part of the window and talks about
  // ad blockers is removed, with its backdrop, and scrolling is restored.
  const WALL = /ad[\s-]?block(?:er|ing)?\b|adblock|(?:disable|pause|turn off|switch off)\s+(?:your\s+)?ad|(?:whitelist|allowlist)\s+(?:us|this site|our site)|support us by (?:disabling|allowing|turning off)|ads? (?:are|is) (?:being )?blocked/i;
  const shadows = new WeakMap<Element, ShadowRoot>();
  wrap(Element.prototype, 'attachShadow', (orig) => function (this: Element, ...args: unknown[]) {
    const root: ShadowRoot = Reflect.apply(orig, this, args);
    shadows.set(this, root);
    return root;
  });

  function textOf(el: Element): string {
    let t = (el as HTMLElement).innerText || el.textContent || '';
    const root = shadows.get(el) || el.shadowRoot;
    if (root) t += ' ' + (root.textContent || '');
    return t.slice(0, 4000);
  }

  function covering(el: Element): boolean {
    const cs = getComputedStyle(el);
    if (cs.position !== 'fixed' && cs.position !== 'absolute' && cs.position !== 'sticky') return false;
    if (cs.display === 'none' || cs.visibility === 'hidden') return false;
    const r = el.getBoundingClientRect();
    return r.width * r.height >= innerWidth * innerHeight * 0.15;
  }

  function unlockScrolling(): void {
    for (const el of [document.documentElement, document.body]) {
      if (!el) continue;
      if (getComputedStyle(el).overflow === 'hidden' || getComputedStyle(el).overflowY === 'hidden') {
        el.style.setProperty('overflow', 'auto', 'important');
      }
      if (el.style.position === 'fixed') el.style.position = '';
    }
    document.querySelectorAll<HTMLElement>('[style*="blur("]').forEach((el) => { el.style.filter = ''; });
  }

  let removed = 0;
  function sweep(candidates: Element[]): void {
    let hit = false;
    for (const el of candidates) {
      if (!el.isConnected || el === document.body || el === document.documentElement) continue;
      if (!covering(el) || !WALL.test(textOf(el))) continue;
      el.remove();
      hit = true;
      removed++;
    }
    if (!hit) return;
    // The dimmed backdrop: an empty full-window layer on top of the page.
    for (const el of Array.from(document.body ? document.body.children : [])) {
      const cs = getComputedStyle(el);
      if (cs.position === 'fixed' && (parseInt(cs.zIndex, 10) || 0) > 100 && !(el.textContent || '').trim() && covering(el)) el.remove();
    }
    unlockScrolling();
    if (removed === 1) console.info('[Wallbreaker] removed an ad-block wall');
  }

  let queued = new Set<Element>();
  let scheduled = false;
  const topLevel = (el: Element) => el.parentElement === document.body || el.parentElement === document.documentElement;
  const observer = new MutationObserver((records) => {
    for (const r of records) {
      if (queued.size > 300) break;
      r.addedNodes.forEach((n) => { if (n.nodeType === 1) queued.add(n as Element); });
      // A wall that was already in the page and is just being shown.
      if (r.type === 'attributes' && r.target.nodeType === 1 && topLevel(r.target as Element)) queued.add(r.target as Element);
    }
    if (scheduled || !queued.size) return;
    scheduled = true;
    // Checking once a frame is plenty.
    requestAnimationFrame(() => {
      scheduled = false;
      const batch = Array.from(queued).slice(0, 200);
      queued = new Set();
      sweep(batch);
    });
  });
  const start = () => {
    observer.observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['style', 'class'] });
    if (document.body) sweep(Array.from(document.body.querySelectorAll('body > *, body > * > *')).slice(0, 400));
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start, { once: true });
  else start();
})();
