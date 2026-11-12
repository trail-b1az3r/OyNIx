import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Push button: default (neutral) or primary (accent).
AbstractButton {
    id: b
    property bool primary: false
    property bool destructive: false
    implicitHeight: 28
    implicitWidth: Math.max(72, lbl.implicitWidth + 24)
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    background: Rectangle {
        radius: 6
        color: b.primary ? (b.pressed ? Qt.darker(Theme.accent, 1.15) : Theme.accent)
                         : (b.pressed ? Theme.pressed : (b.hovered ? Theme.hover : Theme.field))
        border.color: b.primary ? "transparent" : Theme.separator
    }
    contentItem: Txt {
        id: lbl
        text: b.text
        horizontalAlignment: Text.AlignHCenter
        color: b.primary ? "white" : (b.destructive ? Theme.danger : Theme.text)
        font.weight: Font.Medium
        opacity: b.enabled ? 1 : 0.5
    }
}
