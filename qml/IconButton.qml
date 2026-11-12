import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Borderless toolbar button: icon with a soft rounded hover plate.
AbstractButton {
    id: btn
    property string glyph: ""
    property real iconSize: 16
    property color iconColor: enabled ? (checked ? Theme.accent : Theme.text) : Theme.textFaint
    property string tip: ""
    property bool filledIcon: false
    property int badge: 0
    property bool round: false

    implicitWidth: 30
    implicitHeight: 28
    hoverEnabled: true
    focusPolicy: Qt.NoFocus

    background: Rectangle {
        radius: btn.round ? height / 2 : 6
        color: btn.pressed ? Theme.pressed : (btn.hovered && btn.enabled ? Theme.hover : "transparent")
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: btn.glyph
            size: btn.iconSize
            color: btn.iconColor
            filled: btn.filledIcon
        }
        Rectangle {
            visible: btn.badge > 0
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.rightMargin: -3
            anchors.topMargin: -3
            height: 14
            width: Math.max(14, badgeText.implicitWidth + 7)
            radius: 7
            color: Theme.accent
            Text {
                id: badgeText
                anchors.centerIn: parent
                text: btn.badge > 99 ? "99+" : btn.badge
                color: "white"
                font.pixelSize: 9
                font.bold: true
            }
        }
    }

    ToolTip.visible: tip !== "" && hovered && !pressed
    ToolTip.delay: 650
    ToolTip.text: tip
}
