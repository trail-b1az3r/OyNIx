import QtQuick
import OyNIx

// Site icon with a letter-tile fallback (coloured by host).
Item {
    id: f
    property url source
    property string title: ""
    property string url: ""
    property real size: 16
    property bool loading: false
    implicitWidth: size
    implicitHeight: size

    readonly property string host: Core.hostOf(url)
    readonly property string letter: {
        var h = host.replace(/^www\./, "")
        return (h.length ? h[0] : (title.length ? title[0] : "•")).toUpperCase()
    }
    function hue(s) { var x = 0; for (var i = 0; i < s.length; i++) x = (x * 31 + s.charCodeAt(i)) % 360; return x / 360 }

    Image {
        id: img
        anchors.fill: parent
        source: f.source
        sourceSize: Qt.size(f.size * 2, f.size * 2)
        smooth: true
        asynchronous: true
        visible: status === Image.Ready && !f.loading
    }
    Rectangle {
        anchors.fill: parent
        visible: img.status !== Image.Ready && !f.loading
        radius: f.size * 0.25
        color: f.url.indexOf("oynix://") === 0 ? Theme.accent : Qt.hsla(f.hue(f.host), 0.45, Theme.dark ? 0.42 : 0.55, 1)
        Text {
            anchors.centerIn: parent
            text: f.url.indexOf("oynix://") === 0 ? "" : f.letter
            color: "white"
            font.pixelSize: f.size * 0.62
            font.bold: true
        }
        Icon {
            anchors.centerIn: parent
            visible: f.url.indexOf("oynix://") === 0
            name: "star"
            size: f.size * 0.75
            color: "white"
            filled: true
            stroke: 0
        }
    }
    // Spinner while loading
    Item {
        anchors.fill: parent
        visible: f.loading
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: "transparent"
            border.width: 1.6
            border.color: Theme.separator
        }
        Rectangle {
            id: dot
            width: f.size * 0.3; height: width; radius: width / 2
            color: Theme.accent
            x: (f.size - width) / 2 + Math.cos(spin.angle) * (f.size / 2 - width / 2)
            y: (f.size - width) / 2 + Math.sin(spin.angle) * (f.size / 2 - width / 2)
        }
        QtObject { id: spin; property real angle: 0 }
        NumberAnimation on opacity { running: false }
        NumberAnimation {
            target: spin; property: "angle"; from: 0; to: Math.PI * 2
            duration: 900; loops: Animation.Infinite; running: f.loading && f.visible
        }
    }
}
