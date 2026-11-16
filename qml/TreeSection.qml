import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// One part of a tree tab: a heading and indented rows joined by guide
// lines. Long lists scroll on their own and keep the active row in view.
ColumnLayout {
    id: ts
    property string title: ""
    property string glyph: "list"
    /// [{depth, title, active, hint}]
    property var items: []
    property real maxHeight: 300
    signal picked(int index)
    spacing: 2

    readonly property int activeIndex: { for (var i = 0; i < items.length; i++) if (items[i].active) return i; return -1 }
    onActiveIndexChanged: if (activeIndex >= 0) list.positionViewAtIndex(activeIndex, ListView.Contain)

    RowLayout {
        Layout.fillWidth: true
        Layout.leftMargin: 14; Layout.topMargin: 4
        spacing: 6
        Icon { name: ts.glyph; size: 12; color: Theme.textFaint }
        Txt { text: ts.title; font.pixelSize: 10; font.weight: Font.DemiBold; font.capitalization: Font.AllUppercase; font.letterSpacing: 0.6; color: Theme.textFaint; Layout.fillWidth: true }
        Txt { text: ts.items.length; font.pixelSize: 10; color: Theme.textFaint; Layout.rightMargin: 4 }
    }

    ListView {
        id: list
        Layout.fillWidth: true
        Layout.preferredHeight: Math.min(contentHeight, ts.maxHeight)
        clip: true
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds
        model: ts.items
        ScrollBar.vertical: ScrollBar { policy: list.interactive ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff; width: 6 }
        delegate: AbstractButton {
            id: node
            required property var modelData
            required property int index
            width: list.width
            height: 24
            hoverEnabled: true
            readonly property int depth: Math.min(modelData.depth, 6)
            ToolTip.visible: hovered && label.truncated
            ToolTip.delay: 600
            ToolTip.text: modelData.title
            background: Rectangle {
                x: 14 + node.depth * 12 - 4
                width: parent.width - x - 4
                radius: 5
                color: node.modelData.active ? Theme.alpha(Theme.accent, 0.14) : (node.hovered ? Theme.hover : "transparent")
            }
            // Guide lines, one per level above this one.
            Repeater {
                model: node.depth
                delegate: Rectangle {
                    required property int index
                    x: 14 + index * 12 + 3
                    width: 1; height: node.height
                    color: Theme.separator
                }
            }
            contentItem: RowLayout {
                spacing: 6
                Item { Layout.preferredWidth: 14 + node.depth * 12 - 6 }
                Rectangle {
                    Layout.preferredWidth: 5; Layout.preferredHeight: 5; radius: 2.5
                    color: node.modelData.active ? Theme.accent : Theme.textFaint
                }
                Txt {
                    id: label
                    Layout.fillWidth: true
                    text: node.modelData.title
                    font.pixelSize: 12
                    font.weight: node.modelData.active ? Font.DemiBold : Font.Normal
                    color: node.modelData.active ? Theme.text : Theme.textDim
                }
                Txt { visible: !!node.modelData.hint; text: node.modelData.hint || ""; font.pixelSize: 10; color: Theme.textFaint; Layout.rightMargin: 8 }
            }
            onClicked: ts.picked(index)
        }
    }
}
