import QtQuick
import QtWebEngine
import OyNIx

// One tab's content: a WebEngineView (created lazily, destroyed when the
// tab is suspended) or a native page for oynix:// URLs.
Item {
    id: tab
    required property int index
    required property var model
    property var win

    readonly property bool isCurrent: index === win.currentIndex
    readonly property bool internal: (model.url || "").indexOf("oynix://") === 0
    readonly property var view: webLoader.item
    property var pendingRequest: null
    property bool readerActive: false
    property bool readerAvailable: false
    property bool crashed: false
    property string hoveredLink: ""
    property string loadError: ""
    property var certError: null
    // For Claude for OyNIx: recent console output and screenshot scale.
    property var consoleLog: []
    property real agentShotScale: 1
    readonly property var pageItem: pageLoader.item
    readonly property string host: Core.hostOf(model.url || "")

    visible: isCurrent

    function load(u) {
        crashed = false
        loadError = ""
        readerActive = false
        if (u.indexOf("oynix://") === 0) {
            model.url = u
            model.title = titleFor(u)
            return
        }
        if (view) view.url = u
        else model.url = u   // activates the web loader, which loads model.url
    }

    Component.onCompleted: if (internal && !model.title) model.title = titleFor(model.url)
    onInternalChanged: if (internal) model.title = titleFor(model.url)

    function titleFor(u) {
        if (u.indexOf("oynix://settings") === 0) return qsTr("Settings")
        if (u.indexOf("oynix://history") === 0) return qsTr("History")
        if (u.indexOf("oynix://privacy") === 0) return qsTr("Privacy Report")
        return qsTr("Start Page")
    }

    function adopt(request) {
        pendingRequest = request
        model.url = "about:blank"
    }

    function toggleReader() {
        if (!view) return
        view.runJavaScript(Core.readerScript(), function (r) {
            if (r === true) tab.readerActive = true
            else if (r === false) tab.readerActive = false
        })
    }

    function probeReader() {
        if (!view) return
        view.runJavaScript(Core.readerProbeScript(), function (r) { tab.readerAvailable = !!r })
    }

    function applySiteSettings() {
        if (!view) return
        var s = JSON.parse(Core.siteSettings(host))
        var z = s.zoom !== undefined ? s.zoom : Core.defaultZoom
        if (Math.abs(view.zoomFactor - z) > 0.001) view.zoomFactor = z
    }

    function setZoom(z) {
        if (!view) return
        z = Math.max(0.3, Math.min(3.0, Math.round(z * 100) / 100))
        view.zoomFactor = z
        Core.setSiteSetting(host, "zoom", Math.abs(z - Core.defaultZoom) < 0.001 ? "null" : JSON.stringify(z))
    }

    // Light-weight preview for the tab overview / hover cards.
    function snapshot() {
        var target = view ? view : pageLoader.item
        if (!target || !tab.visible || win.privateMode) return
        target.grabToImage(function (res) { tab.model.thumb = res.url.toString() }, Qt.size(480, 300))
    }

    Loader {
        id: webLoader
        anchors.fill: parent
        active: !tab.model.suspended && (!tab.internal || tab.pendingRequest !== null)
        sourceComponent: webComponent
    }

    Loader {
        id: pageLoader
        anchors.fill: parent
        active: tab.internal && !webLoader.active && tab.isCurrent
        sourceComponent: (tab.model.url || "").indexOf("oynix://history") === 0 ? historyPage
                       : (tab.model.url || "").indexOf("oynix://settings") === 0 ? settingsPage : startPage
    }

    Component {
        id: startPage
        StartPage { win: tab.win }
    }
    Component {
        id: settingsPage
        SettingsView {
            app: tab.win.app
            page: { var m = (tab.model.url || "").match(/^oynix:\/\/settings\/?([a-z]*)/); return m && m[1] ? m[1] : "general" }
            onPageChanged: if (tab.model.url !== "oynix://settings/" + page) tab.model.url = "oynix://settings/" + page
        }
    }
    Component {
        id: historyPage
        HistoryPage { win: tab.win }
    }

    // Crashed / failed page overlay
    Rectangle {
        anchors.fill: parent
        visible: tab.crashed || tab.loadError !== ""
        color: Theme.window
        Column {
            anchors.centerIn: parent
            spacing: 14
            width: Math.min(460, parent.width - 48)
            Icon { anchors.horizontalCenter: parent.horizontalCenter; name: tab.crashed ? "warning" : "globe"; size: 44; color: Theme.textFaint; stroke: 1.4 }
            Txt {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: tab.crashed ? qsTr("This page stopped responding") : qsTr("Can’t open this page")
                font.pixelSize: 20; font.weight: Font.DemiBold
            }
            Txt {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                color: Theme.textDim
                text: tab.crashed ? qsTr("The web content process ended unexpectedly.") : tab.loadError
            }
            PushButton {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Try Again"); primary: true
                onClicked: { tab.crashed = false; tab.loadError = ""; if (tab.view) tab.view.reload() }
            }
        }
    }

    Component {
        id: webComponent
        WebEngineView {
            id: view
            profile: tab.win.profile
            // Pages without their own background get white, as in every browser;
            // only force-darkened pages use the dark canvas.
            backgroundColor: settings.forceDarkMode ? Theme.window : "white"
            audioMuted: tab.model.muted
            lifecycleState: (Core.lowPowerMode && !tab.isCurrent && !recentlyAudible)
                            ? WebEngineView.LifecycleState.Frozen : WebEngineView.LifecycleState.Active

            settings.javascriptEnabled: { var s = JSON.parse(Core.siteSettings(tab.host)); return s.javascript !== undefined ? s.javascript : Core.javascript }
            settings.playbackRequiresUserGesture: !Core.autoplayMedia
            settings.fullScreenSupportEnabled: true
            settings.pluginsEnabled: false
            settings.dnsPrefetchEnabled: false
            settings.hyperlinkAuditingEnabled: false
            settings.pdfViewerEnabled: true
            settings.localContentCanAccessRemoteUrls: false
            settings.javascriptCanAccessClipboard: false
            settings.javascriptCanOpenWindows: { var s = JSON.parse(Core.siteSettings(tab.host)); return s.popups === true }
            settings.focusOnNavigationEnabled: false
            settings.webRTCPublicInterfacesOnly: true
            // Dark mode for websites: sites with a dark theme get it via
            // prefers-color-scheme (set app-wide); this additionally darkens
            // sites that have none, when enabled globally or for the site.
            settings.forceDarkMode: {
                if (!Theme.dark) return false
                Core.settingsJson
                var s = JSON.parse(Core.siteSettings(tab.host))
                return s.forceDark !== undefined ? s.forceDark : Core.forceDarkWebsites
            }

            // Cosmetic filtering: element-hiding CSS for the host being
            // loaded, injected at document creation in the isolated world.
            function updateCosmetic(host) {
                var old = userScripts.find("oynix-cosmetic")
                for (var i = 0; i < old.length; i++) userScripts.remove(old[i])
                var src = tab.win.privateMode && !Core.contentBlocker ? "" : Core.cosmeticScript(host)
                if (!src) return
                var sc = WebEngine.script()
                sc.name = "oynix-cosmetic"
                sc.sourceCode = src
                sc.injectionPoint = WebEngineScript.DocumentCreation
                sc.worldId = WebEngineScript.ApplicationWorld
                sc.runsOnSubFrames = false
                userScripts.insert(sc)
            }
            Connections {
                target: tab.win.app
                function onFilterGenerationChanged() {
                    var h = Core.hostOf(view.url.toString())
                    view.updateCosmetic(h)
                    var cos = Core.cosmeticScript(h)
                    if (cos) view.runJavaScript(cos, WebEngineScript.ApplicationWorld)
                }
            }
            onNavigationRequested: function (request) {
                if (request.isMainFrame) updateCosmetic(Core.hostOf(request.url.toString()))
            }

            // Content scripts of Chrome/Firefox extensions talk to OyNIx over
            // this channel; it is only reachable from the isolated world.
            webChannel: tab.win.app.extChannel
            webChannelWorld: WebEngineScript.ApplicationWorld

            function installScripts() {
                userScripts.clear()
                var host = tab.win.app.extensionHost
                host.installScripts(view, JSON.parse(Core.pageScripts()))
                if (!tab.win.privateMode) host.installScripts(view, JSON.parse(Core.extensionScripts(tab.model.uid)))
            }
            Connections {
                target: Core
                function onExtensionsRevisionChanged() { view.installScripts() }
            }

            Component.onCompleted: {
                installScripts()
                if (tab.pendingRequest) {
                    tab.pendingRequest.openIn(view)
                    tab.pendingRequest = null
                } else if (tab.model.url && tab.model.url !== "about:blank") {
                    url = tab.model.url
                }
            }

            onUrlChanged: {
                var u = url.toString()
                if (u !== "" && u !== tab.model.url) tab.model.url = u
                tab.readerActive = false
                tab.readerAvailable = false
                // In-page navigations (pushState, #fragments) don't fire a
                // load; record them once the title has had time to update.
                if (!loading) spaRecord.restart()
            }
            Timer {
                id: spaRecord
                interval: 1200
                onTriggered: if (!view.loading && !tab.win.privateMode) Core.recordVisit(view.url.toString(), view.title)
            }
            onTitleChanged: {
                tab.model.title = title
                if (!tab.win.privateMode) Core.updateTitle(url.toString(), title)
            }
            onIconChanged: tab.model.icon = icon.toString()
            onLoadProgressChanged: tab.model.progress = loadProgress / 100
            onRecentlyAudibleChanged: tab.model.audible = recentlyAudible
            onLinkHovered: function (hoveredUrl) { tab.hoveredLink = hoveredUrl.toString() }

            onLoadingChanged: function (info) {
                tab.model.loading = loading
                if (info.status === WebEngineView.LoadStartedStatus) {
                    tab.loadError = ""
                    tab.certError = null
                } else if (info.status === WebEngineView.LoadSucceededStatus) {
                    // Covers pages restored before the rules finished loading.
                    var cos = Core.cosmeticScript(Core.hostOf(url.toString()))
                    if (cos) runJavaScript(cos, WebEngineScript.ApplicationWorld)
                    if (!tab.win.privateMode) Core.recordVisit(url.toString(), title)
                    tab.win.app.extensionHost.tabEvent("tabs.onUpdated", [tab.model.uid, { status: "complete", url: url.toString(), title: title }, tab.win.app.extensionHost.tabInfo(tab.win, tab.index)])
                    tab.applySiteSettings()
                    tab.probeReader()
                    snapTimer.restart()
                } else if (info.status === WebEngineView.LoadFailedStatus && info.errorCode !== -3) {
                    // -3 = ERR_ABORTED (user stopped / replaced navigation)
                    if (info.errorDomain !== WebEngineView.CertificateErrorDomain)
                        tab.loadError = info.errorString + " (" + Core.hostOf(info.url.toString()) + ")"
                }
            }

            onNewWindowRequested: function (request) { tab.win.handleNewWindow(request) }
            onFullScreenRequested: function (request) {
                request.accept()
                tab.win.setContentFullScreen(request.toggleOn)
            }
            onFeaturePermissionRequested: function (origin, feature) { tab.win.askPermission(view, origin, feature) }
            onCertificateError: function (error) {
                error.defer()
                tab.certError = error
            }
            onRenderProcessTerminated: function (status, code) {
                if (status !== WebEngineView.NormalTerminationStatus) tab.crashed = true
            }
            onFindTextFinished: function (result) { tab.win.findResult(result) }
            onJavaScriptConsoleMessage: function (level, message, lineNumber, sourceID) {
                var l = tab.consoleLog.length > 299 ? tab.consoleLog.slice(-299) : tab.consoleLog.slice()
                l.push({ level: level, message: String(message).slice(0, 2000), line: lineNumber, source: String(sourceID).slice(0, 200), time: Date.now() })
                tab.consoleLog = l
            }
            onContextMenuRequested: function (request) {
                request.accepted = true
                tab.win.showContextMenu(view, request)
            }
            onZoomFactorChanged: tab.model.zoom = zoomFactor

            Timer { id: snapTimer; interval: 800; onTriggered: tab.snapshot() }
        }
    }

    // Certificate error interstitial
    Rectangle {
        anchors.fill: parent
        visible: tab.certError !== null
        color: Theme.window
        Column {
            anchors.centerIn: parent
            width: Math.min(520, parent.width - 48)
            spacing: 14
            Icon { anchors.horizontalCenter: parent.horizontalCenter; name: "warning"; size: 48; color: Theme.danger; stroke: 1.4 }
            Txt {
                width: parent.width; horizontalAlignment: Text.AlignHCenter
                text: qsTr("This connection is not private")
                font.pixelSize: 20; font.weight: Font.DemiBold
            }
            Txt {
                width: parent.width; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                color: Theme.textDim
                text: tab.certError ? (tab.certError.description + "\n" + Core.hostOf(tab.certError.url.toString())) : ""
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 10
                PushButton {
                    text: qsTr("Go Back"); primary: true
                    onClicked: { var e = tab.certError; tab.certError = null; e.rejectCertificate(); if (tab.view && tab.view.canGoBack) tab.view.goBack() }
                }
                PushButton {
                    text: qsTr("Visit Anyway"); destructive: true
                    visible: tab.certError && tab.certError.overridable
                    onClicked: { var e = tab.certError; tab.certError = null; e.acceptCertificate() }
                }
            }
        }
    }
}
