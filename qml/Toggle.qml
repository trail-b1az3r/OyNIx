import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Material 3 switch: 52×32 track, thumb grows from 16 to 24 (28 pressed)
// and shows a check icon when on.
AbstractButton {
    id: sw
    checkable: true
    implicitWidth: 52
    implicitHeight: 32
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    background: Rectangle {
        radius: height / 2
        color: sw.checked ? Theme.primary : Theme.surfaceContainerHighest
        border.width: sw.checked ? 0 : 2
        border.color: Theme.outline
        opacity: sw.enabled ? 1 : 0.38
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
        Rectangle {
            id: thumb
            property real d: sw.pressed ? 28 : (sw.checked ? 24 : 16)
            width: d; height: d; radius: d / 2
            anchors.verticalCenter: parent.verticalCenter
            x: sw.checked ? parent.width - (parent.height + d) / 2 : (parent.height - d) / 2
            color: sw.checked ? Theme.primaryFg : Theme.outline
            Behavior on x { SpringAnimation { spring: Theme.springDefault; damping: Theme.damping } }
            Behavior on d { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
            Icon { anchors.centerIn: parent; visible: sw.checked && thumb.d > 18; name: "check"; size: 16; stroke: 2.2; color: Theme.primaryContainerFg }
            Rectangle { anchors.centerIn: parent; width: 40; height: 40; radius: 20; z: -1
                color: sw.checked ? Theme.primary : Theme.surfaceFg; opacity: sw.pressed ? 0.12 : sw.hovered ? 0.08 : 0 }
        }
    }
    contentItem: Item {}
}
