import QtQuick
import OyNIx

// The neumorphic, slightly 2.5D look of headings in the Neumorphic and
// Graphite styles: a soft shadow down-right, an extruded side, a light rim
// up-left and a face lit from the top-left (assets/shaders/neutext.frag).
// Used as a Text's layer.effect (Txt does this for large text).
Item {
    id: fx
    property var source
    // Depth grows with the text: about 1/20 of its height.
    property real depthPx: Math.max(1.5, Math.min(5, height * 0.06))

    ShaderEffect {
        readonly property real pad: Math.ceil(fx.depthPx * 3 + 6)
        x: -pad
        y: -pad
        width: fx.width + 2 * pad
        height: fx.height + 2 * pad
        property var source: fx.source
        property size srcSize: Qt.size(fx.width, fx.height)
        property real depth: fx.depthPx
        property real blur: fx.depthPx * 1.6
        property real shadowDist: fx.depthPx * 1.5
        property color shadow: Theme.neuShadow
        property color side: Theme.neuSide
        property color rimLight: Theme.neuHighlight
        property real lift: Theme.dark ? 0.3 : 0.34
        fragmentShader: "qrc:/qt/qml/OyNIx/assets/shaders/neutext.frag.qsb"
    }
}
