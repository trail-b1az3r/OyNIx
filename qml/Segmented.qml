import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Material 3 Expressive connected button group (segmented choice).
RowLayout {
    id: seg
    property var options: []        // [{v, t}]
    property var value
    signal picked(var v)
    spacing: 2
    Repeater {
        model: seg.options
        delegate: AbstractButton {
            id: b
            required property var modelData
            required property int index
            readonly property bool selected: modelData.v === seg.value
            Layout.fillWidth: true
            implicitHeight: 40
            implicitWidth: lbl.implicitWidth + 32
            hoverEnabled: true
            onClicked: seg.picked(modelData.v)
            background: Rectangle {
                // Outer corners fully round, inner corners small; the selected
                // segment morphs to a full pill.
                radius: b.selected ? height / 2 : (b.pressed ? Theme.shapeS : Theme.shapeS + 2)
                color: b.selected ? Theme.secondaryContainer : (b.hovered ? Theme.surfaceContainerHigh : Theme.surfaceContainer)
                Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
                Behavior on color { ColorAnimation { duration: Theme.animFast } }
            }
            contentItem: RowLayout {
                spacing: 6
                Item { Layout.fillWidth: true }
                Icon { visible: b.selected; name: "check"; size: 16; color: Theme.secondaryContainerFg }
                Txt { id: lbl; text: b.modelData.t; font.pixelSize: 13; font.weight: Font.Medium; color: b.selected ? Theme.secondaryContainerFg : Theme.text }
                Item { Layout.fillWidth: true }
            }
        }
    }
}
