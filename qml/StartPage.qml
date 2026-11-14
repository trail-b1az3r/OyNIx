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
        GradientStop { position: 0; color: Core.startPageBackground === "plain" ? Theme.surface : Theme.surfaceContainerLow }
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
                Txt {
                    text: { var h = new Date().getHours(); return h < 5 ? qsTr("Good night") : h < 12 ? qsTr("Good morning") : h < 18 ? qsTr("Good afternoon") : qsTr("Good evening") }
                    font.family: Theme.displayFont; font.pixelSize: Theme.displaySize; font.weight: Font.Normal
                    color: Theme.surfaceFg
                    Layout.bottomMargin: 18
                }
                Txt { text: qsTr("Favorites"); font.pixelSize: 22; font.family: Theme.displayFont; color: Theme.surfaceFg }
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
                Txt { text: qsTr("Frequently Visited"); font.pixelSize: 22; font.family: Theme.displayFont }
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
                implicitHeight: Math.max(112, privacyRow.implicitHeight + 40)
                radius: Theme.shapeXL
                color: Theme.primaryContainer
                RowLayout {
                    id: privacyRow
                    anchors.fill: parent
                    anchors.margins: 20
                    spacing: 18
                    MShape {
                        size: 64; shape: "softBurst"; color: Theme.primary
                        Icon { anchors.centerIn: parent; name: "shieldCheck"; size: 30; color: Theme.primaryFg }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 4
                        Txt { text: qsTr("Privacy Report"); font.pixelSize: 18; font.weight: Font.DemiBold; color: Theme.primaryContainerFg }
                        Txt {
                            Layout.fillWidth: true
                            wrapMode: Text.WordWrap
                            color: Theme.primaryContainerFg
                            opacity: 0.85
                            text: Core.contentBlocker
                                  ? qsTr("%1 trackers and %2 ads blocked. OyNIx sends nothing about you anywhere.").arg(Math.round(Core.totalTrackers).toLocaleString()).arg(Math.round(Core.totalAds).toLocaleString())
                                  : qsTr("Content blocking is turned off. Turn it on in Settings › Privacy.")
                        }
                    }
                    Txt {
                        text: Math.round(Core.totalBlocked).toLocaleString(Qt.locale(), "f", 0)
                        font.pixelSize: 48; font.family: Theme.displayFont
                        color: Theme.primaryContainerFg
                    }
                }
                TapHandler { onTapped: page.win.openPrivacyReport() }
            }

            // Reading list
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 10
                visible: Core.startPageShowReadingList && readRep.count > 0
                Txt { text: qsTr("Reading List"); font.pixelSize: 22; font.family: Theme.displayFont }
                Repeater {
                    id: readRep
                    model: { Core.readingRevision; return JSON.parse(Core.readingList()).filter(function (r) { return !r.read }).slice(0, 5) }
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: 56
                        radius: rh.hovered ? Theme.shapeXL : Theme.shapeL
                        color: rh.hovered ? Theme.surfaceContainerHigh : Theme.surfaceContainerLow
                        Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
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

    // Favorite tile on a Material 3 Expressive shape; the shape is picked
    // from the site name so each site keeps its own silhouette.
    component Tile: AbstractButton {
        id: tile
        property var item
        property bool removable: false
        readonly property var shapes: ["cookie9", "flower", "clover4", "cookie6", "softBurst", "circle", "cookie4", "pentagon"]
        readonly property string shapeName: { var h = 0, s = tile.item.host || tile.item.url || ""; for (var i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0; return shapes[h % shapes.length] }
        width: 96
        height: 112
        hoverEnabled: true
        onClicked: page.win.navigate(item.url)
        contentItem: Column {
            spacing: 8
            MShape {
                anchors.horizontalCenter: parent.horizontalCenter
                size: 72
                shape: tile.hovered ? "circle" : tile.shapeName
                color: tile.hovered ? Theme.secondaryContainer : Theme.surfaceContainerHigh
                rotation: tile.hovered ? 20 : 0
                scale: tile.pressed ? 0.92 : 1
                Behavior on scale { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
                Behavior on rotation { SpringAnimation { spring: Theme.springDefault; damping: Theme.damping } }
                Favicon { anchors.centerIn: parent; size: 32; url: tile.item.url; title: tile.item.title; rotation: -parent.rotation }
            }
            Txt {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: tile.item.title || tile.item.host
                font.pixelSize: 12
                color: Theme.surfaceVariantFg
            }
        }
        IconButton {
            visible: tile.removable && tile.hovered
            x: 4; y: -2
            implicitWidth: 24; implicitHeight: 24
            glyph: "close"; iconSize: 12
            tonal: true
            onClicked: Core.removeBookmark(tile.item.id)
        }
    }
}
