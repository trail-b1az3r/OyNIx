import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Material 3 Expressive icon button: round, with a state layer; the shape
// morphs to a rounded square while pressed. Variants: standard (default),
// tonal, filled; `checked` shows the selected (secondary container) state.
AbstractButton {
    id: btn
    property string glyph: ""
    property real iconSize: 20
    property color iconColor: !enabled ? Theme.alpha(Theme.surfaceFg, 0.38)
                              : filled ? Theme.primaryFg
                              : (checked || tonal) ? Theme.secondaryContainerFg : Theme.surfaceVariantFg
    property string tip: ""
    property bool filledIcon: false
    property int badge: 0
    property bool round: true
    property bool tonal: false
    property bool filled: false

    implicitWidth: 40
    implicitHeight: 40
    hoverEnabled: true
    focusPolicy: Qt.NoFocus

    background: Rectangle {
        radius: btn.pressed ? Math.min(height / 2, Theme.shapeM) : height / 2
        color: btn.filled ? Theme.primary : (btn.checked || btn.tonal) ? Theme.secondaryContainer : "transparent"
        Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: btn.filled || btn.checked || btn.tonal ? (btn.filled ? Theme.primaryFg : Theme.secondaryContainerFg) : Theme.surfaceFg
            opacity: !btn.enabled ? 0 : btn.pressed ? 0.12 : btn.hovered ? 0.08 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.animFast } }
        }
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: btn.glyph
            size: btn.iconSize
            color: btn.iconColor
            filled: btn.filledIcon
            scale: btn.pressed ? 0.9 : 1
            Behavior on scale { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
        }
        Rectangle {
            visible: btn.badge > 0
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.rightMargin: 2
            anchors.topMargin: 2
            height: 16
            width: Math.max(16, badgeText.implicitWidth + 8)
            radius: 8
            color: Theme.error
            Text {
                id: badgeText
                anchors.centerIn: parent
                text: btn.badge > 999 ? "999+" : btn.badge
                color: Theme.errorFg
                font.pixelSize: 10
                font.bold: true
            }
        }
    }

    ToolTip.visible: tip !== "" && hovered && !pressed
    ToolTip.delay: 650
    ToolTip.text: tip
}
