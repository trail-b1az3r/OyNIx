import QtQuick
import QtQuick.Layouts
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
    property int loadErrorCode: 0
    property string loadErrorUrl: ""
    property var certError: null
    property var certPending: []
    function answerCert(visit) {
        var list = certPending
        certPending = []
        if (visit && certError) tab.win.app.certAllowed[Core.hostOf(certError.url.toString()) + "|" + certError.type] = true
        certError = null
        for (var i = 0; i < list.length; i++) { if (visit) list[i].acceptCertificate(); else list[i].rejectCertificate() }
    }
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
                       : (tab.model.url || "").indexOf("oynix://settings") === 0 ? settingsPage
                       : (tab.model.url || "").indexOf("oynix://privacy") === 0 ? privacyPage : startPage
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
    Component {
        id: privacyPage
        PrivacyReportPage { win: tab.win }
    }

    // ── Failed page / crashed page / certificate interstitials ──
    // One layout for all three: an expressive shape, a plain-language
    // explanation, the technical detail on request, and the way out.
    component Interstitial: Rectangle {
        id: ist
        property string shape: "cookie9"
        property string icon: "globe"
        property color tone: Theme.secondaryContainer
        property color toneFg: Theme.secondaryContainerFg
        property string heading
        property string body
        property string detail
        default property alias actions: actionRow.data
        anchors.fill: parent
        color: Theme.surface
        MouseArea { anchors.fill: parent; acceptedButtons: Qt.AllButtons }
        ColumnLayout {
            anchors.centerIn: parent
            width: Math.min(560, parent.width - 64)
            spacing: 16
            MShape {
                size: 96; shape: ist.shape; color: ist.tone
                rotation: ist.visible ? 0 : -30
                Behavior on rotation { SpringAnimation { spring: Theme.springDefault; damping: Theme.damping } }
                Icon { anchors.centerIn: parent; name: ist.icon; size: 40; color: ist.toneFg; rotation: -parent.rotation }
            }
            Txt {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                text: ist.heading
                font.family: Theme.displayFont; font.pixelSize: 32; lineHeight: 1.1
                Layout.topMargin: 8
            }
            Txt {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                text: ist.body; color: Theme.surfaceVariantFg; font.pixelSize: 15; lineHeight: 1.25
            }
            Txt {
                id: detailText
                property bool open: false
                visible: ist.detail !== ""
                Layout.fillWidth: true; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone
                text: open ? "▾ " + ist.detail : "▸ " + qsTr("Details")
                color: Theme.textFaint; font.pixelSize: 12; font.family: open ? "monospace" : Theme.font
                TapHandler { onTapped: detailText.open = !detailText.open }
                HoverHandler { cursorShape: Qt.PointingHandCursor }
            }
            RowLayout { id: actionRow; spacing: 10; Layout.topMargin: 8 }
        }
    }

    readonly property var errorText: {
        var c = loadErrorCode, h = Core.hostOf(loadErrorUrl)
        if (c === -105 || c === -137) return [qsTr("Can’t find %1").arg(h), qsTr("No server answers to this name. Check the spelling, or search for it instead."), "cookie4", "search"]
        if (c === -106) return [qsTr("You’re offline"), qsTr("OyNIx can’t reach the internet. Check your Wi-Fi or cable; this page will load when you try again."), "clover4", "globe"]
        if (c === -102) return [qsTr("%1 refused to connect").arg(h), qsTr("The server is reachable but isn’t accepting connections. It may be down, or the address may have the wrong port."), "cookie6", "globe"]
        if (c === -118 || c === -7) return [qsTr("%1 is taking too long").arg(h), qsTr("The server didn’t answer in time. It may be overloaded; trying again in a moment often works."), "softBurst", "globe"]
        if (c === -101 || c === -100 || c === -324) return [qsTr("The connection to %1 dropped").arg(h), qsTr("The server closed the connection before the page arrived."), "cookie6", "globe"]
        if (c === -20) return [qsTr("OyNIx blocked this page"), qsTr("%1 is on one of your filter lists, usually because it serves ads, tracking or malware.").arg(h), "burst", "shield"]
        if (c === -310) return [qsTr("This page keeps redirecting"), qsTr("%1 sent OyNIx round in circles. Clearing its cookies sometimes fixes this.").arg(h), "flower", "reload"]
        if (c === -312) return [qsTr("This port is off-limits"), qsTr("Browsers refuse to connect to port %1 because it belongs to services like mail or file transfer, which a web page could misuse.").arg(loadErrorUrl.replace(/^.*:(\d+).*$/, "$1")), "pentagon", "lock"]
        if (c === -6) return [qsTr("File not found"), qsTr("Nothing exists at this path on your computer."), "pentagon", "folder"]
        return [qsTr("Can’t open this page"), qsTr("Something went wrong while loading %1.").arg(h || qsTr("this address")), "cookie9", "globe"]
    }

    Interstitial {
        visible: tab.crashed || tab.loadError !== ""
        shape: tab.crashed ? "burst" : tab.errorText[2]
        icon: tab.crashed ? "warning" : tab.errorText[3]
        tone: tab.crashed || tab.loadErrorCode === -20 ? Theme.errorContainer : Theme.secondaryContainer
        toneFg: tab.crashed || tab.loadErrorCode === -20 ? Theme.errorContainerFg : Theme.secondaryContainerFg
        heading: tab.crashed ? qsTr("This page stopped working") : tab.errorText[0]
        body: tab.crashed ? qsTr("The process showing it ended unexpectedly. Your other tabs are fine. Reloading usually brings it back.") : tab.errorText[1]
        detail: tab.crashed ? "" : tab.loadError + "  ·  " + tab.loadErrorCode
        PushButton {
            text: qsTr("Try Again"); primary: true
            onClicked: { tab.crashed = false; tab.loadError = ""; if (tab.view) tab.view.reload() }
        }
        PushButton {
            visible: !tab.crashed && (tab.loadErrorCode === -105 || tab.loadErrorCode === -137)
            text: qsTr("Search for “%1”").arg(Core.hostOf(tab.loadErrorUrl))
            onClicked: { var h = Core.hostOf(tab.loadErrorUrl); tab.loadError = ""; tab.load(Core.resolveInput(h.replace(/\.(com|org|net)$/, ""))) }
        }
        PushButton {
            visible: !tab.crashed && tab.loadErrorCode === -20
            text: qsTr("Allow on This Site")
            onClicked: { Core.setSiteSetting(Core.hostOf(tab.loadErrorUrl), "blocker", "false"); tab.loadError = ""; if (tab.view) tab.view.url = tab.loadErrorUrl }
        }
        PushButton {
            visible: tab.view && tab.view.canGoBack
            textOnly: true; text: qsTr("Go Back")
            onClicked: { tab.loadError = ""; tab.crashed = false; tab.view.goBack() }
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
                Core.settingsJson
                var s = JSON.parse(Core.siteSettings(tab.host))
                return s.forceDark !== undefined ? s.forceDark : tab.win.app.websitesDark
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
                    if (info.errorDomain !== WebEngineView.CertificateErrorDomain) {
                        tab.loadErrorCode = info.errorCode
                        tab.loadErrorUrl = info.url.toString()
                        tab.loadError = info.errorString + " (" + Core.hostOf(info.url.toString()) + ")"
                    }
                }
            }

            onNewWindowRequested: function (request) { tab.win.handleNewWindow(request) }
            onFullScreenRequested: function (request) {
                request.accept()
                tab.win.setContentFullScreen(request.toggleOn)
            }
            onFeaturePermissionRequested: function (origin, feature) { tab.win.askPermission(view, origin, feature) }
            onCertificateError: function (error) {
                // Chromium can raise the same error more than once for one
                // load; every deferred copy must be answered. A host the user
                // already chose to visit stays allowed for this session.
                var key = Core.hostOf(error.url.toString()) + "|" + error.type
                if (error.overridable && tab.win.app.certAllowed[key]) { error.acceptCertificate(); return }
                error.defer()
                tab.certPending.push(error)
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
    Interstitial {
        visible: tab.certError !== null
        shape: "burst"; icon: "warning"
        tone: Theme.errorContainer; toneFg: Theme.errorContainerFg
        heading: qsTr("This connection isn’t private")
        body: tab.certError
              ? qsTr("%1 didn’t prove it is who it claims to be, so someone could be reading or changing what you send. Don’t enter passwords or card details here.").arg(Core.hostOf(tab.certError.url.toString()))
              : ""
        detail: tab.certError ? tab.certError.description + "  ·  " + tab.certError.type : ""
        PushButton {
            text: qsTr("Go Back"); primary: true
            onClicked: { tab.answerCert(false); if (tab.view && tab.view.canGoBack) tab.view.goBack() }
        }
        PushButton {
            textOnly: true; text: qsTr("Visit Anyway")
            visible: tab.certError !== null && tab.certError.overridable
            onClicked: tab.answerCert(true)
        }
    }
}
