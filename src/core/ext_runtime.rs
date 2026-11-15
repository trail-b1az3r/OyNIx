//! OyNIx WebExtension runtime.
//!
//! Qt WebEngine's extension support (6.10+) loads extensions and serves
//! their `chrome-extension://` pages, but does not inject content scripts or
//! provide the `chrome.*` API. OyNIx fills that gap with this script: it
//! implements the commonly used subset of the WebExtension API on top of a
//! Qt WebChannel connection to the Rust `ExtBridge` object.
//!
//! Supported: runtime (id, getURL, getManifest, sendMessage, onMessage,
//! onInstalled, onStartup, openOptionsPage, getPlatformInfo, lastError),
//! storage (local, sync, session, onChanged), i18n, tabs (query, get,
//! create, update, remove, reload, sendMessage, getCurrent, onCreated,
//! onUpdated, onActivated, onRemoved), action / browserAction badge & title,
//! alarms, permissions (reports granted). Everything returns promises *and*
//! accepts callbacks, so it serves both `chrome.*` and `browser.*` code.
//!
//! The script is idempotent and defines `__oynixExt(ctx, fn)` (content
//! scripts) and `__oynixExtPage(ctxByExtId)` (extension pages).

pub const RUNTIME: &str = r#"
(function (g) {
  'use strict';
  if (g.__oynixExt) return;

  var ready = null, bridge = null, runtimes = [];

  function channel() {
    if (!ready) ready = new Promise(function (resolve) {
      (function connect() {
        if (typeof qt === 'undefined' || !qt.webChannelTransport || typeof QWebChannel === 'undefined') { setTimeout(connect, 25); return; }
        // One QWebChannel per world: a second client on the same transport
        // would take over its message handler (WebUSB shares this one).
        if (!g.__oynixChannel) g.__oynixChannel = new Promise(function (r) { new QWebChannel(qt.webChannelTransport, r); });
        g.__oynixChannel.then(function (ch) {
          bridge = ch.objects.oynixExtensions;
          bridge.message.connect(function (ext, target, payload) { var p = JSON.parse(payload); runtimes.forEach(function (r) { r._deliver(ext, target, p); }); });
          bridge.replied.connect(function (ext, id, resp) { runtimes.forEach(function (r) { r._reply(ext, id, resp); }); });
          bridge.storageChanged.connect(function (ext, area, changes) { var c = JSON.parse(changes); runtimes.forEach(function (r) { r._storage(ext, area, c); }); });
          bridge.tabsResult.connect(function (id, result) { runtimes.forEach(function (r) { r._tabs(id, result); }); });
          bridge.extEvent.connect(function (name, args) { var a = JSON.parse(args); runtimes.forEach(function (r) { r._event(name, a); }); });
          if (bridge.nativeMessage) bridge.nativeMessage.connect(function (port, json) { runtimes.forEach(function (r) { r._native(port, json, null); }); });
          if (bridge.nativeClosed) bridge.nativeClosed.connect(function (port, err) { runtimes.forEach(function (r) { r._native(port, null, err); }); });
          // Signals meant for the browser UI; acknowledge them so the
          // WebChannel client does not log "Unhandled signal".
          ['badgeChanged', 'optionsRequested', 'tabsRequest'].forEach(function (n) { if (bridge[n]) bridge[n].connect(function () {}); });
          resolve(bridge);
        });
      })();
    });
    return ready;
  }

  function call(method) {
    var args = Array.prototype.slice.call(arguments, 1);
    return channel().then(function (b) { return new Promise(function (res) { b[method].apply(b, args.concat([res])); }); });
  }

  function uid() { return Date.now().toString(36) + Math.random().toString(36).slice(2, 10); }

  function makeEvent() {
    var ls = [];
    return {
      addListener: function (f) { if (typeof f === 'function' && ls.indexOf(f) < 0) ls.push(f); },
      removeListener: function (f) { var i = ls.indexOf(f); if (i >= 0) ls.splice(i, 1); },
      hasListener: function (f) { return ls.indexOf(f) >= 0; },
      hasListeners: function () { return ls.length > 0; },
      _listeners: ls,
      _fire: function () { var a = arguments; return ls.slice().map(function (f) { try { return f.apply(null, a); } catch (e) { console.error(e); } }); }
    };
  }

  function make(ctx) {
    var me = { id: uid() };
    var lastError = null;
    var pending = {}, tabsPending = {}, alarms = {}, seen = {};
    // Events fired from the browser by name ("bookmarks.onCreated", ...).
    var evts = {};
    function evt(name) { return evts[name] || (evts[name] = makeEvent()); }
    var ev = {
      message: makeEvent(), installed: makeEvent(), startup: makeEvent(), connect: makeEvent(),
      storage: makeEvent(), tabCreated: makeEvent(), tabUpdated: makeEvent(), tabActivated: makeEvent(),
      tabRemoved: makeEvent(), alarm: makeEvent(), actionClicked: makeEvent(), menuClicked: makeEvent()
    };

    // Wrap a promise-returning implementation so it also accepts a callback.
    function api(impl) {
      return function () {
        var args = Array.prototype.slice.call(arguments);
        var cb = args.length && typeof args[args.length - 1] === 'function' ? args.pop() : null;
        var p;
        try { p = Promise.resolve(impl.apply(null, args)); } catch (e) { p = Promise.reject(e); }
        if (!cb) return p;
        p.then(function (v) { lastError = null; cb(v); }, function (e) { lastError = { message: String(e && e.message || e) }; try { cb(); } finally { lastError = null; } });
      };
    }

    function tabsCall(op, args) {
      var id = uid();
      return new Promise(function (resolve, reject) {
        tabsPending[id] = { resolve: resolve, reject: reject };
        call('tabs', ctx.ext, id, op, JSON.stringify(args === undefined ? null : args));
      });
    }

    function send(target, msg) {
      var id = uid();
      var sender = { id: ctx.ext, url: location.href, origin: location.origin };
      if (ctx.kind === 'content') sender.tab = { id: g.__oynixTabId || -1, url: location.href, title: document.title };
      var payload = JSON.stringify({ id: id, from: me.id, msg: msg, sender: sender });
      return new Promise(function (resolve, reject) {
        // Background contexts may still be starting: re-post until someone
        // answers (receivers ignore duplicates), then give up after 6s.
        var retry = setInterval(function () { if (pending[id] && !pending[id].heard) call('post', ctx.ext, target, payload); }, 400);
        var t = setTimeout(function () { clearInterval(retry); delete pending[id]; reject(new Error('Could not establish connection. Receiving end does not exist.')); }, 6000);
        pending[id] = { resolve: resolve, reject: reject, timer: t, retry: retry, grace: null, heard: false };
        call('post', ctx.ext, target, payload);
      });
    }

    function area(name) {
      return {
        get: api(function (keys) { return call('storageGet', ctx.ext, name, JSON.stringify(keys === undefined ? null : keys)).then(JSON.parse); }),
        set: api(function (items) { return call('storageSet', ctx.ext, name, JSON.stringify(items || {})).then(function () {}); }),
        remove: api(function (keys) { return call('storageRemove', ctx.ext, name, JSON.stringify(keys)).then(function () {}); }),
        clear: api(function () { return call('storageClear', ctx.ext, name).then(function () {}); }),
        getBytesInUse: api(function () { return 0; }),
        setAccessLevel: api(function () {}),
        onChanged: makeEvent(),
        QUOTA_BYTES: 10485760
      };
    }
    var areas = { local: area('local'), sync: area('sync'), session: area('session'), managed: area('managed') };

    function getMessage(name, subs) {
      var m = ctx.messages && (ctx.messages[name] || ctx.messages[String(name).toLowerCase()]);
      if (!m) {
        if (name === '@@extension_id') return ctx.ext;
        if (name === '@@ui_locale') return (navigator.language || 'en').replace('-', '_');
        return '';
      }
      subs = subs === undefined ? [] : (Array.isArray(subs) ? subs : [subs]);
      var text = m.message || '';
      var ph = m.placeholders || {};
      text = text.replace(/\$([a-zA-Z0-9_@]+)\$/g, function (_, k) {
        var p = ph[k] || ph[k.toLowerCase()];
        return p ? String(p.content || '') : '';
      });
      return text.replace(/\$(\d)/g, function (_, n) { var v = subs[n - 1]; return v === undefined ? '' : String(v); });
    }

    // Script / CSS injection into tabs (tabs.executeScript, scripting.*).
    function readFiles(files) {
      return Promise.all((files || []).map(function (f) {
        return fetch(chrome.runtime.getURL(f)).then(function (r) { if (!r.ok) throw new Error('Could not load ' + f); return r.text(); });
      }));
    }
    function execInTab(tabId, code, world, allFrames) {
      return tabsCall('exec', { tabId: tabId === undefined ? null : tabId, code: code, world: world || 'ISOLATED', allFrames: !!allFrames });
    }
    function cssInTab(tabId, css, remove) {
      return tabsCall(remove ? 'uncss' : 'css', { tabId: tabId === undefined ? null : tabId, css: css, ext: ctx.ext });
    }
    function mv2Exec(tabId, details) {
      if (typeof tabId === 'object' && tabId !== null) { details = tabId; tabId = undefined; }
      details = details || {};
      var codeP = details.code !== undefined ? Promise.resolve([details.code]) : readFiles(details.file ? [details.file] : []);
      return codeP.then(function (parts) { return execInTab(tabId, parts.join('\n;\n'), 'ISOLATED', details.allFrames); }).then(function (r) { return [r]; });
    }
    function mv2Css(tabId, details, remove) {
      if (typeof tabId === 'object' && tabId !== null) { details = tabId; tabId = undefined; }
      details = details || {};
      var p = details.code !== undefined ? Promise.resolve([details.code]) : readFiles(details.file ? [details.file] : []);
      return p.then(function (parts) { return cssInTab(tabId, parts.join('\n'), remove); }).then(function () {});
    }
    function mv3Exec(inj) {
      var t = inj.target || {};
      var codeP;
      if (inj.func) codeP = Promise.resolve(['(' + inj.func.toString() + ').apply(null, ' + JSON.stringify(inj.args || []) + ')']);
      else if (inj.files) codeP = readFiles(inj.files);
      else return Promise.reject(new Error('executeScript needs func or files'));
      return codeP.then(function (parts) { return execInTab(t.tabId, parts.join('\n;\n'), inj.world, t.allFrames); })
                  .then(function (r) { return [{ frameId: 0, documentId: '', result: r === null ? undefined : r }]; });
    }
    function mv3Css(inj, remove) {
      var t = inj.target || {};
      var p = inj.css !== undefined ? Promise.resolve([inj.css]) : readFiles(inj.files);
      return p.then(function (parts) { return cssInTab(t.tabId, parts.join('\n'), remove); }).then(function () {});
    }

    // Context menu items (shown in the page context menu by OyNIx).
    var menuClicks = {};
    function menus(op, props, id) {
      if (props && typeof props.onclick === 'function') { menuClicks[props.id || id] = props.onclick; }
      var clean = props ? JSON.parse(JSON.stringify(props)) : null;
      return tabsCall('menus', { op: op, ext: ctx.ext, extName: ctx.manifest && ctx.manifest.name, id: id, props: clean });
    }

    // Long-lived connections (runtime.connect / tabs.connect / onConnect).
    var ports = {};
    function makePort(portId, name, peer, target, sender) {
      var port = { name: name || '', sender: sender, onMessage: makeEvent(), onDisconnect: makeEvent(), _peer: peer, _queue: [], _closed: false };
      port.postMessage = function (msg) {
        if (port._closed) throw new Error('Attempting to use a disconnected port object');
        var payload = { id: uid(), from: me.id, portMsg: portId, msg: msg };
        if (port._peer) call('post', ctx.ext, 'inst:' + port._peer, JSON.stringify(payload)); else port._queue.push(payload);
      };
      port.disconnect = function () {
        if (port._closed) return;
        port._closed = true; delete ports[portId];
        if (port._peer) call('post', ctx.ext, 'inst:' + port._peer, JSON.stringify({ id: uid(), from: me.id, portClose: portId }));
      };
      ports[portId] = port;
      return port;
    }
    function connect(target, info) {
      var portId = uid();
      var port = makePort(portId, info && info.name, null, target);
      var sender = { id: ctx.ext, url: location.href, origin: location.origin };
      if (ctx.kind === 'content') sender.tab = { id: g.__oynixTabId || -1, url: location.href, title: document.title };
      call('post', ctx.ext, target, JSON.stringify({ id: uid(), from: me.id, connect: { port: portId, name: port.name }, sender: sender }));
      return port;
    }

    var badge = { text: '', color: '#d93025', title: ctx.manifest && ((ctx.manifest.action || ctx.manifest.browser_action || {}).default_title || ctx.manifest.name) || '' };
    var action = {
      setBadgeText: api(function (d) { badge.text = (d && d.text) || ''; return call('setBadge', ctx.ext, badge.text, badge.color, badge.title); }),
      getBadgeText: api(function () { return badge.text; }),
      setBadgeBackgroundColor: api(function (d) { var c = d && d.color; badge.color = Array.isArray(c) ? 'rgb(' + c.slice(0, 3).join(',') + ')' : (c || badge.color); return call('setBadge', ctx.ext, badge.text, badge.color, badge.title); }),
      getBadgeBackgroundColor: api(function () { return [217, 48, 37, 255]; }),
      setBadgeTextColor: api(function () {}),
      setTitle: api(function (d) { badge.title = (d && d.title) || ''; return call('setBadge', ctx.ext, badge.text, badge.color, badge.title); }),
      getTitle: api(function () { return badge.title; }),
      setIcon: api(function () {}), setPopup: api(function () {}), getPopup: api(function () { return ''; }),
      enable: api(function () {}), disable: api(function () {}), openPopup: api(function () {}),
      getUserSettings: api(function () { return { isOnToolbar: true }; }),
      onClicked: ev.actionClicked
    };

    function scheduleAlarm(name, info) {
      if (alarms[name]) { clearTimeout(alarms[name].t); clearInterval(alarms[name].i); }
      var when = info.when || (Date.now() + (info.delayInMinutes || info.periodInMinutes || 0) * 60000);
      var a = { name: name, scheduledTime: when, periodInMinutes: info.periodInMinutes };
      a.t = setTimeout(function () {
        ev.alarm._fire({ name: name, scheduledTime: a.scheduledTime, periodInMinutes: a.periodInMinutes });
        if (a.periodInMinutes) a.i = setInterval(function () { a.scheduledTime = Date.now(); ev.alarm._fire({ name: name, scheduledTime: a.scheduledTime, periodInMinutes: a.periodInMinutes }); }, a.periodInMinutes * 60000);
        else delete alarms[name];
      }, Math.max(0, when - Date.now()));
      alarms[name] = a;
    }
    function alarmInfo(a) { return { name: a.name, scheduledTime: a.scheduledTime, periodInMinutes: a.periodInMinutes }; }

    var stub = function (v) { return api(function () { return v; }); };
    var chrome = {
      runtime: {
        id: ctx.ext,
        getURL: function (p) { return 'chrome-extension://' + ctx.ext + '/' + String(p || '').replace(/^\//, ''); },
        getManifest: function () { return JSON.parse(JSON.stringify(ctx.manifest || {})); },
        sendMessage: api(function (a, b) {
          var msg = (typeof a === 'string' && b !== undefined && !(b && typeof b === 'object' && 'includeTlsChannelId' in b)) ? b : a;
          return send('extension', msg);
        }),
        onMessage: ev.message, onMessageExternal: makeEvent(), onInstalled: ev.installed, onStartup: ev.startup,
        onConnect: ev.connect, onSuspend: makeEvent(), onUpdateAvailable: makeEvent(),
        connect: function (a, b) { var info = typeof a === 'object' ? a : b; return connect('extension', info); },
        openOptionsPage: api(function () { return call('openOptions', ctx.ext); }),
        getPlatformInfo: api(function () { return { os: 'linux', arch: 'x86-64', nacl_arch: 'x86-64' }; }),
        getBackgroundPage: api(function () { return ctx.kind === 'background' ? g : null; }),
        setUninstallURL: stub(undefined), reload: function () { location.reload(); },
        requestUpdateCheck: stub({ status: 'no_update' }), getContexts: stub([]),
        get lastError() { return lastError || undefined; }
      },
      storage: { local: areas.local, sync: areas.sync, session: areas.session, managed: areas.managed, onChanged: ev.storage },
      i18n: {
        getMessage: getMessage,
        getUILanguage: function () { return navigator.language || 'en-US'; },
        getAcceptLanguages: api(function () { return navigator.languages || [navigator.language]; }),
        detectLanguage: api(function () { return { isReliable: false, languages: [] }; })
      },
      tabs: {
        TAB_ID_NONE: -1,
        query: api(function (q) { return tabsCall('query', q || {}); }),
        get: api(function (id) { return tabsCall('get', id); }),
        getCurrent: api(function () { return ctx.kind === 'page-tab' ? tabsCall('get', g.__oynixTabId) : undefined; }),
        create: api(function (p) { return tabsCall('create', p || {}); }),
        update: api(function (a, b) { return typeof a === 'number' ? tabsCall('update', { tabId: a, props: b || {} }) : tabsCall('update', { tabId: null, props: a || {} }); }),
        remove: api(function (ids) { return tabsCall('remove', Array.isArray(ids) ? ids : [ids]); }),
        reload: api(function (id) { return tabsCall('reload', typeof id === 'number' ? id : null); }),
        duplicate: api(function (id) { return tabsCall('duplicate', id); }),
        sendMessage: api(function (tabId, msg) { return send('tab:' + tabId, msg); }),
        executeScript: api(function (tabId, details) { return mv2Exec(tabId, details); }),
        insertCSS: api(function (tabId, details) { return mv2Css(tabId, details, false); }),
        removeCSS: api(function (tabId, details) { return mv2Css(tabId, details, true); }),
        connect: function (tabId, info) { return connect('tab:' + tabId, info); },
        onCreated: ev.tabCreated, onUpdated: ev.tabUpdated, onActivated: ev.tabActivated, onRemoved: ev.tabRemoved,
        onHighlighted: makeEvent(), onMoved: makeEvent(), onReplaced: makeEvent(), onAttached: makeEvent(), onDetached: makeEvent()
      },
      windows: {
        WINDOW_ID_CURRENT: -2,
        getCurrent: api(function () { return tabsCall('window', null); }),
        getLastFocused: api(function () { return tabsCall('window', null); }),
        getAll: api(function () { return tabsCall('windows', null); }),
        create: api(function (p) { return tabsCall('create', { url: p && (Array.isArray(p.url) ? p.url[0] : p.url), newWindow: true }); }),
        onFocusChanged: makeEvent(), onCreated: makeEvent(), onRemoved: makeEvent()
      },
      action: action,
      alarms: {
        create: function (name, info) { if (typeof name === 'object') { info = name; name = ''; } scheduleAlarm(name || '', info || {}); return Promise.resolve(); },
        get: api(function (name) { var a = alarms[name || '']; return a ? alarmInfo(a) : undefined; }),
        getAll: api(function () { return Object.keys(alarms).map(function (k) { return alarmInfo(alarms[k]); }); }),
        clear: api(function (name) { var a = alarms[name || '']; if (!a) return false; clearTimeout(a.t); clearInterval(a.i); delete alarms[name || '']; return true; }),
        clearAll: api(function () { Object.keys(alarms).forEach(function (k) { clearTimeout(alarms[k].t); clearInterval(alarms[k].i); delete alarms[k]; }); return true; }),
        onAlarm: ev.alarm
      },
      permissions: {
        contains: api(function () { return true; }), request: api(function () { return true; }),
        remove: api(function () { return false; }), getAll: api(function () { return { permissions: (ctx.manifest && ctx.manifest.permissions) || [], origins: (ctx.manifest && ctx.manifest.host_permissions) || [] }; }),
        onAdded: makeEvent(), onRemoved: makeEvent()
      },
      contextMenus: {
        create: function (props, cb) { var id = (props && props.id) || uid(); props = Object.assign({}, props, { id: id }); menus('create', props, id).then(function () { if (cb) cb(); }, function () { if (cb) cb(); }); return id; },
        update: api(function (id, props) { return menus('update', props, id); }),
        remove: api(function (id) { delete menuClicks[id]; return menus('remove', null, id); }),
        removeAll: api(function () { menuClicks = {}; return menus('removeAll', null, null); }),
        onClicked: ev.menuClicked,
        ContextType: { ALL: 'all', PAGE: 'page', SELECTION: 'selection', LINK: 'link', IMAGE: 'image', EDITABLE: 'editable', ACTION: 'action' }
      },
      notifications: {
        create: api(function (a, b) { var o = typeof a === 'object' ? a : b; var id = typeof a === 'string' ? a : uid(); return tabsCall('notify', { title: (o && o.title) || (ctx.manifest && ctx.manifest.name) || '', message: (o && o.message) || '' }).then(function () { return id; }); }),
        clear: stub(true), getAll: stub({}), onClicked: makeEvent(), onClosed: makeEvent(), onButtonClicked: makeEvent()
      },
      extension: {
        getURL: function (p) { return chrome.runtime.getURL(p); },
        inIncognitoContext: false,
        isAllowedIncognitoAccess: api(function () { return false; }),
        getViews: function () { return []; }
      },
      scripting: {
        executeScript: api(function (inj) { return mv3Exec(inj || {}); }),
        insertCSS: api(function (inj) { return mv3Css(inj || {}, false); }),
        removeCSS: api(function (inj) { return mv3Css(inj || {}, true); }),
        registerContentScripts: api(function () { throw new Error('scripting.registerContentScripts is not supported in OyNIx'); })
      }
    };
    chrome.browserAction = action;
    chrome.pageAction = Object.assign({}, action, { show: api(function () {}), hide: api(function () {}) });
    chrome.menus = chrome.contextMenus;

    // ── The rest of the WebExtensions API (Manifest V2 and V3) ──────────
    // Each call goes to OyNIx as "namespace.method" with its arguments; the
    // browser checks the manifest permission (and, for powerful APIs,
    // Settings › Extensions › Developer mode › Allow powerful extension APIs).
    function globMatch(pattern, url) {
      if (pattern === '<all_urls>') return /^(https?|wss?|file|ftp):/.test(url);
      var m = /^(\*|https?|wss?|file|ftp):\/\/([^\/]*)(\/.*)$/.exec(pattern);
      if (!m) return false;
      var u; try { u = new URL(url); } catch (e) { return false; }
      var scheme = u.protocol.slice(0, -1);
      if (m[1] === '*' ? !/^(https?|wss?)$/.test(scheme) : m[1] !== scheme) return false;
      var host = m[2];
      if (host !== '*' && host !== '') {
        if (host.indexOf('*.') === 0) { var base = host.slice(2); if (u.hostname !== base && !u.hostname.endsWith('.' + base)) return false; }
        else if (u.hostname !== host) return false;
      }
      var path = new RegExp('^' + m[3].replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*') + '$');
      return path.test(u.pathname + u.search);
    }
    function H(op) { return api(function () { return tabsCall(op, Array.prototype.slice.call(arguments)); }); }
    function events(ns, names) { var o = {}; names.forEach(function (n) { o[n] = evt(ns + '.' + n); }); return o; }
    var unsupported = function (what) { return api(function () { throw new Error(what + ' is not supported in OyNIx'); }); };

    // Native messaging ports.
    var nativePorts = {};
    function connectNative(host) {
      var id = 'n' + uid();
      var port = { name: host, sender: undefined, onMessage: makeEvent(), onDisconnect: makeEvent(), _closed: false, error: null };
      port.postMessage = function (msg) {
        if (port._closed) throw new Error('Attempting to use a disconnected port object');
        call('nativePost', id, JSON.stringify(msg)).then(function (err) { if (err) close(err); });
      };
      function close(err) {
        if (port._closed) return;
        port._closed = true; delete nativePorts[id];
        port.error = err ? { message: err } : null;
        lastError = err ? { message: err } : null;
        try { port.onDisconnect._fire(port); } finally { lastError = null; }
      }
      port.disconnect = function () { if (!port._closed) { port._closed = true; delete nativePorts[id]; call('nativeDisconnect', id); } };
      port._deliver = function (json, err) { if (json !== null) { try { port.onMessage._fire(JSON.parse(json), port); } catch (e) { console.error(e); } } else close(err || ''); };
      nativePorts[id] = port;
      call('nativeConnect', ctx.ext, id, String(host)).then(function (err) { if (err) close(err); });
      return port;
    }
    me._native = function (portId, json, err) { var p = nativePorts[portId]; if (p) p._deliver(json, err); };

    var declared = [].concat((ctx.manifest && ctx.manifest.permissions) || [], (ctx.manifest && ctx.manifest.optional_permissions) || []);
    function has(p) { return declared.indexOf(p) >= 0; }
    if (ctx.kind !== 'content') {
      Object.assign(chrome.runtime, {
        connectNative: connectNative,
        sendNativeMessage: api(function (host, msg) {
          return new Promise(function (resolve, reject) {
            var p = connectNative(host), done = false;
            p.onMessage.addListener(function (m) { if (done) return; done = true; p.disconnect(); resolve(m); });
            p.onDisconnect.addListener(function () { if (!done) { done = true; reject(new Error((p.error && p.error.message) || 'Native host has exited.')); } });
            p.postMessage(msg);
          });
        }),
        getBrowserInfo: api(function () { return { name: 'OyNIx', vendor: 'OyNIx', version: '5.2.1', buildID: '' }; }),
        getPackageDirectoryEntry: unsupported('runtime.getPackageDirectoryEntry'),
        onBrowserUpdateAvailable: makeEvent(), onRestartRequired: makeEvent()
      });
      if (has('bookmarks')) chrome.bookmarks = Object.assign({
        get: H('bookmarks.get'), getChildren: H('bookmarks.getChildren'), getRecent: H('bookmarks.getRecent'),
        getTree: H('bookmarks.getTree'), getSubTree: H('bookmarks.getSubTree'), search: H('bookmarks.search'),
        create: H('bookmarks.create'), move: H('bookmarks.move'), update: H('bookmarks.update'),
        remove: H('bookmarks.remove'), removeTree: H('bookmarks.removeTree')
      }, events('bookmarks', ['onCreated', 'onRemoved', 'onChanged', 'onMoved', 'onChildrenReordered', 'onImportBegan', 'onImportEnded']));
      if (has('history')) chrome.history = Object.assign({
        search: H('history.search'), getVisits: H('history.getVisits'), addUrl: H('history.addUrl'),
        deleteUrl: H('history.deleteUrl'), deleteRange: H('history.deleteRange'), deleteAll: H('history.deleteAll')
      }, events('history', ['onVisited', 'onVisitRemoved', 'onTitleChanged']));
      if (has('topSites')) chrome.topSites = { get: H('topSites.get') };
      if (has('sessions')) chrome.sessions = Object.assign({ getRecentlyClosed: H('sessions.getRecentlyClosed'), getDevices: H('sessions.getDevices'), restore: H('sessions.restore'), MAX_SESSION_RESULTS: 25 },
        events('sessions', ['onChanged']));
      if (has('downloads')) chrome.downloads = Object.assign({
        download: H('downloads.download'), search: H('downloads.search'), pause: H('downloads.pause'), resume: H('downloads.resume'),
        cancel: H('downloads.cancel'), open: H('downloads.open'), show: H('downloads.show'), showDefaultFolder: H('downloads.showDefaultFolder'),
        erase: H('downloads.erase'), removeFile: H('downloads.removeFile'), getFileIcon: H('downloads.getFileIcon'),
        acceptDanger: api(function () {}), setShelfEnabled: function () {}, setUiOptions: api(function () {})
      }, events('downloads', ['onCreated', 'onChanged', 'onErased', 'onDeterminingFilename']));
      if (has('webNavigation')) chrome.webNavigation = Object.assign({ getFrame: H('webNavigation.getFrame'), getAllFrames: H('webNavigation.getAllFrames') },
        events('webNavigation', ['onBeforeNavigate', 'onCommitted', 'onDOMContentLoaded', 'onCompleted', 'onErrorOccurred', 'onHistoryStateUpdated', 'onReferenceFragmentUpdated', 'onCreatedNavigationTarget', 'onTabReplaced']));
      chrome.commands = Object.assign({ getAll: H('commands.getAll') }, events('commands', ['onCommand']));
      var omniboxSuggestions = {};
      chrome.omnibox = Object.assign({ setDefaultSuggestion: H('omnibox.setDefaultSuggestion') },
        events('omnibox', ['onInputStarted', 'onInputChanged', 'onInputEntered', 'onInputCancelled', 'onDeleteSuggestion']));
      if (has('idle')) chrome.idle = Object.assign({ queryState: H('idle.queryState'), setDetectionInterval: function () {}, getAutoLockDelay: api(function () { return 0; }), IdleState: { ACTIVE: 'active', IDLE: 'idle', LOCKED: 'locked' } },
        events('idle', ['onStateChanged']));
      if (has('tts')) chrome.tts = {
        speak: api(function (text, opts) {
          opts = opts || {};
          if (!('speechSynthesis' in g)) throw new Error('Text to speech is not available on this system');
          if (!opts.enqueue) g.speechSynthesis.cancel();
          var u = new SpeechSynthesisUtterance(String(text));
          if (opts.lang) u.lang = opts.lang; if (opts.rate) u.rate = opts.rate; if (opts.pitch) u.pitch = opts.pitch; if (opts.volume !== undefined) u.volume = opts.volume;
          if (typeof opts.onEvent === 'function') { u.onstart = function () { opts.onEvent({ type: 'start' }); }; u.onend = function () { opts.onEvent({ type: 'end' }); }; u.onerror = function (e) { opts.onEvent({ type: 'error', errorMessage: String(e.error) }); }; }
          g.speechSynthesis.speak(u);
        }),
        stop: function () { if ('speechSynthesis' in g) g.speechSynthesis.cancel(); },
        pause: function () { if ('speechSynthesis' in g) g.speechSynthesis.pause(); },
        resume: function () { if ('speechSynthesis' in g) g.speechSynthesis.resume(); },
        isSpeaking: api(function () { return 'speechSynthesis' in g && g.speechSynthesis.speaking; }),
        getVoices: api(function () { return 'speechSynthesis' in g ? g.speechSynthesis.getVoices().map(function (v) { return { voiceName: v.name, lang: v.lang, remote: !v.localService }; }) : []; }),
        onEvent: makeEvent()
      };
      chrome.management = Object.assign({
        getSelf: H('management.getSelf'), get: H('management.get'), getAll: H('management.getAll'),
        setEnabled: H('management.setEnabled'), uninstall: H('management.uninstall'), uninstallSelf: H('management.uninstallSelf'),
        getPermissionWarningsById: api(function () { return []; }), launchApp: unsupported('management.launchApp')
      }, events('management', ['onEnabled', 'onDisabled', 'onInstalled', 'onUninstalled']));
      if (has('cookies')) chrome.cookies = Object.assign({
        get: H('cookies.get'), getAll: H('cookies.getAll'), set: H('cookies.set'), remove: H('cookies.remove'),
        getAllCookieStores: api(function () { return [{ id: '0', tabIds: [] }]; })
      }, events('cookies', ['onChanged']));
      if (has('browsingData')) chrome.browsingData = {
        remove: H('browsingData.remove'), settings: api(function () { return { options: { since: 0 }, dataToRemove: {}, dataRemovalPermitted: { cache: true, cookies: true, history: true, downloads: true } }; })
      };
      if (chrome.browsingData) ['Cache', 'Cookies', 'History', 'Downloads', 'LocalStorage', 'FormData', 'Passwords', 'IndexedDB', 'ServiceWorkers', 'CacheStorage', 'FileSystems', 'WebSQL', 'AppCache', 'PluginData'].forEach(function (k) {
        chrome.browsingData['remove' + k] = api(function (opts) { var d = {}; d[k.charAt(0).toLowerCase() + k.slice(1)] = true; return tabsCall('browsingData.remove', [opts || {}, d]); });
      });
      var wrEvents = events('webRequest', ['onBeforeRequest', 'onBeforeSendHeaders', 'onSendHeaders', 'onHeadersReceived', 'onAuthRequired', 'onResponseStarted', 'onBeforeRedirect', 'onCompleted', 'onErrorOccurred', 'onActionIgnored']);
      Object.keys(wrEvents).forEach(function (k) {
        var e = wrEvents[k], add = e.addListener;
        // Listeners see every request; "blocking" answers can't change it
        // (Qt WebEngine decides synchronously, before an extension could reply).
        e.addListener = function (f, filter, spec) {
          if (spec && spec.indexOf('blocking') >= 0) console.warn('[OyNIx] webRequest "blocking" listeners are called, but their answers cannot cancel or change requests in OyNIx.');
          var urls = filter && filter.urls, types = filter && filter.types;
          var w = function (d) {
            if (types && types.indexOf(d.type) < 0) return;
            if (urls && !urls.some(function (p) { return globMatch(p, d.url); })) return;
            return f(d);
          };
          w._orig = f; add.call(e, w);
          tabsCall('webRequest.subscribe', []).catch(function (err) { console.warn('[OyNIx]', err.message); });
        };
        e.removeListener = function (f) { e._listeners.slice().forEach(function (w, i) { if (w._orig === f) e._listeners.splice(e._listeners.indexOf(w), 1); }); };
        e.hasListener = function (f) { return e._listeners.some(function (w) { return w._orig === f; }); };
      });
      if (has('webRequest')) chrome.webRequest = Object.assign({ handlerBehaviorChanged: api(function () {}), MAX_HANDLER_BEHAVIOR_CHANGED_CALLS_PER_10_MINUTES: 20, ResourceType: {} }, wrEvents);
      function setting(name) { return { get: H('contentSettings.get:' + name), set: H('contentSettings.set:' + name), clear: H('contentSettings.clear:' + name), getResourceIdentifiers: api(function () { return []; }) }; }
      if (has('contentSettings')) chrome.contentSettings = {};
      ['javascript', 'popups', 'cookies', 'images', 'notifications', 'location', 'camera', 'microphone', 'automaticDownloads', 'plugins', 'unsandboxedPlugins', 'clipboard', 'autoVerify'].forEach(function (n) { chrome.contentSettings[n] = setting(n); });
      if (has('fontSettings')) chrome.fontSettings = Object.assign({
        getFontList: H('fontSettings.getFontList'), getFont: api(function () { return { fontId: '', levelOfControl: 'not_controllable' }; }),
        setFont: api(function () {}), clearFont: api(function () {}),
        getDefaultFontSize: api(function () { return { pixelSize: 16, levelOfControl: 'not_controllable' }; }), setDefaultFontSize: api(function () {}),
        getMinimumFontSize: api(function () { return { pixelSize: 0, levelOfControl: 'not_controllable' }; }), setMinimumFontSize: api(function () {})
      }, events('fontSettings', ['onFontChanged', 'onDefaultFontSizeChanged', 'onMinimumFontSizeChanged']));
      if (has('search')) chrome.search = { query: H('search.query') };
      function privacySetting(key) { return { get: H('privacy.get:' + key), set: H('privacy.set:' + key), clear: H('privacy.clear:' + key), onChange: evt('privacy.onChange:' + key) }; }
      if (has('privacy')) chrome.privacy = {
        network: { networkPredictionEnabled: privacySetting('networkPredictionEnabled'), webRTCIPHandlingPolicy: privacySetting('webRTCIPHandlingPolicy') },
        services: { searchSuggestEnabled: privacySetting('searchSuggestEnabled'), spellingServiceEnabled: privacySetting('spellingServiceEnabled') },
        websites: { doNotTrackEnabled: privacySetting('doNotTrackEnabled'), thirdPartyCookiesAllowed: privacySetting('thirdPartyCookiesAllowed'), hyperlinkAuditingEnabled: privacySetting('hyperlinkAuditingEnabled'), referrersEnabled: privacySetting('referrersEnabled') }
      };
      if (has('proxy')) chrome.proxy = { settings: { get: api(function () { return { value: { mode: 'system' }, levelOfControl: 'not_controllable' }; }), set: unsupported('proxy.settings.set'), clear: api(function () {}) }, onProxyError: makeEvent() };
      if (has('debugger')) chrome.debugger = { attach: unsupported('chrome.debugger'), detach: unsupported('chrome.debugger'), sendCommand: unsupported('chrome.debugger'), getTargets: api(function () { return []; }), onEvent: makeEvent(), onDetach: makeEvent() };
      if (has('identity')) chrome.identity = {
        getRedirectURL: function (path) { return 'https://' + ctx.ext + '.chromiumapp.org/' + String(path || ''); },
        launchWebAuthFlow: H('identity.launchWebAuthFlow'),
        getAuthToken: unsupported('identity.getAuthToken (Google sign-in)'), removeCachedAuthToken: api(function () {}), getProfileUserInfo: api(function () { return { email: '', id: '' }; }),
        onSignInChanged: makeEvent()
      };
      Object.assign(chrome.permissions, { contains: H('permissions.contains'), getAll: H('permissions.getAll'), request: H('permissions.request'), remove: api(function () { return false; }) });
      Object.assign(chrome.tabs, {
        getSelected: api(function (windowId) { return tabsCall('query', { active: true, currentWindow: true }).then(function (t) { return t[0]; }); }),
        getAllInWindow: api(function (windowId) { return tabsCall('query', typeof windowId === 'number' ? { windowId: windowId } : { currentWindow: true }); }),
        captureVisibleTab: H('tabs.captureVisibleTab'), move: H('tabs.move'), highlight: H('tabs.highlight'), discard: H('tabs.discard'),
        getZoom: H('tabs.getZoom'), setZoom: H('tabs.setZoom'),
        getZoomSettings: api(function () { return { mode: 'automatic', scope: 'per-origin', defaultZoomFactor: 1 }; }), setZoomSettings: api(function () {}),
        goBack: H('tabs.goBack'), goForward: H('tabs.goForward'), detectLanguage: H('tabs.detectLanguage'),
        toggleReaderMode: H('tabs.toggleReaderMode'), group: unsupported('tabs.group'), ungroup: unsupported('tabs.ungroup'),
        onSelectionChanged: evt('tabs.onSelectionChanged'), onActiveChanged: evt('tabs.onActiveChanged'), onZoomChange: evt('tabs.onZoomChange')
      });
      Object.assign(chrome.windows, {
        get: H('windows.get'), create: H('windows.create'), update: H('windows.update'), remove: H('windows.remove'),
        WINDOW_ID_NONE: -1, onFocusChanged: evt('windows.onFocusChanged'), onCreated: evt('windows.onCreated'), onRemoved: evt('windows.onRemoved')
      });
      Object.assign(chrome.extension, {
        getBackgroundPage: function () { return ctx.kind === 'background' ? g : null; },
        getViews: function (f) { return (!f || !f.type || f.type === ctx.kind) ? [g] : []; },
        getExtensionTabs: function () { return []; },
        isAllowedFileSchemeAccess: api(function () { return true; }),
        setUpdateUrlData: function () {}, lastError: undefined,
        sendRequest: chrome.runtime.sendMessage, onRequest: ev.message, onRequestExternal: makeEvent()
      });
    }

    me._deliver = function (ext, target, p) {
      if (ext !== ctx.ext || p.from === me.id || seen[p.id]) return;
      if (target.indexOf('inst:') === 0) {
        if (target !== 'inst:' + me.id) return;
        seen[p.id] = true;
        var port = ports[p.portMsg || p.portClose || p.portAck];
        if (!port) return;
        if (p.portAck) { port._peer = p.from; port._queue.splice(0).forEach(function (q) { call('post', ctx.ext, 'inst:' + p.from, JSON.stringify(q)); }); }
        else if (p.portMsg) port.onMessage._fire(p.msg, port);
        else if (p.portClose) { port._closed = true; delete ports[p.portClose]; port.onDisconnect._fire(port); }
        return;
      }
      if (p.connect) {
        if (target === 'extension' && ctx.kind === 'content') return;
        if (target.indexOf('tab:') === 0 && !(ctx.kind === 'content' && target === 'tab:' + g.__oynixTabId)) return;
        if (!ev.connect._listeners.length) return;
        seen[p.id] = true;
        var np = makePort(p.connect.port, p.connect.name, p.from, null, p.sender);
        call('post', ctx.ext, 'inst:' + p.from, JSON.stringify({ id: uid(), from: me.id, portAck: p.connect.port }));
        ev.connect._fire(np);
        return;
      }
      if (target === 'extension' && ctx.kind === 'content') return;
      if (target.indexOf('tab:') === 0 && !(ctx.kind === 'content' && target === 'tab:' + g.__oynixTabId)) return;
      var ls = ev.message._listeners;
      if (!ls.length) return;
      seen[p.id] = true;
      var done = false, keep = false;
      function respond(v) { if (done) return; done = true; call('reply', ctx.ext, p.id, JSON.stringify({ value: v === undefined ? null : v })); }
      ls.slice().forEach(function (f) {
        var r;
        try { r = f(p.msg, p.sender, respond); } catch (e) { console.error(e); }
        if (r === true) keep = true;
        else if (r && typeof r.then === 'function') { keep = true; r.then(respond, function (e) { if (!done) { done = true; call('reply', ctx.ext, p.id, JSON.stringify({ error: String(e && e.message || e) })); } }); }
      });
      if (!keep && !done) call('reply', ctx.ext, p.id, JSON.stringify({ none: true }));
    };
    me._reply = function (ext, id, resp) {
      var w = pending[id];
      if (!w || ext !== ctx.ext) return;
      var r = JSON.parse(resp);
      w.heard = true;
      clearInterval(w.retry);
      if (r.none) {
        // Another receiver may still answer; resolve undefined shortly.
        if (!w.grace) w.grace = setTimeout(function () { if (pending[id]) { clearTimeout(w.timer); delete pending[id]; w.resolve(undefined); } }, 150);
        return;
      }
      clearTimeout(w.timer); if (w.grace) clearTimeout(w.grace);
      delete pending[id];
      if (r.error) w.reject(new Error(r.error)); else w.resolve(r.value === null ? undefined : r.value);
    };
    me._storage = function (ext, name, changes) {
      if (ext !== ctx.ext) return;
      if (areas[name]) areas[name].onChanged._fire(changes);
      ev.storage._fire(changes, name);
    };
    me._tabs = function (id, result) {
      var w = tabsPending[id];
      if (!w) return;
      delete tabsPending[id];
      var r = JSON.parse(result);
      if (r.error) w.reject(new Error(r.error)); else w.resolve(r.value === null ? undefined : r.value);
    };
    me._event = function (name, a) {
      if (ctx.kind === 'content') return;
      // Events addressed to one extension: [extId, ...args].
      if (name.indexOf('ext:') === 0) {
        if (a[0] !== ctx.ext) return;
        name = name.slice(4); a = a.slice(1);
      }
      if (name === 'omnibox.onInputChanged') a = a.concat([function () {}]);
      if (evts[name]) evts[name]._fire.apply(null, a);
      var e = { 'tabs.onCreated': ev.tabCreated, 'tabs.onUpdated': ev.tabUpdated, 'tabs.onActivated': ev.tabActivated, 'tabs.onRemoved': ev.tabRemoved }[name];
      if (e) e._fire.apply(null, a);
      if (name === 'action.onClicked' && a[0] === ctx.ext) ev.actionClicked._fire(a[1]);
      if (name === 'contextMenus.onClicked' && a[0] === ctx.ext) {
        var cb = menuClicks[a[1].menuItemId];
        if (cb) { try { cb(a[1], a[2]); } catch (e) { console.error(e); } }
        ev.menuClicked._fire(a[1], a[2]);
      }
    };

    runtimes.push(me);
    channel();
    if (ctx.kind === 'background') {
      setTimeout(function () {
        if (ctx.firstRun) ev.installed._fire({ reason: 'install' });
        else ev.startup._fire();
      }, 300);
    }
    return chrome;
  }

  // Content scripts: run `fn(chrome, browser)` with an API bound to the extension.
  g.__oynixExt = function (ctx, fn) {
    var api = make(ctx);
    try { fn.call(g, api, api); } catch (e) { console.error('[' + (ctx.manifest && ctx.manifest.name) + '] content script error:', e); }
  };

  // Extension pages (background, popup, options): install `chrome` globally.
  g.__oynixExtPage = function (byId) {
    if (location.protocol !== 'chrome-extension:') return;
    var ctx = byId[location.host];
    if (!ctx) return;
    var api = make(ctx);
    var existing = g.chrome && typeof g.chrome === 'object' ? g.chrome : {};
    Object.keys(api).forEach(function (k) { if (!existing[k] || !existing[k].id && k === 'runtime') existing[k] = api[k]; });
    try { g.chrome = existing; } catch (e) {}
    try { Object.defineProperty(g, 'browser', { value: existing, configurable: true, writable: true }); } catch (e) { g.browser = existing; }
    if (ctx.kind === 'background' && typeof g.importScripts !== 'function') {
      g.importScripts = function () { for (var i = 0; i < arguments.length; i++) document.write('<script src="' + String(arguments[i]).replace(/"/g, '&quot;') + '"><\/script>'); };
    }
  };
})(typeof globalThis !== 'undefined' ? globalThis : this);
"#;
