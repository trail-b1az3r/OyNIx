import QtQuick
import QtQuick.Shapes
import OyNIx

// Material 3 Expressive shapes: rounded polygons such as "cookie", "flower"
// and "clover", drawn as a smooth closed curve r(θ) = R·(1 + a·cos(nθ)).
// Children are centred on top, so this works as a shaped container.
Item {
    id: root
    property string shape: "circle"
    property real size: 48
    property color color: Theme.primaryContainer
    property color borderColor: "transparent"
    property real borderWidth: 0
    property real rotationOffset: 0
    // Optional morphing: amplitude override (used by LoadingShape).
    property real amplitude: -1
    property int lobes: -1
    default property alias content: holder.data

    implicitWidth: size
    implicitHeight: size

    readonly property var spec: ({
        "circle": [0, 0], "cookie4": [4, 0.07], "cookie6": [6, 0.06], "cookie7": [7, 0.055], "cookie9": [9, 0.045],
        "cookie12": [12, 0.035], "flower": [8, 0.1], "clover4": [4, 0.16], "clover8": [8, 0.12], "sunny": [8, 0.05],
        "burst": [12, 0.08], "softBurst": [10, 0.06], "pentagon": [5, 0.05], "pill": [2, 0.02]
    })

    function pathFor(n, a, rot) {
        var R = size / 2 / (1 + Math.abs(a)), cx = size / 2, cy = size / 2, steps = 120, d = ""
        for (var i = 0; i <= steps; i++) {
            var th = i / steps * Math.PI * 2
            var r = R * (1 + a * Math.cos(n * (th + rot)))
            var x = cx + r * Math.cos(th), y = cy + r * Math.sin(th)
            d += (i === 0 ? "M" : "L") + x.toFixed(2) + " " + y.toFixed(2) + " "
        }
        return d + "Z"
    }

    readonly property real _a: amplitude >= 0 ? amplitude : (spec[shape] || [0, 0])[1]
    readonly property int _n: lobes > 0 ? lobes : (spec[shape] || [0, 0])[0]

    Shape {
        anchors.fill: parent
        layer.enabled: true
        layer.samples: 4
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            fillColor: root.color
            strokeColor: root.borderWidth > 0 ? root.borderColor : "transparent"
            strokeWidth: root.borderWidth
            PathSvg { path: root.shape === "circle" && root.amplitude < 0 ? root.pathFor(0, 0, 0) : root.pathFor(root._n, root._a, root.rotationOffset) }
        }
    }
    Item { id: holder; anchors.fill: parent }
}
