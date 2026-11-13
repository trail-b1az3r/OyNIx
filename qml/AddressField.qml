import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// The unified address / search field. When idle it shows just the site's
// domain, centred, with a lock; when focused it shows the full URL and a
// suggestion list (bookmarks, history, !bangs and search suggestions).
Rectangle {
    id: field
    property var win
    property var tab: win.currentTab
    readonly property string url: tab ? (tab.model.url || "") : ""
    readonly property bool editing: input.activeFocus
    readonly property real progress: tab && tab.model.loading ? tab.model.progress : 0
    property var items: []
    property int selected: -1
    property string typed: ""

    signal siteInfoRequested(var anchor)

    implicitHeight: 44
    radius: editing ? Theme.shapeL : height / 2
    color: editing ? Theme.surfaceContainerLowest : (hover.hovered ? Theme.surfaceContainerHighest : Theme.surfaceContainerHigh)
    border.width: editing ? 2 : 0
    border.color: Theme.primary
    Behavior on radius { SpringAnimation { spring: Theme.springDefault; damping: Theme.damping } }
    Behavior on color { ColorAnimation { duration: Theme.animFast } }

    HoverHandler { id: hover }

    function focusField() {
        input.forceActiveFocus()
        input.selectAll()
    }

    function displayText() {
        if (!tab) return ""
        if (url.indexOf("oynix://") === 0) return ""
        return Core.displayUrl(url)
    }

    function resetText() {
        input.text = url.indexOf("oynix://") === 0 ? "" : url
    }

    onUrlChanged: if (!editing) resetText()

    function commit(text) {
        var t = text !== undefined ? text : input.text
        if (field.selected >= 0 && field.selected < items.length && items[field.selected].url)
            t = items[field.selected].url
        popup.close()
        win.navigate(t)
        win.focusContent()
    }

    function refresh() {
        var t = input.text
        typed = t
        if (!editing || t.trim() === "" || t === url) { items = []; popup.close(); return }
        var local = JSON.parse(Core.suggestions(t))
        var list = [{ kind: "search", title: t, subtitle: qsTr("Search"), url: "" }]
        var resolved = Core.resolveInput(t)
        if (resolved.indexOf("://") > 0 && resolved.indexOf(t.replace(/\/$/, "")) >= 0 && !/\s/.test(t))
            list = [{ kind: "go", title: resolved, subtitle: qsTr("Open"), url: resolved }]
        items = list.concat(local)
        selected = 0
        if (!popup.opened) popup.open()
        if (!win.privateMode) remote.fetch(t)
    }

    QtObject {
        id: remote
        property var xhr: null
        function fetch(q) {
            var u = Core.suggestUrl(q)
            if (u === "") return
            if (xhr) xhr.abort()
            xhr = new XMLHttpRequest()
            var req = xhr
            req.onreadystatechange = function () {
                if (req.readyState !== XMLHttpRequest.DONE || req.status !== 200 || field.typed !== q) return
                try {
                    var data = JSON.parse(req.responseText)
                    var arr = Array.isArray(data) && Array.isArray(data[1]) ? data[1]
                            : (Array.isArray(data) ? data.map(function (x) { return x.phrase || x.q || "" }) : [])
                    var extra = []
                    for (var i = 0; i < arr.length && extra.length < 5; i++) {
                        var s = typeof arr[i] === "string" ? arr[i] : (arr[i].phrase || arr[i].t || "")
                        if (s && s !== q) extra.push({ kind: "suggest", title: s, subtitle: "", url: "" })
                    }
                    var base = field.items.filter(function (x) { return x.kind !== "suggest" })
                    var head = base.slice(0, 1), tail = base.slice(1)
                    field.items = head.concat(extra).concat(tail)
                } catch (e) {}
            }
            req.open("GET", u)
            req.send()
        }
    }

    // Loading progress: Material 3 Expressive wavy indicator along the bottom.
    WavyProgress {
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        anchors.leftMargin: 18; anchors.rightMargin: 18; anchors.bottomMargin: -1
        height: 8
        value: field.progress
        visible: field.progress > 0 && field.progress < 1 && !field.editing
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 6
        spacing: 2

        IconButton {
            id: lockBtn
            implicitWidth: 32; implicitHeight: 32
            visible: !field.editing && field.url !== "" && field.url.indexOf("oynix://") !== 0
            glyph: field.win.privateMode ? "mask" : (Core.isSecure(field.url) ? "lock" : "warning")
            iconSize: 13
            iconColor: Core.isSecure(field.url) ? Theme.textDim : Theme.warning
            tip: qsTr("Website settings")
            onClicked: field.siteInfoRequested(lockBtn)
        }
        Icon {
            visible: field.editing || field.url === "" || field.url.indexOf("oynix://") === 0
            name: "search"; size: 14; color: Theme.textDim
            Layout.leftMargin: 4; Layout.rightMargin: 2
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            TextInput {
                id: input
                anchors.fill: parent
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.text
                selectionColor: Theme.selection
                selectedTextColor: Theme.text
                font.family: Theme.font
                font.pixelSize: 15
                clip: true
                selectByMouse: true
                opacity: field.editing ? 1 : 0
                onTextEdited: field.refresh()
                onActiveFocusChanged: {
                    if (activeFocus) { field.resetText(); selectAll() }
                    else { popup.close(); field.resetText() }
                }
                Keys.onPressed: function (e) {
                    if (e.key === Qt.Key_Down) { field.selected = Math.min(field.items.length - 1, field.selected + 1); e.accepted = true }
                    else if (e.key === Qt.Key_Up) { field.selected = Math.max(-1, field.selected - 1); e.accepted = true }
                    else if (e.key === Qt.Key_Escape) {
                        if (popup.opened) popup.close()
                        else { field.resetText(); field.win.focusContent() }
                        e.accepted = true
                    } else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) {
                        if (field.selected >= 0 && field.selected < field.items.length) {
                            var it = field.items[field.selected]
                            if (it.kind === "suggest") field.commit(it.title)
                            else if (it.kind === "bang") { input.text = it.title + " "; field.refresh() }
                            else if (it.url) field.commit(it.url)
                            else field.commit()
                        } else field.commit()
                        e.accepted = true
                    }
                }
            }

            Txt {
                id: shown
                anchors.fill: parent
                visible: !field.editing
                horizontalAlignment: Text.AlignHCenter
                text: field.displayText() !== "" ? field.displayText() : qsTr("Search or enter website name")
                color: field.displayText() !== "" ? Theme.text : Theme.textFaint
                font.weight: field.displayText() !== "" ? Font.Medium : Font.Normal
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.IBeamCursor
                    onClicked: field.focusField()
                }
            }
        }

        Txt {
            visible: !field.editing && field.tab && field.tab.view && Math.abs(field.tab.model.zoom - 1) > 0.01
            text: Math.round((field.tab ? field.tab.model.zoom : 1) * 100) + "%"
            color: Theme.textDim
            font.pixelSize: 11
            Layout.rightMargin: 2
            MouseArea { anchors.fill: parent; onClicked: field.tab.setZoom(Core.defaultZoom) }
        }
        IconButton {
            implicitWidth: 32; implicitHeight: 32
            visible: !field.editing && field.tab !== null && field.tab.readerAvailable
            glyph: "reader"; iconSize: 14
            checkable: false
            iconColor: field.tab && field.tab.readerActive ? Theme.accent : Theme.textDim
            tip: qsTr("Reader View") + "  (Ctrl+Shift+R)"
            onClicked: field.tab.toggleReader()
        }
        IconButton {
            implicitWidth: 32; implicitHeight: 32
            visible: !field.editing && field.url !== "" && field.url.indexOf("oynix://") !== 0
            glyph: "star"; iconSize: 14
            property bool marked: Core.bookmarksRevision >= 0 && Core.isBookmarked(field.url)
            filledIcon: marked
            iconColor: marked ? Theme.warning : Theme.textDim
            tip: marked ? qsTr("Remove Bookmark") : qsTr("Bookmark This Page") + "  (Ctrl+D)"
            onClicked: Core.toggleBookmark(field.url, field.tab.model.title)
        }
        IconButton {
            implicitWidth: 32; implicitHeight: 32
            visible: !field.editing && field.tab !== null && field.tab.view !== null
            glyph: field.tab && field.tab.model.loading ? "close" : "reload"
            iconSize: 13
            iconColor: Theme.textDim
            tip: field.tab && field.tab.model.loading ? qsTr("Stop") : qsTr("Reload") + "  (Ctrl+R)"
            onClicked: field.tab.model.loading ? field.tab.view.stop() : field.tab.view.reload()
        }
    }

    Popup {
        id: popup
        y: field.height + 6
        x: -8
        width: field.width + 16
        padding: 6
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
        visible: false
        focus: false
        background: Rectangle {
            radius: Theme.shapeXL - 4
            color: Theme.surfaceContainer
        }
        contentItem: Column {
            spacing: 1
            Repeater {
                model: field.items
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: popup.width - 12
                    height: 44
                    radius: Theme.shapeL
                    color: index === field.selected ? Theme.secondaryContainer : (rowHover.hovered ? Theme.hover : "transparent")
                    HoverHandler { id: rowHover }
                    TapHandler {
                        onTapped: {
                            if (modelData.kind === "suggest") field.commit(modelData.title)
                            else if (modelData.kind === "bang") { input.text = modelData.title + " "; input.forceActiveFocus(); field.refresh() }
                            else if (modelData.url) { field.selected = -1; field.commit(modelData.url) }
                            else { field.selected = -1; field.commit() }
                        }
                    }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 10
                        Icon {
                            name: ({ search: "search", suggest: "search", go: "globe", bookmark: "star", history: "clock", bang: "sparkles" })[modelData.kind]
                            size: 14
                            color: index === field.selected ? Theme.secondaryContainerFg : Theme.textDim
                        }
                        Txt {
                            text: modelData.title
                            color: index === field.selected ? Theme.secondaryContainerFg : Theme.text
                            Layout.maximumWidth: parent.width * 0.6
                        }
                        Txt {
                            text: modelData.subtitle ? "— " + modelData.subtitle : ""
                            color: Theme.textDim
                            Layout.fillWidth: true
                        }
                    }
                }
            }
        }
    }
}
