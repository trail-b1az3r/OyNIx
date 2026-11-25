import QtQuick

// Behind the interface: slow, soft blooms of aurora light, seen through
// the glass bars and wherever the interface is see-through.
Item {
    id: root
    property bool behind: true
    property var ui
    property var window
    Repeater {
        model: 3
        delegate: Rectangle {
            required property int index
            width: root.width * 0.45; height: width; radius: width / 2
            color: index === 1 ? "#9b8cff" : "#7ee0c3"
            opacity: 0.10
            x: root.width * (0.1 + 0.3 * index)
            y: -height * 0.55
            SequentialAnimation on x {
                loops: Animation.Infinite
                NumberAnimation { to: root.width * (0.2 + 0.25 * index); duration: 14000 + 3000 * index; easing.type: Easing.InOutSine }
                NumberAnimation { to: root.width * (0.05 + 0.3 * index); duration: 14000 + 3000 * index; easing.type: Easing.InOutSine }
            }
        }
    }
}
