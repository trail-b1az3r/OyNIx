import QtQuick
import QtQuick.Window
import QtWebEngine
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

    property WebEngineProfile defaultProfile: WebEngineProfile {
        storageName: "OyNIx-" + Core.profile
        offTheRecord: Core.startPrivate
        persistentStoragePath: Core.configPath("data") + "/webengine"
        cachePath: Core.configPath("cache") + "/webengine"
        httpCacheType: WebEngineProfile.DiskHttpCache
        persistentCookiesPolicy: WebEngineProfile.AllowPersistentCookies
        httpUserAgent: Core.userAgent !== "" ? Core.userAgent : httpUserAgent
        spellCheckEnabled: false
        onDownloadRequested: function (download) { app.handleDownload(download) }
        Component.onCompleted: Core.attachProfile(defaultProfile)
    }

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
        download.accept()
        downloads = [download].concat(downloads)
        for (var i = 0; i < windows.length; i++) if (windows[i].active) windows[i].requestActivate()
    }

    function clearFinishedDownloads() {
        downloads = downloads.filter(function (d) { return d.state === WebEngineDownloadRequest.DownloadInProgress })
    }

    function openSettings(page) {
        if (!settingsWindow) settingsWindow = settingsComponent.createObject(null, { app: app })
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

    Component.onCompleted: restore()
}
