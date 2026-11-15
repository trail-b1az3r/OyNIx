import QtQuick
import QtQuick.Effects
import OyNIx

// Clips its children to a rounded rectangle. Plain `clip` is rectangular,
// so web pages and extension panels inside popovers would otherwise keep
// square corners inside the rounded frame.
Item {
    id: rc
    property real radius: Theme.shapeL
    default property alias content: inner.data

    Item {
        id: inner
        anchors.fill: parent
        layer.enabled: rc.width > 0 && rc.height > 0
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: mask
            maskThresholdMin: 0.5
            maskSpreadAtMin: 1.0
        }
    }
    Item {
        id: mask
        anchors.fill: parent
        layer.enabled: true
        visible: false
        Rectangle { anchors.fill: parent; radius: rc.radius; antialiasing: true }
    }
}
