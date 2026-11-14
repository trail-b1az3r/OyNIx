import QtQuick
import QtQuick.Shapes
import OyNIx

// Vector icons drawn with Qt Quick Shapes so they stay crisp at any scale
// and follow the theme colour. All glyphs are on a 24×24 grid.
Item {
    id: root
    property string name: ""
    property color color: Theme.text
    property real size: 16
    property real stroke: name === "more" ? 2.8 : 1.7
    property bool filled: false

    implicitWidth: size
    implicitHeight: size

    readonly property var paths: ({
        "back": "M15 5l-7 7 7 7",
        "forward": "M9 5l7 7-7 7",
        "reload": "M19.5 12a7.5 7.5 0 1 1-2.2-5.3 M19.5 4.5v4.5h-4.5",
        "close": "M6.5 6.5l11 11M17.5 6.5l-11 11",
        "sidebar": "M5 5h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2z M9.5 5v14",
        "plus": "M12 5v14M5 12h14",
        "minus": "M5 12h14",
        "tabs": "M5 4h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M14 4h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z M5 13h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z M14 13h5a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1z",
        "shield": "M12 3l7 3v5.5c0 4.3-2.9 8-7 9.5-4.1-1.5-7-5.2-7-9.5V6z",
        "shieldCheck": "M12 3l7 3v5.5c0 4.3-2.9 8-7 9.5-4.1-1.5-7-5.2-7-9.5V6z M8.8 12l2.3 2.3 4.1-4.6",
        "lock": "M8 11V8a4 4 0 0 1 8 0v3 M6.5 11h11a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1z",
        "warning": "M12 4l9 16H3z M12 10v4.5 M12 17.2v.3",
        "star": "M12 3.8l2.5 5.1 5.6.8-4.05 3.95.96 5.6L12 16.6l-5 2.65.95-5.6L3.9 9.7l5.6-.8z",
        "bookmark": "M7 3.5h10a1 1 0 0 1 1 1V21l-6-4-6 4V4.5a1 1 0 0 1 1-1z",
        "glasses": "M3.5 14.5a3.5 3.5 0 1 0 7 0 3.5 3.5 0 0 0-7 0z M13.5 14.5a3.5 3.5 0 1 0 7 0 3.5 3.5 0 0 0-7 0z M10.5 14c1-.8 2-.8 3 0 M3.8 13L6 6.5h2 M20.2 13L18 6.5h-2",
        "clock": "M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17z M12 7.5V12l3 2",
        "download": "M12 4v11 M7.5 10.5L12 15l4.5-4.5 M5 19.5h14",
        "share": "M12 3.5v11 M8 7.5l4-4 4 4 M8.5 10.5H6v10h12v-10h-2.5",
        "sliders": "M4 7h9 M17 7h3 M15 5v4 M4 12h3 M11 12h9 M9 10v4 M4 17h11 M19 17h1 M17 15v4",
        "search": "M10.5 4.5a6 6 0 1 0 0 12 6 6 0 0 0 0-12z M15 15l4.5 4.5",
        "reader": "M5 6h14 M5 10h14 M5 14h14 M5 18h9",
        "more": "M6 12h.01 M12 12h.01 M18 12h.01",
        "pin": "M9 4h6 M10 4v5.5L7.5 13h9L14 9.5V4 M12 13v7",
        "speaker": "M4.5 9.5h3.5l4.5-4v13l-4.5-4H4.5z M15.5 9.2a4 4 0 0 1 0 5.6 M18 6.8a7.5 7.5 0 0 1 0 10.4",
        "mute": "M4.5 9.5h3.5l4.5-4v13l-4.5-4H4.5z M16 9.5l5 5 M21 9.5l-5 5",
        "mask": "M3 9.5c3.5-2.6 14.5-2.6 18 0-.6 5-4.6 6.5-7 3.5-1.1-1.3-2.9-1.3-4 0-2.4 3-6.4 1.5-7-3.5z",
        "focus": "M4 9V5a1 1 0 0 1 1-1h4 M20 9V5a1 1 0 0 0-1-1h-4 M4 15v4a1 1 0 0 0 1 1h4 M20 15v4a1 1 0 0 1-1 1h-4",
        "folder": "M3.5 7.5a2 2 0 0 1 2-2h3.8l2 2h7.2a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z",
        "trash": "M5 7h14 M10 7V4.5h4V7 M7 7l.9 12.5h8.2L17 7",
        "globe": "M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17z M3.5 12h17 M12 3.5c2.8 2.6 2.8 14.4 0 17 M12 3.5c-2.8 2.6-2.8 14.4 0 17",
        "chevronDown": "M7 10l5 5 5-5",
        "chevronUp": "M7 14l5-5 5 5",
        "chevronRight": "M10 7l5 5-5 5",
        "chevronLeft": "M14 7l-5 5 5 5",
        "sun": "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8z M12 3v2 M12 19v2 M3 12h2 M19 12h2 M5.6 5.6L7 7 M17 17l1.4 1.4 M5.6 18.4L7 17 M17 7l1.4-1.4",
        "moon": "M19.5 14.5A7.5 7.5 0 1 1 9.5 4.5a6 6 0 0 0 10 10z",
        "home": "M4 11l8-7 8 7 M6.5 9.5V20h11V9.5",
        "print": "M7 9V4h10v5 M5 9h14a1 1 0 0 1 1 1v6h-3 M7 16H4v-6 M7 13h10v7H7z",
        "code": "M9 8l-4 4 4 4 M15 8l4 4-4 4",
        "external": "M14 4h6v6 M20 4l-8.5 8.5 M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5",
        "check": "M5 12.5l4.5 4.5L19 7.5",
        "user": "M12 4a4 4 0 1 0 0 8 4 4 0 0 0 0-8z M4.5 20.5c1-3.8 4.2-5.5 7.5-5.5s6.5 1.7 7.5 5.5",
        "copy": "M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1z M5 15H4.5a.5.5 0 0 1-.5-.5V4a1 1 0 0 1 1-1h10.5a.5.5 0 0 1 .5.5V5",
        "window": "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M3 9h18",
        "leaf": "M5 19c0-8.5 5.5-14 15-14 0 9.5-5.5 15-14 15 M5 19l7.5-7.5",
        "sparkles": "M11 4l1.6 4.4L17 10l-4.4 1.6L11 16l-1.6-4.4L5 10l4.4-1.6z M18 14l.8 2.2L21 17l-2.2.8L18 20l-.8-2.2L15 17l2.2-.8z",
        "list": "M9 6h11 M9 12h11 M9 18h11 M4.5 6h.01 M4.5 12h.01 M4.5 18h.01",
        "usb": "M12 3v14 M12 3l-2 2.5h4z M12 17a1.8 1.8 0 1 0 0 3.6 1.8 1.8 0 0 0 0-3.6z M12 13.5l-4.5-3V8 M6.3 6h2.4v2H6.3z M12 11.5l4.5-2.5V7.2 M16.5 5.2a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4z",
        "puzzle": "M8 4.5h3v1.2a1.6 1.6 0 1 0 3.2 0V4.5H18v4h-1.2a1.6 1.6 0 1 0 0 3.2H18v7.8H8v-4H6.8a1.6 1.6 0 1 1 0-3.2H8z",
        "zoom": "M10.5 4.5a6 6 0 1 0 0 12 6 6 0 0 0 0-12z M15 15l4.5 4.5 M8 10.5h5 M10.5 8v5",
        "cookie": "M12 3.5a8.5 8.5 0 1 0 8.5 8.8 3 3 0 0 1-3.6-3.3 3 3 0 0 1-3.5-3.6 3 3 0 0 1-1.4-1.9z M8.5 10.5h.01 M9.5 15h.01 M14 14.5h.01",
        "camera": "M4 8h3l2-2.5h6L17 8h3a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z M12 10a3.2 3.2 0 1 0 0 6.4 3.2 3.2 0 0 0 0-6.4z",
        "mic": "M12 3.5a2.5 2.5 0 0 1 2.5 2.5v6a2.5 2.5 0 0 1-5 0V6A2.5 2.5 0 0 1 12 3.5z M6.5 11.5a5.5 5.5 0 0 0 11 0 M12 17v3.5",
        "location": "M12 21s-6.5-5.6-6.5-11a6.5 6.5 0 0 1 13 0c0 5.4-6.5 11-6.5 11z M12 7.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z",
        "bell": "M6.5 16.5V11a5.5 5.5 0 0 1 11 0v5.5l1.5 1.5H5z M10 20.5h4",
        "send": "M4 12l16-8-6 16-3-7z M11 13l9-9",
        "minimize": "M6 12h12",
        "maximize": "M6 6h12v12H6z",
        "restore": "M8 8h10v10H8z M6 16V6h10",
        "keyboard": "M4 6.5h16a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1z M7 10h.01 M10 10h.01 M13 10h.01 M16 10h.01 M8 14h8"
    })

    Shape {
        id: shape
        anchors.fill: parent
        layer.enabled: true
        layer.samples: 4
        layer.smooth: true
        visible: root.name !== ""

        ShapePath {
            scale: Qt.size(root.size / 24, root.size / 24)
            strokeColor: root.color
            strokeWidth: root.stroke
            fillColor: root.filled ? root.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
