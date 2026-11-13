import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Full-page history (oynix://history) with search and "Clear History…".
Rectangle {
    id: page
    property var win
    color: Theme.window

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32
        anchors.leftMargin: Math.max(32, (parent.width - 860) / 2)
        anchors.rightMargin: anchors.leftMargin
        spacing: 16
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("History"); font.pixelSize: 24; font.weight: Font.Bold; Layout.fillWidth: true }
            Rectangle {
                implicitWidth: 260; implicitHeight: 30; radius: 7; color: Theme.field
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 8
                    Icon { name: "search"; size: 14; color: Theme.textDim }
                    TextInput { id: q; Layout.fillWidth: true; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; clip: true
                        Txt { visible: !q.text; text: qsTr("Search History"); color: Theme.textFaint } }
                }
            }
            PushButton { text: qsTr("Clear History…"); onClicked: clearMenu.open() }
        }
        ListView {
            id: lv
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: { Core.historyRevision; return JSON.parse(Core.history(q.text, 1000)) }
            ScrollBar.vertical: ScrollBar {}
            delegate: Rectangle {
                required property var modelData
                width: lv.width; height: 38; radius: 7
                color: h.hovered ? Theme.hover : "transparent"
                HoverHandler { id: h }
                TapHandler { onTapped: page.win.navigate(modelData.url) }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 6; spacing: 12
                    Txt { text: new Date(modelData.lastVisit * 1000).toLocaleString(Qt.locale(), "dd MMM HH:mm"); color: Theme.textDim; font.pixelSize: 12; Layout.preferredWidth: 96 }
                    Favicon { size: 16; url: modelData.url; title: modelData.title }
                    Txt { text: modelData.title || modelData.url; Layout.fillWidth: true }
                    Txt { text: modelData.host; color: Theme.textDim; font.pixelSize: 12; Layout.maximumWidth: 220 }
                    IconButton { glyph: "trash"; iconSize: 13; iconColor: Theme.textDim; opacity: h.hovered ? 1 : 0; onClicked: Core.removeHistory(modelData.url) }
                }
            }
        }
    }

    Popover {
        id: clearMenu
        x: page.width - width - 32
        y: 70
        width: 240
        contentItem: ColumnLayout {
            spacing: 2
            Txt { text: qsTr("Clear history from"); color: Theme.textDim; font.pixelSize: 12; Layout.bottomMargin: 4 }
            MenuRow { text: qsTr("The last hour"); onClicked: { Core.clearHistory(3600); clearMenu.close() } }
            MenuRow { text: qsTr("Today"); onClicked: { Core.clearHistory(86400); clearMenu.close() } }
            MenuRow { text: qsTr("The last 7 days"); onClicked: { Core.clearHistory(7 * 86400); clearMenu.close() } }
            MenuRow { text: qsTr("All history"); tint: Theme.danger; onClicked: { Core.clearHistory(0); clearMenu.close() } }
        }
    }
}
