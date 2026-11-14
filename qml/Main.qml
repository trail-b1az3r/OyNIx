import QtQuick
import QtQuick.Window
import QtWebEngine
import QtWebChannel
import OyNIx

// Application controller: owns the shared browsing profile, all windows,
// downloads and session persistence. It has no UI of its own.
QtObject {
    id: app

    property var windows: []
    property var downloads: []
    readonly property int activeDownloads: {
        var n = 0
        for (var i = 0; i < downloads.length; i++) if (downloads[i].state === WebEngineDownloadRequest.DownloadInProgress) n++
        return n
    }
    property var settingsWindow: null
    property var devToolsWindows: []
    property bool quitting: false

    // Qt ≥ 6.9: profiles are created from a prototype, then configured.
    property WebEngineProfilePrototype profilePrototype: WebEngineProfilePrototype {
        storageName: Core.startPrivate ? "" : "OyNIx-" + Core.profile
        persistentStoragePath: Core.configPath("data") + "/webengine"
        cachePath: Core.configPath("cache") + "/webengine"
        httpCacheType: WebEngineProfile.DiskHttpCache
        persistentCookiesPolicy: WebEngineProfile.AllowPersistentCookies
    }
    property var defaultProfile: null

    property Connections profileEvents: Connections {
        target: app.defaultProfile
        function onDownloadRequested(download) { app.handleDownload(download) }
    }

    // Websites follow OyNIx's light/dark theme (prefers-color-scheme).
    property bool themeDark: Theme.dark
    // Should websites be shown dark? (auto follows the OyNIx theme)
    readonly property bool websitesDark: Core.websiteAppearance === "dark" || (Core.websiteAppearance !== "light" && Theme.dark)
    onThemeDarkChanged: Core.applyColorScheme(themeDark)
    property Connections schemeEvents: Connections {
        target: Core
        function onDarkWebsitesChanged() { Core.applyColorScheme(Theme.dark) }
        function onThemeChanged() { Core.applyColorScheme(Theme.dark) }
        function onWebsiteAppearanceChanged() { Core.applyColorScheme(Theme.dark) }
    }

    // ── Content-blocker list updates ──
    // Subscribed filter lists are fetched in the background on start-up
    // (when older than the update interval) and on demand from Settings.
    property var filterQueue: []
    property bool filterUpdating: false
    property string filterStatus: ""
    property Timer filterStartup: Timer { interval: 4000; running: true; onTriggered: app.updateFilterLists(false) }
    property Timer filterPeriodic: Timer { interval: 6 * 3600 * 1000; running: true; repeat: true; onTriggered: app.updateFilterLists(false) }

    function updateFilterLists(force) {
        if (filterUpdating) return
        filterQueue = JSON.parse(Core.dueFilterLists(force))
        if (!filterQueue.length) return
        filterUpdating = true
        nextFilterList(0)
    }

    function nextFilterList(done) {
        if (!filterQueue.length) {
            filterUpdating = false
            filterStatus = ""
            if (done > 0) Core.reloadFilterLists()
            return
        }
        var item = filterQueue.shift()
        filterStatus = qsTr("Downloading %1…").arg(item.id)
        var xhr = new XMLHttpRequest()
        xhr.onreadystatechange = function () {
            if (xhr.readyState !== XMLHttpRequest.DONE) return
            if (xhr.status === 200) {
                var err = Core.storeFilterList(item.id, xhr.responseText)
                if (err) console.warn("OyNIx: filter list", item.id, err)
                app.nextFilterList(done + (err ? 0 : 1))
            } else {
                Core.filterListFailed(item.id, xhr.status ? "HTTP " + xhr.status : qsTr("network error"))
                app.nextFilterList(done)
            }
        }
        xhr.open("GET", item.url)
        xhr.send()
    }

    // Bumped when a freshly parsed rule set is installed, so open pages can
    // re-apply element hiding.
    property int filterGeneration: 0
    property Timer filterWatch: Timer {
        interval: 700; repeat: true; running: true
        onTriggered: { var g = Core.blockerGeneration(); if (g !== app.filterGeneration) app.filterGeneration = g }
    }

    function setupProfile() {
        defaultProfile = profilePrototype.instance()
        if (Core.userAgent !== "") defaultProfile.httpUserAgent = Core.userAgent
        defaultProfile.spellCheckEnabled = false
        Core.attachProfile(defaultProfile)
        Core.applyColorScheme(Theme.dark)
        extensionHost.start()
        if (Core.agentEnabled) agentBridge.start()
    }

    property ExtensionHost extensionHost: ExtensionHost { app: app }

    // ── Claude for OyNIx ──
    property Agent agent: Agent { app: app }
    property AgentBridge agentBridge: AgentBridge {
        onToolRequested: function (id, name, args, source) {
            var parsed = {}
            try { parsed = JSON.parse(args) } catch (e) {}
            app.agent.call(name, parsed, source, function (res) { app.agentBridge.reply(id, JSON.stringify(res)) })
        }
    }
    property Connections agentSwitch: Connections {
        target: Core
        function onAgentEnabledChanged() { if (Core.agentEnabled) app.agentBridge.start(); else { app.agent.stop(); app.agentBridge.stop() } }
    }

    // WebChannel endpoint used by the extension runtime in pages.
    property ExtBridge extBridge: ExtBridge { WebChannel.id: "oynixExtensions" }
    property WebChannel extChannel: WebChannel { registeredObjects: [app.extBridge, app.usb] }

    // WebUSB: device chooser and connect/disconnect detection.
    property UsbBridge usb: UsbBridge { WebChannel.id: "oynixUsb" }
    readonly property string siteUrl: "https://trail-b1az3r.github.io/OyNIx/"
    // Open a web address in the frontmost browser window (settings may be
    // in their own window, which has no tabs).
    function openInBrowser(u) {
        var w = activeWindow()
        if (w && w.openTab) w.openTab(u, {})
        else if (windows.length) windows[0].openTab(u, {})
    }
    property int usbGrants: 0
    // host|error type pairs the user chose to visit despite a certificate error (this session only)
    property var certAllowed: ({})
    function refreshUsbGrants() { usbGrants = JSON.parse(usb.permissions()).length }
    property Connections usbEvents: Connections {
        target: app.usb
        function onChooserRequested(id, origin, devices) {
            var w = app.activeWindow()
            if (!w) { app.usb.choose(id, ""); return }
            w.askUsbDevice(origin, JSON.parse(devices), function (key) { app.usb.choose(id, key || "") })
        }
        function onPermissionsChanged() { app.refreshUsbGrants() }
    }
    property Timer usbPoll: Timer {
        interval: 2000; repeat: true
        running: app.usbGrants > 0 && JSON.parse(Core.settingsJson).webusb === true
        onTriggered: app.usb.poll()
    }

    // Tab and window ids are global so extensions can address them.
    property int lastTabId: 0
    property int lastWindowId: 0
    function nextTabId() { lastTabId += 1; return lastTabId }
    function nextWindowId() { lastWindowId += 1; return lastWindowId }

    property Component windowComponent: Component { BrowserWindow {} }
    property Component settingsComponent: Component { SettingsWindow {} }
    property Component devToolsComponent: Component {
        Window {
            id: dw
            property var inspected
            width: 1000; height: 640
            visible: true
            title: qsTr("Developer Tools — %1").arg(inspected ? inspected.title : "")
            WebEngineView {
                anchors.fill: parent
                profile: dw.inspected ? dw.inspected.profile : app.defaultProfile
                inspectedView: dw.inspected
            }
            onClosing: app.devToolsWindows = app.devToolsWindows.filter(function (w) { return w !== dw })
        }
    }

    property Timer saveTimer: Timer { interval: 1500; onTriggered: app.saveSession() }
    property Timer flushTimer: Timer {
        interval: 30000; running: true; repeat: true
        onTriggered: { Core.flush(); app.saveSession() }
    }
    property Timer statsTimer: Timer { interval: 2000; running: true; repeat: true; onTriggered: Core.refreshStats() }

    function scheduleSave() { saveTimer.restart() }

    function newWindow(opts) {
        opts = opts || {}
        var w = windowComponent.createObject(null, {
            app: app,
            privateMode: !!opts.private || Core.startPrivate,
            initialTabs: opts.tabs || [],
            initialIndex: opts.current || 0
        })
        if (!w) { console.error("OyNIx: could not create window"); return null }
        if (opts.geometry) {
            var g = opts.geometry
            if (g.w > 300 && g.h > 200) { w.width = g.w; w.height = g.h }
            if (g.maximized) w.showMaximized()
        }
        if (opts.sidebar !== undefined) w.sidebarOpen = opts.sidebar
        windows = windows.concat([w])
        w.requestActivate()
        return w
    }

    function windowClosing(w) {
        if (!quitting) {
            if (windows.length === 1) saveSession()          // keep last window's tabs for next launch
            windows = windows.filter(function (x) { return x !== w })
            if (windows.length > 0) saveSession()
        }
        if (windows.length === 0) shutdown()
    }

    function saveSession() {
        if (Core.startPrivate || quitting && windows.length === 0) return
        var out = []
        for (var i = 0; i < windows.length; i++) {
            if (windows[i].privateMode) continue
            out.push(windows[i].sessionData())
        }
        if (out.length) Core.saveSession(JSON.stringify({ version: 1, windows: out }))
    }

    function restore() {
        if (Core.startUrl !== "") { newWindow({ tabs: [{ url: Core.startUrl }] }); return }
        if (Core.onLaunch === "restore" && !Core.startPrivate) {
            try {
                var s = JSON.parse(Core.loadSession() || "{}")
                if (s.windows && s.windows.length) {
                    for (var i = 0; i < s.windows.length; i++) {
                        var w = s.windows[i]
                        if (!w.tabs || !w.tabs.length) continue
                        newWindow({ tabs: w.tabs, current: w.current, sidebar: w.sidebar, geometry: w })
                    }
                    if (windows.length) return
                }
            } catch (e) { console.warn("OyNIx: session restore failed:", e) }
        }
        if (Core.onLaunch === "homepage") newWindow({ tabs: [{ url: Core.homepage }] })
        else newWindow({})
    }

    function handleDownload(download) {
        if (download.state !== WebEngineDownloadRequest.DownloadRequested) return
        download.downloadDirectory = Core.downloadDir
        for (var wi = 0; wi < windows.length; wi++) windows[wi].settleDownloadTab(download.url.toString())
        if (isExtensionPackage(download)) { offerExtensionInstall(download); return }
        var ask = JSON.parse(Core.settingsJson).askWhereToSave
        var w = null
        for (var k = 0; k < windows.length; k++) if (windows[k].active) w = windows[k]
        if (!w && windows.length) w = windows[windows.length - 1]
        if (ask && w) {
            // The request stays pending until the user picks a location.
            w.askDownloadLocation(download, function (ok) {
                if (!ok) { download.cancel(); return }
                download.accept()
                app.downloads = [download].concat(app.downloads)
            })
            return
        }
        download.accept()
        downloads = [download].concat(downloads)
        for (var i = 0; i < windows.length; i++) if (windows[i].active) windows[i].requestActivate()
    }

    // ── Extension packages (.xpi / .crx), e.g. from addons.mozilla.org ──
    function isExtensionPackage(d) {
        var n = String(d.downloadFileName || "").toLowerCase(), m = String(d.mimeType || "").toLowerCase()
        return m === "application/x-xpinstall" || m === "application/x-chrome-extension" || /\.(xpi|crx|oynix)$/.test(n)
    }

    function activeWindow() {
        for (var k = 0; k < windows.length; k++) if (windows[k].active) return windows[k]
        return windows.length ? windows[windows.length - 1] : null
    }

    function offerExtensionInstall(download) {
        var w = activeWindow()
        var source = Core.hostOf(download.url.toString())
        var name = String(download.downloadFileName).replace(/\.(xpi|crx|oynix)$/i, "").replace(/[-_]\d[\d.]*$/, "").replace(/[_-]+/g, " ")
        var go = function (ok) {
            if (!ok) { download.cancel(); return }
            download.downloadDirectory = Core.configPath("cache") + "/extension-downloads"
            download.accept()
            var done = function () {
                if (download.state !== WebEngineDownloadRequest.DownloadCompleted) {
                    if (download.state === WebEngineDownloadRequest.DownloadInterrupted || download.state === WebEngineDownloadRequest.DownloadCancelled)
                        w && w.toastMessage(qsTr("Extension download failed"))
                    return
                }
                download.stateChanged.disconnect(done)
                var r = app.extensionHost.install(download.downloadDirectory + "/" + download.downloadFileName)
                if (w) w.toastMessage(r.ok ? qsTr("Added “%1”").arg(r.extension.name) : qsTr("Could not install: %1").arg(r.error))
            }
            download.stateChanged.connect(done)
            done()
        }
        if (w) w.askInstallExtension(name, source, go)
        else go(true)
    }

    function clearFinishedDownloads() {
        downloads = downloads.filter(function (d) { return d.state === WebEngineDownloadRequest.DownloadInProgress })
    }

    function openSettings(page) {
        var w = null
        for (var k = 0; k < windows.length; k++) if (windows[k].active) w = windows[k]
        if (!w && windows.length) w = windows[windows.length - 1]
        if (JSON.parse(Core.settingsJson).settingsOpenIn !== "window" && w) {
            // In the browser window, like any other page.
            w.openInternal("oynix://settings/" + (page || ""))
            var t = w.currentTab
            if (page && t && t.pageItem && t.pageItem.page !== undefined) t.pageItem.page = page
            w.raise(); w.requestActivate()
            return
        }
        if (!settingsWindow) settingsWindow = settingsComponent.createObject(null, { app: app, transientParent: w })
        if (page) settingsWindow.page = page
        settingsWindow.show()
        settingsWindow.raise()
        settingsWindow.requestActivate()
    }

    function openDevTools(view) {
        for (var i = 0; i < devToolsWindows.length; i++) {
            if (devToolsWindows[i].inspected === view) { devToolsWindows[i].raise(); return }
        }
        var w = devToolsComponent.createObject(null, { inspected: view })
        devToolsWindows = devToolsWindows.concat([w])
    }

    function shutdown() {
        agentBridge.stop()
        Core.flush()
        if (settingsWindow) settingsWindow.close()
        for (var i = 0; i < devToolsWindows.length; i++) devToolsWindows[i].close()
        Qt.quit()
    }

    function quit() {
        saveSession()
        quitting = true
        var ws = windows.slice()
        for (var i = 0; i < ws.length; i++) ws[i].close()
        shutdown()
    }

    Component.onCompleted: { setupProfile(); restore(); refreshUsbGrants() }
}
