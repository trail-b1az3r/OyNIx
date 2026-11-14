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

/// addons.mozilla.org: AMO hides its install button from non-Firefox
/// browsers, so OyNIx adds its own "Add to OyNIx" button on add-on pages.
/// It asks AMO's public API for the add-on's current .xpi and navigates to
/// it; OyNIx then recognises the download and offers to install it.
pub const AMO_INSTALL: &str = r#"// ==UserScript==
// @name oynix-amo-install
// @match https://addons.mozilla.org/*
// @run-at document-idle
// ==/UserScript==
(function () {
  var BTN = 'oynix-amo-add';
  function slug() {
    var m = location.pathname.match(/\/addon\/([^\/?#]+)/);
    return m ? decodeURIComponent(m[1]) : '';
  }
  function name(n) {
    if (!n) return '';
    if (typeof n === 'string') return n;
    var lang = (document.documentElement.lang || 'en-US');
    return n[lang] || n['en-US'] || n[Object.keys(n)[0]] || '';
  }
  function update() {
    var s = slug(), old = document.getElementById(BTN);
    if (!s) { if (old) old.remove(); return; }
    if (old && old.dataset.slug === s) return;
    if (old) old.remove();
    var b = document.createElement('button');
    b.id = BTN; b.dataset.slug = s;
    b.textContent = 'Add to OyNIx';
    b.style.cssText = 'position:fixed;right:24px;bottom:24px;z-index:2147483647;padding:14px 26px;border:0;border-radius:28px;' +
      'background:#2463d8;color:#fff;font:600 16px system-ui,sans-serif;box-shadow:0 4px 16px rgba(0,0,0,.25);cursor:pointer';
    b.addEventListener('click', function () {
      b.disabled = true; b.textContent = 'Preparing…';
      fetch('/api/v5/addons/addon/' + encodeURIComponent(s) + '/')
        .then(function (r) { if (!r.ok) throw new Error('HTTP ' + r.status); return r.json(); })
        .then(function (d) {
          var f = d.current_version && d.current_version.file;
          if (!f || !f.url) throw new Error('No downloadable file for this add-on');
          if (d.type && d.type !== 'extension') throw new Error('Only extensions can be installed (this is a ' + d.type + ')');
          b.textContent = 'Downloading ' + name(d.name) + '…';
          location.href = f.url;
          setTimeout(function () { b.disabled = false; b.textContent = 'Add to OyNIx'; }, 4000);
        })
        .catch(function (e) { b.disabled = false; b.textContent = 'Add to OyNIx (' + e.message + ')'; });
    });
    (document.body || document.documentElement).appendChild(b);
  }
  update();
  // AMO is a single-page app: follow client-side navigation.
  var last = location.href;
  setInterval(function () { if (location.href !== last) { last = location.href; update(); } }, 700);
})();
"#;

/// WebUSB, page side (main world): `navigator.usb`, `USBDevice` and the
/// transfer results. Calls go to the isolated-world relay below through
/// DOM events carrying JSON strings; the relay adds the page's origin and
/// talks to OyNIx, which enforces permissions.
pub const WEBUSB_MAIN: &str = r#"
(function () {
  'use strict';
  if (!window.isSecureContext || window.top !== window || (navigator.usb && navigator.usb.__oynix)) return;
  var REQ = 'oynix-usb-req', RES = 'oynix-usb-res', EVT = 'oynix-usb-evt';
  var pending = {}, seq = 0, devices = {};
  var dispatch = EventTarget.prototype.dispatchEvent.bind(document);

  function toError(s) {
    var m = /^(\w+): ([\s\S]*)$/.exec(String(s || 'NetworkError: USB error'));
    var name = m ? m[1] : 'NetworkError', msg = m ? m[2] : String(s);
    if (name === 'TypeError') return new TypeError(msg);
    return new DOMException(msg, name);
  }
  function send(op, body) {
    return new Promise(function (resolve, reject) {
      var id = 'u' + (++seq) + '_' + Math.random().toString(36).slice(2);
      pending[id] = { resolve: resolve, reject: reject };
      var m = Object.assign({ id: id, op: op }, body || {});
      dispatch(new CustomEvent(REQ, { detail: JSON.stringify(m) }));
    });
  }
  document.addEventListener(RES, function (e) {
    var m; try { m = JSON.parse(e.detail); } catch (x) { return; }
    var p = pending[m.id]; if (!p) return;
    delete pending[m.id];
    if (m.ok) p.resolve(m); else p.reject(toError(m.error));
  });

  function toB64(data) {
    if (data === undefined || data === null) return '';
    var u8 = ArrayBuffer.isView(data) ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength) : new Uint8Array(data);
    var s = '';
    for (var i = 0; i < u8.length; i += 0x8000) s += String.fromCharCode.apply(null, u8.subarray(i, i + 0x8000));
    return btoa(s);
  }
  function fromB64(s) {
    var bin = atob(s || ''), u8 = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
    return new DataView(u8.buffer);
  }

  function USBInTransferResult(status, data) { this.status = status; this.data = data; }
  function USBOutTransferResult(status, n) { this.status = status; this.bytesWritten = n; }
  function USBConnectionEvent(type, init) { var e = new Event(type, init); e.device = init && init.device; return e; }

  function USBDevice(info) { this._apply(info); this._opened = false; this._claimed = {}; this._alt = {}; }
  USBDevice.prototype._apply = function (info) {
    this._info = info;
    var self = this;
    ['usbVersionMajor', 'usbVersionMinor', 'usbVersionSubminor', 'deviceClass', 'deviceSubclass', 'deviceProtocol',
     'vendorId', 'productId', 'deviceVersionMajor', 'deviceVersionMinor', 'deviceVersionSubminor'].forEach(function (k) { self['_' + k] = info[k]; });
    this._active = info.activeConfiguration || 0;
  };
  ['usbVersionMajor', 'usbVersionMinor', 'usbVersionSubminor', 'deviceClass', 'deviceSubclass', 'deviceProtocol',
   'vendorId', 'productId', 'deviceVersionMajor', 'deviceVersionMinor', 'deviceVersionSubminor'].forEach(function (k) {
    Object.defineProperty(USBDevice.prototype, k, { get: function () { return this['_' + k]; } });
  });
  function str(v) { return v ? v : null; }
  Object.defineProperties(USBDevice.prototype, {
    manufacturerName: { get: function () { return str(this._info.manufacturerName); } },
    productName: { get: function () { return str(this._info.productName); } },
    serialNumber: { get: function () { return str(this._info.serialNumber); } },
    opened: { get: function () { return this._opened; } },
    configurations: { get: function () { var self = this; return this._info.configurations.map(function (c) { return self._config(c); }); } },
    configuration: { get: function () {
      var self = this, c = this._info.configurations.filter(function (c) { return c.configurationValue === self._active; })[0];
      return c ? this._config(c) : null;
    } }
  });
  USBDevice.prototype._config = function (c) {
    var self = this, cfg = { configurationValue: c.configurationValue, configurationName: str(c.configurationName) };
    cfg.interfaces = c.interfaces.map(function (i) {
      var alts = i.alternates.map(function (a) {
        return { alternateSetting: a.alternateSetting, interfaceClass: a.interfaceClass, interfaceSubclass: a.interfaceSubclass,
                 interfaceProtocol: a.interfaceProtocol, interfaceName: str(a.interfaceName),
                 endpoints: a.endpoints.map(function (e) { return { endpointNumber: e.endpointNumber, direction: e.direction, type: e.type, packetSize: e.packetSize }; }) };
      });
      var cur = self._alt[i.interfaceNumber] || 0;
      return { interfaceNumber: i.interfaceNumber, alternates: alts,
               alternate: alts.filter(function (a) { return a.alternateSetting === cur; })[0] || alts[0],
               claimed: !!self._claimed[i.interfaceNumber] };
    });
    return cfg;
  };
  USBDevice.prototype._call = function (op, args) {
    if (!this._opened && op !== 'open' && op !== 'forget') return Promise.reject(new DOMException('The device must be opened first.', 'InvalidStateError'));
    return send('call', { key: this._info.key, method: op, args: args || {} });
  };
  USBDevice.prototype.open = function () {
    var self = this;
    if (this._opened) return Promise.resolve();
    return this._call('open').then(function (r) { if (r.device) self._apply(r.device); self._opened = true; });
  };
  USBDevice.prototype.close = function () {
    var self = this;
    if (!this._opened) return Promise.resolve();
    return this._call('close').then(function () { self._opened = false; self._claimed = {}; self._alt = {}; });
  };
  USBDevice.prototype.forget = function () {
    var self = this;
    return this._call('forget').then(function () { self._opened = false; self._claimed = {}; delete devices[self._info.key]; });
  };
  USBDevice.prototype.selectConfiguration = function (v) {
    var self = this;
    return this._call('selectConfiguration', { configurationValue: v }).then(function () { self._active = v; self._claimed = {}; self._alt = {}; });
  };
  USBDevice.prototype.claimInterface = function (n) {
    var self = this;
    if (this._claimed[n]) return Promise.resolve();
    return this._call('claimInterface', { interfaceNumber: n }).then(function () { self._claimed[n] = true; });
  };
  USBDevice.prototype.releaseInterface = function (n) {
    var self = this;
    return this._call('releaseInterface', { interfaceNumber: n }).then(function () { delete self._claimed[n]; delete self._alt[n]; });
  };
  USBDevice.prototype.selectAlternateInterface = function (n, alt) {
    var self = this;
    return this._call('selectAlternateInterface', { interfaceNumber: n, alternateSetting: alt }).then(function () { self._alt[n] = alt; });
  };
  USBDevice.prototype.controlTransferIn = function (setup, length) {
    return this._call('controlTransferIn', { setup: setup, length: length }).then(function (r) { return new USBInTransferResult(r.status, fromB64(r.data)); });
  };
  USBDevice.prototype.controlTransferOut = function (setup, data) {
    return this._call('controlTransferOut', { setup: setup, data: toB64(data) }).then(function (r) { return new USBOutTransferResult(r.status, r.bytesWritten); });
  };
  USBDevice.prototype.transferIn = function (ep, length) {
    return this._call('transferIn', { endpointNumber: ep, length: length }).then(function (r) { return new USBInTransferResult(r.status, fromB64(r.data)); });
  };
  USBDevice.prototype.transferOut = function (ep, data) {
    return this._call('transferOut', { endpointNumber: ep, data: toB64(data) }).then(function (r) { return new USBOutTransferResult(r.status, r.bytesWritten); });
  };
  USBDevice.prototype.clearHalt = function (direction, ep) { return this._call('clearHalt', { direction: direction, endpointNumber: ep }).then(function () {}); };
  USBDevice.prototype.reset = function () { return this._call('reset').then(function () {}); };
  USBDevice.prototype.isochronousTransferIn = USBDevice.prototype.isochronousTransferOut = function () {
    return Promise.reject(new DOMException('Isochronous transfers are not supported.', 'NotSupportedError'));
  };

  function device(info) {
    var d = devices[info.key];
    if (!d) d = devices[info.key] = new USBDevice(info);
    return d;
  }

  function USB() { var t = new EventTarget(); Object.setPrototypeOf(t, USB.prototype); t.onconnect = null; t.ondisconnect = null; return t; }
  USB.prototype = Object.create(EventTarget.prototype);
  USB.prototype.constructor = USB;
  USB.prototype.__oynix = true;
  USB.prototype.getDevices = function () {
    return send('getDevices').then(function (r) { return r.devices.map(device); });
  };
  USB.prototype.requestDevice = function (options) {
    if (!options || !Array.isArray(options.filters)) return Promise.reject(new TypeError("Failed to execute 'requestDevice' on 'USB': required member filters is undefined."));
    var bad = [].concat(options.filters, options.exclusionFilters || []).filter(function (f) {
      return !f || (f.productId !== undefined && f.vendorId === undefined) || (f.subclassCode !== undefined && f.classCode === undefined) ||
        (f.protocolCode !== undefined && f.subclassCode === undefined);
    });
    if (bad.length) return Promise.reject(new TypeError("Failed to execute 'requestDevice' on 'USB': invalid filter (productId needs vendorId, subclassCode needs classCode, protocolCode needs subclassCode)."));
    if (navigator.userActivation && !navigator.userActivation.isActive)
      return Promise.reject(new DOMException('Must be handling a user gesture to show a permission request.', 'SecurityError'));
    return send('requestDevice', { options: options }).then(function (r) { return device(r.device); });
  };
  var usb = new USB();
  document.addEventListener(EVT, function (e) {
    var m; try { m = JSON.parse(e.detail); } catch (x) { return; }
    var d = device(m.device);
    if (m.kind === 'disconnect') { d._opened = false; d._claimed = {}; }
    var ev = new USBConnectionEvent(m.kind, { device: d });
    var h = usb['on' + m.kind];
    if (typeof h === 'function') { try { h.call(usb, ev); } catch (x) { setTimeout(function () { throw x; }); } }
    usb.dispatchEvent(ev);
  });

  try { Object.defineProperty(Navigator.prototype, 'usb', { get: function () { return usb; }, configurable: true, enumerable: true }); } catch (e) {}
  [['USB', USB], ['USBDevice', USBDevice], ['USBInTransferResult', USBInTransferResult], ['USBOutTransferResult', USBOutTransferResult], ['USBConnectionEvent', USBConnectionEvent]]
    .forEach(function (p) { try { Object.defineProperty(window, p[0], { value: p[1], configurable: true, writable: true }); } catch (e) {} });
})();
"#;

/// WebUSB, isolated-world relay: adds the page's real origin to each call,
/// enforces the user-gesture rule for the chooser, and forwards to OyNIx's
/// `oynixUsb` WebChannel object. Prefixed with qwebchannel.js at runtime.
pub const WEBUSB_RELAY: &str = r#"
(function () {
  'use strict';
  if (window.__oynixUsbRelay || window.top !== window || !window.isSecureContext) return;
  window.__oynixUsbRelay = true;
  var origin = location.origin;
  var token = Array.prototype.map.call(crypto.getRandomValues(new Uint8Array(12)), function (b) { return ('0' + b.toString(16)).slice(-2); }).join('');
  var waiting = {}, ready = null;
  function out(type, obj) { document.dispatchEvent(new CustomEvent(type, { detail: JSON.stringify(obj) })); }
  function channel() {
    if (!ready) ready = new Promise(function (resolve) {
      (function connect() {
        if (typeof qt === 'undefined' || !qt.webChannelTransport || typeof QWebChannel === 'undefined') { setTimeout(connect, 25); return; }
        if (!window.__oynixChannel) window.__oynixChannel = new Promise(function (r) { new QWebChannel(qt.webChannelTransport, r); });
        window.__oynixChannel.then(function (ch) {
          var b = ch.objects.oynixUsb;
          if (!b) return;
          b.result.connect(function (id, payload) { var cb = waiting[id]; if (cb) { delete waiting[id]; cb(JSON.parse(payload)); } });
          b.deviceEvent.connect(function (kind, payload) {
            var p = JSON.parse(payload);
            if (p.origins.indexOf(origin) >= 0) out('oynix-usb-evt', { kind: kind, device: p.device });
          });
          ['chooserRequested', 'permissionsChanged'].forEach(function (n) { if (b[n]) b[n].connect(function () {}); });
          resolve(b);
        });
      })();
    });
    return ready;
  }
  document.addEventListener('oynix-usb-req', function (e) {
    var m; try { m = JSON.parse(e.detail); } catch (x) { return; }
    if (typeof m.id !== 'string') return;
    var reply = function (r) { r.id = m.id; out('oynix-usb-res', r); };
    if (m.op === 'requestDevice' && navigator.userActivation && !navigator.userActivation.isActive)
      return reply({ ok: false, error: 'SecurityError: Must be handling a user gesture to show a permission request.' });
    channel().then(function (b) {
      var bid = token + ':' + m.id;
      waiting[bid] = reply;
      if (m.op === 'requestDevice') b.requestDevice(origin, JSON.stringify(m.options || {}), bid);
      else if (m.op === 'getDevices') b.getDevices(origin, bid);
      else if (m.op === 'call') b.call(origin, bid, String(m.key || ''), String(m.method || ''), JSON.stringify(m.args || {}));
    });
  });
  channel();
})();
"#;

/// The OyNIx website and Extension Gallery: tell the page it is open in
/// OyNIx (and which version) so it can offer one-click installs.
pub const GALLERY_MARK: &str = r#"// ==UserScript==
// @name oynix-gallery
// @match https://trail-b1az3r.github.io/OyNIx/*
// @run-at document-start
// ==/UserScript==
(function () {
  function mark() { document.documentElement.setAttribute('data-oynix', '__OYNIX_VERSION__'); }
  if (document.documentElement) return mark();
  // At document start the <html> element may not exist yet.
  new MutationObserver(function (_, o) { if (document.documentElement) { o.disconnect(); mark(); } })
    .observe(document, { childList: true });
})();
"#;
