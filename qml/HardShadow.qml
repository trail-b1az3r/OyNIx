import QtQuick
import OyNIx

// A hypertheme's hard offset shadow (Neubrutalism): the part of a solid
// copy of the shape, shifted by the theme's shadow_x/shadow_y, that shows
// beside it. Only the exposed strips are drawn (a child can't be drawn
// under its parent's fill), so it never covers the control. Put it inside
// the shape and give it the shape's radius.
Item {
    id: hs
    property real offsetX: Theme.hardX
    property real offsetY: Theme.hardY
    property real radius: 0
    property color color: Theme.hardColor
    visible: Theme.hard
    readonly property real w: parent ? parent.width : 0
    readonly property real h: parent ? parent.height : 0
    // Beside the shape (right, or left for a negative x offset).
    Rectangle {
        visible: hs.offsetX !== 0
        x: hs.offsetX > 0 ? hs.w : hs.offsetX
        y: hs.offsetY + (hs.offsetY > 0 ? hs.radius : 0)
        width: Math.abs(hs.offsetX)
        height: Math.max(0, hs.h - (hs.offsetY > 0 ? hs.radius : 0))
        color: hs.color
        bottomRightRadius: hs.radius
    }
    // Below it (or above for a negative y offset), corner included.
    Rectangle {
        visible: hs.offsetY !== 0
        x: hs.offsetX + (hs.offsetX > 0 ? hs.radius : 0)
        y: hs.offsetY > 0 ? hs.h : hs.offsetY
        width: Math.max(0, hs.w - (hs.offsetX > 0 ? hs.radius : 0))
        height: Math.abs(hs.offsetY)
        color: hs.color
        bottomRightRadius: hs.radius
    }
}
