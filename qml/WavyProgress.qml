import QtQuick
import QtQuick.Shapes
import OyNIx

// Material 3 Expressive linear progress: a wavy active track with a flat
// remaining track. `value` 0..1; when indeterminate the wave travels.
Item {
    id: wp
    property real value: 0
    property color color: Theme.primary
    property color trackColor: Theme.secondaryContainer
    property real amplitude: 2.2
    property real wavelength: 18
    property real phase: 0
    implicitHeight: 8

    NumberAnimation on phase { from: 0; to: 1; duration: 900; loops: Animation.Infinite; running: wp.visible && !Core.lowPowerMode }

    readonly property real done: Math.max(0, Math.min(1, value)) * width
    function wave(w) {
        var d = "", mid = height / 2
        for (var x = 0; x <= w; x += 2) {
            var y = mid + amplitude * Math.sin((x / wavelength + phase) * Math.PI * 2)
            d += (x === 0 ? "M" : "L") + x.toFixed(1) + " " + y.toFixed(2) + " "
        }
        return d
    }
    Shape {
        anchors.fill: parent
        layer.enabled: true; layer.samples: 4
        ShapePath {
            strokeColor: wp.color; strokeWidth: 3.5; fillColor: "transparent"; capStyle: ShapePath.RoundCap
            PathSvg { path: wp.done > 2 ? wp.wave(wp.done) : "" }
        }
        ShapePath {
            strokeColor: wp.trackColor; strokeWidth: 3.5; fillColor: "transparent"; capStyle: ShapePath.RoundCap
            PathSvg { path: wp.width - wp.done > 8 ? "M" + (wp.done + 6) + " " + wp.height / 2 + " L" + wp.width + " " + wp.height / 2 : "" }
        }
    }
}
