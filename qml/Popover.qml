import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Floating rounded panel with a soft shadow, anchored under a toolbar item.
Popup {
    id: pop
    padding: 14
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animFast }
        NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: Theme.animFast; easing.type: Easing.OutCubic }
    }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: 90 } }

    background: Item {
        Rectangle {
            anchors.fill: parent
            anchors.margins: -1
            anchors.topMargin: 2
            radius: Theme.radiusLarge + 1
            color: Theme.shadow
            opacity: 0.35
        }
        Rectangle {
            anchors.fill: parent
            radius: Theme.radiusLarge
            color: Theme.popover
            border.color: Theme.separator
            border.width: 1
        }
    }
}
