import QtQuick
import OyNIx

// Material 3 filled text field (single line).
Rectangle {
    id: f
    property alias text: input.text
    property alias echoMode: input.echoMode
    property alias readOnly: input.readOnly
    property string placeholder: ""
    signal accepted()
    implicitHeight: 44
    implicitWidth: 200
    radius: Theme.shapeM
    color: Theme.field
    border.width: Theme.edge > 0 ? Theme.edge : input.activeFocus ? 2 : 0
    border.color: input.activeFocus ? Theme.primary : Theme.edgeColor
    HardShadow { radius: f.radius; visible: Theme.hard && input.activeFocus }
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
