import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Window
import QtWebEngine
import QtQuick.Dialogs
import OyNIx

ApplicationWindow {
    id: win
    property var app
    readonly property var winRef: win
    property bool privateMode: false
    property var initialTabs: []
    property int initialIndex: 0
    property var profile: privateMode ? privateProfileInstance() : app.defaultProfile

    property alias tabs: tabModel
    property int currentIndex: 0
    property var currentTab: null
    readonly property var currentView: currentTab ? currentTab.view : null

    property bool sidebarOpen: Core.tabLayout === "vertical"
    property real sidebarWidth: Theme.sidebarWidth
    property bool focusMode: false
    property bool peek: false
    property bool overviewOpen: false
    property bool contentFullScreen: false
    property int uidCounter: 0
    property int windowId: 0
    property var permission: null

    readonly property bool chromeVisible: !contentFullScreen && (!focusMode || peek)
    readonly property bool compactTabs: Core.tabLayout === "compact"
    readonly property bool verticalTabs: Core.tabLayout === "vertical"

    width: 1280
    height: 820
    minimumWidth: 560
    minimumHeight: 380
    visible: true
    color: Theme.window
    // Theme Qt's own popups (tooltips, file dialogs) with the M3 scheme.
    palette.window: Theme.surfaceContainer
    palette.windowText: Theme.surfaceFg
    palette.base: Theme.surfaceContainerLowest
    palette.alternateBase: Theme.surfaceContainerLow
    palette.text: Theme.surfaceFg
    palette.button: Theme.secondaryContainer
    palette.buttonText: Theme.secondaryContainerFg
    palette.highlight: Theme.primary
    palette.highlightedText: Theme.primaryFg
    palette.toolTipBase: Theme.inverseSurface
    palette.toolTipText: Theme.inverseOnSurface
    palette.placeholderText: Theme.surfaceVariantFg
    palette.mid: Theme.outlineVariant
    title: (currentTab && currentTab.model.title ? currentTab.model.title + " — " : "") + (privateMode ? qsTr("OyNIx Private") : "OyNIx")
    flags: Core.useSystemTitleBar ? Qt.Window : (Qt.Window | Qt.FramelessWindowHint)

    onCurrentIndexChanged: syncCurrent()
    onClosing: function (close) { app.windowClosing(win) }

    // Private windows get their own off-the-record profile (no storage name).
    WebEngineProfilePrototype { id: privatePrototype }
    property var privateProfile: null
    function privateProfileInstance() {
        if (!privateProfile) {
            privateProfile = privatePrototype.instance()
            if (Core.userAgent !== "") privateProfile.httpUserAgent = Core.userAgent
            privateProfile.downloadRequested.connect(function (d) { win.app.handleDownload(d) })
            Core.attachProfile(privateProfile)
        }
        return privateProfile
    }

    ListModel { id: tabModel }

    // ── Tab management ──────────────────────────────────────────────
    function tabItem(i) { return tabRepeater.itemAt(i) }

    function syncCurrent() {
        currentTab = (currentIndex >= 0 && currentIndex < tabRepeater.count) ? tabRepeater.itemAt(currentIndex) : null
    }

    function makeTab(url, extra) {
        uidCounter = app.nextTabId()
        return Object.assign({
            uid: uidCounter, url: url, title: "", icon: "", loading: false, progress: 0,
            pinned: false, suspended: false, audible: false, muted: false,
            lastActive: Date.now(), thumb: "", zoom: Core.defaultZoom
        }, extra || {})
    }

    function newTabUrl() {
        if (Core.newTabPage === "blank") return "about:blank"
        if (Core.newTabPage === "homepage") return Core.homepage
        return "oynix://start"
    }

    function firstUnpinned() {
        for (var i = 0; i < tabModel.count; i++) if (!tabModel.get(i).pinned) return i
        return tabModel.count
    }

    function openTab(url, opts) {
        opts = opts || {}
        var idx = opts.afterCurrent ? Math.max(firstUnpinned(), currentIndex + 1) : tabModel.count
        if (opts.pinned) idx = firstUnpinned()
        tabModel.insert(idx, makeTab(url, { title: opts.title || "", pinned: !!opts.pinned, muted: !!opts.muted, suspended: !!opts.suspended }))
        if (opts.request) tabRepeater.itemAt(idx).adopt(opts.request)
        app.extensionHost.tabEvent("tabs.onCreated", [app.extensionHost.tabInfo(win, idx)])
        if (opts.background) {
            if (idx <= currentIndex) currentIndex += 1
            syncCurrent()
        } else {
            activate(idx)
        }
        return idx
    }

    function newTab() {
        openTab(newTabUrl(), {})
        if (verticalTabs) sidebarOpen = true
        Qt.callLater(addressField.focusField)
    }

    function activate(i) {
        if (i < 0 || i >= tabModel.count) return
        if (currentTab && i !== currentIndex) currentTab.snapshot()
        if (tabModel.get(i).suspended) tabModel.setProperty(i, "suspended", false)
        tabModel.setProperty(i, "lastActive", Date.now())
        currentIndex = i
        syncCurrent()
        app.extensionHost.tabEvent("tabs.onActivated", [{ tabId: tabModel.get(i).uid, windowId: win.windowId }])
    }

    function closeTab(i, noHistory) {
        if (i < 0 || i >= tabModel.count) return
        var t = tabModel.get(i)
        var closedId = t.uid
        if (!privateMode && !noHistory) Core.pushClosedTab(t.url, t.title)
        if (tabModel.count === 1) {
            if (Core.closeWindowWithLastTab) { win.close(); return }
            tabModel.insert(1, makeTab(newTabUrl(), {}))
        }
        var next = currentIndex
        if (i < currentIndex) next = currentIndex - 1
        else if (i === currentIndex) next = Math.min(i, tabModel.count - 2)
        tabModel.remove(i)
        app.extensionHost.tabEvent("tabs.onRemoved", [closedId, { windowId: win.windowId, isWindowClosing: false }])
        currentIndex = -1
        currentIndex = Math.max(0, next)
        app.scheduleSave()
    }

    function closeOtherTabs(i) {
        for (var k = tabModel.count - 1; k >= 0; k--) if (k !== i && !tabModel.get(k).pinned) { closeTab(k); if (k < i) i-- }
    }
    function closeTabsToRight(i) {
        for (var k = tabModel.count - 1; k > i; k--) closeTab(k)
    }

    function moveTab(from, to) {
        if (from === to || from < 0 || to < 0 || from >= tabModel.count || to >= tabModel.count) return
        var cur = currentIndex
        tabModel.move(from, to, 1)
        if (from === cur) cur = to
        else if (from < cur && to >= cur) cur -= 1
        else if (from > cur && to <= cur) cur += 1
        currentIndex = cur
        syncCurrent()
        app.scheduleSave()
    }

    function togglePin(i) {
        var pinned = !tabModel.get(i).pinned
        tabModel.setProperty(i, "pinned", pinned)
        var target = pinned ? firstUnpinned() : firstUnpinned() - 1
        if (pinned) {
            // firstUnpinned() now may be i itself; place at end of pinned group
            var n = 0
            for (var k = 0; k < tabModel.count; k++) if (tabModel.get(k).pinned && k !== i) n++
            target = n
        } else {
            var p = 0
            for (var j = 0; j < tabModel.count; j++) if (tabModel.get(j).pinned) p++
            target = p
        }
        moveTab(i, Math.min(target, tabModel.count - 1))
    }

    function toggleMute(i) { tabModel.setProperty(i, "muted", !tabModel.get(i).muted) }

    function duplicateTab(i) { openTab(tabModel.get(i).url, { afterCurrent: true }) }

    function reopenClosedTab() {
        var j = Core.popClosedTab()
        if (j) { var c = JSON.parse(j); openTab(c.url, { title: c.title, afterCurrent: true }) }
    }

    function moveToNewWindow(i) {
        var t = tabModel.get(i)
        app.newWindow({ tabs: [{ url: t.url, title: t.title }], private: privateMode })
        closeTab(i, true)
    }

    function navigate(text) {
        var u = Core.resolveInput(text)
        if (u.indexOf("oynix://settings") === 0) { app.openSettings(""); return }
        if (u.indexOf("oynix://privacy") === 0) { openPrivacyReport(); return }
        if (!currentTab) { openTab(u, {}); return }
        currentTab.load(u)
        focusContent()
    }

    function handleNewWindow(request) {
        // 0 InNewWindow, 1 InNewTab, 2 InNewDialog, 3 InNewBackgroundTab
        var bg = request.destination === 3 && Core.openLinksInBackground
        openTab("", { request: request, background: bg, afterCurrent: true })
    }

    function focusContent() {
        if (currentView) currentView.forceActiveFocus()
        else contentArea.forceActiveFocus()
    }

    function setContentFullScreen(on) {
        contentFullScreen = on
        if (on) win.showFullScreen()
        else win.showNormal()
    }

    function askPermission(view, origin, feature) { permission = { view: view, origin: origin, feature: feature } }

    function findResult(r) { findBar.setResult(r) }

    function copyText(t) { clipHelper.text = t; clipHelper.selectAll(); clipHelper.copy() }

    // Open an internal page (history, settings…) or switch to it if open.
    function openInternal(u) {
        for (var i = 0; i < tabModel.count; i++) if ((tabModel.get(i).url || "").indexOf(u) === 0) { activate(i); return }
        var cur = currentTab
        if (cur && (cur.model.url === "oynix://start" || cur.model.url === "about:blank")) cur.load(u)
        else openTab(u, { afterCurrent: true })
    }

    // Back/forward history list for the current tab.
    function showNavHistory(anchor, back) {
        if (!currentView) return
        navMenu.back = back
        var p = anchor.mapToItem(win.contentItem, 0, anchor.height + 6)
        navMenu.x = p.x; navMenu.y = p.y
        navMenu.open()
    }

    function openPrivacyReport() { openInternal("oynix://privacy") }

    // A tab whose own address turned out to be a download keeps no page:
    // point it back at the last page it showed, or close it (as Chrome
    // does), so session restore doesn't download the file again.
    function settleDownloadTab(u) {
        for (var i = tabModel.count - 1; i >= 0; i--) {
            if (tabModel.get(i).url !== u) continue
            var t = tabItem(i), shown = t && t.view ? t.view.url.toString() : ""
            if (shown && shown !== u && shown !== "about:blank") tabModel.setProperty(i, "url", shown)
            else closeTab(i, true)
        }
    }

    function showTabMenu(i, item) {
        tabMenu.index = i
        var p = item.mapToItem(win.contentItem, 0, item.height + 4)
        tabMenu.x = Math.min(p.x, win.width - tabMenu.width - 8)
        tabMenu.y = p.y
        tabMenu.open()
    }

    function showContextMenu(view, request) {
        ctxMenu.req = request
        ctxMenu.view = view
        var p = view.mapToItem(win.contentItem, request.position.x, request.position.y)
        ctxMenu.x = Math.min(p.x, win.width - ctxMenu.width - 8)
        ctxMenu.y = Math.min(p.y, win.height - ctxMenu.implicitHeight - 8)
        ctxMenu.open()
    }

    function openDevTools() {
        if (!currentView) return
        app.openDevTools(currentView)
    }

    function printPage() {
        if (!currentView) return
        var name = (currentTab.model.title || "page").replace(/[\/\\:*?"<>|]+/g, " ").trim().substring(0, 80) || "page"
        currentView.printToPdf(Core.downloadDir + "/" + name + ".pdf")
        toast.show(qsTr("Saved “%1.pdf” to Downloads").arg(name))
    }

    function sessionData() {
        var list = []
        for (var i = 0; i < tabModel.count; i++) {
            var t = tabModel.get(i)
            if (!t.url || t.url === "about:blank") continue
            list.push({ url: t.url, title: t.title, pinned: t.pinned, muted: t.muted })
        }
        return { tabs: list, current: currentIndex, sidebar: sidebarOpen, x: win.x, y: win.y, w: win.width, h: win.height, maximized: win.visibility === Window.Maximized }
    }

    Component.onCompleted: {
        windowId = app.nextWindowId()
        if (initialTabs && initialTabs.length) {
            for (var i = 0; i < initialTabs.length; i++) {
                var t = initialTabs[i]
                // Only the selected tab loads now; the rest restore lazily.
                tabModel.append(makeTab(t.url, { title: t.title || "", pinned: !!t.pinned, muted: !!t.muted, suspended: i !== initialIndex }))
            }
            activate(Math.min(Math.max(0, initialIndex), tabModel.count - 1))
        } else {
            openTab(newTabUrl(), {})
            if (newTabUrl() === "oynix://start") Qt.callLater(addressField.focusField)
        }
    }

    function toastMessage(t) { toast.show(t) }

    // "Install this extension?" (downloads of .xpi / .crx files)
    property var installAsk: null

    // WebUSB device chooser: {origin, devices:[{key,name,ids,paired}], cb}
    property var usbAsk: null
    property string usbPick: ""
    function askUsbDevice(origin, devices, cb) {
        if (usbAsk) usbAsk.cb("")
        usbPick = devices.length ? devices[0].key : ""
        usbAsk = { origin: origin, devices: devices, cb: cb }
    }
    function answerUsb(key) { var a = usbAsk; usbAsk = null; if (a) a.cb(key) }
    function askInstallExtension(name, source, cb) { installAsk = { name: name, source: source, cb: cb } }

    // "Ask where to save each file"
    FileDialog {
        id: downloadDialog
        property var download: null
        property var done: null
        title: qsTr("Save File")
        fileMode: FileDialog.SaveFile
        onAccepted: {
            console.log("OyNIx: save download to", selectedFile)
            var p = Core.localPath(selectedFile.toString())
            var slash = p.lastIndexOf("/")
            download.downloadDirectory = p.substring(0, slash)
            download.downloadFileName = p.substring(slash + 1)
            var cb = done; done = null; cb(true)
        }
        onRejected: { console.log("OyNIx: download save cancelled"); var cb = done; done = null; if (cb) cb(false) }
    }
    function askDownloadLocation(download, cb) {
        downloadDialog.download = download
        downloadDialog.done = cb
        downloadDialog.selectedFile = "file://" + download.downloadDirectory + "/" + download.downloadFileName
        downloadDialog.open()
    }

    // Hidden helper for clipboard writes.
    TextEdit { id: clipHelper; visible: false }

    // ── Timers ──────────────────────────────────────────────────────
    Timer {
        // Unload tabs that have not been looked at for a while.
        interval: 60 * 1000
        repeat: true
        running: Core.suspendInactiveTabs
        onTriggered: {
            var limit = Date.now() - Math.max(1, Core.suspendAfterMinutes) * 60 * 1000
            for (var i = 0; i < tabModel.count; i++) {
                var t = tabModel.get(i)
                if (i !== win.currentIndex && !t.suspended && !t.audible && t.lastActive < limit && t.url.indexOf("oynix://") !== 0)
                    tabModel.setProperty(i, "suspended", true)
            }
        }
    }

    // ── Shortcuts ───────────────────────────────────────────────────
    Shortcut { sequences: ["Ctrl+T"]; onActivated: win.newTab() }
    Shortcut { sequences: ["Ctrl+W", "Ctrl+F4"]; onActivated: win.closeTab(win.currentIndex) }
    Shortcut { sequences: ["Ctrl+Shift+T"]; onActivated: win.reopenClosedTab() }
    Shortcut { sequences: ["Ctrl+N"]; onActivated: win.app.newWindow({}) }
    Shortcut { sequences: ["Ctrl+Shift+N", "Ctrl+Shift+P"]; onActivated: win.app.newWindow({ private: true }) }
    Shortcut { sequences: ["Ctrl+Shift+W"]; onActivated: win.close() }
    Shortcut { sequences: ["Ctrl+Q"]; onActivated: win.app.quit() }
    Shortcut { sequences: ["Ctrl+L", "Alt+D", "F6"]; onActivated: { if (win.focusMode) win.peek = true; addressField.focusField() } }
    Shortcut { sequences: ["Ctrl+K", "Ctrl+E"]; onActivated: { addressField.focusField() } }
    Shortcut { sequences: ["Ctrl+R", "F5"]; onActivated: if (win.currentView) win.currentView.reload() }
    Shortcut { sequences: ["Ctrl+Shift+R", "Shift+F5"]; context: Qt.WindowShortcut; onActivated: if (win.currentTab) win.currentTab.toggleReader() }
    Shortcut { sequences: ["Ctrl+F5"]; onActivated: if (win.currentView) win.currentView.reloadAndBypassCache() }
    Shortcut { sequences: ["Escape"]; enabled: win.currentTab && win.currentTab.model.loading; onActivated: win.currentView.stop() }
    Shortcut { sequences: ["Alt+Left", "Ctrl+["]; onActivated: if (win.currentView) win.currentView.goBack() }
    Shortcut { sequences: ["Alt+Right", "Ctrl+]"]; onActivated: if (win.currentView) win.currentView.goForward() }
    Shortcut { sequences: ["Alt+Home"]; onActivated: win.navigate(Core.homepage) }
    Shortcut { sequences: ["Ctrl+F"]; onActivated: findBar.open() }
    Shortcut { sequences: ["Ctrl+G", "F3"]; onActivated: findBar.find(false) }
    Shortcut { sequences: ["Ctrl+Shift+G", "Shift+F3"]; onActivated: findBar.find(true) }
    Shortcut { sequences: ["Ctrl+D"]; onActivated: if (win.currentTab) Core.toggleBookmark(win.currentTab.model.url, win.currentTab.model.title) }
    Shortcut { sequences: ["Ctrl+Shift+D"]; onActivated: if (win.currentTab) { Core.addToReadingList(win.currentTab.model.url, win.currentTab.model.title); toast.show(qsTr("Added to Reading List")) } }
    Shortcut { sequences: ["Ctrl+Shift+L", "Ctrl+Alt+S"]; onActivated: win.sidebarOpen = !win.sidebarOpen }
    Shortcut { sequences: ["Ctrl+H"]; onActivated: win.openInternal("oynix://history") }
    Shortcut { sequences: ["Ctrl+Shift+H"]; onActivated: { win.sidebarOpen = true; sidebar.panel = "history" } }
    Shortcut { sequences: ["Ctrl+Shift+B", "Ctrl+B"]; onActivated: { win.sidebarOpen = true; sidebar.panel = "bookmarks" } }
    Shortcut { sequences: ["Ctrl+Shift+\\", "Ctrl+Shift+A"]; onActivated: win.overviewOpen = !win.overviewOpen }
    Shortcut { sequences: ["Ctrl+Shift+F"]; onActivated: { win.focusMode = !win.focusMode; win.peek = false } }
    Shortcut { sequences: ["F11"]; onActivated: win.visibility === Window.FullScreen ? win.showNormal() : win.showFullScreen() }
    Shortcut { sequences: ["Ctrl+Tab", "Ctrl+PgDown"]; onActivated: win.activate((win.currentIndex + 1) % tabModel.count) }
    Shortcut { sequences: ["Ctrl+Shift+Tab", "Ctrl+PgUp"]; onActivated: win.activate((win.currentIndex - 1 + tabModel.count) % tabModel.count) }
    Shortcut { sequences: ["Ctrl+Shift+PgDown"]; onActivated: win.moveTab(win.currentIndex, Math.min(tabModel.count - 1, win.currentIndex + 1)) }
    Shortcut { sequences: ["Ctrl+Shift+PgUp"]; onActivated: win.moveTab(win.currentIndex, Math.max(0, win.currentIndex - 1)) }
    Shortcut { sequences: ["Ctrl+1"]; onActivated: win.activate(0) }
    Shortcut { sequences: ["Ctrl+2"]; onActivated: win.activate(1) }
    Shortcut { sequences: ["Ctrl+3"]; onActivated: win.activate(2) }
    Shortcut { sequences: ["Ctrl+4"]; onActivated: win.activate(3) }
    Shortcut { sequences: ["Ctrl+5"]; onActivated: win.activate(4) }
    Shortcut { sequences: ["Ctrl+6"]; onActivated: win.activate(5) }
    Shortcut { sequences: ["Ctrl+7"]; onActivated: win.activate(6) }
    Shortcut { sequences: ["Ctrl+8"]; onActivated: win.activate(7) }
    Shortcut { sequences: ["Ctrl+9"]; onActivated: win.activate(tabModel.count - 1) }
    Shortcut { sequences: ["Ctrl+=", "Ctrl++", "Ctrl+Shift+="]; onActivated: if (win.currentTab) win.currentTab.setZoom(win.currentTab.model.zoom + 0.1) }
    Shortcut { sequences: ["Ctrl+-"]; onActivated: if (win.currentTab) win.currentTab.setZoom(win.currentTab.model.zoom - 0.1) }
    Shortcut { sequences: ["Ctrl+0"]; onActivated: if (win.currentTab) win.currentTab.setZoom(Core.defaultZoom) }
    Shortcut { sequences: ["Ctrl+P"]; onActivated: win.printPage() }
    Shortcut { sequences: ["Ctrl+S"]; onActivated: if (win.currentView) win.currentView.triggerWebAction(WebEngineView.SavePage) }
    Shortcut { sequences: ["Ctrl+U"]; onActivated: if (win.currentTab) win.openTab("view-source:" + win.currentTab.model.url, { afterCurrent: true }) }
    Shortcut { sequences: ["F12", "Ctrl+Shift+I", "Ctrl+Alt+I"]; onActivated: win.openDevTools() }
    Shortcut { sequences: ["Ctrl+,"]; onActivated: win.app.openSettings("") }
    Shortcut { sequences: ["Ctrl+J", "Ctrl+Shift+Y"]; onActivated: downloadsPopover.open() }

    // ── Layout ──────────────────────────────────────────────────────
    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Toolbar
        Rectangle {
            id: toolbar
            Layout.fillWidth: true
            Layout.preferredHeight: win.chromeVisible ? Theme.toolbarHeight : 0
            visible: Layout.preferredHeight > 0
            clip: true
            color: win.privateMode ? Theme.toolbarPrivate : Theme.toolbar
            Behavior on Layout.preferredHeight { enabled: win.focusMode; NumberAnimation { duration: Theme.animFast } }

            HoverHandler { id: toolbarHover; onHoveredChanged: if (!hovered && win.focusMode && !addressField.editing) win.peek = false }

            // Drag the window by its toolbar; double-click to zoom.
            Item {
                anchors.fill: parent
                DragHandler { target: null; onActiveChanged: if (active) win.startSystemMove() }
                TapHandler {
                    acceptedButtons: Qt.LeftButton
                    onDoubleTapped: win.visibility === Window.Maximized ? win.showNormal() : win.showMaximized()
                }
            }

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 10
                spacing: 4

                WindowControls {
                    window: win
                    visible: !Core.useSystemTitleBar && Core.windowControlsSide !== "right"
                    Layout.rightMargin: 10
                }
                IconButton {
                    glyph: "sidebar"; tip: qsTr("Show Sidebar") + "  (Ctrl+Shift+L)"
                    checked: win.sidebarOpen
                    iconColor: Theme.text
                    onClicked: win.sidebarOpen = !win.sidebarOpen
                }
                IconButton {
                    id: backBtn
                    glyph: "back"; tip: qsTr("Back — right-click for history")
                    enabled: win.currentView !== null && win.currentView.canGoBack
                    onClicked: win.currentView.goBack()
                    TapHandler { acceptedButtons: Qt.RightButton; onTapped: win.showNavHistory(backBtn, true) }
                    onPressAndHold: win.showNavHistory(backBtn, true)
                }
                IconButton {
                    id: fwdBtn
                    glyph: "forward"; tip: qsTr("Forward — right-click for history")
                    enabled: win.currentView !== null && win.currentView.canGoForward
                    onClicked: win.currentView.goForward()
                    TapHandler { acceptedButtons: Qt.RightButton; onTapped: win.showNavHistory(fwdBtn, false) }
                    onPressAndHold: win.showNavHistory(fwdBtn, false)
                }

                Item { Layout.fillWidth: !win.compactTabs; Layout.preferredWidth: win.compactTabs ? 4 : -1 }

                AddressField {
                    id: addressField
                    win: winRef
                    Layout.preferredWidth: win.compactTabs ? Math.min(380, toolbar.width * 0.3) : Math.min(700, Math.max(280, toolbar.width * 0.46))
                    Layout.minimumWidth: 200
                    onSiteInfoRequested: function (anchor) {
                        var p = anchor.mapToItem(win.contentItem, 0, anchor.height + 6)
                        sitePopover.x = p.x - 8; sitePopover.y = p.y
                        sitePopover.open()
                    }
                    onEditingChanged: if (!editing && win.focusMode && !toolbarHover.hovered) win.peek = false
                }

                TabStrip {
                    visible: win.compactTabs
                    win: winRef
                    compact: true
                    Layout.fillWidth: true
                    Layout.leftMargin: 6
                }
                Item { Layout.fillWidth: !win.compactTabs; visible: !win.compactTabs }

                IconButton {
                    id: claudeBtn
                    visible: Core.agentEnabled && !win.privateMode
                    glyph: "sparkles"; tip: qsTr("Claude")
                    checked: win.sidebarOpen && sidebar.panel === "claude"
                    onClicked: {
                        if (win.sidebarOpen && sidebar.panel === "claude") win.sidebarOpen = false
                        else { win.sidebarOpen = true; sidebar.panel = "claude" }
                    }
                }
                IconButton {
                    id: shieldBtn
                    glyph: "shield"
                    tip: qsTr("Privacy Report for This Site")
                    property int count: 0
                    badge: Core.contentBlocker ? count : 0
                    iconColor: Core.contentBlocker ? Theme.text : Theme.textFaint
                    Timer {
                        interval: 1200; repeat: true; running: win.active && win.currentTab !== null
                        onTriggered: shieldBtn.count = JSON.parse(Core.siteReport(win.currentTab.host)).blocked
                    }
                    onClicked: {
                        var p = shieldBtn.mapToItem(win.contentItem, shieldBtn.width - privacyPopover.width, shieldBtn.height + 6)
                        privacyPopover.x = p.x; privacyPopover.y = p.y
                        privacyPopover.open()
                    }
                }
                IconButton {
                    id: dlBtn
                    visible: win.app.downloads.length > 0
                    glyph: "download"; tip: qsTr("Downloads")
                    badge: win.app.activeDownloads
                    onClicked: downloadsPopover.open()
                }
                IconButton {
                    id: shareBtn
                    glyph: "share"; tip: qsTr("Share")
                    enabled: win.currentTab !== null && !win.currentTab.internal
                    onClicked: {
                        var p = shareBtn.mapToItem(win.contentItem, shareBtn.width - shareMenu.width, shareBtn.height + 6)
                        shareMenu.x = p.x; shareMenu.y = p.y
                        shareMenu.open()
                    }
                }
                // Extension action buttons (Chrome / Firefox extensions with a popup)
                Repeater {
                    model: {
                        var host = win.app.extensionHost
                        host.revision; Core.extensionsRevision
                        if (win.privateMode || !host.supported) return []
                        return JSON.parse(Core.extensions()).filter(function (e) { return e.enabled && e.kind !== "oynix" && host.isLoaded(e.id) })
                    }
                    delegate: AbstractButton {
                        id: extBtn
                        required property var modelData
                        implicitWidth: 30; implicitHeight: 28
                        hoverEnabled: true
                        focusPolicy: Qt.NoFocus
                        background: Rectangle { radius: 6; color: extBtn.pressed ? Theme.pressed : (extBtn.hovered ? Theme.hover : "transparent") }
                        contentItem: Item {
                            Image {
                                anchors.centerIn: parent
                                width: 16; height: 16
                                sourceSize: Qt.size(32, 32)
                                source: extBtn.modelData.icon ? "file://" + extBtn.modelData.icon : ""
                                visible: status === Image.Ready
                            }
                            Icon { anchors.centerIn: parent; visible: !extBtn.modelData.icon; name: "puzzle"; size: 16 }
                            Rectangle {
                                readonly property var b: win.app.extensionHost.badgeFor(extBtn.modelData)
                                visible: b !== null && b.text !== ""
                                anchors.right: parent.right; anchors.bottom: parent.bottom
                                anchors.rightMargin: -2; anchors.bottomMargin: -1
                                height: 12; width: Math.max(12, bt.implicitWidth + 6); radius: 6
                                color: b ? b.color : "transparent"
                                Text { id: bt; anchors.centerIn: parent; text: parent.b ? parent.b.text : ""; color: "white"; font.pixelSize: 8; font.bold: true }
                            }
                        }
                        ToolTip.visible: hovered; ToolTip.delay: 600
                        ToolTip.text: { var b = win.app.extensionHost.badgeFor(modelData); return b && b.title ? b.title : modelData.name }
                        onClicked: {
                            var u = win.app.extensionHost.popupUrl(modelData.id)
                            if (!u) {
                                // No popup: the extension handles clicks itself.
                                var info = win.app.extensionHost.tabInfo(win, win.currentIndex)
                                win.app.extBridge.dispatch("action.onClicked", JSON.stringify([modelData.webId, info]))
                                return
                            }
                            extPopup.url = u
                            var p = extBtn.mapToItem(win.contentItem, extBtn.width, extBtn.height + 6)
                            extPopup.x = Math.max(8, p.x - extPopup.width); extPopup.y = p.y
                            extPopup.open()
                        }
                    }
                }
                IconButton {
                    id: extMenuBtn
                    glyph: "puzzle"; tip: qsTr("Extensions")
                    visible: !win.privateMode && Core.extensionsRevision >= 0 && JSON.parse(Core.extensions()).length > 0
                    onClicked: {
                        var p = extMenuBtn.mapToItem(win.contentItem, extMenuBtn.width - extMenu.width, extMenuBtn.height + 6)
                        extMenu.x = p.x; extMenu.y = p.y
                        extMenu.open()
                    }
                }
                IconButton { glyph: "plus"; tip: qsTr("New Tab") + "  (Ctrl+T)"; onClicked: win.newTab() }
                IconButton {
                    glyph: "tabs"; tip: qsTr("Show All Tabs") + "  (Ctrl+Shift+\\)"
                    checked: win.overviewOpen
                    iconColor: Theme.text
                    onClicked: win.overviewOpen = !win.overviewOpen
                }
                IconButton {
                    id: moreBtn
                    glyph: "more"; tip: qsTr("Menu")
                    onClicked: {
                        var p = moreBtn.mapToItem(win.contentItem, moreBtn.width - mainMenu.width, moreBtn.height + 6)
                        mainMenu.x = p.x; mainMenu.y = p.y
                        mainMenu.open()
                    }
                }
                WindowControls {
                    window: win
                    visible: !Core.useSystemTitleBar && Core.windowControlsSide === "right"
                    Layout.leftMargin: 10
                }
            }
        }

        // Standard tab bar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: tabStrip.implicitHeight
            visible: win.chromeVisible && Core.tabLayout === "standard" && tabModel.count > 1
            color: win.privateMode ? Theme.toolbarPrivate : Theme.toolbar
            TabStrip {
                id: tabStrip
                anchors.fill: parent
                anchors.leftMargin: 8
                anchors.rightMargin: 8
                win: winRef
            }
        }

        // Favorites bar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            visible: win.chromeVisible && Core.showFavoritesBar
            color: win.privateMode ? Theme.toolbarPrivate : Theme.toolbar
            Row {
                anchors.fill: parent
                anchors.leftMargin: 10
                spacing: 2
                clip: true
                Repeater {
                    model: { Core.bookmarksRevision; return JSON.parse(Core.bookmarks("favorites")) }
                    delegate: AbstractButton {
                        required property var modelData
                        height: 24
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(180, favRow.implicitWidth + 16)
                        hoverEnabled: true
                        background: Rectangle { radius: 5; color: parent.hovered ? Theme.hover : "transparent" }
                        contentItem: Row {
                            id: favRow
                            spacing: 6
                            leftPadding: 2
                            Favicon { size: 14; url: modelData.url; title: modelData.title; anchors.verticalCenter: parent.verticalCenter }
                            Txt { text: modelData.title; font.pixelSize: 12; width: Math.min(implicitWidth, 140); anchors.verticalCenter: parent.verticalCenter }
                        }
                        onClicked: win.navigate(modelData.url)
                    }
                }
            }
        }

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 1; color: Theme.separator; visible: win.chromeVisible }

        FindBar { id: findBar; win: winRef; visible: false; Layout.fillWidth: true }

        // Permission request bar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            visible: win.permission !== null
            color: Theme.card
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.separator }
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 10
                spacing: 10
                Icon {
                    name: !win.permission ? "" : (win.permission.feature === WebEngineView.Geolocation ? "location"
                          : win.permission.feature === WebEngineView.Notifications ? "bell"
                          : win.permission.feature === WebEngineView.MediaAudioCapture ? "mic" : "camera")
                    size: 16; color: Theme.accent
                }
                Txt {
                    Layout.fillWidth: true
                    text: !win.permission ? "" : qsTr("“%1” wants to %2").arg(Core.hostOf(win.permission.origin.toString())).arg(
                        win.permission.feature === WebEngineView.Geolocation ? qsTr("use your location")
                      : win.permission.feature === WebEngineView.Notifications ? qsTr("show notifications")
                      : win.permission.feature === WebEngineView.MediaAudioCapture ? qsTr("use your microphone")
                      : win.permission.feature === WebEngineView.MediaVideoCapture ? qsTr("use your camera")
                      : win.permission.feature === WebEngineView.MediaAudioVideoCapture ? qsTr("use your camera and microphone")
                      : qsTr("share your screen"))
                }
                PushButton { text: qsTr("Don’t Allow"); onClicked: { win.permission.view.grantFeaturePermission(win.permission.origin, win.permission.feature, false); win.permission = null } }
                PushButton { text: qsTr("Allow"); primary: true; onClicked: { win.permission.view.grantFeaturePermission(win.permission.origin, win.permission.feature, true); win.permission = null } }
            }
        }

        // Content row
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Sidebar {
                id: sidebar
                win: winRef
                Layout.fillHeight: true
                Layout.preferredWidth: win.sidebarOpen && !win.focusMode && !win.contentFullScreen ? win.sidebarWidth : 0
                visible: Layout.preferredWidth > 0
                Behavior on Layout.preferredWidth { NumberAnimation { duration: Theme.anim; easing.type: Easing.OutCubic } }
                // Resize grip
                MouseArea {
                    anchors.right: parent.right
                    width: 5; height: parent.height
                    cursorShape: Qt.SizeHorCursor
                    property real startX: 0
                    onPressed: function (m) { startX = m.x }
                    onPositionChanged: function (m) { win.sidebarWidth = Math.max(180, Math.min(480, win.sidebarWidth + m.x - startX)) }
                }
            }

            Item {
                id: contentArea
                Layout.fillWidth: true
                Layout.fillHeight: true
                focus: true
                clip: true

                Repeater {
                    id: tabRepeater
                    model: tabModel
                    onCountChanged: win.syncCurrent()
                    onItemAdded: win.syncCurrent()
                    delegate: WebTab {
                        win: winRef
                        anchors.fill: parent
                    }
                }

                // Focus mode: hover the top edge to reveal the toolbar.
                MouseArea {
                    anchors.top: parent.top
                    width: parent.width
                    height: 6
                    visible: win.focusMode && !win.peek
                    hoverEnabled: true
                    onEntered: win.peek = true
                }

                // Link status bubble
                Rectangle {
                    anchors.left: parent.left
                    anchors.bottom: parent.bottom
                    anchors.margins: 6
                    visible: win.currentTab !== null && win.currentTab.hoveredLink !== ""
                    width: Math.min(statusText.implicitWidth + 16, parent.width * 0.6)
                    height: 22
                    radius: 6
                    color: Theme.popover
                    border.color: Theme.separator
                    Txt {
                        id: statusText
                        anchors.fill: parent
                        anchors.leftMargin: 8; anchors.rightMargin: 8
                        text: win.currentTab ? win.currentTab.hoveredLink : ""
                        font.pixelSize: 12
                        color: Theme.textDim
                        elide: Text.ElideMiddle
                    }
                }

                TabOverview {
                    anchors.fill: parent
                    win: winRef
                    shown: win.overviewOpen
                    z: 20
                }

                // ── Claude is working: glow border + status chip ──
                readonly property bool agentHere: win.app.agent.working && win.currentTab !== null && win.currentTab.model.uid === win.app.agent.activeTabUid
                Rectangle {
                    id: agentGlow
                    anchors.fill: parent
                    z: 40
                    color: "transparent"
                    radius: 0
                    border.width: 3
                    border.color: Theme.primary
                    visible: parent.agentHere
                    opacity: 0.6 + 0.4 * Math.sin(glowPhase.p)
                    QtObject { id: glowPhase; property real p: 0 }
                    NumberAnimation { target: glowPhase; property: "p"; from: 0; to: Math.PI * 2; duration: 1800; loops: Animation.Infinite; running: agentGlow.visible }
                }
                Rectangle {
                    z: 41
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.top: parent.top
                    anchors.topMargin: 10
                    visible: parent.agentHere || (win.app.agent.stopped && win.app.agent.lastActivity > 0 && Date.now() - win.app.agent.lastActivity < 60000)
                    height: 40
                    width: chipRow.implicitWidth + 16
                    radius: 20
                    color: Theme.inverseSurface
                    Row {
                        id: chipRow
                        anchors.centerIn: parent
                        spacing: 10
                        LoadingShape { visible: !win.app.agent.stopped; size: 20; color: Theme.inversePrimary; anchors.verticalCenter: parent.verticalCenter }
                        Txt {
                            anchors.verticalCenter: parent.verticalCenter
                            color: Theme.inverseOnSurface
                            text: win.app.agent.stopped ? qsTr("Claude stopped") : (win.app.agent.currentAction ? qsTr("Claude: %1").arg(win.app.agent.currentAction) : qsTr("Claude is using this tab"))
                            width: Math.min(implicitWidth, 360)
                        }
                        PushButton {
                            anchors.verticalCenter: parent.verticalCenter
                            implicitHeight: 30
                            text: win.app.agent.stopped ? qsTr("Resume") : qsTr("Stop")
                            primary: true
                            onClicked: win.app.agent.stopped ? win.app.agent.resume() : win.app.agent.stop()
                        }
                    }
                }

                // ── Permission request for Claude ──
                Rectangle {
                    z: 50
                    anchors.fill: parent
                    visible: win.app.agent.prompt !== null && win.app.agent.prompt.win === win
                    color: Theme.alpha("#000000", 0.32)
                    MouseArea { anchors.fill: parent }
                    Rectangle {
                        anchors.centerIn: parent
                        width: Math.min(440, parent.width - 48)
                        height: permCol.implicitHeight + 48
                        radius: Theme.shapeXL
                        color: Theme.surfaceContainerHigh
                        ColumnLayout {
                            id: permCol
                            anchors.fill: parent
                            anchors.margins: 24
                            spacing: 14
                            MShape { Layout.alignment: Qt.AlignHCenter; shape: "cookie9"; size: 56; color: Theme.primaryContainer
                                Icon { anchors.centerIn: parent; name: "sparkles"; size: 26; color: Theme.primaryContainerFg } }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                font.pixelSize: 22; font.family: Theme.displayFont
                                text: win.app.agent.prompt ? qsTr("Allow Claude to use %1?").arg(win.app.agent.prompt.host) : ""
                            }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                color: Theme.surfaceVariantFg
                                text: win.app.agent.prompt ? qsTr("%1 wants to %2. Claude will be able to read this site and act on it for you.")
                                      .arg(win.app.agent.prompt.source === "mcp" ? qsTr("Claude (from the Claude app)") : qsTr("Claude"))
                                      .arg(win.app.agent.prompt.detail) : ""
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 8
                                PushButton { Layout.fillWidth: true; primary: true; text: qsTr("Allow for this task"); onClicked: win.app.agent.prompt.answer("task") }
                                PushButton { Layout.fillWidth: true; text: qsTr("Always allow on this site"); onClicked: win.app.agent.prompt.answer("always") }
                                PushButton { Layout.fillWidth: true; textOnly: true; text: qsTr("Don’t allow"); onClicked: win.app.agent.prompt.answer("deny") }
                            }
                        }
                    }
                }

                // ── WebUSB device chooser ──
                Rectangle {
                    z: 50
                    anchors.fill: parent
                    visible: win.usbAsk !== null
                    onVisibleChanged: if (visible) forceActiveFocus()
                    color: Theme.alpha("#000000", 0.32)
                    MouseArea { anchors.fill: parent }
                    Keys.onEscapePressed: win.answerUsb("")
                    Keys.onReturnPressed: if (win.usbPick) win.answerUsb(win.usbPick)
                    Rectangle {
                        anchors.centerIn: parent
                        width: Math.min(460, parent.width - 48)
                        height: usbCol.implicitHeight + 48
                        radius: Theme.shapeXL
                        color: Theme.surfaceContainerHigh
                        ColumnLayout {
                            id: usbCol
                            anchors.fill: parent
                            anchors.margins: 24
                            spacing: 14
                            MShape { Layout.alignment: Qt.AlignHCenter; shape: "cookie9"; size: 56; color: Theme.tertiaryContainer
                                Icon { anchors.centerIn: parent; name: "usb"; size: 26; color: Theme.tertiaryContainerFg } }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                font.pixelSize: 20; font.family: Theme.displayFont
                                text: win.usbAsk ? qsTr("%1 wants to connect to a USB device").arg(Core.hostOf(win.usbAsk.origin)) : ""
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Math.min(240, Math.max(64, usbList.contentHeight + 8))
                                radius: Theme.shapeL
                                color: Theme.surfaceContainerLow
                                ListView {
                                    id: usbList
                                    anchors.fill: parent; anchors.margins: 4
                                    clip: true
                                    model: win.usbAsk ? win.usbAsk.devices : []
                                    delegate: Rectangle {
                                        required property var modelData
                                        width: usbList.width; height: 52
                                        radius: Theme.shapeM
                                        color: win.usbPick === modelData.key ? Theme.secondaryContainer : (devHover.hovered ? Theme.alpha(Theme.text, 0.06) : "transparent")
                                        HoverHandler { id: devHover }
                                        TapHandler { onTapped: win.usbPick = modelData.key; onDoubleTapped: win.answerUsb(modelData.key) }
                                        ColumnLayout {
                                            anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14
                                            spacing: 0
                                            Item { Layout.fillHeight: true }
                                            Txt { Layout.fillWidth: true; text: modelData.name; font.weight: Font.Medium }
                                            Txt { Layout.fillWidth: true; text: modelData.ids + (modelData.paired ? " · " + qsTr("allowed before") : ""); color: Theme.textDim; font.pixelSize: 12 }
                                            Item { Layout.fillHeight: true }
                                        }
                                    }
                                }
                                Txt {
                                    anchors.centerIn: parent
                                    visible: usbList.count === 0
                                    text: qsTr("No compatible devices found")
                                    color: Theme.textDim
                                }
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Item { Layout.fillWidth: true }
                                PushButton { textOnly: true; text: qsTr("Cancel"); onClicked: win.answerUsb("") }
                                PushButton { primary: true; text: qsTr("Connect"); enabled: win.usbPick !== ""; onClicked: win.answerUsb(win.usbPick) }
                            }
                        }
                    }
                }

                // ── Install extension prompt ──
                Rectangle {
                    z: 50
                    anchors.fill: parent
                    visible: win.installAsk !== null
                    onVisibleChanged: if (visible) forceActiveFocus()
                    color: Theme.alpha("#000000", 0.32)
                    MouseArea { anchors.fill: parent }
                    Keys.onEscapePressed: { var a = win.installAsk; win.installAsk = null; a.cb(false) }
                    Rectangle {
                        anchors.centerIn: parent
                        width: Math.min(420, parent.width - 48)
                        height: instCol.implicitHeight + 48
                        radius: Theme.shapeXL
                        color: Theme.surfaceContainerHigh
                        ColumnLayout {
                            id: instCol
                            anchors.fill: parent
                            anchors.margins: 24
                            spacing: 14
                            MShape { Layout.alignment: Qt.AlignHCenter; shape: "flower"; size: 56; color: Theme.secondaryContainer
                                Icon { anchors.centerIn: parent; name: "puzzle"; size: 26; color: Theme.secondaryContainerFg } }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                font.pixelSize: 22; font.family: Theme.displayFont
                                text: win.installAsk ? qsTr("Add “%1”?").arg(win.installAsk.name) : ""
                            }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                color: Theme.surfaceVariantFg
                                text: win.installAsk ? qsTr("This extension comes from %1. Extensions can read and change data on websites you visit — only install ones you trust.").arg(win.installAsk.source) : ""
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Item { Layout.fillWidth: true }
                                PushButton { textOnly: true; text: qsTr("Cancel"); onClicked: { var a = win.installAsk; win.installAsk = null; a.cb(false) } }
                                PushButton { primary: true; text: qsTr("Add Extension"); onClicked: { var a = win.installAsk; win.installAsk = null; a.cb(true) } }
                            }
                        }
                    }
                }

                // Toast
                Rectangle {
                    id: toast
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 26
                    width: toastText.implicitWidth + 28
                    height: 32
                    radius: 16
                    color: Theme.dark ? "#3a3a3d" : "#2c2c2e"
                    opacity: 0
                    visible: opacity > 0
                    z: 30
                    function show(t) { toastText.text = t; opacity = 1; toastTimer.restart() }
                    Behavior on opacity { NumberAnimation { duration: Theme.anim } }
                    Txt { id: toastText; anchors.centerIn: parent; color: "white" }
                    Timer { id: toastTimer; interval: 2200; onTriggered: toast.opacity = 0 }
                }
            }
        }
    }

    // ── Frameless resize edges ──────────────────────────────────────
    Repeater {
        model: Core.useSystemTitleBar || win.visibility === Window.Maximized || win.visibility === Window.FullScreen ? [] : [
            { e: Qt.LeftEdge, c: Qt.SizeHorCursor }, { e: Qt.RightEdge, c: Qt.SizeHorCursor },
            { e: Qt.TopEdge, c: Qt.SizeVerCursor }, { e: Qt.BottomEdge, c: Qt.SizeVerCursor },
            { e: Qt.LeftEdge | Qt.TopEdge, c: Qt.SizeFDiagCursor }, { e: Qt.RightEdge | Qt.BottomEdge, c: Qt.SizeFDiagCursor },
            { e: Qt.RightEdge | Qt.TopEdge, c: Qt.SizeBDiagCursor }, { e: Qt.LeftEdge | Qt.BottomEdge, c: Qt.SizeBDiagCursor }
        ]
        delegate: MouseArea {
            required property var modelData
            readonly property bool l: modelData.e & Qt.LeftEdge
            readonly property bool r: modelData.e & Qt.RightEdge
            readonly property bool t: modelData.e & Qt.TopEdge
            readonly property bool b: modelData.e & Qt.BottomEdge
            readonly property int g: 5
            x: r ? win.width - g : 0
            y: b ? win.height - g : 0
            width: (l || r) ? g : win.width
            height: (t || b) ? g : win.height
            z: 1000
            cursorShape: modelData.c
            onPressed: win.startSystemResize(modelData.e)
        }
    }

    // ── Popovers & menus ────────────────────────────────────────────
    PrivacyPopover { id: privacyPopover; win: winRef; parent: win.contentItem }
    SitePopover { id: sitePopover; win: winRef; parent: win.contentItem }
    DownloadsPopover {
        id: downloadsPopover; app: win.app; parent: win.contentItem
        x: win.width - width - 12; y: Theme.toolbarHeight + 4
    }

    // Extension action popup: a small web view sized to the popup's content.
    Popover {
        id: extPopup
        parent: win.contentItem
        property string url: ""
        padding: 0
        width: Math.max(220, Math.min(800, (popupView.contentsSize.width || 360) + 2))
        height: Math.max(60, Math.min(600, (popupView.contentsSize.height || 420) + 2))
        onClosed: url = ""
        contentItem: Rectangle {
            clip: true
            radius: Theme.shapeL
            color: "white"
            WebEngineView {
                id: popupView
                anchors.fill: parent
                anchors.margins: 1
                profile: win.app.defaultProfile
                url: extPopup.url
                // Extension popups are designed for a white canvas (as in Chrome).
                backgroundColor: "white"
                webChannel: win.app.extChannel
                webChannelWorld: WebEngineScript.MainWorld
                settings.showScrollBars: false
                Component.onCompleted: win.app.extensionHost.installScripts(popupView, JSON.parse(Core.extensionPageScripts("popup")))
                Connections {
                    target: Core
                    function onExtensionsRevisionChanged() { popupView.userScripts.clear(); win.app.extensionHost.installScripts(popupView, JSON.parse(Core.extensionPageScripts("popup"))) }
                }
                onNavigationRequested: function (request) {
                    var u = request.url.toString()
                    if (request.isMainFrame && u !== "" && u.indexOf("chrome-extension://") !== 0) { request.reject(); win.openTab(u, { afterCurrent: true }); extPopup.close() }
                }
                onWindowCloseRequested: extPopup.close()
                onNewWindowRequested: function (request) { win.openTab(request.requestedUrl.toString(), { afterCurrent: true }); extPopup.close() }
            }
        }
    }

    Popover {
        id: extMenu
        parent: win.contentItem
        width: 280
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            Txt { text: qsTr("Extensions"); color: Theme.textDim; font.pixelSize: 12; Layout.leftMargin: 8; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            Repeater {
                model: { Core.extensionsRevision; win.app.extensionHost.revision; return extMenu.opened ? JSON.parse(Core.extensions()) : [] }
                delegate: RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    Layout.leftMargin: 8; Layout.rightMargin: 6
                    Layout.preferredHeight: 30
                    spacing: 8
                    Image { Layout.preferredWidth: 16; Layout.preferredHeight: 16; sourceSize: Qt.size(32, 32); source: modelData.icon ? "file://" + modelData.icon : ""; visible: !!modelData.icon }
                    Icon { visible: !modelData.icon; name: "puzzle"; size: 16; color: Theme.textDim }
                    Txt { text: modelData.name; Layout.fillWidth: true; color: modelData.enabled ? Theme.text : Theme.textDim }
                    Toggle { checked: modelData.enabled; onToggled: win.app.extensionHost.setEnabled(modelData.id, checked) }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Manage Extensions…"); glyph: "sliders"; onClicked: { extMenu.close(); win.app.openSettings("extensions") } }
        }
    }

    Popover {
        id: navMenu
        parent: win.contentItem
        property bool back: true
        width: 320
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            Repeater {
                model: navMenu.opened && win.currentView ? (navMenu.back ? win.currentView.history.backItems : win.currentView.history.forwardItems) : null
                delegate: MenuRow {
                    required property url url
                    required property string title
                    required property int offset
                    glyph: "clock"
                    text: title || Core.prettyUrl(url.toString())
                    onClicked: { navMenu.close(); win.currentView.goBackOrForward(offset) }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { glyph: "list"; text: qsTr("Show Full History"); shortcut: "Ctrl+H"; onClicked: { navMenu.close(); win.openInternal("oynix://history") } }
        }
    }

    Popover {
        id: shareMenu
        parent: win.contentItem
        width: 230
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            MenuRow { text: qsTr("Copy Link"); glyph: "copy"; onClicked: { win.copyText(win.currentTab.model.url); shareMenu.close(); toast.show(qsTr("Link copied")) } }
            MenuRow { text: qsTr("Copy Clean Link"); glyph: "sparkles"; onClicked: { win.copyText(win.currentTab.model.url.split("#")[0]); shareMenu.close(); toast.show(qsTr("Link copied")) } }
            MenuRow { text: qsTr("Add to Reading List"); glyph: "glasses"; onClicked: { Core.addToReadingList(win.currentTab.model.url, win.currentTab.model.title); shareMenu.close(); toast.show(qsTr("Added to Reading List")) } }
            MenuRow { text: qsTr("Add to Favorites"); glyph: "star"; onClicked: { Core.addBookmark(win.currentTab.model.url, win.currentTab.model.title, "favorites"); shareMenu.close(); toast.show(qsTr("Added to Favorites")) } }
            MenuRow { text: qsTr("Email Link…"); glyph: "external"; onClicked: { Qt.openUrlExternally("mailto:?subject=" + encodeURIComponent(win.currentTab.model.title) + "&body=" + encodeURIComponent(win.currentTab.model.url)); shareMenu.close() } }
            MenuRow { text: qsTr("Open in Default Browser"); glyph: "globe"; onClicked: { Qt.openUrlExternally(win.currentTab.model.url); shareMenu.close() } }
        }
    }

    Popover {
        id: mainMenu
        parent: win.contentItem
        width: 270
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            MenuRow { text: qsTr("New Tab"); glyph: "plus"; shortcut: "Ctrl+T"; onClicked: { mainMenu.close(); win.newTab() } }
            MenuRow { text: qsTr("New Window"); glyph: "window"; shortcut: "Ctrl+N"; onClicked: { mainMenu.close(); win.app.newWindow({}) } }
            MenuRow { text: qsTr("New Private Window"); glyph: "mask"; shortcut: "Ctrl+Shift+N"; onClicked: { mainMenu.close(); win.app.newWindow({ private: true }) } }
            MenuRow { text: qsTr("Reopen Closed Tab"); glyph: "reload"; shortcut: "Ctrl+Shift+T"; onClicked: { mainMenu.close(); win.reopenClosedTab() } }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 8
                spacing: 2
                Icon { name: "zoom"; size: 15; color: Theme.text }
                Txt { text: qsTr("Zoom"); Layout.fillWidth: true; Layout.leftMargin: 7 }
                IconButton { glyph: "minus"; enabled: win.currentView !== null; onClicked: win.currentTab.setZoom(win.currentTab.model.zoom - 0.1) }
                Txt { text: Math.round((win.currentTab ? win.currentTab.model.zoom : 1) * 100) + "%"; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignHCenter; font.pixelSize: 12 }
                IconButton { glyph: "plus"; enabled: win.currentView !== null; onClicked: win.currentTab.setZoom(win.currentTab.model.zoom + 0.1) }
                IconButton { glyph: "focus"; tip: qsTr("Full Screen"); onClicked: { mainMenu.close(); win.visibility === Window.FullScreen ? win.showNormal() : win.showFullScreen() } }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Find on Page…"); glyph: "search"; shortcut: "Ctrl+F"; enabled: win.currentView !== null; onClicked: { mainMenu.close(); findBar.open() } }
            MenuRow { text: win.currentTab && win.currentTab.readerActive ? qsTr("Hide Reader") : qsTr("Show Reader"); glyph: "reader"; shortcut: "Ctrl+Shift+R"; enabled: win.currentView !== null; onClicked: { mainMenu.close(); win.currentTab.toggleReader() } }
            MenuRow { text: win.focusMode ? qsTr("Exit Focus Mode") : qsTr("Focus Mode"); glyph: "focus"; shortcut: "Ctrl+Shift+F"; onClicked: { mainMenu.close(); win.focusMode = !win.focusMode; win.peek = false } }
            MenuRow { text: qsTr("Print to PDF…"); glyph: "print"; shortcut: "Ctrl+P"; enabled: win.currentView !== null; onClicked: { mainMenu.close(); win.printPage() } }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Bookmarks"); glyph: "bookmark"; shortcut: "Ctrl+B"; onClicked: { mainMenu.close(); win.sidebarOpen = true; sidebar.panel = "bookmarks" } }
            MenuRow { text: qsTr("Reading List"); glyph: "glasses"; onClicked: { mainMenu.close(); win.sidebarOpen = true; sidebar.panel = "reading" } }
            MenuRow { text: qsTr("History"); glyph: "clock"; shortcut: "Ctrl+H"; onClicked: { mainMenu.close(); win.openInternal("oynix://history") } }
            MenuRow { text: qsTr("Downloads"); glyph: "download"; shortcut: "Ctrl+J"; onClicked: { mainMenu.close(); downloadsPopover.open() } }
            MenuRow { text: qsTr("Privacy Report"); glyph: "shieldCheck"; onClicked: { mainMenu.close(); win.openPrivacyReport() } }
            MenuRow { text: qsTr("Extensions"); glyph: "puzzle"; onClicked: { mainMenu.close(); win.app.openSettings("extensions") } }
            MenuRow { text: qsTr("Profiles"); glyph: "user"; onClicked: { mainMenu.close(); win.app.openSettings("profiles") } }
            MenuRow { text: qsTr("Developer Tools"); glyph: "code"; shortcut: "F12"; visible: Core.developerMode; enabled: win.currentView !== null; onClicked: { mainMenu.close(); win.openDevTools() } }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Settings"); glyph: "sliders"; shortcut: "Ctrl+,"; onClicked: { mainMenu.close(); win.app.openSettings("") } }
            MenuRow { text: qsTr("Quit OyNIx"); glyph: "close"; shortcut: "Ctrl+Q"; onClicked: { mainMenu.close(); win.app.quit() } }
        }
    }

    Popover {
        id: tabMenu
        parent: win.contentItem
        property int index: -1
        readonly property var t: index >= 0 && index < tabModel.count ? tabModel.get(index) : null
        width: 240
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            MenuRow { text: qsTr("New Tab to the Right"); glyph: "plus"; onClicked: { tabMenu.close(); win.activate(tabMenu.index); win.openTab(win.newTabUrl(), { afterCurrent: true }) } }
            MenuRow { text: qsTr("Reload"); glyph: "reload"; onClicked: { tabMenu.close(); var it = tabRepeater.itemAt(tabMenu.index); if (it && it.view) it.view.reload() } }
            MenuRow { text: qsTr("Duplicate"); glyph: "copy"; onClicked: { tabMenu.close(); win.duplicateTab(tabMenu.index) } }
            MenuRow { text: tabMenu.t && tabMenu.t.pinned ? qsTr("Unpin Tab") : qsTr("Pin Tab"); glyph: "pin"; onClicked: { tabMenu.close(); win.togglePin(tabMenu.index) } }
            MenuRow { text: tabMenu.t && tabMenu.t.muted ? qsTr("Unmute Tab") : qsTr("Mute Tab"); glyph: tabMenu.t && tabMenu.t.muted ? "speaker" : "mute"; onClicked: { tabMenu.close(); win.toggleMute(tabMenu.index) } }
            MenuRow { text: qsTr("Bookmark Tab"); glyph: "star"; onClicked: { tabMenu.close(); Core.addBookmark(tabMenu.t.url, tabMenu.t.title, "") } }
            MenuRow { text: qsTr("Copy Link"); glyph: "share"; onClicked: { tabMenu.close(); win.copyText(tabMenu.t.url) } }
            MenuRow { text: qsTr("Move to New Window"); glyph: "window"; onClicked: { tabMenu.close(); win.moveToNewWindow(tabMenu.index) } }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Close Tab"); glyph: "close"; onClicked: { tabMenu.close(); win.closeTab(tabMenu.index) } }
            MenuRow { text: qsTr("Close Other Tabs"); onClicked: { tabMenu.close(); win.closeOtherTabs(tabMenu.index) } }
            MenuRow { text: qsTr("Close Tabs to the Right"); onClicked: { tabMenu.close(); win.closeTabsToRight(tabMenu.index) } }
        }
    }

    // Web page context menu
    Popover {
        id: ctxMenu
        parent: win.contentItem
        property var req: null
        property var view: null
        readonly property bool hasLink: req !== null && req.linkUrl.toString() !== ""
        readonly property bool hasImage: req !== null && req.mediaType === ContextMenuRequest.MediaTypeImage
        readonly property bool hasSelection: req !== null && req.selectedText !== ""
        readonly property bool editable: req !== null && req.isContentEditable
        width: 250
        padding: 6
        function act(a) { close(); view.triggerWebAction(a) }
        contentItem: ColumnLayout {
            spacing: 1
            MenuRow { visible: ctxMenu.hasLink; text: qsTr("Open Link in New Tab"); glyph: "plus"; onClicked: { ctxMenu.close(); win.openTab(ctxMenu.req.linkUrl.toString(), { afterCurrent: true, background: Core.openLinksInBackground }) } }
            MenuRow { visible: ctxMenu.hasLink; text: qsTr("Open Link in New Window"); glyph: "window"; onClicked: { ctxMenu.close(); win.app.newWindow({ tabs: [{ url: ctxMenu.req.linkUrl.toString() }], private: win.privateMode }) } }
            MenuRow { visible: ctxMenu.hasLink && !win.privateMode; text: qsTr("Open Link in Private Window"); glyph: "mask"; onClicked: { ctxMenu.close(); win.app.newWindow({ tabs: [{ url: ctxMenu.req.linkUrl.toString() }], private: true }) } }
            MenuRow { visible: ctxMenu.hasLink; text: qsTr("Add Link to Reading List"); glyph: "glasses"; onClicked: { ctxMenu.close(); Core.addToReadingList(ctxMenu.req.linkUrl.toString(), ctxMenu.req.linkText) } }
            MenuRow { visible: ctxMenu.hasLink; text: qsTr("Copy Link"); glyph: "copy"; onClicked: ctxMenu.act(WebEngineView.CopyLinkToClipboard) }
            MenuRow { visible: ctxMenu.hasLink; text: qsTr("Download Linked File"); glyph: "download"; onClicked: ctxMenu.act(WebEngineView.DownloadLinkToDisk) }
            MenuRow { visible: ctxMenu.hasImage; text: qsTr("Open Image in New Tab"); glyph: "plus"; onClicked: { ctxMenu.close(); win.openTab(ctxMenu.req.mediaUrl.toString(), { afterCurrent: true }) } }
            MenuRow { visible: ctxMenu.hasImage; text: qsTr("Copy Image"); glyph: "copy"; onClicked: ctxMenu.act(WebEngineView.CopyImageToClipboard) }
            MenuRow { visible: ctxMenu.hasImage; text: qsTr("Save Image…"); glyph: "download"; onClicked: ctxMenu.act(WebEngineView.DownloadImageToDisk) }
            MenuRow { visible: ctxMenu.editable; text: qsTr("Cut"); onClicked: ctxMenu.act(WebEngineView.Cut) }
            MenuRow { visible: ctxMenu.hasSelection || ctxMenu.editable; text: qsTr("Copy"); glyph: "copy"; onClicked: ctxMenu.act(WebEngineView.Copy) }
            MenuRow { visible: ctxMenu.editable; text: qsTr("Paste"); onClicked: ctxMenu.act(WebEngineView.Paste) }
            MenuRow { visible: ctxMenu.hasSelection; text: qsTr("Search for “%1”").arg(ctxMenu.req ? ctxMenu.req.selectedText.substring(0, 24) : ""); glyph: "search"; onClicked: { ctxMenu.close(); win.openTab(Core.resolveInput(ctxMenu.req.selectedText), { afterCurrent: true }) } }
            MenuRow { visible: !ctxMenu.hasLink && !ctxMenu.hasImage && !ctxMenu.hasSelection && !ctxMenu.editable; text: qsTr("Back"); glyph: "back"; enabled: ctxMenu.view && ctxMenu.view.canGoBack; onClicked: ctxMenu.act(WebEngineView.Back) }
            MenuRow { visible: !ctxMenu.hasLink && !ctxMenu.hasImage && !ctxMenu.hasSelection && !ctxMenu.editable; text: qsTr("Forward"); glyph: "forward"; enabled: ctxMenu.view && ctxMenu.view.canGoForward; onClicked: ctxMenu.act(WebEngineView.Forward) }
            MenuRow { visible: !ctxMenu.hasLink && !ctxMenu.hasImage && !ctxMenu.hasSelection && !ctxMenu.editable; text: qsTr("Reload"); glyph: "reload"; onClicked: ctxMenu.act(WebEngineView.Reload) }
            MenuRow { visible: !ctxMenu.hasLink && !ctxMenu.hasImage && !ctxMenu.editable; text: qsTr("Show Reader"); glyph: "reader"; onClicked: { ctxMenu.close(); win.currentTab.toggleReader() } }
            MenuRow { visible: !ctxMenu.hasLink && !ctxMenu.hasImage && !ctxMenu.editable; text: qsTr("View Page Source"); glyph: "code"; onClicked: { ctxMenu.close(); win.openTab("view-source:" + win.currentTab.model.url, { afterCurrent: true }) } }
            // Extension context-menu items
            Rectangle { visible: ctxExtRep.count > 0; Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            Repeater {
                id: ctxExtRep
                model: ctxMenu.req && !win.privateMode ? win.app.extensionHost.menuItemsFor(ctxMenu.req) : []
                delegate: MenuRow {
                    required property var modelData
                    glyph: "puzzle"
                    text: String(modelData.title || "").replace("%s", ctxMenu.req ? ctxMenu.req.selectedText.substring(0, 30) : "")
                    enabled: modelData.enabled !== false
                    onClicked: {
                        var r = ctxMenu.req
                        ctxMenu.close()
                        win.app.extensionHost.menuClicked(modelData, r, win.app.extensionHost.tabInfo(win, win.currentIndex))
                    }
                }
            }
            Rectangle { visible: Core.developerMode; Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { visible: Core.developerMode; text: qsTr("Inspect Element"); glyph: "code"; onClicked: { ctxMenu.close(); win.app.openDevTools(ctxMenu.view); ctxMenu.view.triggerWebAction(WebEngineView.InspectElement) } }
        }
    }
}
