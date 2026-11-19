import QtQuick
import OyNIx

Text {
    // Neumorphic and Graphite styles: large text (headings) is set in
    // OyNIx Neu with light, shadow and a little depth (NeuTextEffect).
    readonly property bool neuHeading: Theme.neu && font.pixelSize >= 18
    color: Theme.text
    font.family: neuHeading ? Theme.displayFont : Theme.font
    font.pixelSize: Theme.fontSize
    elide: Text.ElideRight
    verticalAlignment: Text.AlignVCenter
    renderType: neuHeading ? Text.QtRendering : Text.NativeRendering
    layer.enabled: neuHeading && visible && width > 0 && height > 0
    layer.effect: NeuTextEffect {}
}
