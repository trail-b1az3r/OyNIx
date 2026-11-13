import QtQuick
import OyNIx

// Material 3 filled text field (single line).
Rectangle {
    id: f
    property alias text: input.text
    property alias echoMode: input.echoMode
    property string placeholder: ""
    signal accepted()
    implicitHeight: 44
    implicitWidth: 200
    radius: Theme.shapeM
    color: Theme.surfaceContainerHighest
    border.width: input.activeFocus ? 2 : 0
    border.color: Theme.primary
    TextInput {
        id: input
        anchors.fill: parent; anchors.leftMargin: 14; anchors.rightMargin: 14
        verticalAlignment: TextInput.AlignVCenter
        color: Theme.text; font.family: Theme.font; font.pixelSize: 14
        selectionColor: Theme.selection; clip: true
        onAccepted: f.accepted()
        Txt { visible: !input.text && !input.activeFocus; text: f.placeholder; color: Theme.textFaint; anchors.verticalCenter: parent.verticalCenter }
    }
}
