import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// Full-page history (oynix://history): grouped by day, searchable, with
// per-entry and per-site removal and a "Clear Browsing Data" dialog.
Rectangle {
    id: page
    property var win
    color: Theme.surface

    readonly property var entries: {
        Core.historyRevision
        var h = JSON.parse(Core.history(search.text, 3000))
        var today = new Date(); today.setHours(0, 0, 0, 0)
        for (var i = 0; i < h.length; i++) {
            var d = new Date(h[i].lastVisit * 1000)
            var day = new Date(d.getFullYear(), d.getMonth(), d.getDate())
            var diff = Math.round((today - day) / 86400000)
            h[i].day = diff <= 0 ? qsTr("Today") : diff === 1 ? qsTr("Yesterday") : d.toLocaleDateString(Qt.locale(), "dddd d MMMM yyyy")
        }
        return h
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: 32
        anchors.leftMargin: Math.max(24, (parent.width - 900) / 2)
        anchors.rightMargin: anchors.leftMargin
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            spacing: 12
            Txt { text: qsTr("History"); font.family: Theme.displayFont; font.pixelSize: Theme.headlineSize + 4; Layout.fillWidth: true }
            PushButton { text: qsTr("Clear Browsing Data…"); onClicked: clearDialog.open() }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 52
            radius: 26
            color: Theme.surfaceContainerHigh
            RowLayout {
                anchors.fill: parent; anchors.leftMargin: 18; anchors.rightMargin: 12
                spacing: 12
                Icon { name: "search"; size: 20; color: Theme.surfaceVariantFg }
                TextInput {
                    id: search
                    Layout.fillWidth: true
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 15; clip: true
                    selectionColor: Theme.selection
                    Txt { visible: !search.text; text: qsTr("Search history"); color: Theme.textFaint; font.pixelSize: 15 }
                }
                IconButton { visible: search.text !== ""; glyph: "close"; onClicked: search.text = "" }
            }
        }

        ListView {
            id: lv
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 2
            model: page.entries
            ScrollBar.vertical: ScrollBar {}
            section.property: "day"
            section.delegate: Txt {
                required property string section
                width: lv.width; height: 44
                verticalAlignment: Text.AlignBottom
                bottomPadding: 6
                leftPadding: 8
                text: section
                color: Theme.primary
                font.pixelSize: 14; font.weight: Font.DemiBold
            }
            delegate: Rectangle {
                id: row
                required property var modelData
                width: lv.width
                height: 52
                radius: h.hovered ? Theme.shapeL : Theme.shapeS
                color: h.hovered ? Theme.surfaceContainerHigh : "transparent"
                Behavior on radius { SpringAnimation { spring: Theme.springFast; damping: Theme.damping } }
                HoverHandler { id: h }
                MouseArea {
                    anchors.fill: parent
                    acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                    onClicked: function (m) {
                        if (m.button === Qt.MiddleButton || (m.modifiers & Qt.ControlModifier)) page.win.openTab(row.modelData.url, { background: true, afterCurrent: true })
                        else page.win.navigate(row.modelData.url)
                    }
                }
                RowLayout {
                    anchors.fill: parent; anchors.leftMargin: 12; anchors.rightMargin: 6
                    spacing: 14
                    Txt { text: new Date(row.modelData.lastVisit * 1000).toLocaleTimeString(Qt.locale(), "HH:mm"); color: Theme.textDim; font.pixelSize: 12; Layout.preferredWidth: 44 }
                    Favicon { size: 20; url: row.modelData.url; title: row.modelData.title }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Txt { Layout.fillWidth: true; text: row.modelData.title || Core.prettyUrl(row.modelData.url) }
                        Txt { Layout.fillWidth: true; text: Core.prettyUrl(row.modelData.url); color: Theme.textDim; font.pixelSize: 12 }
                    }
                    Txt { visible: row.modelData.visits > 1; text: qsTr("%1 visits").arg(row.modelData.visits); color: Theme.textFaint; font.pixelSize: 11 }
                    IconButton { opacity: h.hovered ? 1 : 0; glyph: "globe"; tip: qsTr("Remove all visits to %1").arg(row.modelData.host)
                        onClicked: { var all = JSON.parse(Core.history(row.modelData.host, 5000)); for (var i = 0; i < all.length; i++) if (all[i].host === row.modelData.host) Core.removeHistory(all[i].url) } }
                    IconButton { opacity: h.hovered ? 1 : 0; glyph: "trash"; tip: qsTr("Remove from history"); onClicked: Core.removeHistory(row.modelData.url) }
                }
            }
            ColumnLayout {
                anchors.centerIn: parent
                visible: lv.count === 0
                spacing: 12
                MShape { Layout.alignment: Qt.AlignHCenter; shape: "cookie9"; size: 72; color: Theme.secondaryContainer
                    Icon { anchors.centerIn: parent; name: "clock"; size: 32; color: Theme.secondaryContainerFg } }
                Txt { Layout.alignment: Qt.AlignHCenter; text: search.text ? qsTr("No matches") : qsTr("Pages you visit will appear here"); color: Theme.textDim }
            }
        }
    }

    // ── Clear Browsing Data ──
    Popup {
        id: clearDialog
        anchors.centerIn: parent
        width: Math.min(440, page.width - 48)
        padding: 24
        modal: true
        dim: true
        property string range: "hour"
        background: Rectangle { radius: Theme.shapeXL; color: Theme.surfaceContainerHigh }
        contentItem: ColumnLayout {
            spacing: 16
            Txt { text: qsTr("Clear browsing data"); font.family: Theme.displayFont; font.pixelSize: 22 }
            Txt { text: qsTr("Time range"); font.weight: Font.DemiBold }
            Segmented {
                Layout.fillWidth: true
                options: [{ v: "hour", t: qsTr("Hour") }, { v: "day", t: qsTr("Day") }, { v: "week", t: qsTr("Week") }, { v: "all", t: qsTr("All") }]
                value: clearDialog.range
                onPicked: function (v) { clearDialog.range = v }
            }
            RowLayout { Layout.fillWidth: true; Txt { text: qsTr("Browsing history"); Layout.fillWidth: true } Toggle { id: cHistory; checked: true } }
            RowLayout {
                Layout.fillWidth: true
                ColumnLayout { Layout.fillWidth: true; spacing: 0
                    Txt { text: qsTr("Cookies and site data") }
                    Txt { text: qsTr("Signs you out of most sites (always clears all time)"); color: Theme.textDim; font.pixelSize: 12 } }
                Toggle { id: cCookies; checked: false }
            }
            RowLayout { Layout.fillWidth: true; Txt { text: qsTr("Cached images and files"); Layout.fillWidth: true } Toggle { id: cCache; checked: true } }
            RowLayout {
                Layout.fillWidth: true
                Item { Layout.fillWidth: true }
                PushButton { textOnly: true; text: qsTr("Cancel"); onClicked: clearDialog.close() }
                PushButton {
                    primary: true; text: qsTr("Clear Data")
                    onClicked: {
                        var secs = { hour: 3600, day: 86400, week: 7 * 86400, all: 0 }[clearDialog.range]
                        if (cHistory.checked) Core.clearHistory(secs)
                        if (cCookies.checked) Core.clearCookies(page.win.app.defaultProfile)
                        if (cCache.checked) page.win.app.defaultProfile.clearHttpCache()
                        clearDialog.close()
                    }
                }
            }
        }
    }
}
