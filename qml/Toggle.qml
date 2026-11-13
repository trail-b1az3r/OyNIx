import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Pill switch.
AbstractButton {
    id: sw
    checkable: true
    implicitWidth: 34
    implicitHeight: 20
    focusPolicy: Qt.NoFocus
    background: Rectangle {
        radius: height / 2
        color: sw.checked ? Theme.accent : (Theme.dark ? "#48484c" : "#d4d4d8")
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
        Rectangle {
            width: parent.height - 4
            height: width
            radius: width / 2
            y: 2
            x: sw.checked ? parent.width - width - 2 : 2
            color: "white"
            Behavior on x { NumberAnimation { duration: Theme.animFast; easing.type: Easing.OutCubic } }
        }
    }
    contentItem: Item {}
}
