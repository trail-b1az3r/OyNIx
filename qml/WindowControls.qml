import QtQuick
import OyNIx

// Close / minimise / zoom dots for the frameless window. Glyphs appear
// on hover of the group, like a classic desktop title bar.
Row {
    id: ctl
    property var window
    spacing: 8
    visible: !Core.useSystemTitleBar
    HoverHandler { id: grp }

    Repeater {
        model: [
            { c: Theme.ctlClose, g: "close", a: "close" },
            { c: Theme.ctlMin, g: "minus", a: "min" },
            { c: Theme.ctlMax, g: "plus", a: "max" }
        ]
        delegate: Rectangle {
            required property var modelData
            width: 12; height: 12; radius: 6
            color: ctl.window.active || grp.hovered ? modelData.c : (Theme.dark ? "#4a4a4d" : "#d0d0d3")
            border.color: Qt.darker(color, 1.15)
            border.width: 0.5
            Icon {
                anchors.centerIn: parent
                visible: grp.hovered
                name: modelData.g; size: 9; stroke: 1.6
                color: Qt.rgba(0, 0, 0, 0.55)
            }
            TapHandler {
                onTapped: {
                    if (modelData.a === "close") ctl.window.close()
                    else if (modelData.a === "min") ctl.window.showMinimized()
                    else ctl.window.visibility === Window.Maximized ? ctl.window.showNormal() : ctl.window.showMaximized()
                }
            }
        }
    }
}
