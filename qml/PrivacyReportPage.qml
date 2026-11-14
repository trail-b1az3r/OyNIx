import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import OyNIx

// oynix://privacy — what the content blocker has stopped, which trackers
// follow you across the most sites, and what is doing the blocking.
Rectangle {
    id: page
    property var win
    color: Theme.surface

    readonly property var report: { Core.totalBlocked; page.win.app.filterGeneration; return JSON.parse(Core.privacyReport()) }
    readonly property int maxSites: report.top.length ? report.top[0].sites : 1
    function n(v) { return Math.round(v || 0).toLocaleString(Qt.locale(), "f", 0) }

    Timer { interval: 3000; running: page.visible; repeat: true; onTriggered: Core.refreshStats() }

    Flickable {
        anchors.fill: parent
        contentHeight: col.implicitHeight + 96
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        ColumnLayout {
            id: col
            width: Math.min(900, page.width - 48)
            x: (page.width - width) / 2
            y: 32
            spacing: 22

            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Txt { text: qsTr("Privacy Report"); font.family: Theme.displayFont; font.pixelSize: Theme.headlineSize + 4; Layout.fillWidth: true }
                PushButton { textOnly: true; text: qsTr("Reset Counts"); onClicked: Core.resetPrivacyStats() }
                PushButton { text: qsTr("Filter Lists…"); onClicked: page.win.app.openSettings("privacy") }
            }

            // Headline: the total, on the same shape as the Start Page card.
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: hero.implicitHeight + 48
                radius: Theme.shapeXL
                color: Core.contentBlocker ? Theme.primaryContainer : Theme.errorContainer
                RowLayout {
                    id: hero
                    anchors.fill: parent; anchors.margins: 24
                    spacing: 22
                    MShape {
                        size: 88; shape: Core.contentBlocker ? "softBurst" : "cookie4"
                        color: Core.contentBlocker ? Theme.primary : Theme.error
                        Icon { anchors.centerIn: parent; name: Core.contentBlocker ? "shieldCheck" : "shield"; size: 40; color: Core.contentBlocker ? Theme.primaryFg : Theme.errorFg }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 2
                        Txt {
                            text: page.n(page.report.blocked)
                            font.family: Theme.displayFont; font.pixelSize: 56
                            color: Core.contentBlocker ? Theme.primaryContainerFg : Theme.errorContainerFg
                        }
                        Txt {
                            Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                            color: Core.contentBlocker ? Theme.primaryContainerFg : Theme.errorContainerFg
                            text: Core.contentBlocker
                                  ? qsTr("requests blocked before they left your computer. None of this is reported anywhere: OyNIx has no telemetry.")
                                  : qsTr("Content blocking is off, so nothing is being blocked. Turn it on in Settings › Privacy.")
                        }
                    }
                }
            }

            // Split of the total.
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Repeater {
                    model: [
                        { k: page.report.trackers, t: qsTr("Trackers"), d: qsTr("Analytics and cross-site profiling"), s: "cookie9", c: Theme.tertiaryContainer, f: Theme.tertiaryContainerFg },
                        { k: page.report.ads, t: qsTr("Ads"), d: qsTr("Ad servers and ad frames"), s: "flower", c: Theme.secondaryContainer, f: Theme.secondaryContainerFg },
                        { k: Math.max(0, page.report.blocked - page.report.trackers - page.report.ads), t: qsTr("Other"), d: qsTr("Annoyances, cookie notices, malware hosts"), s: "clover4", c: Theme.surfaceContainerHigh, f: Theme.surfaceFg }
                    ]
                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        implicitHeight: 132
                        radius: Theme.shapeXL - 4
                        color: Theme.surfaceContainerLow
                        ColumnLayout {
                            anchors.fill: parent; anchors.margins: 18
                            spacing: 4
                            RowLayout {
                                spacing: 10
                                MShape { size: 28; shape: modelData.s; color: modelData.c }
                                Txt { text: modelData.t; font.weight: Font.Medium }
                            }
                            Item { Layout.fillHeight: true }
                            Txt { text: page.n(modelData.k); font.family: Theme.displayFont; font.pixelSize: 32 }
                            Txt { Layout.fillWidth: true; text: modelData.d; color: Theme.textDim; font.pixelSize: 12 }
                        }
                    }
                }
            }

            // Trackers seen on the most sites.
            Txt { text: qsTr("Trackers that followed you furthest"); font.family: Theme.displayFont; font.pixelSize: 20; Layout.topMargin: 6 }
            Txt {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                color: Theme.textDim
                text: qsTr("Each of these tried to load on several of the sites you visited, which is how one company can build a picture of your browsing. Counted since you last reset.")
            }
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: topCol.implicitHeight + 16
                radius: Theme.shapeXL - 4
                color: Theme.surfaceContainerLow
                ColumnLayout {
                    id: topCol
                    anchors.fill: parent; anchors.margins: 8
                    spacing: 2
                    Repeater {
                        model: page.report.top
                        delegate: Item {
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            implicitHeight: 48
                            Rectangle {
                                anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter
                                height: 40; radius: 20
                                width: Math.max(height, parent.width * modelData.sites / page.maxSites)
                                color: Theme.alpha(index === 0 ? Theme.tertiary : Theme.primary, index === 0 ? 0.22 : 0.12)
                            }
                            RowLayout {
                                anchors.fill: parent; anchors.leftMargin: 16; anchors.rightMargin: 16
                                Txt { text: modelData.domain; font.weight: Font.Medium; Layout.fillWidth: true }
                                Txt { text: modelData.sites === 1 ? qsTr("1 site") : qsTr("%1 sites").arg(modelData.sites); color: Theme.textDim }
                            }
                        }
                    }
                    Txt {
                        visible: page.report.top.length === 0
                        Layout.margins: 12
                        color: Theme.textDim
                        text: qsTr("Nothing yet. Trackers appear here as you browse.")
                    }
                }
            }

            // What's doing the work.
            Txt { text: qsTr("How it works"); font.family: Theme.displayFont; font.pixelSize: 20; Layout.topMargin: 6 }
            Txt {
                Layout.fillWidth: true; wrapMode: Text.WordWrap; elide: Text.ElideNone
                color: Theme.textDim
                text: page.report.loading
                      ? qsTr("Filter lists are loading…")
                      : qsTr("%1 network rules and %2 element-hiding rules from %3 filter lists. Requests are checked inside OyNIx before they are sent; lists update themselves in the background.")
                        .arg(page.n(page.report.rules)).arg(page.n(page.report.cosmetic)).arg(page.report.lists)
            }
        }
    }
}
