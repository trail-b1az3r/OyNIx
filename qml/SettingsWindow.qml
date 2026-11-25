import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Window
import OyNIx

// Settings in its own window. It is a dialog of the browser window
// (transient parent), so tiling compositors such as Hyprland float it by
// default; match title "OyNIx Settings" in window rules to change that.
ApplicationWindow {
    id: w
    property var app
    property alias page: view.page
    width: 960
    height: 700
    minimumWidth: 620
    minimumHeight: 440
    title: qsTr("OyNIx Settings")
    // See BrowserWindow: a see-through window's tint is drawn, not cleared.
    color: Theme.seeThrough ? "transparent" : Theme.surface
    Rectangle { anchors.fill: parent; visible: Theme.seeThrough; color: Theme.surface }
    SettingsView { id: view; anchors.fill: parent; app: w.app; shown: w.visible }
    Shortcut { sequences: ["Escape", "Ctrl+W"]; onActivated: w.close() }
}
