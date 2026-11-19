import QtQuick
import OyNIx

// Liquid Glass with real refraction: captures the live content behind
// this item (`backdrop`, e.g. the window's content) and bends it at the
// rounded rim like a lens, with frost, a tint and a specular rim lit from
// the pointer. Use it as the background of glass that floats over content
// (popovers, menus); over a flat colour it would look like plain glass.
Item {
    id: glass
    // What's behind the glass. Must not contain the glass itself.
    property Item backdrop: null
    property real radius: Theme.shapeL
    property real bend: 14
    property real frost: 4
    // Strong enough that text on the glass stays legible (≥ 4.5:1) over
    // busy pages, while the page still shows through and bends at the rim.
    property color tint: Theme.alpha(Theme.surfaceContainer, Theme.dark ? 0.84 : 0.80)
    property color rim: Theme.dark ? Theme.alpha("#ffffff", 0.35) : Theme.alpha("#ffffff", 0.9)
    // Pointer in glass coordinates, for the moving highlight.
    property point pointer: Qt.point(-10000, -10000)

    HoverHandler {
        id: hover
        onPointChanged: glass.pointer = point.position
        onHoveredChanged: if (!hovered) glass.pointer = Qt.point(-10000, -10000)
    }

    ShaderEffectSource {
        id: src
        sourceItem: glass.backdrop
        live: true
        hideSource: false
        visible: false
        // The part of the backdrop under the glass (kept in step as it moves).
        sourceRect: {
            glass.x; glass.y; glass.width; glass.height; glass.backdrop
            if (!glass.backdrop) return Qt.rect(0, 0, 0, 0)
            var p = glass.mapToItem(glass.backdrop, 0, 0)
            return Qt.rect(p.x, p.y, glass.width, glass.height)
        }
    }

    // Fallback while there's nothing to refract: frosted tint with a rim.
    Rectangle {
        anchors.fill: parent
        visible: !glass.backdrop
        radius: glass.radius
        color: Theme.alpha(Theme.surfaceContainer, 0.94)
        border.width: Theme.edgeW(1)
        border.color: Theme.glassRim
    }

    ShaderEffect {
        anchors.fill: parent
        visible: glass.backdrop !== null
        property var source: src
        property size size: Qt.size(width, height)
        property real radius: glass.radius
        property real bend: glass.bend
        property real frost: glass.frost
        property point pointer: glass.pointer
        property color tint: glass.tint
        property color rim: glass.rim
        fragmentShader: "qrc:/qt/qml/OyNIx/assets/shaders/liquidglass.frag.qsb"
    }
}
