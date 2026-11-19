import QtQuick
import QtQuick.Controls.Basic
import OyNIx

// Material 3 surface for menus and popovers: surface container, large
// corner radius, soft elevation shadow, and a spring scale-in.
Popup {
    id: pop
    padding: 16
    // Keep menus and popovers fully inside the window.
    margins: 8
    modal: false
    focus: true
    // Popovers attached to the window (most of them) must close on any press
    // outside themselves; "outside the parent" would never happen.
    readonly property bool fillsWindow: parent !== null && parent.Window.window !== null
        && parent.width >= parent.Window.width - 2 && parent.height >= parent.Window.height - 2
    closePolicy: fillsWindow ? Popup.CloseOnEscape | Popup.CloseOnPressOutside
                             : Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
    transformOrigin: Item.Top

    // The press that closes a popover on its own button is followed by that
    // button's click; recentlyClosed() lets the button act as a toggle.
    property real closedAt: 0
    onAboutToHide: closedAt = Date.now()
    function recentlyClosed() { return Date.now() - closedAt < 300 }

    // Presses outside close the popover without being swallowed: an
    // invisible layer over the window watches each press (a passive point
    // handler doesn't take the event), so the click still reaches the
    // button underneath. Qt's own outside-press handling doesn't always
    // fire for popovers whose content comes from web views or extension
    // panels.
    Item {
        parent: pop.fillsWindow ? pop.parent : null
        anchors.fill: parent
        z: 100000
        visible: pop.opened
        PointHandler {
            acceptedButtons: Qt.AllButtons
            onActiveChanged: {
                if (!active || !pop.opened) return
                var p = point.scenePosition
                var r = pop.contentItem.mapToItem(null, -pop.leftPadding, -pop.topPadding)
                var inside = p.x >= r.x && p.x <= r.x + pop.width && p.y >= r.y && p.y <= r.y + pop.height
                if (!inside) pop.close()
            }
        }
    }

    // Escape also closes it when a web page inside has keyboard focus.
    Shortcut {
        sequence: "Escape"
        enabled: pop.opened
        context: Qt.WindowShortcut
        onActivated: pop.close()
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.animFast }
        SpringAnimation { property: "scale"; from: 0.85; to: 1; spring: Theme.springDefault; damping: Theme.damping }
    }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: 100 } }

    background: Item {
        Rectangle { visible: !Theme.neu && !Theme.hard; anchors.fill: parent; anchors.topMargin: 3; anchors.margins: -1; radius: Theme.shapeL + 2; color: Theme.shadow; opacity: 0.45 }
        // 5.4 Liquid Glass: refracts the page behind it (the window's
        // content; popovers live in the overlay, so they aren't in it).
        // Classic and neumorphic styles: a solid surface (raised in neumorphic)
        // instead of refracting glass.
        Rectangle {
            visible: !Theme.glass
            anchors.fill: parent
            radius: Theme.shapeL
            color: Theme.neu ? Theme.surface : Theme.popover
            border.width: Theme.edgeW(1)
            border.color: Theme.glassRim
            Neu { z: -1; anchors.fill: parent; radius: parent.radius; depth: 8 }
            HardShadow { radius: parent.radius }
        }
        LiquidGlass {
            visible: Theme.glass
            anchors.fill: parent
            radius: Theme.shapeL
            backdrop: Window.window ? Window.window.contentItem : null
        }
    }
}
