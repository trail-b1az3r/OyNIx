import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Material 3 Expressive button: filled (primary), tonal (default),
// outlined or text; destructive uses the error role. Pill shaped, morphing
// to a rounded square while pressed.
AbstractButton {
    id: b
    property bool primary: false
    property bool destructive: false
    property bool outlined: false
    property bool textOnly: false
    implicitHeight: 40
    implicitWidth: Math.max(64, lbl.implicitWidth + 32)
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    readonly property color fg: !enabled ? Theme.alpha(Theme.surfaceFg, 0.38)
                               : primary ? (destructive ? Theme.errorFg : Theme.primaryFg)
                               : destructive ? Theme.error
                               : (outlined || textOnly) ? Theme.primary : Theme.secondaryContainerFg
    background: Rectangle {
        radius: b.pressed ? Theme.shapeM : height / 2
        color: !b.enabled ? Theme.alpha(Theme.surfaceFg, b.textOnly || b.outlined ? 0 : 0.12)
               : b.primary ? (b.destructive ? Theme.error : Theme.primary)
               : (b.outlined || b.textOnly) ? "transparent"
               : (b.destructive ? Theme.errorContainer : Theme.secondaryContainer)
        border.width: b.outlined ? 1 : 0
        border.color: Theme.outlineVariant
        Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
        Rectangle {
            anchors.fill: parent; radius: parent.radius
            color: b.fg
            opacity: !b.enabled ? 0 : b.pressed ? 0.12 : b.hovered ? 0.08 : 0
        }
    }
    contentItem: Txt {
        id: lbl
        text: b.text
        horizontalAlignment: Text.AlignHCenter
        color: b.fg
        font.weight: Font.Medium
        font.pixelSize: 14
    }
}
