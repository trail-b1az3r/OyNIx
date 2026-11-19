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

    // 5.4 Liquid Glass: clear at rest; a glass capsule (translucent fill,
    // light rim) while hovered, pressed or checked; filled stays solid.
    readonly property bool glassy: !filled && enabled && (hovered || pressed || checked || tonal)
    background: Rectangle {
        radius: btn.pressed ? Math.min(Theme.pill(height), Theme.shapeM) : Theme.pill(height)
        color: btn.filled ? Theme.primary : btn.glassy ? ((btn.checked || btn.tonal) ? Qt.tint(Theme.glassFill, Theme.alpha(Theme.secondaryContainer, 0.55)) : Theme.glassFill) : "transparent"
        border.width: Theme.edgeW(btn.glassy ? 1 : 0)
        border.color: Theme.glassRim
        Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
        // Neumorphic theme: raised while hovered or checked, pressed in while pressed.
        Neu { z: -1; anchors.fill: parent; radius: parent.radius; visible: Theme.neu && btn.glassy; inset: btn.pressed; depth: 3 }
        HardShadow { radius: parent.radius; visible: Theme.hard && (btn.filled || btn.checked) && !btn.pressed }
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
            scale: btn.pressed ? 0.88 : 1
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
            radius: Theme.r(8)
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
