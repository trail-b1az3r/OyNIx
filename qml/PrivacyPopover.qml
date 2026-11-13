import QtQuick
import QtQuick.Layouts
import OyNIx

// Per-site blocker report shown from the shield button.
Popover {
    id: pp
    property var win
    property string host: win.currentTab ? win.currentTab.host : ""
    property var report: ({ blocked: 0, domains: [], enabled: true })
    width: 300
    onAboutToShow: report = JSON.parse(Core.siteReport(host))

    contentItem: ColumnLayout {
        spacing: 10
        RowLayout {
            spacing: 10
            Icon { name: "shieldCheck"; size: 26; color: pp.report.enabled ? Theme.accent : Theme.textFaint }
            ColumnLayout {
                spacing: 0
                Txt { text: pp.host || qsTr("Privacy"); font.weight: Font.DemiBold; Layout.maximumWidth: 220 }
                Txt {
                    text: pp.report.enabled ? (pp.report.blocked === 1 ? qsTr("1 tracker or ad blocked") : qsTr("%1 trackers and ads blocked").arg(pp.report.blocked)) : qsTr("Content blocker off for this site")
                    color: Theme.textDim; font.pixelSize: 12
                }
            }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.separator }
        Repeater {
            model: pp.report.domains.slice(0, 8)
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                Icon { name: "cookie"; size: 13; color: Theme.textDim }
                Txt { text: modelData; font.pixelSize: 12; Layout.fillWidth: true }
            }
        }
        Txt { visible: pp.report.domains.length === 0 && pp.report.enabled; text: qsTr("No trackers found on this page."); color: Theme.textDim; font.pixelSize: 12 }
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Block trackers and ads on this site"); Layout.fillWidth: true; font.pixelSize: 12 }
            Toggle {
                checked: pp.report.enabled
                enabled: Core.contentBlocker && pp.host !== ""
                onToggled: {
                    Core.setSiteSetting(pp.host, "blocker", checked ? "null" : "false")
                    pp.report = JSON.parse(Core.siteReport(pp.host))
                    if (pp.win.currentView) pp.win.currentView.reload()
                }
            }
        }
        MenuRow { text: qsTr("Privacy Report"); glyph: "list"; onClicked: { pp.close(); pp.win.openPrivacyReport() } }
    }
}
