import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Material 3 surface for menus and popovers: surface container, large
// corner radius, soft elevation shadow, and a spring scale-in.
Popup {
    id: pop
    padding: 16
    // Keep menus and popovers fully inside the window.
    margins: 8
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
    transformOrigin: Item.Top

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animFast }
        SpringAnimation { property: "scale"; from: 0.85; to: 1; spring: Theme.springDefault; damping: Theme.damping }
    }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: 100 } }

    background: Item {
        Rectangle { anchors.fill: parent; anchors.topMargin: 3; anchors.margins: -1; radius: Theme.shapeL + 2; color: Theme.shadow; opacity: 0.45 }
        Rectangle { anchors.fill: parent; radius: Theme.shapeL; color: Theme.surfaceContainer }
    }
}
