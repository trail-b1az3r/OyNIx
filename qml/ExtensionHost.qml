import QtQuick
import QtQuick.Window
import QtWebEngine
import OyNIx

// Bridges the Rust extension registry and Qt WebEngine's extension manager
// (Qt ≥ 6.10). Chrome and Firefox extensions are loaded into the default
// profile; OyNIx-format extensions are injected per page by WebTab.
QtObject {
    id: host
    property var app
    readonly property var manager: app && app.defaultProfile ? app.defaultProfile.extensionManager : null
    readonly property bool supported: manager !== null && manager !== undefined

    // OyNIx id -> webEngineExtension (QWebEngineExtensionInfo)
    property var infos: ({})
    property var errors: ({})
    property int revision: 0
    property var pendingReload: ({})
    property var pendingRemove: ({})

    function norm(p) { return String(p || "").replace(/\/+$/, "") }

    function entries() { return JSON.parse(Core.extensions()) }

    function entryForPath(path) {
        var list = entries(), p = norm(path)
        for (var i = 0; i < list.length; i++) if (norm(list[i].path) === p) return list[i]
        return null
    }

    function setInfo(id, info) {
        var m = Object.assign({}, infos)
        if (info) m[id] = info; else delete m[id]
        infos = m
        revision++
    }

    function setError(id, err) {
        var m = Object.assign({}, errors)
        if (err) m[id] = err; else delete m[id]
        errors = m
        revision++
    }

    property Connections managerEvents: Connections {
        target: host.supported ? host.manager : null
        function onLoadFinished(info) {
            var e = host.entryForPath(info.path)
            if (!e) return
            if (info.error) { host.setError(e.id, info.error); console.warn("OyNIx: extension", e.name, "failed:", info.error) }
            else host.setError(e.id, "")
            host.setInfo(e.id, info.isLoaded ? info : null)
            if (info.isLoaded) {
                Core.setExtensionWebId(e.id, info.id)
                host.startBackground(e.id)
            }
        }
        function onUnloadFinished(info) {
            var e = host.entryForPath(info.path)
            var id = e ? e.id : null
            if (!id) {
                for (var k in host.infos) if (host.norm(host.infos[k].path) === host.norm(info.path)) id = k
            }
            if (!id) return
            host.stopBackground(id)
            host.setInfo(id, null)
            if (host.pendingReload[id]) {
                delete host.pendingReload[id]
                if (e && e.enabled) host.manager.loadExtension(e.path)
            }
            if (host.pendingRemove[id]) {
                delete host.pendingRemove[id]
                if (e && e.webId) host.app.extBridge.forget(e.webId)
                Core.removeExtension(id)
            }
        }
    }

    property Connections registryEvents: Connections {
        target: Core
        function onExtensionsRevisionChanged() { host.sync() }
    }

    function start() { sync() }

    // Make the loaded set match the registry's enabled set.
    function sync() {
        if (!supported) return
        var list = entries(), wanted = {}
        for (var i = 0; i < list.length; i++) {
            var e = list[i]
            if (e.kind === "oynix") continue
            wanted[e.id] = true
            var info = infos[e.id]
            if (e.enabled && !info && !errors[e.id]) manager.loadExtension(e.path)
            else if (!e.enabled && info) manager.unloadExtension(info)
        }
        for (var id in infos) if (!wanted[id] && !pendingRemove[id]) manager.unloadExtension(infos[id])
    }

    /// Install from a file/folder URL or path. Returns {ok, error, extension}.
    function install(pathOrUrl) {
        var r = JSON.parse(Core.installExtension(String(pathOrUrl)))
        if (r.ok && r.extension.kind !== "oynix" && supported) {
            setError(r.extension.id, "")
            if (infos[r.extension.id]) { pendingReload[r.extension.id] = true; manager.unloadExtension(infos[r.extension.id]) }
            else sync()
        }
        return r
    }

    function remove(id) {
        if (infos[id] && supported) { pendingRemove[id] = true; manager.unloadExtension(infos[id]) }
        else Core.removeExtension(id)
    }

    function setEnabled(id, on) {
        setError(id, "")
        Core.setExtensionEnabled(id, on)
    }

    function popupUrl(id) { var i = infos[id]; return i ? i.actionPopupUrl.toString() : "" }
    function webId(id) { var i = infos[id]; return i ? i.id : "" }
    function isLoaded(id) { return !!infos[id] }
    function errorFor(id) { return errors[id] || "" }

    function optionsUrl(e) {
        var w = webId(e.id)
        return w && e.optionsPage ? "chrome-extension://" + w + "/" + e.optionsPage.replace(/^\//, "") : ""
    }

    // ── Script installation (shared by tabs, popups, options, background) ──
    function installScripts(view, list) {
        for (var i = 0; i < list.length; i++) {
            var it = list[i]
            var sc = WebEngine.script()
            sc.name = it.name
            sc.sourceCode = it.source          // also parses ==UserScript== headers
            if (!it.greasemonkey)
                sc.injectionPoint = (it.start || it.main) ? WebEngineScript.DocumentCreation : WebEngineScript.DocumentReady
            sc.worldId = it.main ? WebEngineScript.MainWorld : WebEngineScript.ApplicationWorld
            sc.runsOnSubFrames = it.allFrames !== undefined ? !!it.allFrames : (it.main && !it.greasemonkey)
            view.userScripts.insert(sc)
        }
    }

    readonly property bool hasWebExtensions: {
        revision; Core.extensionsRevision
        for (var k in infos) return true
        return false
    }

    // ── Badges (action.setBadgeText) ──
    property var badges: ({})
    property Connections bridgeEvents: Connections {
        target: host.app ? host.app.extBridge : null
        function onBadgeChanged(ext, text, color, title) {
            var m = Object.assign({}, host.badges)
            m[ext] = { text: text, color: color, title: title }
            host.badges = m
        }
        function onOptionsRequested(ext) {
            var list = host.entries()
            for (var i = 0; i < list.length; i++) if (list[i].webId === ext) { host.openOptions(list[i]); return }
        }
        function onTabsRequest(ext, id, op, args) { host.handleTabs(ext, id, op, args) }
    }
    function badgeFor(e) { return badges[e.webId] || null }

    // ── tabs.* implementation ──
    function allTabs() {
        var out = []
        for (var w = 0; w < app.windows.length; w++) {
            var win = app.windows[w]
            if (win.privateMode) continue
            for (var i = 0; i < win.tabs.count; i++) out.push(tabInfo(win, i))
        }
        return out
    }

    function tabInfo(win, i) {
        if (!win || i < 0 || i >= win.tabs.count) return null
        var t = win.tabs.get(i)
        return {
            id: t.uid, index: i, windowId: win.windowId, url: t.url, title: t.title || "",
            favIconUrl: t.icon || "", active: i === win.currentIndex, highlighted: i === win.currentIndex,
            pinned: !!t.pinned, audible: !!t.audible, discarded: !!t.suspended, incognito: false,
            mutedInfo: { muted: !!t.muted }, status: t.loading ? "loading" : "complete", selected: i === win.currentIndex
        }
    }

    function findTab(id) {
        for (var w = 0; w < app.windows.length; w++) {
            var win = app.windows[w]
            for (var i = 0; i < win.tabs.count; i++) if (win.tabs.get(i).uid === id) return { win: win, index: i }
        }
        return null
    }

    function focusedWindow() {
        for (var w = 0; w < app.windows.length; w++) if (app.windows[w].active && !app.windows[w].privateMode) return app.windows[w]
        for (var k = app.windows.length - 1; k >= 0; k--) if (!app.windows[k].privateMode) return app.windows[k]
        return null
    }

    function globMatch(pattern, url) {
        if (pattern === "<all_urls>") return /^(https?|file|ftp):/.test(url)
        var re = "^" + pattern.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*") + "$"
        return new RegExp(re).test(url)
    }

    function handleTabs(ext, id, op, argsJson) {
        var args = JSON.parse(argsJson || "null")
        var value = null, error = ""
        try {
            if (op === "query") {
                var q = args || {}, fw = focusedWindow()
                value = allTabs().filter(function (t) {
                    if (q.active !== undefined && t.active !== q.active) return false
                    if ((q.currentWindow || q.lastFocusedWindow) && (!fw || t.windowId !== fw.windowId)) return false
                    if (q.windowId !== undefined && q.windowId !== -2 && t.windowId !== q.windowId) return false
                    if (q.pinned !== undefined && t.pinned !== q.pinned) return false
                    if (q.audible !== undefined && t.audible !== q.audible) return false
                    if (q.url) { var pats = Array.isArray(q.url) ? q.url : [q.url]; if (!pats.some(function (p) { return host.globMatch(p, t.url) })) return false }
                    if (q.title && !host.globMatch(q.title, t.title)) return false
                    return true
                })
            } else if (op === "get") {
                var f = findTab(args); if (!f) throw "No tab with id: " + args
                value = tabInfo(f.win, f.index)
            } else if (op === "create") {
                var p = args || {}
                var url = p.url ? (/^[a-z-]+:/.test(p.url) ? p.url : "chrome-extension://" + ext + "/" + p.url.replace(/^\//, "")) : "oynix://start"
                if (p.newWindow) { var nw = app.newWindow({ tabs: [{ url: url }] }); value = tabInfo(nw, 0) }
                else {
                    var w = focusedWindow() || app.newWindow({})
                    var idx = w.openTab(url, { background: p.active === false, pinned: !!p.pinned })
                    value = tabInfo(w, idx)
                }
            } else if (op === "update") {
                var target = args.tabId === null ? (function () { var fw2 = focusedWindow(); return fw2 ? { win: fw2, index: fw2.currentIndex } : null })() : findTab(args.tabId)
                if (!target) throw "No tab with id: " + args.tabId
                var pr = args.props || {}
                if (pr.url) { target.win.activate(target.index); target.win.tabs.setProperty(target.index, "suspended", false); var it = target.win.currentTab; if (it) it.load(pr.url) }
                if (pr.active) target.win.activate(target.index)
                if (pr.muted !== undefined) target.win.tabs.setProperty(target.index, "muted", !!pr.muted)
                if (pr.pinned !== undefined && !!pr.pinned !== !!target.win.tabs.get(target.index).pinned) target.win.togglePin(target.index)
                value = tabInfo(target.win, target.index)
            } else if (op === "remove") {
                (args || []).forEach(function (tid) { var r = host.findTab(tid); if (r) r.win.closeTab(r.index) })
            } else if (op === "reload") {
                var rt = args === null ? (function () { var fw3 = focusedWindow(); return fw3 ? { win: fw3, index: fw3.currentIndex } : null })() : findTab(args)
                if (rt) { var item = rt.win.tabItem(rt.index); if (item && item.view) item.view.reload() }
            } else if (op === "duplicate") {
                var d = findTab(args); if (!d) throw "No tab with id: " + args
                d.win.duplicateTab(d.index); value = tabInfo(d.win, d.win.currentIndex)
            } else if (op === "window" || op === "windows") {
                var mk = function (w2) { return { id: w2.windowId, focused: w2.active, incognito: w2.privateMode, type: "normal", state: "normal", alwaysOnTop: false, width: w2.width, height: w2.height } }
                value = op === "window" ? (focusedWindow() ? mk(focusedWindow()) : null) : app.windows.filter(function (w3) { return !w3.privateMode }).map(mk)
            } else {
                throw "Unsupported tabs operation: " + op
            }
        } catch (e) { error = String(e) }
        app.extBridge.tabsReply(id, JSON.stringify(error ? { error: error } : { value: value === undefined ? null : value }))
    }

    function tabEvent(name, args) {
        if (!hasWebExtensions || !app.extBridge) return
        app.extBridge.dispatch(name, JSON.stringify(args))
    }

    // ── Options pages ──
    property Component pageWindowComponent: Component {
        Window {
            id: pw
            property string url
            width: 760; height: 620
            visible: true
            title: pageView.title
            color: Theme.window
            WebEngineView {
                id: pageView
                anchors.fill: parent
                profile: host.app.defaultProfile
                webChannel: host.app.extChannel
                webChannelWorld: WebEngineScript.MainWorld
                Component.onCompleted: { host.installScripts(pageView, JSON.parse(Core.extensionPageScripts("page"))); url = pw.url }
                // Extension pages may only navigate within the extension.
                onNavigationRequested: function (request) {
                    var u = request.url.toString()
                    if (u.indexOf("chrome-extension://") !== 0 && request.isMainFrame) {
                        request.reject()
                        var w = host.focusedWindow(); if (w) w.openTab(u, {})
                    }
                }
                onNewWindowRequested: function (request) { var w = host.focusedWindow(); if (w) w.openTab(request.requestedUrl.toString(), {}) }
            }
            onClosing: destroy()
        }
    }

    function openOptions(e) {
        var u = optionsUrl(e)
        if (u) pageWindowComponent.createObject(null, { url: u })
    }

    // ── Background contexts (hidden pages) ──
    property var backgrounds: ({})
    property Window backgroundWindow: Window { visible: false; width: 1; height: 1; flags: Qt.Tool }
    property Component backgroundComponent: Component {
        WebEngineView {
            id: bg
            property string startUrl
            width: 1; height: 1
            profile: host.app.defaultProfile
            webChannel: host.app.extChannel
            webChannelWorld: WebEngineScript.MainWorld
            Component.onCompleted: { host.installScripts(bg, JSON.parse(Core.extensionPageScripts("background"))); url = startUrl }
            onLoadingChanged: function (info) { if (info.status === WebEngineView.LoadFailedStatus) console.warn("OyNIx: extension background failed:", info.errorString, info.url) }
            onJavaScriptConsoleMessage: function (level, message, line, source) {
                if (level >= WebEngineView.WarningMessageLevel || Core.developerMode) console.warn("[extension background]", message, source + ":" + line)
            }
            onNavigationRequested: function (request) { if (request.url.toString().indexOf("chrome-extension://") !== 0 && request.isMainFrame) request.reject() }
        }
    }

    function startBackground(id) {
        var list = entries()
        for (var i = 0; i < list.length; i++) {
            var e = list[i]
            if (e.id !== id || !e.backgroundPage || backgrounds[id]) continue
            var w = webId(id)
            if (!w) return
            // Read first-run before creating the page so onInstalled fires once.
            Core.extensionsRevision
            var v = backgroundComponent.createObject(backgroundWindow.contentItem, { startUrl: "chrome-extension://" + w + "/" + e.backgroundPage })
            var m = Object.assign({}, backgrounds); m[id] = v; backgrounds = m
            Qt.callLater(function () { Core.consumeFirstRun(id) })
        }
    }

    function stopBackground(id) {
        var v = backgrounds[id]
        if (!v) return
        var m = Object.assign({}, backgrounds); delete m[id]; backgrounds = m
        v.destroy()
    }
}
