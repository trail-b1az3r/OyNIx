import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Material 3 Expressive menu item: rounded state layer, leading icon,
// trailing shortcut text.
AbstractButton {
    id: row
    property string glyph: ""
    property string shortcut: ""
    property color tint: Theme.surfaceFg
    implicitHeight: 44
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    Layout.fillWidth: true
    background: Rectangle {
        radius: Theme.shapeM
        color: row.pressed ? Theme.secondaryContainer : (row.hovered && row.enabled ? Theme.hover : "transparent")
        Behavior on color { ColorAnimation { duration: Theme.animFast } }
    }
    contentItem: RowLayout {
        spacing: 12
        Icon {
            visible: row.glyph !== ""
            name: row.glyph
            size: 20
            color: row.enabled ? (row.tint === Theme.surfaceFg ? Theme.surfaceVariantFg : row.tint) : Theme.alpha(Theme.surfaceFg, 0.38)
            Layout.leftMargin: 12
        }
        Txt {
            text: row.text
            Layout.fillWidth: true
            Layout.leftMargin: row.glyph === "" ? 12 : 0
            color: row.enabled ? row.tint : Theme.alpha(Theme.surfaceFg, 0.38)
            font.pixelSize: 14
        }
        Txt {
            visible: row.shortcut !== ""
            text: row.shortcut
            color: Theme.surfaceVariantFg
            font.pixelSize: 12
            Layout.rightMargin: 12
        }
    }
}
