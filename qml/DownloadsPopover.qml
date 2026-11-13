import QtQuick
import QtQuick.Layouts
import QtQuick.Controls.Basic
import QtWebEngine
import OyNIx

Popover {
    id: dp
    property var app
    width: 340
    contentItem: ColumnLayout {
        spacing: 6
        RowLayout {
            Layout.fillWidth: true
            Txt { text: qsTr("Downloads"); font.weight: Font.DemiBold; Layout.fillWidth: true }
            PushButton { text: qsTr("Clear"); onClicked: dp.app.clearFinishedDownloads() }
        }
        Txt { visible: dp.app.downloads.length === 0; text: qsTr("No downloads"); color: Theme.textDim }
        Repeater {
            model: dp.app.downloads
            delegate: RowLayout {
                id: d
                required property var modelData
                readonly property var dl: modelData
                readonly property bool done: dl.state === WebEngineDownloadRequest.DownloadCompleted
                Layout.fillWidth: true
                spacing: 10
                Icon { name: "download"; size: 22; color: d.done ? Theme.accent : Theme.textDim }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3
                    Txt { text: d.dl.downloadFileName; Layout.fillWidth: true; font.pixelSize: 12 }
                    Rectangle {
                        Layout.fillWidth: true; height: 4; radius: 2; color: Theme.field
                        visible: d.dl.state === WebEngineDownloadRequest.DownloadInProgress
                        Rectangle { height: parent.height; radius: 2; color: Theme.accent; width: parent.width * (d.dl.totalBytes > 0 ? d.dl.receivedBytes / d.dl.totalBytes : 0) }
                    }
                    Txt {
                        font.pixelSize: 11; color: Theme.textDim
                        text: d.done ? Core.formatBytes(d.dl.totalBytes)
                              : d.dl.state === WebEngineDownloadRequest.DownloadInProgress ? Core.formatBytes(d.dl.receivedBytes) + " / " + (d.dl.totalBytes > 0 ? Core.formatBytes(d.dl.totalBytes) : "?")
                              : d.dl.state === WebEngineDownloadRequest.DownloadCancelled ? qsTr("Cancelled")
                              : d.dl.state === WebEngineDownloadRequest.DownloadInterrupted ? qsTr("Failed: %1").arg(d.dl.interruptReasonString) : ""
                    }
                }
                IconButton {
                    visible: d.dl.state === WebEngineDownloadRequest.DownloadInProgress
                    glyph: d.dl.isPaused ? "download" : "minus"
                    tip: d.dl.isPaused ? qsTr("Resume") : qsTr("Pause")
                    onClicked: d.dl.isPaused ? d.dl.resume() : d.dl.pause()
                }
                IconButton {
                    visible: d.done
                    glyph: "external"; tip: qsTr("Open")
                    onClicked: Core.openExternally(d.dl.downloadDirectory + "/" + d.dl.downloadFileName)
                }
                IconButton {
                    glyph: d.done ? "folder" : "close"
                    tip: d.done ? qsTr("Show in Folder") : qsTr("Cancel")
                    onClicked: {
                        if (d.done) Core.openExternally(d.dl.downloadDirectory)
                        else d.dl.cancel()
                    }
                }
            }
        }
    }
}
