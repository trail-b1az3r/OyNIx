import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Tabs listed in the sidebar: pinned tabs as a grid of tiles on top, then
// the rest as rows. Drag rows to reorder.
ColumnLayout {
    id: vt
    property var win
    spacing: 6

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
            height: model.pinned ? 0 : 34
            visible: !model.pinned

            Rectangle {
                id: plate
                anchors.fill: parent
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
                        if (d > row.height * 0.8 && row.index < vt.win.tabs.count - 1) { vt.win.moveTab(row.index, row.index + 1); pressY += row.height }
                        else if (d < -row.height * 0.8 && row.index > 0 && !vt.win.tabs.get(row.index - 1).pinned) { vt.win.moveTab(row.index, row.index - 1); pressY -= row.height }
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
                        visible: rh.hovered
                        implicitWidth: 20; implicitHeight: 20
                        glyph: "close"; iconSize: 11; iconColor: Theme.textDim
                        onClicked: vt.win.closeTab(row.index)
                    }
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
