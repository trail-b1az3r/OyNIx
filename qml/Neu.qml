import QtQuick
import QtQuick.Effects
import OyNIx

// Neumorphic depth for the Neumorphic theme: a soft light shadow up-left
// and a dark one down-right make a surface look raised out of the
// background; `inset` presses it in instead (dark edge on top, light
// edge below, inside the shape). Put it behind a surface with
// `z: -1; anchors.fill: parent`. Draws nothing unless Theme.neu.
Item {
    id: neu
    property real radius: 0
    property bool inset: false
    // How far the surface stands out, in pixels.
    property real depth: 5
    visible: Theme.neu

    RectangularShadow {
        visible: !neu.inset
        anchors.fill: parent
        radius: neu.radius
        offset.x: -neu.depth * 0.8
        offset.y: -neu.depth * 0.8
        blur: neu.depth * 2
        color: Theme.neuHighlight
    }
    RectangularShadow {
        visible: !neu.inset
        anchors.fill: parent
        radius: neu.radius
        offset.x: neu.depth
        offset.y: neu.depth
        blur: neu.depth * 2.2
        color: Theme.neuShadow
    }
    // Inset: a darker top-left rim and a lighter bottom-right rim, inside.
    Rectangle {
        visible: neu.inset
        anchors.fill: parent
        radius: neu.radius
        color: "transparent"
        border.width: 1
        border.color: Theme.alpha(Theme.neuShadow, 0.9)
        clip: true
    }
    Rectangle {
        visible: neu.inset
        anchors.fill: parent
        anchors.topMargin: 2
        anchors.leftMargin: 2
        radius: neu.radius
        color: "transparent"
        border.width: 1
        border.color: Theme.neuHighlight
        opacity: 0.8
    }
}
