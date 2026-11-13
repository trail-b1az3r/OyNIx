import QtQuick
import OyNIx

// Material 3 Expressive loading indicator: a shape that rotates while
// morphing between lobed forms.
MShape {
    id: ls
    size: 24
    color: Theme.primary
    shape: "cookie7"
    property real phase: 0
    lobes: 7
    amplitude: 0.04 + 0.08 * (0.5 + 0.5 * Math.sin(phase * 2))
    rotation: phase * 57.2958
    NumberAnimation on phase { from: 0; to: Math.PI * 2; duration: 2400; loops: Animation.Infinite; running: ls.visible && !Core.lowPowerMode }
}
