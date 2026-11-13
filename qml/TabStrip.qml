import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Horizontal tabs. Pinned tabs collapse to icons on the left; the rest share
// the remaining width equally. Tabs can be dragged to reorder.
Item {
    id: strip
    property var win
    property bool compact: false
    readonly property int count: win.tabs.count
    readonly property int pinnedCount: { var n = 0; for (var i = 0; i < win.tabs.count; i++) if (win.tabs.get(i).pinned) n++; return n }
    readonly property real pinnedWidth: 40
    readonly property real tabWidth: Math.max(compact ? 90 : 110, Math.min(compact ? 220 : 260, (width - pinnedCount * (pinnedWidth + 4)) / Math.max(1, count - pinnedCount) - 4))
    implicitHeight: compact ? 30 : Theme.tabHeight + 8

    function xFor(i) {
        var x = 0
        for (var k = 0; k < i; k++) x += (win.tabs.get(k).pinned ? pinnedWidth : tabWidth) + 4
        return x
    }

    Flickable {
        id: flick
        anchors.fill: parent
        anchors.topMargin: strip.compact ? 0 : 4
        anchors.bottomMargin: strip.compact ? 0 : 4
        contentWidth: strip.xFor(strip.count)
        clip: true
        interactive: contentWidth > width
        boundsBehavior: Flickable.StopAtBounds

        Repeater {
            model: strip.win.tabs
            delegate: Rectangle {
                id: t
                required property int index
                required property var model
                readonly property bool current: index === strip.win.currentIndex
                readonly property bool pinned: model.pinned
                x: dragArea.drag.active ? x : strip.xFor(index)
                width: pinned ? strip.pinnedWidth : strip.tabWidth
                height: flick.height
                z: dragArea.drag.active ? 10 : 0
                radius: 7
                color: current ? Theme.tabActive : (hov.hovered ? Theme.tabHover : "transparent")
                border.color: current && !Theme.dark ? Theme.separator : "transparent"
                Behavior on x { enabled: !dragArea.drag.active; NumberAnimation { duration: Theme.anim; easing.type: Easing.OutCubic } }
                Behavior on width { NumberAnimation { duration: Theme.animFast } }
                HoverHandler { id: hov }

                ToolTip.visible: hov.hovered && !dragArea.pressed && Core.tabHoverPreviews
                ToolTip.delay: 700
                ToolTip.text: (model.title || "") + "\n" + Core.displayUrl(model.url || "")

                MouseArea {
                    id: dragArea
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
                    drag.target: strip.count > 1 ? t : null
                    drag.axis: Drag.XAxis
                    drag.threshold: 8
                    property int startIndex: -1
                    onPressed: function (m) {
                        if (m.button === Qt.LeftButton) { strip.win.activate(t.index); startIndex = t.index }
                    }
                    onClicked: function (m) {
                        if (m.button === Qt.MiddleButton) strip.win.closeTab(t.index)
                        else if (m.button === Qt.RightButton) strip.win.showTabMenu(t.index, t)
                    }
                    onDoubleClicked: strip.win.togglePin(t.index)
                    onReleased: {
                        if (!drag.active) return
                        var center = t.x + t.width / 2
                        var target = 0
                        for (var i = 0; i < strip.count; i++) {
                            var w = strip.win.tabs.get(i).pinned ? strip.pinnedWidth : strip.tabWidth
                            if (center > strip.xFor(i) + w / 2) target = i
                        }
                        if (startIndex >= 0 && target !== startIndex) strip.win.moveTab(startIndex, target)
                        t.x = Qt.binding(function () { return strip.xFor(t.index) })
                    }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: t.pinned ? 0 : 9
                    anchors.rightMargin: t.pinned ? 0 : 5
                    spacing: 7
                    Item { Layout.fillWidth: t.pinned; visible: t.pinned }
                    Favicon {
                        source: t.model.icon || ""
                        url: t.model.url || ""
                        title: t.model.title || ""
                        loading: t.model.loading
                        size: 16
                        opacity: t.model.suspended ? 0.5 : 1
                    }
                    Item { Layout.fillWidth: t.pinned; visible: t.pinned }
                    Txt {
                        visible: !t.pinned
                        Layout.fillWidth: true
                        text: t.model.title || Core.displayUrl(t.model.url || "") || qsTr("Start Page")
                        color: t.current ? Theme.text : Theme.textDim
                        font.pixelSize: 12
                        opacity: t.model.suspended ? 0.6 : 1
                    }
                    IconButton {
                        visible: !t.pinned && (t.model.audible || t.model.muted)
                        implicitWidth: 20; implicitHeight: 20
                        glyph: t.model.muted ? "mute" : "speaker"
                        iconSize: 13
                        iconColor: Theme.textDim
                        onClicked: strip.win.toggleMute(t.index)
                    }
                    IconButton {
                        visible: !t.pinned && (hov.hovered || t.current)
                        implicitWidth: 20; implicitHeight: 20
                        glyph: "close"; iconSize: 11
                        iconColor: Theme.textDim
                        tip: qsTr("Close Tab")
                        onClicked: strip.win.closeTab(t.index)
                    }
                }
                // Private-window accent underline on the active tab
                Rectangle {
                    visible: t.current && strip.win.privateMode
                    anchors.bottom: parent.bottom; anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width * 0.5; height: 2; radius: 1; color: Theme.purple
                }
            }
        }
    }
}
