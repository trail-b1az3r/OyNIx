import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// "Show All Tabs": a zoomed-out grid of tab previews with search.
Rectangle {
    id: ov
    property var win
    color: Theme.dark ? "#161618" : "#e9e9ec"
    opacity: 0
    visible: opacity > 0
    property bool shown: false
    onShownChanged: { if (shown) { search.text = ""; search.forceActiveFocus() } }
    states: State { when: ov.shown; PropertyChanges { target: ov; opacity: 1 } }
    transitions: Transition { NumberAnimation { property: "opacity"; duration: Theme.anim } }

    Keys.onEscapePressed: win.overviewOpen = false

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            Txt { text: win.privateMode ? qsTr("Private Tabs") : (win.tabs.count === 1 ? qsTr("1 Tab") : qsTr("%1 Tabs").arg(win.tabs.count)); font.pixelSize: 20; font.weight: Font.Bold; Layout.fillWidth: true }
            Rectangle {
                implicitWidth: 260; implicitHeight: 30; radius: 8; color: Theme.field
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 8
                    Icon { name: "search"; size: 14; color: Theme.textDim }
                    TextInput {
                        id: search
                        Layout.fillWidth: true; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; clip: true
                        Keys.onEscapePressed: ov.win.overviewOpen = false
                        Keys.onReturnPressed: { if (grid.firstMatch >= 0) { ov.win.activate(grid.firstMatch); ov.win.overviewOpen = false } }
                        Txt { visible: !search.text; text: qsTr("Search Tabs"); color: Theme.textFaint }
                    }
                }
            }
            IconButton { glyph: "plus"; tip: qsTr("New Tab"); onClicked: { ov.win.newTab(); ov.win.overviewOpen = false } }
            PushButton { text: qsTr("Done"); primary: true; onClicked: ov.win.overviewOpen = false }
        }

        GridView {
            id: grid
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            readonly property int cols: Math.max(1, Math.floor(width / 280))
            cellWidth: width / cols
            cellHeight: cellWidth * 0.68 + 34
            model: ov.win.tabs
            property int firstMatch: {
                var q = search.text.toLowerCase()
                for (var i = 0; i < ov.win.tabs.count; i++) {
                    var t = ov.win.tabs.get(i)
                    if (!q || (t.title || "").toLowerCase().indexOf(q) >= 0 || (t.url || "").toLowerCase().indexOf(q) >= 0) return i
                }
                return -1
            }
            delegate: Item {
                id: cell
                required property int index
                required property var model
                readonly property bool matches: { var q = search.text.toLowerCase(); return !q || (model.title || "").toLowerCase().indexOf(q) >= 0 || (model.url || "").toLowerCase().indexOf(q) >= 0 }
                width: grid.cellWidth
                height: grid.cellHeight
                opacity: matches ? 1 : 0.18
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    spacing: 8
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        radius: 10
                        color: Theme.window
                        border.width: cell.index === ov.win.currentIndex ? 3 : 1
                        border.color: cell.index === ov.win.currentIndex ? Theme.accent : Theme.separator
                        clip: true
                        scale: ch.hovered ? 1.02 : 1
                        Behavior on scale { NumberAnimation { duration: Theme.animFast } }
                        HoverHandler { id: ch }
                        Image {
                            anchors.fill: parent; anchors.margins: parent.border.width
                            source: cell.model.thumb || ""
                            fillMode: Image.PreserveAspectCrop
                            verticalAlignment: Image.AlignTop
                            visible: status === Image.Ready
                        }
                        Favicon {
                            anchors.centerIn: parent
                            visible: !cell.model.thumb
                            size: 40; url: cell.model.url || ""; title: cell.model.title || ""; source: cell.model.icon || ""
                        }
                        TapHandler { onTapped: { ov.win.activate(cell.index); ov.win.overviewOpen = false } }
                        IconButton {
                            anchors.top: parent.top; anchors.left: parent.left; anchors.margins: 6
                            visible: ch.hovered
                            round: true; implicitWidth: 22; implicitHeight: 22
                            glyph: "close"; iconSize: 10
                            background: Rectangle { radius: 11; color: Theme.popover; border.color: Theme.separator }
                            onClicked: ov.win.closeTab(cell.index)
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6
                        Favicon { size: 14; url: cell.model.url || ""; title: cell.model.title || ""; source: cell.model.icon || "" }
                        Txt { Layout.fillWidth: true; text: cell.model.title || Core.displayUrl(cell.model.url || "") || qsTr("Start Page"); font.pixelSize: 12 }
                        Icon { visible: cell.model.pinned; name: "pin"; size: 12; color: Theme.textDim }
                    }
                }
            }
        }
    }
}
