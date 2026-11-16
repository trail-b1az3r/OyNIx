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
            privateProfile.httpUserAgent = win.app.userAgentFor(privateProfile)
            privateProfile.downloadRequested.connect(function (d) { win.app.handleDownload(d) })
            Core.attachProfile(privateProfile)
        }
        return privateProfile
    }

    ListModel { id: tabModel }

    // ── Tab management ──────────────────────────────────────────────
    function tabItem(i) { return tabRepeater.itemAt(i) }

    // Open an extension's panel or popup under `anchor`, or send it a click.
    function activateExtension(e, anchor) {
        if (e.kind === "oynix" && (e.hasPanel || e.hasPage)) { openOynixPanel(e, anchor); return }
        var u = app.extensionHost.popupUrl(e.id)
        if (!u) {
            // No popup: the extension handles clicks itself.
            var info = app.extensionHost.tabInfo(win, currentIndex)
            app.extBridge.dispatch("action.onClicked", JSON.stringify([e.webId, info]))
            return
        }
        if (extPopup.recentlyClosed() && extPopup.lastUrl === u) return
        extPopup.url = u
        var p = anchor.mapToItem(win.contentItem, anchor.width, anchor.height + 6)
        extPopup.x = Math.max(8, p.x - extPopup.width); extPopup.y = p.y
        extPopup.open()
    }
    // Can the extension be clicked (panel, page, popup or action)?
    function extensionClickable(e) {
        if (!e.enabled) return false
        if (e.kind === "oynix" && (e.hasPanel || e.hasPage)) return true
        if (e.kind === "oynix" && !e.manifestVersion) return false
        return app.extensionHost.isLoaded(e.id)
    }

    function syncCurrent() {
        currentTab = (currentIndex >= 0 && currentIndex < tabRepeater.count) ? tabRepeater.itemAt(currentIndex) : null
    }

    function makeTab(url, extra) {
        uidCounter = app.nextTabId()
        return Object.assign({
            uid: uidCounter, url: url, title: "", icon: "", loading: false, progress: 0,
            pinned: false, suspended: false, audible: false, muted: false,
            lastActive: Date.now(), thumb: "", zoom: Core.defaultZoom,
            // Tree tabs: pages of the site visited in this tab, the
            // document's outline (both JSON), the section being read.
            pages: "", outline: "", outlineActive: -1, treeOpen: true
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
        // Extension keywords (manifest "omnibox"): "kw some text".
        if (!privateMode) {
            var kws = JSON.parse(Core.webextOmnibox())
            for (var k = 0; k < kws.length; k++) {
                var kw = kws[k].keyword + " "
                if (String(text).indexOf(kw) === 0) {
                    app.extensionHost.tabEvent("ext:omnibox.onInputEntered", [kws[k].ext, String(text).slice(kw.length), "currentTab"])
                    focusContent()
                    return
                }
            }
        }
        var u = Core.resolveInput(text)
        if (u.indexOf("oynix://settings") === 0) { app.openSettings(u.replace(/^oynix:\/\/settings\/?/, "")); return }
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
        // One tab per internal page; settings sub-pages share the Settings tab.
        var base = u.indexOf("oynix://settings") === 0 ? "oynix://settings" : u
        for (var i = 0; i < tabModel.count; i++) {
            if ((tabModel.get(i).url || "").indexOf(base) !== 0) continue
            activate(i)
            if (tabModel.get(i).url !== u && u.replace(/\/$/, "") !== base) tabItem(i).load(u)
            return
        }
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

    readonly property var reservedKeys: ["alt+d", "alt+home", "alt+left", "alt+right", "ctrl++", "ctrl+-", "ctrl+0", "ctrl+1", "ctrl+2", "ctrl+3", "ctrl+4",
        "ctrl+5", "ctrl+6", "ctrl+7", "ctrl+8", "ctrl+9", "ctrl+=", "ctrl+alt+i", "ctrl+alt+s", "ctrl+b", "ctrl+d", "ctrl+e", "ctrl+f", "ctrl+f4", "ctrl+f5",
        "ctrl+g", "ctrl+h", "ctrl+j", "ctrl+k", "ctrl+l", "ctrl+n", "ctrl+p", "ctrl+pgdown", "ctrl+pgup", "ctrl+q", "ctrl+r", "ctrl+s", "ctrl+shift+=",
        "ctrl+shift+a", "ctrl+shift+b", "ctrl+shift+d", "ctrl+shift+f", "ctrl+shift+g", "ctrl+shift+h", "ctrl+shift+i", "ctrl+shift+l", "ctrl+shift+n",
        "ctrl+shift+p", "ctrl+shift+pgdown", "ctrl+shift+pgup", "ctrl+shift+r", "ctrl+shift+t", "ctrl+shift+tab", "ctrl+shift+w", "ctrl+shift+y",
        "ctrl+shift+\\", "ctrl+t", "ctrl+tab", "ctrl+u", "ctrl+w", "ctrl+,", "escape", "f11", "f12", "f3", "f5", "f6", "shift+f3", "shift+f5"]
    // Extension keyboard shortcuts (manifest "commands").
    // (Inside items, so each Shortcut belongs to this window.)
    Repeater {
        model: {
            Core.extensionsRevision
            if (win.privateMode) return []
            // OyNIx's own shortcuts win: two bindings on one key would make
            // Qt fire neither, so a clashing extension shortcut is skipped.
            return JSON.parse(Core.webextCommands()).filter(function (c) {
                var k = String(c.key).replace(/\s+/g, "").toLowerCase()
                var clash = win.reservedKeys.indexOf(k) >= 0
                if (clash) console.warn("OyNIx: extension shortcut " + c.key + " (" + c.name + ") is already used by OyNIx and was skipped")
                return !clash
            })
        }
        delegate: Item {
            id: cmdItem
            required property var modelData
            Shortcut {
                sequence: cmdItem.modelData.key
                context: Qt.WindowShortcut
                onActivated: win.runExtensionCommand(cmdItem.modelData)
            }
        }
    }
    function runExtensionCommand(c) {
        if (/^_execute_(browser_|page_)?action$/.test(c.name)) {
            var e = JSON.parse(Core.extensions()).filter(function (x) { return x.webId === c.ext })[0]
            var u = e ? app.extensionHost.popupUrl(e.id) : ""
            if (u) {
                extPopup.url = u
                var p = extMenuBtn.mapToItem(win.contentItem, extMenuBtn.width, extMenuBtn.height + 6)
                extPopup.x = Math.max(8, p.x - extPopup.width); extPopup.y = p.y
                extPopup.open()
            } else app.extBridge.dispatch("action.onClicked", JSON.stringify([c.ext, app.extensionHost.tabInfo(win, currentIndex)]))
            return
        }
        app.extensionHost.tabEvent("ext:commands.onCommand", [c.ext, c.name, app.extensionHost.tabInfo(win, currentIndex)])
    }

    // chrome.idle: note when the person last used the window.
    HoverHandler {
        onPointChanged: { var n = Date.now(); if (n - win.app.extensionHost.apis.lastInput > 5000) win.app.extensionHost.apis.lastInput = n }
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
        // The invisible right-click from Ctrl+Shift+C: inspect, no menu.
        if (silentContextMenu) {
            silentContextMenu = false
            view.triggerWebAction(WebEngineView.InspectElement)
            return
        }
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
    // DevTools on the element under the pointer. Chromium inspects where
    // the last context menu was asked for, so ask for one there, unseen.
    property bool silentContextMenu: false
    function inspectElement() {
        if (!currentView || !currentTab) return
        app.openDevTools(currentView)
        if (!currentTab.pointerInside) return
        silentContextMenu = true
        Core.sendMouse(currentView, currentTab.pointerPos.x, currentTab.pointerPos.y, 2, 1, 0, 1)
    }

    // Open a file from this computer in a new tab.
    FileDialog {
        id: openFileDialog
        title: qsTr("Open File")
        fileMode: FileDialog.OpenFile
        nameFilters: [qsTr("Web pages and documents (*.html *.htm *.xhtml *.svg *.pdf *.md *.txt *.json)"), qsTr("Images (*.png *.jpg *.jpeg *.gif *.webp *.avif)"), qsTr("All files (*)")]
        onAccepted: win.openTab(selectedFile.toString(), {})
    }

    function showShortcuts() {
        shortcutsSheet.x = Math.max(16, (win.width - shortcutsSheet.width) / 2)
        shortcutsSheet.y = 70
        shortcutsSheet.open()
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
            list.push({ url: t.url, title: t.title, pinned: t.pinned, muted: t.muted, pages: t.pages, treeOpen: t.treeOpen })
        }
        return { tabs: list, current: currentIndex, sidebar: sidebarOpen, x: win.x, y: win.y, w: win.width, h: win.height, maximized: win.visibility === Window.Maximized }
    }

    Component.onCompleted: {
        windowId = app.nextWindowId()
        if (initialTabs && initialTabs.length) {
            for (var i = 0; i < initialTabs.length; i++) {
                var t = initialTabs[i]
                // Only the selected tab loads now; the rest restore lazily.
                tabModel.append(makeTab(t.url, { title: t.title || "", pinned: !!t.pinned, muted: !!t.muted, suspended: i !== initialIndex,
                                                 pages: t.pages || "", treeOpen: t.treeOpen !== false }))
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

    // "Save password for example.com?" under the address bar.
    function offerSavePassword(url, user, pass, would) {
        pwSave.url = url; pwSave.pass = pass; pwSave.would = would
        pwSave.userText = user
        var p = addressField.mapToItem(win.contentItem, addressField.width - pwSave.width, addressField.height + 6)
        pwSave.x = Math.max(8, p.x); pwSave.y = p.y
        pwSave.open()
    }
    Popover {
        id: pwSave
        parent: win.contentItem
        width: 340
        padding: 18
        property string url: ""
        property string pass: ""
        property string would: "saved"
        property string userText: ""
        property bool shown: false
        onClosed: { pass = ""; shown = false }
        contentItem: ColumnLayout {
            spacing: 12
            RowLayout {
                spacing: 12
                MShape { size: 40; shape: "cookie4"; color: Theme.secondaryContainer
                    Icon { anchors.centerIn: parent; name: "key"; size: 18; color: Theme.secondaryContainerFg } }
                Txt {
                    Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; font.weight: Font.DemiBold
                    text: pwSave.would === "updated" ? qsTr("Update the password for %1?").arg(Core.hostOf(pwSave.url)) : qsTr("Save password for %1?").arg(Core.hostOf(pwSave.url))
                }
            }
            MTextField { id: pwUserField; Layout.fillWidth: true; placeholder: qsTr("Username"); text: pwSave.userText }
            RowLayout {
                Layout.fillWidth: true
                MTextField { Layout.fillWidth: true; readOnly: true; echoMode: pwSave.shown ? TextInput.Normal : TextInput.Password; text: pwSave.pass }
                IconButton { glyph: "eye"; tip: pwSave.shown ? qsTr("Hide") : qsTr("Show"); onClicked: pwSave.shown = !pwSave.shown }
            }
            RowLayout {
                Layout.fillWidth: true
                PushButton { textOnly: true; text: qsTr("Never"); visible: pwSave.would === "saved"; onClicked: { Core.neverSaveLogin(pwSave.url); pwSave.close() } }
                Item { Layout.fillWidth: true }
                PushButton { textOnly: true; text: qsTr("Not Now"); onClicked: pwSave.close() }
                PushButton {
                    primary: true; text: pwSave.would === "updated" ? qsTr("Update") : qsTr("Save")
                    onClicked: {
                        var r = Core.saveLogin(pwSave.url, pwUserField.text, pwSave.pass)
                        pwSave.close()
                        toast.show(r.indexOf("error") === 0 ? qsTr("Couldn’t save the password: %1").arg(r) : qsTr("Password saved"))
                    }
                }
            }
        }
    }

    // ── Bookmarks: the star / Ctrl+D add the page, then show an editor ──
    function editBookmark(anchor) {
        if (!currentTab || !currentTab.model.url || currentTab.internal) return
        var u = currentTab.model.url
        if (!Core.isBookmarked(u)) {
            Core.addBookmark(u, currentTab.model.title || u, "")
            bmEdit.justAdded = true
        } else bmEdit.justAdded = false
        bmEdit.mark = JSON.parse(Core.bookmarkFor(u))
        var a = anchor || addressField
        var p = a.mapToItem(win.contentItem, a.width - bmEdit.width, a.height + 6)
        bmEdit.x = Math.max(8, p.x); bmEdit.y = p.y
        bmEdit.open()
    }
    Popover {
        id: bmEdit
        parent: win.contentItem
        width: 320
        padding: 18
        property var mark: null
        property bool justAdded: false
        // "" = Bookmarks, "favorites" = favourites bar, else a folder
        readonly property var folderChoices: {
            Core.bookmarksRevision
            var f = [{ v: "favorites", t: qsTr("Favourites bar") }, { v: "", t: qsTr("Bookmarks") }]
            JSON.parse(Core.bookmarkFolders()).forEach(function (x) { f.push({ v: x, t: x }) })
            f.push({ v: "\u0001new", t: qsTr("New folder…") })
            return f
        }
        function save() {
            if (!mark) return
            var folder = bmFolder.currentValue === "\u0001new" ? (bmNewFolder.text.trim() || "") : bmFolder.currentValue
            Core.editBookmark(mark.id, bmName.text.trim() || mark.url, mark.url, folder)
        }
        onAboutToShow: {
            bmName.text = mark ? mark.title : ""
            bmNewFolder.text = ""
            for (var i = 0; i < folderChoices.length; i++) if (mark && folderChoices[i].v === mark.folder) bmFolder.currentIndex = i
        }
        contentItem: ColumnLayout {
            spacing: 12
            RowLayout {
                spacing: 12
                MShape { size: 40; shape: "cookie4"; color: Theme.tertiaryContainer
                    Icon { anchors.centerIn: parent; name: "star"; filled: true; size: 18; color: Theme.tertiaryContainerFg } }
                Txt { Layout.fillWidth: true; font.weight: Font.DemiBold; text: bmEdit.justAdded ? qsTr("Bookmark added") : qsTr("Edit bookmark") }
            }
            MTextField { id: bmName; Layout.fillWidth: true; placeholder: qsTr("Name"); onAccepted: { bmEdit.save(); bmEdit.close() } }
            ComboBox {
                id: bmFolder
                Layout.fillWidth: true; implicitHeight: 36
                model: bmEdit.folderChoices
                textRole: "t"; valueRole: "v"
            }
            MTextField { id: bmNewFolder; visible: bmFolder.currentValue === "\u0001new"; Layout.fillWidth: true; placeholder: qsTr("Folder name") }
            RowLayout {
                Layout.fillWidth: true
                PushButton { textOnly: true; text: qsTr("Remove"); onClicked: { if (bmEdit.mark) Core.removeBookmark(bmEdit.mark.id); bmEdit.mark = null; bmEdit.close(); toast.show(qsTr("Bookmark removed")) } }
                Item { Layout.fillWidth: true }
                PushButton { primary: true; text: qsTr("Done"); onClicked: { bmEdit.save(); bmEdit.close() } }
            }
        }
    }

    // Every bookmark by folder, from the end of the favourites bar.
    Popover {
        id: allBookmarks
        parent: win.contentItem
        width: 320
        padding: 6
        contentItem: Flickable {
            implicitHeight: Math.min(allCol.implicitHeight, win.height - 160)
            contentHeight: allCol.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ColumnLayout {
                id: allCol
                width: parent.width
                spacing: 1
                Repeater {
                    model: {
                        Core.bookmarksRevision
                        if (!allBookmarks.opened) return []
                        var groups = [{ name: qsTr("Bookmarks"), key: "" }].concat(JSON.parse(Core.bookmarkFolders()).map(function (f) { return { name: f, key: f } }))
                        return groups.map(function (g) { return { name: g.name, items: JSON.parse(Core.bookmarks(g.key)) } }).filter(function (g) { return g.items.length })
                    }
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 1
                        RowLayout {
                            Layout.margins: 8; spacing: 6
                            Icon { name: "folder"; size: 13; color: Theme.textFaint }
                            Txt { text: modelData.name; color: Theme.textDim; font.pixelSize: 12; font.weight: Font.DemiBold }
                        }
                        Repeater {
                            model: modelData.items
                            delegate: MenuRow {
                                required property var modelData
                                text: modelData.title || modelData.url
                                glyph: "star"
                                onClicked: { allBookmarks.close(); win.navigate(modelData.url) }
                            }
                        }
                    }
                }
                Txt { visible: JSON.parse(Core.bookmarks("*")).length === 0; text: qsTr("No bookmarks yet. Press Ctrl+D on a page you like."); color: Theme.textDim; Layout.margins: 10; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            }
        }
    }

    // Saved logins for this site, from the key in the address bar.
    function showLoginMenu(anchor) {
        if (loginMenu.recentlyClosed()) return
        var p = anchor.mapToItem(win.contentItem, 0, anchor.height + 6)
        loginMenu.x = p.x - 8; loginMenu.y = p.y
        loginMenu.open()
    }
    Popover {
        id: loginMenu
        parent: win.contentItem
        width: 300
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            Txt { text: qsTr("Saved logins for %1").arg(win.currentTab ? win.currentTab.host : ""); color: Theme.textDim; font.pixelSize: 12; Layout.margins: 8 }
            Repeater {
                model: loginMenu.opened && win.currentTab ? win.currentTab.loginsHere() : []
                delegate: MenuRow {
                    required property var modelData
                    glyph: "user"
                    text: modelData.username || qsTr("(no username)")
                    onClicked: { loginMenu.close(); win.currentTab.fillLogins([modelData], true) }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Manage Passwords…"); glyph: "key"; onClicked: { loginMenu.close(); win.app.openSettings("passwords") } }
        }
    }

    // A yes/no question in a dialog over this window.
    property var askState: null
    function ask(o, cb) { askState = Object.assign({ cb: cb }, o) }
    function answer(yes) { var a = askState; askState = null; if (a && a.cb) a.cb(yes) }

    // Addresses for other apps (mailto:, zoommtg:, an app's sign-in
    // redirect such as com.example.app:/oauth2redirect…): ask, then hand
    // them to the desktop instead of failing with a broken-address error.
    function openInOtherApp(u) {
        var scheme = u.split(":")[0]
        ask({ title: qsTr("Open in another app?"), icon: "external", ok: qsTr("Open App"),
              body: qsTr("This page wants to open a “%1:” link, which belongs to an app on your computer, not to the web.\n\n%2").arg(scheme).arg(u.length > 300 ? u.slice(0, 300) + "…" : u) },
            function (yes) { if (yes) Qt.openUrlExternally(u) })
    }

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
    Shortcut { sequences: ["Ctrl+D"]; onActivated: win.editBookmark(null) }
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
    Shortcut { sequences: ["F12", "Ctrl+Shift+I", "Ctrl+Alt+I", "Ctrl+Shift+J"]; onActivated: win.openDevTools() }
    Shortcut { sequences: ["Ctrl+Shift+C"]; onActivated: win.inspectElement() }
    Shortcut { sequences: ["Ctrl+Shift+M"]; onActivated: if (win.currentIndex >= 0) { win.toggleMute(win.currentIndex); toast.show(tabModel.get(win.currentIndex).muted ? qsTr("Tab muted") : qsTr("Tab unmuted")) } }
    Shortcut { sequences: ["Ctrl+Shift+K"]; onActivated: if (win.currentIndex >= 0) win.duplicateTab(win.currentIndex) }
    Shortcut { sequences: ["Ctrl+O"]; onActivated: openFileDialog.open() }
    Shortcut { sequences: ["Ctrl+Shift+Del"]; onActivated: win.openInternal("oynix://history") }
    Shortcut { sequences: ["Ctrl+Alt+G"]; onActivated: win.openInternal("oynix://play") }
    Shortcut { sequences: ["Ctrl+/", "F1"]; onActivated: win.showShortcuts() }
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
                        if (sitePopover.recentlyClosed()) return
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
                        if (privacyPopover.recentlyClosed()) return
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
                    onClicked: { if (!downloadsPopover.recentlyClosed()) downloadsPopover.open() }
                }
                IconButton {
                    id: shareBtn
                    glyph: "share"; tip: qsTr("Share")
                    enabled: win.currentTab !== null && !win.currentTab.internal
                    onClicked: {
                        if (shareMenu.recentlyClosed()) return
                        var p = shareBtn.mapToItem(win.contentItem, shareBtn.width - shareMenu.width, shareBtn.height + 6)
                        shareMenu.x = p.x; shareMenu.y = p.y
                        shareMenu.open()
                    }
                }
                ToolbarDivider { visible: extMenuBtn.visible }
                // Pinned extensions (pin them from the puzzle menu)
                Repeater {
                    model: {
                        var host = win.app.extensionHost
                        host.revision; Core.extensionsRevision
                        if (win.privateMode || !host.supported) return []
                        return JSON.parse(Core.extensions()).filter(function (e) {
                            // Only pinned extensions get a toolbar button; the rest are in the puzzle menu.
                            if (!e.enabled || !Core.isExtensionPinned(e.id)) return false
                            // OyNIx extensions get a button when they have a panel or page.
                            if (e.kind === "oynix" && (e.hasPanel || e.hasPage)) return true
                            // Power extensions may also have a Chrome/Firefox half.
                            if (e.kind === "oynix" && !e.manifestVersion) return false
                            return host.isLoaded(e.id)
                        })
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
                        onClicked: win.activateExtension(modelData, extBtn)
                    }
                }
                IconButton {
                    id: extMenuBtn
                    glyph: "puzzle"; tip: qsTr("Extensions")
                    visible: !win.privateMode && Core.extensionsRevision >= 0 && JSON.parse(Core.extensions()).length > 0
                    onClicked: {
                        if (extMenu.recentlyClosed()) return
                        var p = extMenuBtn.mapToItem(win.contentItem, extMenuBtn.width - extMenu.width, extMenuBtn.height + 6)
                        extMenu.x = p.x; extMenu.y = p.y
                        extMenu.open()
                    }
                }
                ToolbarDivider {}
                // Red while an OyNIx update is waiting, even after "Later" on the banner.
                IconButton {
                    id: updateBtn
                    readonly property var u: win.app.updater
                    readonly property var rel: u.release ? JSON.parse(u.release) : null
                    visible: rel !== null && !win.privateMode
                        && ["available", "downloading", "ready", "installing", "installed"].indexOf(u.state) >= 0
                    glyph: u.state === "installed" ? "reload" : "download"
                    iconColor: Theme.danger
                    tip: !rel ? "" : u.state === "installed" ? qsTr("OyNIx %1 is installed: restart to use it").arg(rel.version)
                        : u.state === "downloading" ? qsTr("Downloading OyNIx %1…").arg(rel.version)
                        : qsTr("OyNIx %1 is available").arg(rel.version)
                    onClicked: {
                        if (u.state === "installed") {
                            if (!win.app.restartForUpdate()) win.toastMessage(qsTr("Couldn’t restart OyNIx; please start it again yourself."))
                        } else if (u.state === "ready" || u.state === "installing" || (u.state === "available" && updateBanner.external)) {
                            updateBanner.dismissed = ""   // bring the banner back
                        } else {
                            win.app.openSettings("about")
                        }
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
                        if (mainMenu.recentlyClosed()) return
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
                anchors.rightMargin: allBmBtn.width + 16
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
            AbstractButton {
                id: allBmBtn
                anchors.right: parent.right; anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                height: 24; width: allBmRow.implicitWidth + 16
                hoverEnabled: true
                background: Rectangle { radius: 5; color: allBmBtn.hovered ? Theme.hover : "transparent" }
                contentItem: Row {
                    id: allBmRow
                    spacing: 6; leftPadding: 2
                    Icon { name: "folder"; size: 14; color: Theme.textDim; anchors.verticalCenter: parent.verticalCenter }
                    Txt { text: qsTr("All Bookmarks"); font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
                }
                onClicked: {
                    if (allBookmarks.recentlyClosed()) return
                    var p = allBmBtn.mapToItem(win.contentItem, allBmBtn.width - allBookmarks.width, allBmBtn.height + 4)
                    allBookmarks.x = Math.max(8, p.x); allBookmarks.y = p.y
                    allBookmarks.open()
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

                // ── Update banner ──
                // Shown once an update is ready to install (or, for copies
                // OyNIx can't update itself, when one is available).
                Rectangle {
                    id: updateBanner
                    readonly property var u: win.app.updater
                    readonly property var rel: u.release ? JSON.parse(u.release) : null
                    readonly property bool external: u.kind === "manual" || u.kind === "aur"
                    property string dismissed: ""
                    readonly property bool wanted: rel !== null && dismissed !== rel.version && !win.privateMode
                        && (u.state === "ready" || u.state === "installing" || u.state === "installed" || (u.state === "available" && external))
                    z: 40
                    visible: wanted && win.active
                    anchors.right: parent.right; anchors.bottom: parent.bottom
                    anchors.margins: 16
                    width: Math.min(420, parent.width - 32)
                    height: bannerCol.implicitHeight + 32
                    radius: Theme.shapeXL
                    color: Theme.primaryContainer
                    ColumnLayout {
                        id: bannerCol
                        anchors.fill: parent; anchors.margins: 16
                        spacing: 10
                        RowLayout {
                            spacing: 14
                            MShape { size: 44; shape: "softBurst"; color: Theme.primary
                                Icon { anchors.centerIn: parent; name: updateBanner.u.state === "installed" ? "check" : "download"; size: 20; color: Theme.primaryFg } }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Txt {
                                    Layout.fillWidth: true; color: Theme.primaryContainerFg; font.weight: Font.DemiBold
                                    text: !updateBanner.rel ? "" : updateBanner.u.state === "installed" ? qsTr("OyNIx %1 is installed").arg(updateBanner.rel.version)
                                        : qsTr("OyNIx %1 is available").arg(updateBanner.rel.version)
                                }
                                Txt {
                                    Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                    color: Theme.primaryContainerFg; opacity: 0.85; font.pixelSize: 12
                                    text: updateBanner.u.error !== "" ? updateBanner.u.error
                                        : updateBanner.u.state === "installed" ? qsTr("Restart to start using it. Your tabs will come back.")
                                        : updateBanner.u.state === "installing" ? qsTr("Installing… your system may ask for your password.")
                                        : updateBanner.u.kind === "aur" ? qsTr("Update it with your AUR helper, for example: yay -Syu oynix-bin")
                                        : updateBanner.u.kind === "manual" ? qsTr("This copy wasn’t installed from a package, so update it the way you installed it.")
                                        : qsTr("Downloaded and checked. Installing asks for your password and takes a few seconds.")
                                }
                            }
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 6
                            Item { Layout.fillWidth: true }
                            PushButton {
                                textOnly: true; text: qsTr("Later")
                                visible: updateBanner.u.state !== "installing"
                                onClicked: updateBanner.dismissed = updateBanner.rel.version
                            }
                            PushButton {
                                textOnly: true; text: qsTr("What’s New")
                                visible: updateBanner.u.state !== "installed"
                                onClicked: win.app.openSettings("about")
                            }
                            PushButton {
                                primary: true
                                enabled: updateBanner.u.state !== "installing"
                                text: updateBanner.u.state === "installed" ? qsTr("Restart")
                                    : updateBanner.external ? qsTr("Release Page")
                                    : updateBanner.u.state === "installing" ? qsTr("Installing…") : qsTr("Install")
                                onClicked: {
                                    var u = updateBanner.u
                                    if (u.state === "installed") { if (!win.app.restartForUpdate()) win.toastMessage(qsTr("Couldn’t restart OyNIx; please start it again yourself.")) }
                                    else if (updateBanner.external) win.openTab(updateBanner.rel.page, {})
                                    else u.install()
                                }
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
                                Layout.preferredHeight: Math.min(300, Math.max(72, usbList.contentHeight + 16))
                                radius: Theme.shapeL
                                color: Theme.surfaceContainerLow
                                ListView {
                                    id: usbList
                                    anchors.fill: parent; anchors.margins: 8
                                    clip: true
                                    spacing: 8
                                    model: win.usbAsk ? win.usbAsk.devices : []
                                    // Each device is its own card, so neighbours never run together.
                                    delegate: Rectangle {
                                        id: devCard
                                        required property var modelData
                                        readonly property bool picked: win.usbPick === modelData.key
                                        width: usbList.width; height: 64
                                        radius: picked ? Theme.shapeL : Theme.shapeM
                                        color: picked ? Theme.secondaryContainer : (devHover.hovered ? Theme.surfaceContainerHighest : Theme.surfaceContainer)
                                        border.width: picked ? 0 : 1
                                        border.color: Theme.alpha(Theme.outlineVariant, 0.7)
                                        Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
                                        HoverHandler { id: devHover; cursorShape: Qt.PointingHandCursor }
                                        TapHandler { onTapped: win.usbPick = devCard.modelData.key; onDoubleTapped: win.answerUsb(devCard.modelData.key) }
                                        RowLayout {
                                            anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 14
                                            spacing: 12
                                            MShape {
                                                size: 36; shape: devCard.picked ? "cookie9" : "circle"
                                                color: devCard.picked ? Theme.secondary : Theme.surfaceContainerHigh
                                                Icon { anchors.centerIn: parent; name: devCard.picked ? "check" : "usb"; size: 18; color: devCard.picked ? Theme.secondaryFg : Theme.surfaceVariantFg }
                                            }
                                            ColumnLayout {
                                                Layout.fillWidth: true
                                                spacing: 2
                                                Txt { Layout.fillWidth: true; text: devCard.modelData.name; font.weight: Font.Medium; color: devCard.picked ? Theme.secondaryContainerFg : Theme.text }
                                                Txt { Layout.fillWidth: true; text: devCard.modelData.ids + (devCard.modelData.paired ? " · " + qsTr("allowed before") : ""); color: devCard.picked ? Theme.secondaryContainerFg : Theme.textDim; opacity: devCard.picked ? 0.8 : 1; font.pixelSize: 12 }
                                            }
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

                // ── General question: win.ask({title, body, icon, ok, cancel}, cb) ──
                Rectangle {
                    z: 50
                    anchors.fill: parent
                    visible: win.askState !== null
                    onVisibleChanged: if (visible) forceActiveFocus()
                    color: Theme.alpha("#000000", 0.32)
                    MouseArea { anchors.fill: parent }
                    Keys.onEscapePressed: win.answer(false)
                    Rectangle {
                        anchors.centerIn: parent
                        width: Math.min(440, parent.width - 48)
                        height: askCol.implicitHeight + 48
                        radius: Theme.shapeXL
                        color: Theme.surfaceContainerHigh
                        ColumnLayout {
                            id: askCol
                            anchors.fill: parent
                            anchors.margins: 24
                            spacing: 14
                            MShape { Layout.alignment: Qt.AlignHCenter; shape: "cookie9"; size: 56; color: Theme.secondaryContainer
                                Icon { anchors.centerIn: parent; name: win.askState ? win.askState.icon || "external" : "external"; size: 26; color: Theme.secondaryContainerFg } }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap; elide: Text.ElideNone
                                font.pixelSize: 22; font.family: Theme.displayFont
                                text: win.askState ? win.askState.title : ""
                            }
                            Txt {
                                Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WrapAnywhere; elide: Text.ElideNone
                                color: Theme.surfaceVariantFg
                                text: win.askState ? win.askState.body : ""
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                Item { Layout.fillWidth: true }
                                PushButton { textOnly: true; text: win.askState && win.askState.cancel ? win.askState.cancel : qsTr("Cancel"); onClicked: win.answer(false) }
                                PushButton { primary: true; text: win.askState && win.askState.ok ? win.askState.ok : qsTr("OK"); onClicked: win.answer(true) }
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

    // ── OyNIx extension panels: graphics.qt (restricted QML) or extra.html ──
    function openOynixPanel(ext, anchor) {
        // A second click on the same button closes it.
        if (anchor && oyPopup.recentlyClosed() && oyPopup.lastId === ext.id) return
        oyPopup.close()
        oyPopup.ext = ext
        var a = anchor || extMenuBtn
        var p = a.mapToItem(win.contentItem, a.width, a.height + 6)
        oyPopup.anchorRight = p.x; oyPopup.y = p.y
        oyPopup.open()
    }
    Popover {
        id: oyPopup
        parent: win.contentItem
        property var ext: null
        property string lastId: ""
        onExtChanged: if (ext) lastId = ext.id
        property real anchorRight: 0
        property string error: ""
        readonly property bool qml: ext !== null && ext.hasPanel
        padding: 0
        width: qml ? Math.max(240, Math.min(480, panelHost.implicitWidth || 320)) : 360
        height: qml ? Math.max(80, Math.min(640, (panelHost.implicitHeight || 240) + (error ? 60 : 0))) : 460
        x: Math.max(8, anchorRight - width)
        ExtensionPanel {
            id: panelApi
            onPosted: function (payload) { if (oyPopup.ext) win.app.extBridge.post(oyPopup.ext.id, "content", payload) }
            onCloseRequested: oyPopup.close()
        }
        Connections {
            target: win.app.extBridge
            function onMessage(ext, target, payload) {
                if (oyPopup.opened && oyPopup.ext && ext === oyPopup.ext.id && target === "panel") panelApi.deliver(payload)
            }
        }
        onOpened: {
            error = ""
            if (!qml) return
            panelApi.id = ext.id
            panelApi.data = Core.extensionData(ext.id)
            error = Core.loadExtensionPanel(panelHost, ext.id, panelApi)
        }
        onClosed: {
            Core.unloadExtensionPanel(panelHost)
            panelHost.implicitWidth = 0; panelHost.implicitHeight = 0
            ext = null
        }
        contentItem: RoundedClip {
            radius: Theme.shapeL
            Item {
                id: panelHost
                visible: oyPopup.qml
                anchors.fill: parent
                // The panel's root item fills the popover.
                onChildrenChanged: {
                    for (var i = 0; i < children.length; i++) {
                        children[i].width = Qt.binding(function () { return panelHost.width })
                        children[i].height = Qt.binding(function () { return panelHost.height })
                    }
                }
            }
            Txt {
                visible: oyPopup.error !== ""
                anchors.fill: parent; anchors.margins: 16
                wrapMode: Text.WordWrap; elide: Text.ElideNone
                color: Theme.danger; font.pixelSize: 12
                text: oyPopup.ext ? qsTr("%1 couldn’t show its panel:\n%2").arg(oyPopup.ext.name).arg(oyPopup.error) : ""
            }
            Loader {
                anchors.fill: parent
                active: oyPopup.opened && oyPopup.ext !== null && !oyPopup.qml
                sourceComponent: WebEngineView {
                    id: pageView
                    profile: win.app.defaultProfile
                    backgroundColor: Theme.surfaceContainer
                    webChannel: win.app.extChannel
                    webChannelWorld: WebEngineScript.MainWorld
                    settings.localContentCanAccessRemoteUrls: false
                    settings.localContentCanAccessFileUrls: false
                    Component.onCompleted: {
                        var sc = WebEngine.script()
                        sc.name = "oynix-page-api"
                        sc.sourceCode = Core.webchannelScript() + "\n" + Core.extensionPageApi(oyPopup.ext.id)
                        sc.injectionPoint = WebEngineScript.DocumentCreation
                        sc.worldId = WebEngineScript.MainWorld
                        userScripts.insert(sc)
                        url = Core.extensionPageUrl(oyPopup.ext.id)
                    }
                    // The page stays inside the extension; links open in tabs.
                    onNavigationRequested: function (request) {
                        var u = request.url.toString()
                        if (request.isMainFrame && u.indexOf("file://") !== 0) { request.reject(); win.openTab(u, { afterCurrent: true }); oyPopup.close() }
                    }
                    onNewWindowRequested: function (request) { win.openTab(request.requestedUrl.toString(), { afterCurrent: true }); oyPopup.close() }
                }
            }
        }
    }

    // Extension action popup: a small web view sized to the popup's content.
    Popover {
        id: extPopup
        parent: win.contentItem
        property string url: ""
        property string lastUrl: ""
        onUrlChanged: if (url) lastUrl = url
        padding: 0
        width: Math.max(220, Math.min(800, (popupView.contentsSize.width || 360) + 2))
        height: Math.max(60, Math.min(600, (popupView.contentsSize.height || 420) + 2))
        onClosed: url = ""
        contentItem: RoundedClip {
            radius: Theme.shapeL
            Rectangle { anchors.fill: parent; color: "white" }
            WebEngineView {
                id: popupView
                anchors.fill: parent
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
        width: 320
        padding: 6
        contentItem: ColumnLayout {
            spacing: 1
            Txt { text: qsTr("Extensions"); color: Theme.textDim; font.pixelSize: 12; Layout.leftMargin: 8; Layout.topMargin: 4; Layout.bottomMargin: 2 }
            Txt { text: qsTr("Pin one to keep it in the toolbar."); color: Theme.textFaint; font.pixelSize: 11; Layout.leftMargin: 8; Layout.bottomMargin: 4 }
            Repeater {
                model: { Core.extensionsRevision; win.app.extensionHost.revision; return extMenu.opened ? JSON.parse(Core.extensions()) : [] }
                delegate: Rectangle {
                    id: extRow
                    required property var modelData
                    readonly property bool clickable: win.extensionClickable(modelData)
                    readonly property bool pinned: Core.extensionsRevision >= 0 && Core.isExtensionPinned(modelData.id)
                    Layout.fillWidth: true
                    Layout.preferredHeight: 36
                    radius: 8
                    color: rowHover.hovered && clickable ? Theme.hover : "transparent"
                    HoverHandler { id: rowHover }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8; anchors.rightMargin: 4
                        spacing: 8
                        Item {
                            Layout.fillWidth: true; Layout.fillHeight: true
                            RowLayout {
                                anchors.fill: parent
                                spacing: 8
                                Image { Layout.preferredWidth: 16; Layout.preferredHeight: 16; sourceSize: Qt.size(32, 32); source: extRow.modelData.icon ? "file://" + extRow.modelData.icon : ""; visible: !!extRow.modelData.icon }
                                Icon { visible: !extRow.modelData.icon; name: "puzzle"; size: 16; color: Theme.textDim }
                                Txt { text: extRow.modelData.name; Layout.fillWidth: true; color: extRow.modelData.enabled ? Theme.text : Theme.textDim }
                            }
                            TapHandler {
                                enabled: extRow.clickable
                                onTapped: { extMenu.close(); win.activateExtension(extRow.modelData, extMenuBtn) }
                            }
                        }
                        IconButton {
                            implicitWidth: 28; implicitHeight: 28
                            glyph: "pin"; iconSize: 15
                            visible: extRow.clickable || extRow.pinned
                            checked: extRow.pinned
                            iconColor: extRow.pinned ? Theme.accent : Theme.textFaint
                            tip: extRow.pinned ? qsTr("Unpin from toolbar") : qsTr("Pin to toolbar")
                            onClicked: Core.setExtensionPinned(extRow.modelData.id, !extRow.pinned)
                        }
                        Toggle { checked: extRow.modelData.enabled; onToggled: win.app.extensionHost.setEnabled(extRow.modelData.id, checked) }
                    }
                }
            }
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Manage Extensions…"); glyph: "sliders"; onClicked: { extMenu.close(); win.app.openSettings("extensions") } }
        }
    }

    // Ctrl+/ or F1: every keyboard shortcut, grouped.
    Popover {
        id: shortcutsSheet
        parent: win.contentItem
        width: Math.min(720, win.width - 32)
        height: Math.min(sheetCol.implicitHeight + 40, win.height - 100)
        padding: 20
        readonly property var groups: [
            { title: qsTr("Tabs & windows"), keys: [
                ["Ctrl+T", qsTr("New tab")], ["Ctrl+W", qsTr("Close tab")], ["Ctrl+Shift+T", qsTr("Reopen closed tab")],
                ["Ctrl+Tab", qsTr("Next tab")], ["Ctrl+Shift+Tab", qsTr("Previous tab")], ["Ctrl+1 … 8", qsTr("Go to tab")], ["Ctrl+9", qsTr("Last tab")],
                ["Ctrl+Shift+PgUp/PgDn", qsTr("Move tab")], ["Ctrl+Shift+K", qsTr("Duplicate tab")], ["Ctrl+Shift+M", qsTr("Mute tab")],
                ["Ctrl+N", qsTr("New window")], ["Ctrl+Shift+N", qsTr("Private window")], ["Ctrl+Shift+W", qsTr("Close window")],
                ["Ctrl+Shift+\\", qsTr("Show all tabs")], ["Ctrl+Shift+L", qsTr("Sidebar")], ["Ctrl+Shift+F", qsTr("Focus mode")], ["F11", qsTr("Full screen")] ] },
            { title: qsTr("Page"), keys: [
                ["Ctrl+F", qsTr("Find on page")], ["Ctrl+G / F3", qsTr("Next match")], ["Ctrl+Shift+G", qsTr("Previous match")],
                ["Ctrl+R / F5", qsTr("Reload")], ["Ctrl+F5", qsTr("Reload, skipping the cache")], ["Esc", qsTr("Stop loading")],
                ["Alt+Left", qsTr("Back")], ["Alt+Right", qsTr("Forward")], ["Alt+Home", qsTr("Home page")],
                ["Ctrl+= / Ctrl+-", qsTr("Zoom in / out")], ["Ctrl+0", qsTr("Reset zoom")], ["Ctrl+Shift+R", qsTr("Reader view")],
                ["Ctrl+P", qsTr("Print to PDF")], ["Ctrl+S", qsTr("Save page")], ["Ctrl+O", qsTr("Open a file")] ] },
            { title: qsTr("Address bar & lists"), keys: [
                ["Ctrl+L / F6", qsTr("Edit the address")], ["Ctrl+K", qsTr("Search")], ["Ctrl+D", qsTr("Bookmark this page")],
                ["Ctrl+Shift+D", qsTr("Add to Reading List")], ["Ctrl+B", qsTr("Bookmarks")], ["Ctrl+H", qsTr("History")], ["Ctrl+Shift+H", qsTr("History in the sidebar")],
                ["Ctrl+J", qsTr("Downloads")], ["Ctrl+Shift+Del", qsTr("Clear browsing data")], ["Ctrl+,", qsTr("Settings")] ] },
            { title: qsTr("Developer"), keys: [
                ["F12 / Ctrl+Shift+I", qsTr("Developer tools")], ["Ctrl+Shift+J", qsTr("Developer tools (console)")],
                ["Ctrl+Shift+C", qsTr("Inspect the element under the pointer")], ["Ctrl+U", qsTr("View page source")] ] },
            { title: qsTr("Other"), keys: [
                ["Ctrl+Alt+G", qsTr("Games (Go, Blackjack)")], ["Ctrl+/ or F1", qsTr("This list")], ["Ctrl+Q", qsTr("Quit")] ] }
        ]
        contentItem: Flickable {
            contentHeight: sheetCol.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ColumnLayout {
                id: sheetCol
                width: parent.width
                spacing: 6
                RowLayout {
                    Layout.fillWidth: true
                    Txt { text: qsTr("Keyboard Shortcuts"); font.family: Theme.displayFont; font.pixelSize: 24; Layout.fillWidth: true }
                    IconButton { glyph: "close"; onClicked: shortcutsSheet.close() }
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: shortcutsSheet.width > 560 ? 2 : 1
                    columnSpacing: 28; rowSpacing: 14
                    Repeater {
                        model: shortcutsSheet.groups
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignTop
                            spacing: 3
                            Txt { text: modelData.title; color: Theme.accent; font.weight: Font.DemiBold; font.pixelSize: 13; Layout.bottomMargin: 2 }
                            Repeater {
                                model: modelData.keys
                                delegate: RowLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    spacing: 10
                                    Txt { text: modelData[1]; font.pixelSize: 12; Layout.fillWidth: true }
                                    Rectangle {
                                        implicitWidth: keyText.implicitWidth + 12; implicitHeight: 20; radius: 5
                                        color: Theme.surfaceContainerHigh
                                        border.width: 1; border.color: Theme.separator
                                        Txt { id: keyText; anchors.centerIn: parent; text: modelData[0]; font.pixelSize: 11; font.family: "monospace"; color: Theme.textDim }
                                    }
                                }
                            }
                        }
                    }
                }
            }
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
            MenuRow { text: qsTr("Developer Tools"); glyph: "code"; shortcut: "F12"; enabled: win.currentView !== null; onClicked: { mainMenu.close(); win.openDevTools() } }
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
            Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator; Layout.topMargin: 4; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("Inspect Element"); glyph: "code"; shortcut: "Ctrl+Shift+C"; onClicked: { ctxMenu.close(); win.app.openDevTools(ctxMenu.view); ctxMenu.view.triggerWebAction(WebEngineView.InspectElement) } }
        }
    }
}
