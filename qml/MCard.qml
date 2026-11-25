import QtQuick
import QtQuick.Layouts
import OyNIx

// Material 3 filled card with an optional title.
Rectangle {
    id: card
    property string title: ""
    property color tone: Theme.surfaceContainerLow
    default property alias content: col.data
    implicitHeight: col.implicitHeight + 2 * Theme.sx("card", "padding", 16)
    radius: Theme.sx("card", "radius", Theme.shapeXL - 4)
    color: tone
    border.width: Theme.edgeW(1)
    border.color: Theme.glassRim
    Neu { z: -1; anchors.fill: parent; radius: card.radius; depth: 7 }
    HardShadow { radius: card.radius }
    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: Theme.sx("card", "padding", 16)
        spacing: 10
        Txt { visible: card.title !== ""; text: card.title; font.pixelSize: Theme.titleSize; font.weight: Font.DemiBold }
    }
}
