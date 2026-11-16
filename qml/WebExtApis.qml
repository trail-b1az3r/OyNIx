import QtQuick
import QtWebEngine
import OyNIx

// The browser side of the WebExtensions API beyond tabs: bookmarks,
// history, downloads, cookies, sessions and the rest. Calls arrive from
// the extension runtime as "namespace.method" with an argument array.
//
// Every call is checked against the extension's manifest permissions;
// powerful APIs (marked * below) also need Settings › Extensions ›
// Developer mode › Allow powerful extension APIs.
QtObject {
    id: apis
    property var host
    property var app

    // namespace -> manifest permission ("*" prefix: powerful)
    readonly property var needs: ({
        bookmarks: "bookmarks", history: "history", topSites: "topSites", sessions: "sessions",
        downloads: "downloads", webNavigation: "webNavigation", idle: "idle", contentSettings: "contentSettings",
        fontSettings: "fontSettings", search: "search", privacy: "privacy", identity: "identity",
        cookies: "*cookies", browsingData: "*browsingData", webRequest: "*webRequest"
    })

    function check(info, op) {
        var ns = op.split(".")[0], method = op.split(".")[1].split(":")[0]
        var need = needs[ns]
        if (ns === "management" && ["setEnabled", "uninstall", "get", "getAll"].indexOf(method) >= 0) need = "*management"
        if (ns === "tabs" && method === "captureVisibleTab") {
            if (info.permissions.indexOf("activeTab") < 0 && info.origins.indexOf("<all_urls>") < 0 && info.origins.indexOf("*://*/*") < 0)
                throw "tabs.captureVisibleTab needs the \"<all_urls>\" or \"activeTab\" permission"
            return
        }
        if (!need) return
        var powerful = need.charAt(0) === "*"
        var perm = powerful ? need.slice(1) : need
        if (info.permissions.indexOf(perm) < 0) throw "The \"" + perm + "\" permission is required (add it to the manifest)"
        if (powerful && !info.powerful) throw ns + " is a powerful API: turn on Settings › Extensions › Developer mode, then Allow powerful extension APIs"
    }

    // ── helpers ──
    function reply(id, value, error) { app.extBridge.tabsReply(id, JSON.stringify(error ? { error: String(error) } : { value: value === undefined ? null : value })) }
    function currentTab() { var w = host.focusedWindow(); return w ? { win: w, index: w.currentIndex, item: w.tabItem(w.currentIndex) } : null }
    function tabById(tid) {
        if (tid === undefined || tid === null) return currentTab()
        var f = host.findTab(tid)
        return f ? { win: f.win, index: f.index, item: f.win.tabItem(f.index) } : null
    }
    function windowById(wid) {
        if (wid === undefined || wid === null || wid === -2) return host.focusedWindow()
        for (var i = 0; i < app.windows.length; i++) if (app.windows[i].windowId === wid) return app.windows[i]
        return null
    }
    function hostOf(u) { return Core.hostOf(String(u || "")) }
    function patternHost(p) {
        var m = /^[^:]+:\/\/(\*\.)?([^\/:*]+)/.exec(String(p || ""))
        return m ? m[2] : ""
    }

    // ── bookmarks: OyNIx folders as a Chrome-style tree ──
    // "0" root → "1" Favorites bar (folder "favorites"), "2" Other
    // bookmarks (no folder) → "f:<name>" named folders.
    function allBookmarks() {
        var out = [], folders = ["", "favorites"].concat(JSON.parse(Core.bookmarkFolders()))
        var seen = {}
        folders.forEach(function (f) { if (seen[f]) return; seen[f] = true; JSON.parse(Core.bookmarks(f)).forEach(function (b) { out.push(b) }) })
        return out
    }
    function folderId(folder) { return folder === "favorites" ? "1" : folder ? "f:" + folder : "2" }
    function folderOf(parentId) {
        if (parentId === "1") return "favorites"
        if (!parentId || parentId === "2" || parentId === "0") return ""
        if (String(parentId).indexOf("f:") === 0) return String(parentId).slice(2)
        throw "Can't find parent bookmark node: " + parentId
    }
    function bnode(b, index) {
        return { id: String(b.id), parentId: folderId(b.folder || ""), index: index, title: b.title || "", url: b.url, dateAdded: (b.added || 0) * 1000 }
    }
    function children(id) {
        var all = allBookmarks()
        if (id === "0") return [folderNode("1"), folderNode("2")]
        var folder = folderOf(id)
        var kids = all.filter(function (b) { return (b.folder || "") === folder }).map(bnode)
        if (id === "2") JSON.parse(Core.bookmarkFolders()).filter(function (f) { return f && f !== "favorites" }).forEach(function (f) { kids.push(folderNode("f:" + f)) })
        return kids.map(function (k, i) { k.index = i; return k })
    }
    function folderNode(id) {
        var title = id === "0" ? "" : id === "1" ? "Favorites bar" : id === "2" ? "Other bookmarks" : String(id).slice(2)
        return { id: id, parentId: id === "0" ? undefined : (id === "1" || id === "2" ? "0" : "2"), title: title, dateAdded: 0, index: 0 }
    }
    function subtree(id) {
        var n = (id === "0" || id === "1" || id === "2" || String(id).indexOf("f:") === 0) ? folderNode(id) : null
        if (!n) { var b = allBookmarks().filter(function (x) { return String(x.id) === String(id) })[0]; if (!b) throw "Can't find bookmark for id."; return bnode(b, 0) }
        n.children = children(id).map(function (c) { return c.url ? c : subtree(c.id) })
        return n
    }
    function bookmarkById(id) {
        var b = allBookmarks().filter(function (x) { return String(x.id) === String(id) })[0]
        if (!b) throw "Can't find bookmark for id."
        return b
    }

    // ── history ──
    function historyItem(h) {
        return { id: String(Core.hostOf(h.url)) + ":" + h.url.length, url: h.url, title: h.title || "", lastVisitTime: (h.lastVisit || 0) * 1000, visitCount: h.visits || 1, typedCount: 0 }
    }

    // ── downloads ──
    function dlState(d) {
        return d.state === WebEngineDownloadRequest.DownloadCompleted ? "complete"
             : d.state === WebEngineDownloadRequest.DownloadInProgress ? "in_progress" : "interrupted"
    }
    function dlItem(d) {
        return {
            id: d.id, url: d.url.toString(), finalUrl: d.url.toString(), filename: d.downloadDirectory + "/" + d.downloadFileName,
            mime: d.mimeType, state: dlState(d), paused: d.isPaused, canResume: d.isPaused,
            bytesReceived: d.receivedBytes, totalBytes: d.totalBytes, fileSize: d.totalBytes,
            danger: "safe", exists: true, incognito: false, startTime: new Date().toISOString(),
            error: d.state === WebEngineDownloadRequest.DownloadInterrupted ? d.interruptReasonString : undefined
        }
    }
    function dlById(id) {
        var d = app.downloads.filter(function (x) { return x.id === id })[0]
        if (!d) throw "Invalid download id " + id
        return d
    }
    function dlMatch(q) {
        q = q || {}
        return app.downloads.filter(function (d) {
            var it = dlItem(d)
            if (q.id !== undefined && it.id !== q.id) return false
            if (q.url && it.url !== q.url) return false
            if (q.state && it.state !== q.state) return false
            if (q.filenameRegex && !new RegExp(q.filenameRegex).test(it.filename)) return false
            if (q.query && !q.query.every(function (t) { return (it.url + " " + it.filename).toLowerCase().indexOf(String(t).toLowerCase()) >= 0 })) return false
            return true
        }).slice(0, q.limit || 1000)
    }
    // downloads.download: load the URL in an invisible view; the download
    // handler reports back when the file starts.
    property var pendingDownloads: []
    property Component hiddenView: Component {
        WebEngineView {
            width: 1; height: 1; visible: false
            profile: apis.app.defaultProfile
        }
    }
    function startDownload(url, cb) {
        var v = hiddenView.createObject(apis.app.windows.length ? apis.app.windows[0].contentItem : null)
        if (!v) { cb(null, "Couldn't start the download"); return }
        pendingDownloads = pendingDownloads.concat([{ url: url, cb: cb, view: v, t: Date.now() }])
        v.url = url
        v.loadingChanged.connect(function (info) {
            if (info.status === WebEngineView.LoadSucceededStatus) {
                // It loaded as a page instead of downloading.
                apis.finishDownload(url, null, "This address is a web page, not a file, so it can't be downloaded")
            } else if (info.status === WebEngineView.LoadFailedStatus && info.errorCode !== -3) {
                apis.finishDownload(url, null, info.errorString)
            }
        })
    }
    function finishDownload(url, d, error) {
        var keep = []
        pendingDownloads.forEach(function (p) {
            if (p.url === url || (d && p.url === d.url.toString())) { p.cb(d ? d.id : null, error); Qt.callLater(function () { p.view.destroy() }) }
            else keep.push(p)
        })
        pendingDownloads = keep
    }
    // Called by Main.handleDownload for every new download.
    function downloadStarted(d) {
        var u = d.url.toString()
        if (pendingDownloads.some(function (p) { return p.url === u })) finishDownload(u, d, "")
        host.tabEvent("downloads.onCreated", [dlItem(d)])
        d.stateChanged.connect(function () { host.tabEvent("downloads.onChanged", [{ id: d.id, state: { current: apis.dlState(d) } }]) })
    }

    // ── identity.launchWebAuthFlow ──
    property var authFlows: ({})
    // WebTab asks before navigating: is this the end of an auth flow?
    function interceptAuthRedirect(url) {
        for (var k in authFlows) {
            if (url.indexOf(k) === 0) { var f = authFlows[k]; var m = Object.assign({}, authFlows); delete m[k]; authFlows = m; f(url); return true }
        }
        return false
    }

    // ── idle ──
    property real lastInput: Date.now()
    property string idleState: "active"
    function idleNow(threshold) { return Date.now() - lastInput > Math.max(15, threshold || 60) * 1000 ? "idle" : "active" }
    property Timer idleTimer: Timer {
        interval: 15000; running: apis.host && apis.host.hasWebExtensions; repeat: true
        onTriggered: { var s = apis.idleNow(60); if (s !== apis.idleState) { apis.idleState = s; apis.host.tabEvent("idle.onStateChanged", [s]) } }
    }

    // ── events from outside QML (webRequest, cookies) ──
    property bool observeRequests: false
    property bool observeCookies: false
    property Timer drainTimer: Timer {
        interval: 250; repeat: true
        running: apis.observeRequests || apis.observeCookies
        onTriggered: {
            var evs = JSON.parse(Core.webextDrainEvents())
            for (var i = 0; i < evs.length; i++) apis.app.extBridge.dispatch(evs[i][0], JSON.stringify(evs[i][1]))
        }
    }
    // Turning the setting off stops observation straight away.
    property Connections settingsWatch: Connections {
        target: Core
        function onSettingsJsonChanged() { Core.webextSetObserving(apis.observeRequests, apis.observeCookies) }
    }
    function setObserving(requests, cookies) {
        observeRequests = observeRequests || requests
        observeCookies = observeCookies || cookies
        Core.webextSetObserving(observeRequests, observeCookies)
    }

    function contentSetting(name, hostName) {
        var site = JSON.parse(Core.siteSettings(hostName))
        var st = JSON.parse(Core.settingsJson)
        if (name === "javascript") return (site.javascript !== undefined ? site.javascript : st.javascript) ? "allow" : "block"
        if (name === "popups") return site.popups ? "allow" : "block"
        if (name === "cookies") return st.blockThirdPartyCookies ? "session_only" : "allow"
        return "ask"
    }

    // ── dispatch ──
    function handle(ext, id, op, args) {
        var info = JSON.parse(Core.webextInfo(ext) || "null")
        if (!info) return reply(id, null, "Unknown extension")
        try { check(info, op) } catch (e) { return reply(id, null, e) }
        args = args || []
        var a0 = args[0], a1 = args[1]
        var t, w, list
        try {
            switch (op) {
            // bookmarks
            case "bookmarks.getTree": return reply(id, [subtree("0")])
            case "bookmarks.getSubTree": return reply(id, [subtree(String(a0))])
            case "bookmarks.getChildren": return reply(id, children(String(a0)))
            case "bookmarks.get":
                return reply(id, (Array.isArray(a0) ? a0 : [a0]).map(function (bid) {
                    bid = String(bid)
                    return (bid === "0" || bid === "1" || bid === "2" || bid.indexOf("f:") === 0) ? folderNode(bid) : bnode(bookmarkById(bid), 0)
                }))
            case "bookmarks.getRecent":
                return reply(id, allBookmarks().sort(function (x, y) { return (y.added || 0) - (x.added || 0) }).slice(0, a0 || 10).map(bnode))
            case "bookmarks.search": {
                var q = typeof a0 === "string" ? { query: a0 } : (a0 || {})
                return reply(id, allBookmarks().filter(function (b) {
                    if (q.url && b.url !== q.url) return false
                    if (q.title && b.title !== q.title) return false
                    if (q.query) return String(q.query).toLowerCase().split(/\s+/).every(function (term) { return (b.title + " " + b.url).toLowerCase().indexOf(term) >= 0 })
                    return true
                }).map(bnode))
            }
            case "bookmarks.create": {
                var c = a0 || {}, folder = folderOf(c.parentId || "2")
                if (!c.url) { var fid = "f:" + (c.title || "New folder"); host.tabEvent("bookmarks.onCreated", [fid, folderNode(fid)]); return reply(id, folderNode(fid)) }
                Core.addBookmark(c.url, c.title || c.url, folder)
                var made = allBookmarks().filter(function (b) { return b.url === c.url }).sort(function (x, y) { return y.id - x.id })[0]
                var node = bnode(made, 0)
                host.tabEvent("bookmarks.onCreated", [node.id, node])
                return reply(id, node)
            }
            case "bookmarks.update": {
                var b = bookmarkById(a0), ch = a1 || {}
                Core.editBookmark(Number(b.id), ch.title !== undefined ? ch.title : b.title, ch.url !== undefined ? ch.url : b.url, b.folder || "")
                host.tabEvent("bookmarks.onChanged", [String(a0), { title: ch.title, url: ch.url }])
                return reply(id, bnode(bookmarkById(a0), 0))
            }
            case "bookmarks.move": {
                var mb = bookmarkById(a0), dest = a1 || {}
                if (dest.parentId !== undefined) Core.editBookmark(Number(mb.id), mb.title, mb.url, folderOf(dest.parentId))
                if (dest.index !== undefined) Core.moveBookmark(Number(mb.id), dest.index)
                host.tabEvent("bookmarks.onMoved", [String(a0), { parentId: dest.parentId || folderId(mb.folder || ""), index: dest.index || 0 }])
                return reply(id, bnode(bookmarkById(a0), dest.index || 0))
            }
            case "bookmarks.remove": case "bookmarks.removeTree": {
                var rid = String(a0)
                if (rid.indexOf("f:") === 0) allBookmarks().filter(function (x) { return (x.folder || "") === rid.slice(2) }).forEach(function (x) { Core.removeBookmark(Number(x.id)) })
                else { bookmarkById(rid); Core.removeBookmark(Number(rid)) }
                host.tabEvent("bookmarks.onRemoved", [rid, { parentId: "2", index: 0 }])
                return reply(id, null)
            }

            // history
            case "history.search": {
                var hq = a0 || {}
                list = JSON.parse(Core.history(hq.text || "", hq.maxResults || 100))
                if (hq.startTime) list = list.filter(function (h) { return h.lastVisit * 1000 >= hq.startTime })
                if (hq.endTime) list = list.filter(function (h) { return h.lastVisit * 1000 <= hq.endTime })
                return reply(id, list.map(historyItem))
            }
            case "history.getVisits": {
                var hv = JSON.parse(Core.history("", 3000)).filter(function (h) { return h.url === (a0 || {}).url })[0]
                return reply(id, hv ? [{ id: historyItem(hv).id, visitId: "1", visitTime: hv.lastVisit * 1000, referringVisitId: "0", transition: "link" }] : [])
            }
            case "history.addUrl": Core.recordVisit((a0 || {}).url || "", (a0 || {}).title || ""); return reply(id, null)
            case "history.deleteUrl": Core.removeHistory((a0 || {}).url || ""); host.tabEvent("history.onVisitRemoved", [{ allHistory: false, urls: [(a0 || {}).url] }]); return reply(id, null)
            case "history.deleteRange": {
                var r = a0 || {}
                JSON.parse(Core.history("", 5000)).forEach(function (h) { var tm = h.lastVisit * 1000; if (tm >= (r.startTime || 0) && tm <= (r.endTime || Date.now())) Core.removeHistory(h.url) })
                return reply(id, null)
            }
            case "history.deleteAll": Core.clearHistory(0); host.tabEvent("history.onVisitRemoved", [{ allHistory: true, urls: [] }]); return reply(id, null)

            case "topSites.get": return reply(id, JSON.parse(Core.mostVisited(12)).map(function (s) { return { url: s.url, title: s.title || "" } }))

            // sessions
            case "sessions.getRecentlyClosed":
                return reply(id, JSON.parse(Core.closedTabs()).slice(0, (a0 || {}).maxResults || 25).map(function (c, i) {
                    return { lastModified: Math.floor(Date.now() / 1000), tab: { sessionId: String(i), url: c.url, title: c.title || "", index: 0, windowId: -1, active: false, pinned: false, highlighted: false, incognito: false } }
                }))
            case "sessions.getDevices": return reply(id, [])
            case "sessions.restore": {
                var closed = JSON.parse(Core.closedTabs()), k = a0 !== undefined && a0 !== null ? Number(a0) : 0
                if (!closed[k]) throw "No recently closed session with that id"
                w = host.focusedWindow(); if (!w) throw "No window"
                var idx = w.openTab(closed[k].url, {})
                return reply(id, { lastModified: Math.floor(Date.now() / 1000), tab: host.tabInfo(w, idx) })
            }

            // downloads
            case "downloads.download": {
                var dopt = a0 || {}
                if (!dopt.url) throw "downloads.download needs a url"
                startDownload(dopt.url, function (did, err) { reply(id, did, err) })
                return
            }
            case "downloads.search": return reply(id, dlMatch(a0).map(dlItem))
            case "downloads.pause": dlById(a0).pause(); return reply(id, null)
            case "downloads.resume": dlById(a0).resume(); return reply(id, null)
            case "downloads.cancel": dlById(a0).cancel(); return reply(id, null)
            case "downloads.open": { var od = dlById(a0); Core.openExternally(od.downloadDirectory + "/" + od.downloadFileName); return reply(id, null) }
            case "downloads.show": { Core.openExternally(dlById(a0).downloadDirectory); return reply(id, null) }
            case "downloads.showDefaultFolder": Core.openExternally(Core.downloadDir); return reply(id, null)
            case "downloads.erase": {
                var gone = dlMatch(a0).map(function (d) { return d.id })
                app.downloads = app.downloads.filter(function (d) { return gone.indexOf(d.id) < 0 })
                gone.forEach(function (g) { host.tabEvent("downloads.onErased", [g]) })
                return reply(id, gone)
            }
            case "downloads.removeFile": { var rd = dlById(a0); if (!Core.removeDownloadFile(rd.downloadDirectory + "/" + rd.downloadFileName)) throw "Couldn't remove the file"; return reply(id, null) }
            case "downloads.getFileIcon": throw "downloads.getFileIcon is not supported in OyNIx"

            // webNavigation
            case "webNavigation.getFrame": {
                t = tabById((a0 || {}).tabId); if (!t) throw "No tab with that id"
                return reply(id, (a0 || {}).frameId ? null : { url: t.win.tabs.get(t.index).url, parentFrameId: -1, errorOccurred: false, frameId: 0 })
            }
            case "webNavigation.getAllFrames": {
                t = tabById((a0 || {}).tabId); if (!t) throw "No tab with that id"
                return reply(id, [{ url: t.win.tabs.get(t.index).url, parentFrameId: -1, errorOccurred: false, frameId: 0, processId: -1 }])
            }

            // commands / omnibox
            case "commands.getAll":
                return reply(id, JSON.parse(Core.webextCommands()).filter(function (c) { return c.ext === ext }).map(function (c) { return { name: c.name, shortcut: c.key, description: c.description || "" } }))
            case "omnibox.setDefaultSuggestion": return reply(id, null)

            case "idle.queryState": return reply(id, idleNow(a0))

            // management
            case "management.getSelf": return reply(id, { id: ext, name: info.name, enabled: true, type: "extension", installType: "normal", mayDisable: true, permissions: info.permissions, hostPermissions: info.origins })
            case "management.getAll": case "management.get": {
                list = JSON.parse(Core.extensions()).filter(function (e) { return e.kind !== "oynix" || e.manifestVersion > 0 }).map(function (e) {
                    return { id: e.webId || e.id, name: e.name, version: e.version, description: e.description, enabled: e.enabled, type: "extension", installType: "normal", mayDisable: true }
                })
                if (op === "management.get") { var one = list.filter(function (e) { return e.id === a0 })[0]; if (!one) throw "No extension with that id"; return reply(id, one) }
                return reply(id, list)
            }
            case "management.setEnabled": case "management.uninstall": case "management.uninstallSelf": {
                var targetId = op === "management.uninstallSelf" ? ext : a0
                var ent = JSON.parse(Core.extensions()).filter(function (e) { return e.webId === targetId || e.id === targetId })[0]
                if (!ent) throw "No extension with that id"
                if (op === "management.setEnabled") { host.setEnabled(ent.id, !!a1); host.tabEvent(a1 ? "management.onEnabled" : "management.onDisabled", [{ id: targetId, name: ent.name }]) }
                else { host.remove(ent.id); host.tabEvent("management.onUninstalled", [targetId]) }
                return reply(id, null)
            }

            // cookies (powerful)
            case "cookies.getAll": case "cookies.get": {
                setObserving(false, true)
                var res = JSON.parse(Core.webextCookies(ext, app.defaultProfile))
                if (res.error) throw res.error
                var cq = a0 || {}, cu = cq.url ? new URL(cq.url) : null
                list = res.value.filter(function (c) {
                    var d = c.domain.replace(/^\./, "")
                    if (cu && !(cu.hostname === d || cu.hostname.endsWith("." + d))) return false
                    if (cu && c.path && cu.pathname.indexOf(c.path) !== 0) return false
                    if (cq.domain && !(d === cq.domain.replace(/^\./, "") || d.endsWith("." + cq.domain.replace(/^\./, "")))) return false
                    if (cq.name !== undefined && c.name !== cq.name) return false
                    if (cq.path !== undefined && c.path !== cq.path) return false
                    if (cq.secure !== undefined && c.secure !== cq.secure) return false
                    if (cq.session !== undefined && c.session !== cq.session) return false
                    return true
                })
                return reply(id, op === "cookies.get" ? (list[0] || null) : list)
            }
            case "cookies.set": {
                var err = Core.webextCookieSet(ext, app.defaultProfile, JSON.stringify(a0 || {}))
                if (err) throw err
                return reply(id, Object.assign({ path: "/", secure: false, httpOnly: false, session: !(a0 || {}).expirationDate, storeId: "0" }, a0 || {}))
            }
            case "cookies.remove": {
                var rerr = Core.webextCookieRemove(ext, app.defaultProfile, (a0 || {}).url || "", (a0 || {}).name || "")
                if (rerr) throw rerr
                return reply(id, { url: (a0 || {}).url, name: (a0 || {}).name, storeId: "0" })
            }

            // browsingData (powerful)
            case "browsingData.remove": {
                var since = (a0 || {}).since || 0, what = a1 || {}
                var ago = since > 0 ? Math.max(1, Math.round((Date.now() - since) / 1000)) : 0
                if (what.history) Core.clearHistory(ago)
                if (what.cookies) Core.clearCookies(app.defaultProfile)
                if (what.cache || what.cacheStorage) app.defaultProfile.clearHttpCache()
                if (what.downloads) app.clearFinishedDownloads()
                return reply(id, null)
            }

            // webRequest (powerful): start observing
            case "webRequest.subscribe": setObserving(true, false); return reply(id, null)

            // contentSettings
            default:
                if (op.indexOf("contentSettings.get:") === 0) return reply(id, { setting: contentSetting(op.split(":")[1], hostOf((a0 || {}).primaryUrl)) })
                if (op.indexOf("contentSettings.set:") === 0) {
                    var name = op.split(":")[1], d0 = a0 || {}, allow = d0.setting === "allow"
                    var hn = d0.primaryPattern === "<all_urls>" ? "" : patternHost(d0.primaryPattern)
                    if (name === "javascript") { if (hn) Core.setSiteSetting(hn, "javascript", JSON.stringify(allow)); else Core.setSetting("javascript", JSON.stringify(allow)) }
                    else if (name === "popups" && hn) Core.setSiteSetting(hn, "popups", JSON.stringify(allow))
                    return reply(id, null)
                }
                if (op.indexOf("contentSettings.clear:") === 0) return reply(id, null)
                if (op.indexOf("privacy.get:") === 0) {
                    var key = op.split(":")[1], st = JSON.parse(Core.settingsJson)
                    var map = { doNotTrackEnabled: st.sendDoNotTrack, thirdPartyCookiesAllowed: !st.blockThirdPartyCookies, searchSuggestEnabled: st.searchSuggestions,
                                networkPredictionEnabled: false, spellingServiceEnabled: false, hyperlinkAuditingEnabled: false, referrersEnabled: true, webRTCIPHandlingPolicy: "default" }
                    var controllable = ["doNotTrackEnabled", "thirdPartyCookiesAllowed", "searchSuggestEnabled"].indexOf(key) >= 0
                    return reply(id, { value: map[key], levelOfControl: controllable ? "controllable_by_this_extension" : "not_controllable" })
                }
                if (op.indexOf("privacy.set:") === 0) {
                    var pk = op.split(":")[1], pv = (a0 || {}).value
                    if (pk === "doNotTrackEnabled") Core.setSetting("sendDoNotTrack", JSON.stringify(!!pv))
                    else if (pk === "thirdPartyCookiesAllowed") Core.setSetting("blockThirdPartyCookies", JSON.stringify(!pv))
                    else if (pk === "searchSuggestEnabled") Core.setSetting("searchSuggestions", JSON.stringify(!!pv))
                    else throw "privacy." + pk + " can't be changed in OyNIx"
                    return reply(id, null)
                }
                if (op.indexOf("privacy.clear:") === 0) return reply(id, null)
                break
            }

            switch (op) {
            case "fontSettings.getFontList": return reply(id, Qt.fontFamilies().map(function (f) { return { fontId: f, displayName: f } }))
            case "search.query": {
                var sq = a0 || {}, url = Core.searchUrl(sq.text || "")
                if (sq.disposition === "NEW_TAB" || sq.disposition === "NEW_WINDOW") { w = host.focusedWindow(); if (w) w.openTab(url, {}) }
                else { t = tabById(sq.tabId); if (t && t.item) t.item.load(url) }
                return reply(id, null)
            }
            case "permissions.contains": {
                var pc = a0 || {}
                var okP = (pc.permissions || []).every(function (p) { return info.permissions.indexOf(p) >= 0 })
                var okO = (pc.origins || []).every(function (o) { return info.origins.indexOf(o) >= 0 || info.origins.indexOf("<all_urls>") >= 0 })
                return reply(id, okP && okO)
            }
            case "permissions.getAll": return reply(id, { permissions: info.permissions, origins: info.origins })
            case "permissions.request": {
                // Optional permissions declared in the manifest are granted;
                // anything else isn't.
                var pr = a0 || {}
                return reply(id, (pr.permissions || []).every(function (p) { return info.permissions.indexOf(p) >= 0 }) && (pr.origins || []).every(function (o) { return info.origins.indexOf(o) >= 0 || info.origins.indexOf("<all_urls>") >= 0 }))
            }

            // tabs extras
            case "tabs.captureVisibleTab": {
                w = windowById(typeof a0 === "number" ? a0 : null)
                var capOpts = (typeof a0 === "object" && a0) ? a0 : (a1 || {})
                var cv = w ? w.tabItem(w.currentIndex) : null
                if (!cv || !cv.view) throw "The current tab has no page to capture"
                var fmt = capOpts.format === "jpeg" ? "jpg" : "png"
                var path = Core.configPath("cache") + "/capture-" + id.replace(/[^a-z0-9]/gi, "") + "." + fmt
                cv.view.grabToImage(function (res) {
                    if (!res.saveToFile(path)) return apis.reply(id, null, "Couldn't capture the tab")
                    apis.reply(id, Core.fileDataUrl(path, fmt === "jpg" ? "image/jpeg" : "image/png", true))
                })
                return
            }
            case "tabs.move": {
                var ids = Array.isArray(a0) ? a0 : [a0], mp = a1 || {}, moved = []
                ids.forEach(function (tid, n) {
                    var f = host.findTab(tid); if (!f) return
                    var to = mp.index === -1 || mp.index === undefined ? f.win.tabs.count - 1 : Math.min(mp.index + n, f.win.tabs.count - 1)
                    f.win.moveTab(f.index, to); moved.push(host.tabInfo(f.win, to))
                })
                return reply(id, Array.isArray(a0) ? moved : moved[0])
            }
            case "tabs.highlight": {
                w = windowById((a0 || {}).windowId); if (!w) throw "No window"
                var hi = (a0 || {}).tabs; hi = Array.isArray(hi) ? hi[0] : hi
                w.activate(hi)
                return reply(id, { id: w.windowId, focused: w.active, incognito: false, type: "normal", state: "normal" })
            }
            case "tabs.discard": { t = tabById(a0); if (!t) throw "No tab with that id"; if (t.index !== t.win.currentIndex) t.win.tabs.setProperty(t.index, "suspended", true); return reply(id, host.tabInfo(t.win, t.index)) }
            case "tabs.getZoom": { t = tabById(a0); return reply(id, t && t.item && t.item.view ? t.item.view.zoomFactor : 1) }
            case "tabs.setZoom": {
                t = tabById(typeof a0 === "number" ? a0 : null); var z = typeof a0 === "number" ? a1 : a0
                if (t && t.item && t.item.view) t.item.view.zoomFactor = z > 0 ? z : 1
                return reply(id, null)
            }
            case "tabs.goBack": case "tabs.goForward": {
                t = tabById(a0); if (!t || !t.item || !t.item.view) throw "No tab with that id"
                if (op === "tabs.goBack") t.item.view.goBack(); else t.item.view.goForward()
                return reply(id, null)
            }
            case "tabs.detectLanguage": {
                t = tabById(a0); if (!t || !t.item || !t.item.view) return reply(id, "und")
                t.item.view.runJavaScript("(document.documentElement.lang || navigator.language || 'und').split('-')[0]", function (r) { apis.reply(id, r || "und") })
                return
            }
            case "tabs.toggleReaderMode": { t = tabById(a0); if (t && t.item) t.item.toggleReader(); return reply(id, null) }

            // windows extras
            case "windows.get": {
                w = windowById(a0); if (!w) throw "No window with id: " + a0
                var wi = { id: w.windowId, focused: w.active, incognito: w.privateMode, type: "normal", state: w.visibility === Window.Maximized ? "maximized" : "normal", alwaysOnTop: false, width: w.width, height: w.height, left: w.x, top: w.y }
                if (a1 && a1.populate) { wi.tabs = []; for (var ti = 0; ti < w.tabs.count; ti++) wi.tabs.push(host.tabInfo(w, ti)) }
                return reply(id, wi)
            }
            case "windows.create": {
                var cw = a0 || {}, urls = cw.url ? (Array.isArray(cw.url) ? cw.url : [cw.url]) : ["oynix://start"]
                urls = urls.map(function (u) { return /^[a-z-]+:/.test(u) ? u : "chrome-extension://" + ext + "/" + u.replace(/^\//, "") })
                var nw = app.newWindow({ tabs: urls.map(function (u) { return { url: u } }), private: !!cw.incognito })
                if (cw.width) nw.width = cw.width; if (cw.height) nw.height = cw.height
                if (cw.left !== undefined) nw.x = cw.left; if (cw.top !== undefined) nw.y = cw.top
                host.tabEvent("windows.onCreated", [{ id: nw.windowId, focused: true, incognito: !!cw.incognito, type: "normal" }])
                return reply(id, { id: nw.windowId, focused: true, incognito: !!cw.incognito, type: "normal", tabs: urls.map(function (u, i) { return host.tabInfo(nw, i) }) })
            }
            case "windows.update": {
                w = windowById(a0); if (!w) throw "No window with id: " + a0
                var up = a1 || {}
                if (up.state === "maximized") w.showMaximized(); else if (up.state === "minimized") w.showMinimized(); else if (up.state === "fullscreen") w.showFullScreen(); else if (up.state === "normal") w.showNormal()
                if (up.width) w.width = up.width; if (up.height) w.height = up.height
                if (up.left !== undefined) w.x = up.left; if (up.top !== undefined) w.y = up.top
                if (up.focused) { w.raise(); w.requestActivate() }
                return reply(id, { id: w.windowId, focused: w.active, incognito: w.privateMode, type: "normal" })
            }
            case "windows.remove": { w = windowById(a0); if (!w) throw "No window with id: " + a0; w.close(); host.tabEvent("windows.onRemoved", [a0]); return reply(id, null) }

            case "identity.launchWebAuthFlow": {
                var flow = a0 || {}
                if (!flow.url) throw "launchWebAuthFlow needs a url"
                var prefix = "https://" + ext + ".chromiumapp.org/"
                w = host.focusedWindow(); if (!w) throw "No window"
                var authIdx = w.openTab(flow.url, {})
                var authTab = w.tabs.get(authIdx).uid
                var m2 = Object.assign({}, authFlows)
                m2[prefix] = function (url) { var f = host.findTab(authTab); if (f) f.win.closeTab(f.index, true); apis.reply(id, url) }
                authFlows = m2
                return
            }
            }
            throw "Unsupported operation: " + op
        } catch (e) { reply(id, null, e) }
    }
}
