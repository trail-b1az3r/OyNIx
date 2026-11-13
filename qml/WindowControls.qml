import QtQuick
import QtQuick.Window
import OyNIx

// Window buttons for the frameless window, as Material 3 icon buttons.
// Close gets the error container on hover.
Row {
    id: ctl
    property var window
    spacing: 2
    visible: !Core.useSystemTitleBar

    IconButton {
        glyph: "minimize"; iconSize: 18; tip: qsTr("Minimise")
        onClicked: ctl.window.showMinimized()
    }
    IconButton {
        glyph: ctl.window && ctl.window.visibility === Window.Maximized ? "restore" : "maximize"
        iconSize: 16; tip: ctl.window && ctl.window.visibility === Window.Maximized ? qsTr("Restore") : qsTr("Maximise")
        onClicked: ctl.window.visibility === Window.Maximized ? ctl.window.showNormal() : ctl.window.showMaximized()
    }
    IconButton {
        id: closeBtn
        glyph: "close"; iconSize: 18; tip: qsTr("Close")
        iconColor: hovered ? Theme.errorContainerFg : Theme.surfaceVariantFg
        background: Rectangle {
            radius: closeBtn.pressed ? Theme.shapeM : height / 2
            color: closeBtn.hovered ? Theme.errorContainer : "transparent"
            Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
            Behavior on color { ColorAnimation { duration: Theme.animFast } }
        }
        onClicked: ctl.window.close()
    }
}
