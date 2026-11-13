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
        sourceComponent: (tab.model.url || "").indexOf("oynix://history") === 0 ? historyPage : startPage
    }

    Component {
        id: startPage
        StartPage { win: tab.win }
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
            backgroundColor: Theme.window
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

            function installScripts() {
                var list = JSON.parse(Core.pageScripts())
                for (var i = 0; i < list.length; i++) {
                    var s = WebEngine.script()
                    s.name = list[i].name
                    s.sourceCode = list[i].source
                    s.injectionPoint = list[i].main ? WebEngineScript.DocumentCreation : WebEngineScript.DocumentReady
                    s.worldId = list[i].main ? WebEngineScript.MainWorld : WebEngineScript.ApplicationWorld
                    s.runsOnSubFrames = list[i].main
                    userScripts.insert(s)
                }
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
                    if (!tab.win.privateMode) Core.recordVisit(url.toString(), title)
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
