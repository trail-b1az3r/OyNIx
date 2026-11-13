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
        new QWebChannel(qt.webChannelTransport, function (ch) {
          bridge = ch.objects.oynixExtensions;
          bridge.message.connect(function (ext, target, payload) { var p = JSON.parse(payload); runtimes.forEach(function (r) { r._deliver(ext, target, p); }); });
          bridge.replied.connect(function (ext, id, resp) { runtimes.forEach(function (r) { r._reply(ext, id, resp); }); });
          bridge.storageChanged.connect(function (ext, area, changes) { var c = JSON.parse(changes); runtimes.forEach(function (r) { r._storage(ext, area, c); }); });
          bridge.tabsResult.connect(function (id, result) { runtimes.forEach(function (r) { r._tabs(id, result); }); });
          bridge.extEvent.connect(function (name, args) { var a = JSON.parse(args); runtimes.forEach(function (r) { r._event(name, a); }); });
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
    var ev = {
      message: makeEvent(), installed: makeEvent(), startup: makeEvent(), connect: makeEvent(),
      storage: makeEvent(), tabCreated: makeEvent(), tabUpdated: makeEvent(), tabActivated: makeEvent(),
      tabRemoved: makeEvent(), alarm: makeEvent(), actionClicked: makeEvent()
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
        connect: function () { var p = { name: '', onMessage: makeEvent(), onDisconnect: makeEvent(), postMessage: function () {}, disconnect: function () {} }; return p; },
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
        executeScript: api(function () { throw new Error('tabs.executeScript is not supported in OyNIx'); }),
        insertCSS: api(function () { throw new Error('tabs.insertCSS is not supported in OyNIx'); }),
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
      contextMenus: { create: function () { return uid(); }, update: stub(undefined), remove: stub(undefined), removeAll: stub(undefined), onClicked: makeEvent() },
      notifications: {
        create: api(function (a, b) { var o = typeof a === 'object' ? a : b; try { if (g.Notification && Notification.permission === 'granted') new Notification(o.title || '', { body: o.message || '' }); } catch (e) {} return typeof a === 'string' ? a : uid(); }),
        clear: stub(true), getAll: stub({}), onClicked: makeEvent(), onClosed: makeEvent(), onButtonClicked: makeEvent()
      },
      extension: {
        getURL: function (p) { return chrome.runtime.getURL(p); },
        inIncognitoContext: false,
        isAllowedIncognitoAccess: api(function () { return false; }),
        getViews: function () { return []; }
      },
      scripting: {
        executeScript: api(function () { throw new Error('scripting.executeScript is not supported in OyNIx'); }),
        insertCSS: api(function () { throw new Error('scripting.insertCSS is not supported in OyNIx'); }),
        registerContentScripts: api(function () { throw new Error('scripting.registerContentScripts is not supported in OyNIx'); })
      }
    };
    chrome.browserAction = action;
    chrome.pageAction = action;
    chrome.menus = chrome.contextMenus;

    me._deliver = function (ext, target, p) {
      if (ext !== ctx.ext || p.from === me.id || seen[p.id]) return;
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
      var e = { 'tabs.onCreated': ev.tabCreated, 'tabs.onUpdated': ev.tabUpdated, 'tabs.onActivated': ev.tabActivated, 'tabs.onRemoved': ev.tabRemoved }[name];
      if (e) e._fire.apply(null, a);
      if (name === 'action.onClicked' && a[0] === ctx.ext) ev.actionClicked._fire(a[1]);
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
