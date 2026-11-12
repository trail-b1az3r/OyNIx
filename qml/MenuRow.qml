import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// A row in a popover menu: icon, label, optional shortcut / trailing text.
AbstractButton {
    id: row
    property string glyph: ""
    property string shortcut: ""
    property color tint: Theme.text
    implicitHeight: 28
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    Layout.fillWidth: true
    background: Rectangle {
        radius: 5
        color: row.hovered && row.enabled ? Theme.accent : "transparent"
    }
    contentItem: RowLayout {
        spacing: 9
        Icon {
            visible: row.glyph !== ""
            name: row.glyph
            size: 15
            color: row.hovered && row.enabled ? "white" : row.tint
            Layout.leftMargin: 6
        }
        Txt {
            text: row.text
            Layout.fillWidth: true
            Layout.leftMargin: row.glyph === "" ? 8 : 0
            color: row.hovered && row.enabled ? "white" : (row.enabled ? row.tint : Theme.textFaint)
        }
        Txt {
            visible: row.shortcut !== ""
            text: row.shortcut
            color: row.hovered ? "white" : Theme.textFaint
            font.pixelSize: 12
            Layout.rightMargin: 8
        }
    }
}
