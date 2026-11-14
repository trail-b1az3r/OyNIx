import QtQuick
import QtQuick.Layouts
import QtQuick.Controls.Basic
import OyNIx

// "Website settings" for the current site (from the lock icon).
Popover {
    id: sp
    property var win
    readonly property var tab: win.currentTab
    property string host: tab ? tab.host : ""
    property var s: ({})
    width: 320
    onAboutToShow: s = JSON.parse(Core.siteSettings(host))

    function set(key, value) {
        Core.setSiteSetting(host, key, value === null ? "null" : JSON.stringify(value))
        s = JSON.parse(Core.siteSettings(host))
    }

    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            spacing: 10
            Favicon { size: 22; url: sp.tab ? sp.tab.model.url : ""; source: sp.tab ? sp.tab.model.icon : "" }
            ColumnLayout {
                spacing: 0
                Txt { text: sp.host; font.weight: Font.DemiBold; Layout.maximumWidth: 240 }
                Txt {
                    text: sp.win.privateMode ? qsTr("Private window") : (Core.isSecure(sp.tab ? sp.tab.model.url : "") ? qsTr("Connection is secure") : qsTr("Connection is not secure"))
                    color: Core.isSecure(sp.tab ? sp.tab.model.url : "") ? Theme.success : Theme.warning
                    font.pixelSize: 12
                }
            }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator }

        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Page Zoom"); Layout.fillWidth: true }
            IconButton { glyph: "minus"; onClicked: sp.tab.setZoom(sp.tab.model.zoom - 0.1) }
            Txt { text: Math.round((sp.tab ? sp.tab.model.zoom : 1) * 100) + "%"; Layout.preferredWidth: 42; horizontalAlignment: Text.AlignHCenter }
            IconButton { glyph: "plus"; onClicked: sp.tab.setZoom(sp.tab.model.zoom + 0.1) }
        }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Content Blocker"); Layout.fillWidth: true }
            Toggle { checked: sp.s.blocker !== false; enabled: Core.contentBlocker; onToggled: { sp.set("blocker", checked ? null : false); sp.win.currentView && sp.win.currentView.reload() } }
        }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("JavaScript"); Layout.fillWidth: true }
            Toggle { checked: sp.s.javascript !== undefined ? sp.s.javascript : Core.javascript; onToggled: { sp.set("javascript", checked === Core.javascript ? null : checked); sp.win.currentView && sp.win.currentView.reload() } }
        }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Allow Pop-up Windows"); Layout.fillWidth: true }
            Toggle { checked: sp.s.popups === true; onToggled: sp.set("popups", checked ? true : null) }
        }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Dark Mode"); Layout.fillWidth: true }
            Toggle {
                checked: sp.s.forceDark !== undefined ? sp.s.forceDark : sp.win.app.websitesDark
                onToggled: sp.set("forceDark", checked === sp.win.app.websitesDark ? null : checked)
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Open in Reader View"); Layout.fillWidth: true }
            Toggle { checked: sp.s.reader === true; onToggled: sp.set("reader", checked ? true : null) }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator }
        MenuRow { text: qsTr("Copy Link"); glyph: "copy"; onClicked: { sp.win.copyText(sp.tab.model.url); sp.close() } }
        MenuRow { text: qsTr("Reset Settings for This Website"); glyph: "trash"; onClicked: { ["zoom", "blocker", "javascript", "popups", "reader", "forceDark"].forEach(function (k) { Core.setSiteSetting(sp.host, k, "null") }); sp.s = {}; sp.tab.applySiteSettings() } }
    }
}
