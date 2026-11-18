import QtQuick
import QtQuick.Layouts
import OyNIx

// Material 3 filled card with an optional title.
Rectangle {
    id: card
    property string title: ""
    property color tone: Theme.surfaceContainerLow
    default property alias content: col.data
    implicitHeight: col.implicitHeight + 32
    // 5.4: cards sit softly raised: larger radius and a light edge.
    radius: Math.round(24 * Theme.cornerScale)
    color: tone
    border.width: 1
    border.color: Theme.highlightEdge
    ColumnLayout {
        id: col
        anchors.fill: parent
        anchors.margins: 16
        spacing: 10
        Txt { visible: card.title !== ""; text: card.title; font.pixelSize: Theme.titleSize; font.weight: Font.DemiBold }
    }
}
