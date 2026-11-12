import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtWebEngine
import OyNIx

// Find-in-page bar that slides in under the toolbar.
Rectangle {
    id: fb
    property var win
    property int matches: 0
    property int active: 0
    property bool caseSensitive: false
    implicitHeight: 38
    color: Theme.toolbar
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.separator }

    function open() { visible = true; input.forceActiveFocus(); input.selectAll(); if (input.text) find(false) }
    function close() {
        visible = false
        var v = win.currentView
        if (v) v.findText("")
        win.focusContent()
    }
    function find(backward) {
        var v = win.currentView
        if (!v) return
        var flags = 0
        if (backward) flags |= WebEngineView.FindBackward
        if (caseSensitive) flags |= WebEngineView.FindCaseSensitively
        v.findText(input.text, flags)
    }
    function setResult(r) { matches = r.numberOfMatches; active = r.activeMatch }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 12; anchors.rightMargin: 12
        spacing: 8
        Rectangle {
            Layout.preferredWidth: 300; implicitHeight: 26; radius: 6
            color: Theme.fieldFocus
            border.color: fb.matches === 0 && input.text ? Theme.danger : Theme.separator
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 8; anchors.rightMargin: 8
                Icon { name: "search"; size: 13; color: Theme.textDim }
                TextInput {
                    id: input
                    Layout.fillWidth: true; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; clip: true
                    onTextChanged: fb.find(false)
                    Keys.onReturnPressed: function (e) { fb.find(e.modifiers & Qt.ShiftModifier) }
                    Keys.onEscapePressed: fb.close()
                }
                Txt { visible: input.text !== ""; text: fb.matches ? (fb.active + "/" + fb.matches) : qsTr("No results"); color: Theme.textDim; font.pixelSize: 11 }
            }
        }
        IconButton { glyph: "chevronUp"; tip: qsTr("Previous"); onClicked: fb.find(true) }
        IconButton { glyph: "chevronDown"; tip: qsTr("Next"); onClicked: fb.find(false) }
        AbstractButton {
            implicitHeight: 24; implicitWidth: 34; checkable: true
            checked: fb.caseSensitive
            onToggled: { fb.caseSensitive = checked; fb.find(false) }
            background: Rectangle { radius: 5; color: parent.checked ? Theme.selection : "transparent"; border.color: Theme.separator }
            contentItem: Txt { text: "Aa"; horizontalAlignment: Text.AlignHCenter; font.pixelSize: 12 }
            ToolTip.visible: hovered; ToolTip.text: qsTr("Match Case")
        }
        Item { Layout.fillWidth: true }
        PushButton { text: qsTr("Done"); onClicked: fb.close() }
    }
}
