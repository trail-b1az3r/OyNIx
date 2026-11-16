import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Tabs listed in the sidebar: pinned tabs as a grid of tiles on top, then
// the rest as rows. Drag rows to reorder. With tree tabs on (Settings ›
// Tabs), the current tab opens into a tree: the outline of a Markdown or
// PDF document, and the pages of the site you've visited in it.
ColumnLayout {
    id: vt
    property var win
    spacing: 6

    // Outline entries as tree rows (depth from heading level).
    function outlineItems(m) {
        if (!m.outline) return []
        var o = JSON.parse(m.outline), min = 9
        for (var i = 0; i < o.items.length; i++) min = Math.min(min, o.items[i].level)
        return o.items.map(function (it, k) {
            return { depth: it.level - min, title: it.title, active: k === m.outlineActive, hint: o.kind === "pdf" ? qsTr("p. %1").arg(it.page) : "" }
        })
    }
    function outlineKind(m) { try { return JSON.parse(m.outline).kind } catch (e) { return "" } }

    // The site's pages as a tree by URL path: a page sits under the
    // nearest other visited page whose path contains it.
    function sitePages(m) {
        if (!m.pages) return []
        var list = JSON.parse(m.pages)
        if (list.length < 2) return []
        var cur = String(m.url || "").split("#")[0]
        var segs = function (u) {
            var a = /^[a-z]+:\/\/[^\/]*(\/[^?#]*)?(\?[^#]*)?/i.exec(u)
            var path = (a && a[1] ? a[1] : "/").replace(/\/+$/, "")
            var parts = path.split("/").filter(function (x) { return x.length })
            // /docs/index.html is the page for the folder /docs/.
            if (parts.length && /^(index|default)(\.[a-z0-9]+)?$/i.test(parts[parts.length - 1])) parts.pop()
            if (a && a[2]) parts.push(a[2])
            return parts
        }
        var rows = list.map(function (p) { return { url: p.url, title: p.title, parts: segs(p.url) } })
        rows.sort(function (a, b) {
            for (var i = 0; i < Math.min(a.parts.length, b.parts.length); i++) if (a.parts[i] !== b.parts[i]) return a.parts[i] < b.parts[i] ? -1 : 1
            return a.parts.length - b.parts.length
        })
        var isPrefix = function (a, b) {
            if (a.length >= b.length) return false
            for (var i = 0; i < a.length; i++) if (a[i] !== b[i]) return false
            return true
        }
        return rows.map(function (r) {
            var depth = 0
            for (var j = 0; j < rows.length; j++) if (rows[j] !== r && isPrefix(rows[j].parts, r.parts)) depth++
            var last = r.parts.length ? decodeURIComponent(r.parts[r.parts.length - 1]) : Core.hostOf(r.url)
            return { depth: depth, title: r.title || last, active: r.url === cur, url: r.url, hint: "" }
        })
    }

    // Pinned tiles
    GridLayout {
        Layout.fillWidth: true
        Layout.leftMargin: 8; Layout.rightMargin: 8
        columns: 4
        columnSpacing: 6; rowSpacing: 6
        visible: pinnedRep.count > 0 && hasPinned
        property bool hasPinned: { for (var i = 0; i < vt.win.tabs.count; i++) if (vt.win.tabs.get(i).pinned) return true; return false }
        Repeater {
            id: pinnedRep
            model: vt.win.tabs
            delegate: Rectangle {
                required property int index
                required property var model
                visible: model.pinned
                Layout.fillWidth: true
                Layout.preferredHeight: model.pinned ? 38 : 0
                radius: 8
                color: index === vt.win.currentIndex ? Theme.tabActive : (ph.hovered ? Theme.hover : Theme.card)
                HoverHandler { id: ph }
                Favicon { anchors.centerIn: parent; size: 18; source: model.icon || ""; url: model.url || ""; title: model.title || ""; loading: model.loading }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: function (m) { m.button === Qt.RightButton ? vt.win.showTabMenu(index, parent) : vt.win.activate(index) }
                }
            }
        }
    }

    ListView {
        id: list
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        spacing: 2
        model: vt.win.tabs
        boundsBehavior: Flickable.StopAtBounds
        moveDisplaced: Transition { NumberAnimation { properties: "y"; duration: Theme.anim; easing.type: Easing.OutCubic } }

        delegate: Item {
            id: row
            required property int index
            required property var model
            width: list.width
            height: model.pinned ? 0 : 34 + (tree.visible ? tree.implicitHeight + 6 : 0)
            visible: !model.pinned

            readonly property bool isCurrent: index === vt.win.currentIndex
            readonly property var outline: Core.treeTabs && isCurrent ? vt.outlineItems(model) : []
            readonly property var pages: Core.treeTabs && isCurrent ? vt.sitePages(model) : []
            readonly property bool hasTree: outline.length > 0 || pages.length > 0

            Rectangle {
                id: plate
                anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
                height: 34
                anchors.leftMargin: 8; anchors.rightMargin: 8
                radius: 7
                color: row.index === vt.win.currentIndex ? Theme.tabActive : (rh.hovered ? Theme.hover : "transparent")
                border.color: row.index === vt.win.currentIndex && !Theme.dark ? Theme.separator : "transparent"
                HoverHandler { id: rh }

                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
                    property real pressY: 0
                    onPressed: function (m) { pressY = m.y; if (m.button === Qt.LeftButton) vt.win.activate(row.index) }
                    onPositionChanged: function (m) {
                        if (!(m.buttons & Qt.LeftButton)) return
                        var d = m.y - pressY
                        if (d > 34 * 0.8 && row.index < vt.win.tabs.count - 1) { vt.win.moveTab(row.index, row.index + 1); pressY += 34 }
                        else if (d < -34 * 0.8 && row.index > 0 && !vt.win.tabs.get(row.index - 1).pinned) { vt.win.moveTab(row.index, row.index - 1); pressY -= 34 }
                    }
                    onClicked: function (m) {
                        if (m.button === Qt.MiddleButton) vt.win.closeTab(row.index)
                        else if (m.button === Qt.RightButton) vt.win.showTabMenu(row.index, plate)
                    }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 9; anchors.rightMargin: 4
                    spacing: 8
                    Favicon { size: 16; source: row.model.icon || ""; url: row.model.url || ""; title: row.model.title || ""; loading: row.model.loading; opacity: row.model.suspended ? 0.5 : 1 }
                    Txt {
                        Layout.fillWidth: true
                        text: row.model.title || Core.displayUrl(row.model.url || "") || qsTr("Start Page")
                        font.pixelSize: 12
                        color: row.model.suspended ? Theme.textDim : Theme.text
                    }
                    Icon { visible: row.model.audible || row.model.muted; name: row.model.muted ? "mute" : "speaker"; size: 13; color: Theme.textDim }
                    IconButton {
                        visible: row.hasTree
                        implicitWidth: 20; implicitHeight: 20
                        glyph: "chevronRight"; iconSize: 11; iconColor: Theme.textDim
                        rotation: row.model.treeOpen ? 90 : 0
                        tip: row.model.treeOpen ? qsTr("Hide the tree") : qsTr("Show the tree")
                        onClicked: vt.win.tabs.setProperty(row.index, "treeOpen", !row.model.treeOpen)
                    }
                    IconButton {
                        visible: rh.hovered
                        implicitWidth: 20; implicitHeight: 20
                        glyph: "close"; iconSize: 11; iconColor: Theme.textDim
                        onClicked: vt.win.closeTab(row.index)
                    }
                }
            }

            ColumnLayout {
                id: tree
                anchors.left: parent.left; anchors.right: parent.right; anchors.top: plate.bottom
                anchors.topMargin: 2; anchors.rightMargin: 8
                visible: row.hasTree && row.model.treeOpen
                spacing: 4
                TreeSection {
                    Layout.fillWidth: true
                    visible: row.outline.length > 0
                    title: vt.outlineKind(row.model) === "pdf" ? qsTr("PDF contents") : qsTr("Contents")
                    glyph: "list"
                    items: row.outline
                    maxHeight: 320
                    onPicked: function (i) { var t = vt.win.tabItem(row.index); if (t) t.openOutlineItem(i) }
                }
                TreeSection {
                    Layout.fillWidth: true
                    visible: row.pages.length > 0
                    title: qsTr("Pages on %1").arg(Core.hostOf(row.model.url || "") || qsTr("this computer"))
                    glyph: "globe"
                    items: row.pages
                    maxHeight: 220
                    onPicked: function (i) { var t = vt.win.tabItem(row.index); if (t) t.openPage(row.pages[i].url) }
                }
            }
        }
    }

    AbstractButton {
        Layout.fillWidth: true
        Layout.leftMargin: 8; Layout.rightMargin: 8
        implicitHeight: 32
        hoverEnabled: true
        background: Rectangle { radius: 7; color: parent.hovered ? Theme.hover : "transparent" }
        contentItem: RowLayout {
            spacing: 8
            Icon { name: "plus"; size: 14; color: Theme.textDim; Layout.leftMargin: 10 }
            Txt { text: qsTr("New Tab"); color: Theme.textDim; font.pixelSize: 12; Layout.fillWidth: true }
        }
        onClicked: vt.win.newTab()
    }
}
