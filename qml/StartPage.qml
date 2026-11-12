import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Native new-tab page: Favorites, Frequently Visited, Privacy Report,
// Reading List. Rendered by Qt Quick, so it opens instantly.
Rectangle {
    id: page
    property var win
    color: Theme.window

    gradient: Gradient {
        GradientStop { position: 0; color: Core.startPageBackground === "plain" ? Theme.window : (Theme.dark ? "#202024" : "#f7f8fb") }
        GradientStop { position: 1; color: Theme.window }
    }

    Flickable {
        anchors.fill: parent
        contentHeight: col.implicitHeight + 120
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        ColumnLayout {
            id: col
            width: Math.min(820, page.width - 64)
            x: (page.width - width) / 2
            y: Math.max(48, page.height * 0.12)
            spacing: 34

            // Favorites
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 14
                Txt { text: qsTr("Favorites"); font.pixelSize: 17; font.weight: Font.Bold }
                Flow {
                    Layout.fillWidth: true
                    spacing: 18
                    Repeater {
                        model: { Core.bookmarksRevision; return JSON.parse(Core.bookmarks("favorites")) }
                        delegate: Tile { required property var modelData; item: modelData; removable: true }
                    }
                }
            }

            // Frequently visited
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 14
                visible: Core.startPageShowRecent && freq.count > 0 && !page.win.privateMode
                Txt { text: qsTr("Frequently Visited"); font.pixelSize: 17; font.weight: Font.Bold }
                Flow {
                    Layout.fillWidth: true
                    spacing: 18
                    Repeater {
                        id: freq
                        model: { Core.historyRevision; return JSON.parse(Core.mostVisited(8)) }
                        delegate: Tile { required property var modelData; item: modelData }
                    }
                }
            }

            // Privacy report
            Rectangle {
                Layout.fillWidth: true
                visible: Core.startPageShowPrivacyReport
                implicitHeight: 96
                radius: 14
                color: Theme.card
                border.color: Theme.separator
                RowLayout {
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 18
                    Rectangle {
                        width: 52; height: 52; radius: 14
                        color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.14)
                        Icon { anchors.centerIn: parent; name: "shieldCheck"; size: 28; color: Theme.accent }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Txt { text: qsTr("Privacy Report"); font.pixelSize: 15; font.weight: Font.DemiBold }
                        Txt {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: Theme.textDim
                            text: Core.contentBlocker
                                  ? qsTr("OyNIx has blocked %1 trackers and %2 ads. Nothing you do is sent to us — there is no telemetry.").arg(Math.round(Core.totalTrackers).toLocaleString()).arg(Math.round(Core.totalAds).toLocaleString())
                                  : qsTr("Content blocking is turned off. Turn it on in Settings › Privacy.")
                        }
                    }
                    Txt {
                        text: Math.round(Core.totalBlocked).toLocaleString(Qt.locale(), "f", 0)
                        font.pixelSize: 34; font.weight: Font.Bold
                        color: Theme.accent
                    }
                }
                TapHandler { onTapped: page.win.openPrivacyReport() }
            }

            // Reading list
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10
                visible: Core.startPageShowReadingList && readRep.count > 0
                Txt { text: qsTr("Reading List"); font.pixelSize: 17; font.weight: Font.Bold }
                Repeater {
                    id: readRep
                    model: { Core.readingRevision; return JSON.parse(Core.readingList()).filter(function (r) { return !r.read }).slice(0, 5) }
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: 48
                        radius: 10
                        color: rh.hovered ? Theme.hover : Theme.card
                        HoverHandler { id: rh }
                        TapHandler { onTapped: { Core.markRead(modelData.url, true); page.win.navigate(modelData.url) } }
                        RowLayout {
                            anchors.fill: parent; anchors.margins: 12
                            spacing: 12
                            Favicon { size: 20; url: modelData.url; title: modelData.title }
                            ColumnLayout {
                                spacing: 1
                                Layout.fillWidth: true
                                Txt { Layout.fillWidth: true; text: modelData.title || modelData.url; font.weight: Font.Medium }
                                Txt { Layout.fillWidth: true; text: Core.displayUrl(modelData.url); font.pixelSize: 11; color: Theme.textDim }
                            }
                        }
                    }
                }
            }

            Txt {
                Layout.alignment: Qt.AlignHCenter
                visible: page.win.privateMode
                color: Theme.purple
                text: qsTr("Private Browsing — history, cookies and site data from this window are forgotten when it closes.")
                font.pixelSize: 12
            }
        }
    }

    component Tile: AbstractButton {
        id: tile
        property var item
        property bool removable: false
        width: 84
        height: 98
        hoverEnabled: true
        onClicked: page.win.navigate(item.url)
        contentItem: Column {
            spacing: 8
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 64; height: 64; radius: 16
                color: tile.hovered ? Theme.tabHover : Theme.card
                border.color: Theme.separator
                scale: tile.pressed ? 0.95 : 1
                Behavior on scale { NumberAnimation { duration: 90 } }
                Favicon { anchors.centerIn: parent; size: 30; url: tile.item.url; title: tile.item.title }
            }
            Txt {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: tile.item.title || tile.item.host
                font.pixelSize: 12
                color: Theme.textDim
            }
        }
        IconButton {
            visible: tile.removable && tile.hovered
            x: 2; y: -4
            implicitWidth: 20; implicitHeight: 20; round: true
            glyph: "close"; iconSize: 9
            background: Rectangle { radius: 10; color: Theme.popover; border.color: Theme.separator }
            onClicked: Core.removeBookmark(tile.item.id)
        }
    }
}
