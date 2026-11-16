import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Something to do while the internet is gone: Go or Blackjack against the
// computer, entirely inside OyNIx. Shown on the "You're offline" page
// (Settings › Misc chooses which game) and at oynix://play.
Rectangle {
    id: og
    property bool offline: false      // show the offline banner and retry
    property string heading: ""
    property string body: ""
    property string game: Core.offlineGame === "blackjack" ? "blackjack" : "go"
    signal retry()
    color: Theme.surface

    // Watch for the connection coming back.
    property bool backOnline: false
    Timer {
        interval: 3000; repeat: true
        running: og.offline && og.visible && !og.backOnline
        onTriggered: og.backOnline = Core.networkOnline()
    }

    Flickable {
        anchors.fill: parent
        contentHeight: col.implicitHeight + 64
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        ScrollBar.vertical: ScrollBar {}

        ColumnLayout {
            id: col
            x: (parent.width - width) / 2; y: 32
            width: Math.min(520, og.width - 48)
            spacing: 18

            // Offline banner
            Rectangle {
                visible: og.offline
                Layout.fillWidth: true
                implicitHeight: banner.implicitHeight + 28
                radius: Theme.shapeL
                color: og.backOnline ? Theme.primaryContainer : Theme.secondaryContainer
                RowLayout {
                    id: banner
                    anchors.fill: parent; anchors.margins: 14
                    spacing: 14
                    MShape { size: 44; shape: "clover4"; color: og.backOnline ? Theme.primary : Theme.surface
                        Icon { anchors.centerIn: parent; name: og.backOnline ? "check" : "globe"; size: 20; color: og.backOnline ? Theme.primaryFg : Theme.textDim } }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Txt { Layout.fillWidth: true; text: og.backOnline ? qsTr("You’re back online") : og.heading; font.weight: Font.DemiBold; font.pixelSize: 15 }
                        Txt { Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone; color: Theme.textDim; font.pixelSize: 12
                              text: og.backOnline ? qsTr("Load the page whenever you’re ready; your game will wait.") : og.body }
                    }
                    PushButton { text: qsTr("Try Again"); primary: og.backOnline; onClicked: og.retry() }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                Txt { text: og.offline ? qsTr("While you wait") : qsTr("Games"); font.family: Theme.displayFont; font.pixelSize: 26; Layout.fillWidth: true }
                Segmented {
                    options: [{ v: "go", t: qsTr("Go") }, { v: "blackjack", t: qsTr("Blackjack") }]
                    value: og.game
                    onPicked: function (v) { og.game = v }
                }
            }

            Loader {
                Layout.fillWidth: true
                sourceComponent: og.game === "blackjack" ? bjComp : goComp
            }
        }
    }

    Component { id: goComp; GoGame {} }
    Component { id: bjComp; BlackjackGame {} }
}
