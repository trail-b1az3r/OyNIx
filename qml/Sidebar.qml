import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Left sidebar: Tabs (vertical-tab layout), Bookmarks, Reading List, History.
Rectangle {
    id: sb
    property var win
    property string panel: "tabs"
    color: Theme.sidebar

    readonly property bool showTabs: Core.tabLayout === "vertical"
    onShowTabsChanged: if (!showTabs && panel === "tabs") panel = "bookmarks"
    Component.onCompleted: if (!showTabs && panel === "tabs") panel = "bookmarks"

    Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.separator }

    ColumnLayout {
        anchors.fill: parent
        anchors.rightMargin: 1
        spacing: 8

        // Segmented switcher
        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 10; Layout.rightMargin: 10; Layout.topMargin: 8
            implicitHeight: 28
            radius: 7
            color: Theme.field
            RowLayout {
                anchors.fill: parent
                anchors.margins: 2
                spacing: 2
                Repeater {
                    model: (sb.showTabs ? [{ id: "tabs", icon: "tabs", tip: qsTr("Tabs") }] : []).concat([
                        { id: "bookmarks", icon: "bookmark", tip: qsTr("Bookmarks") },
                        { id: "reading", icon: "glasses", tip: qsTr("Reading List") },
                        { id: "history", icon: "clock", tip: qsTr("History") }
                    ])
                    delegate: AbstractButton {
                        required property var modelData
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        hoverEnabled: true
                        background: Rectangle {
                            radius: 5
                            color: sb.panel === modelData.id ? Theme.tabActive : "transparent"
                        }
                        contentItem: Item { Icon { anchors.centerIn: parent; name: modelData.icon; size: 15; color: sb.panel === modelData.id ? Theme.text : Theme.textDim } }
                        ToolTip.visible: hovered; ToolTip.delay: 600; ToolTip.text: modelData.tip
                        onClicked: sb.panel = modelData.id
                    }
                }
            }
        }

        // Search box for library panels
        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 10; Layout.rightMargin: 10
            implicitHeight: 26
            radius: 6
            color: Theme.field
            visible: sb.panel !== "tabs"
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 8; anchors.rightMargin: 6
                Icon { name: "search"; size: 13; color: Theme.textDim }
                TextInput {
                    id: filter
                    Layout.fillWidth: true
                    color: Theme.text
                    font.pixelSize: 12
                    font.family: Theme.font
                    clip: true
                    Txt { visible: !filter.text && !filter.activeFocus; text: qsTr("Search"); color: Theme.textFaint; font.pixelSize: 12 }
                }
            }
        }

        Loader {
            Layout.fillWidth: true
            Layout.fillHeight: true
            sourceComponent: sb.panel === "tabs" ? tabsPanel : libraryPanel
        }
    }

    Component { id: tabsPanel; VerticalTabs { win: sb.win } }

    Component {
        id: libraryPanel
        ListView {
            id: lv
            clip: true
            spacing: 1
            boundsBehavior: Flickable.StopAtBounds
            section.property: sb.panel === "bookmarks" ? "folder" : (sb.panel === "history" ? "day" : "")
            section.delegate: Txt {
                required property string section
                width: lv.width
                height: 26
                leftPadding: 18
                text: sb.panel === "bookmarks" ? (section === "favorites" ? qsTr("Favorites") : (section === "" ? qsTr("Bookmarks") : section)) : section
                color: Theme.textDim
                font.pixelSize: 11
                font.weight: Font.DemiBold
            }
            model: {
                var q = filter.text.toLowerCase()
                var match = function (x) { return !q || (x.title || "").toLowerCase().indexOf(q) >= 0 || (x.url || "").toLowerCase().indexOf(q) >= 0 }
                if (sb.panel === "bookmarks") {
                    Core.bookmarksRevision
                    var b = JSON.parse(Core.bookmarks("*")).filter(match)
                    b.sort(function (a, c) { var o = function (f) { return f === "favorites" ? 0 : (f === "" ? 1 : 2) }; return o(a.folder) - o(c.folder) || a.folder.localeCompare(c.folder) })
                    return b
                }
                if (sb.panel === "reading") { Core.readingRevision; return JSON.parse(Core.readingList()).filter(match) }
                if (sb.panel === "history") {
                    Core.historyRevision
                    var h = JSON.parse(Core.history(q, 400))
                    var today = new Date(); today.setHours(0, 0, 0, 0)
                    for (var i = 0; i < h.length; i++) {
                        var d = new Date(h[i].lastVisit * 1000)
                        var diff = Math.floor((today - new Date(d.getFullYear(), d.getMonth(), d.getDate())) / 86400000)
                        h[i].day = diff <= 0 ? qsTr("Today") : diff === 1 ? qsTr("Yesterday") : d.toLocaleDateString(Qt.locale(), "dddd, d MMMM")
                    }
                    return h
                }
                return []
            }
            delegate: Rectangle {
                required property var modelData
                width: lv.width - 16
                x: 8
                height: 40
                radius: 7
                color: ih.hovered ? Theme.hover : "transparent"
                HoverHandler { id: ih }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onClicked: function (m) {
                        if (m.button === Qt.MiddleButton || (m.modifiers & Qt.ControlModifier)) sb.win.openTab(modelData.url, { background: true })
                        else sb.win.navigate(modelData.url)
                        if (sb.panel === "reading") Core.markRead(modelData.url, true)
                    }
                }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 10; anchors.rightMargin: 4
                    spacing: 9
                    Favicon { size: 16; url: modelData.url; title: modelData.title; opacity: modelData.read ? 0.5 : 1 }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Txt { Layout.fillWidth: true; text: modelData.title || modelData.url; font.pixelSize: 12; color: modelData.read ? Theme.textDim : Theme.text }
                        Txt { Layout.fillWidth: true; text: Core.displayUrl(modelData.url); font.pixelSize: 11; color: Theme.textDim }
                    }
                    IconButton {
                        visible: ih.hovered
                        implicitWidth: 22; implicitHeight: 22
                        glyph: "trash"; iconSize: 13; iconColor: Theme.textDim
                        onClicked: {
                            if (sb.panel === "bookmarks") Core.removeBookmark(modelData.id)
                            else if (sb.panel === "reading") Core.removeFromReadingList(modelData.url)
                            else Core.removeHistory(modelData.url)
                        }
                    }
                }
            }
            Txt {
                anchors.centerIn: parent
                visible: lv.count === 0
                color: Theme.textFaint
                text: sb.panel === "reading" ? qsTr("Pages you save to read later appear here") :
                      sb.panel === "history" ? qsTr("No history") : qsTr("No bookmarks")
            }
        }
    }
}
