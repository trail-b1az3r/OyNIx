import QtQuick
import QtQuick.Layouts
import OyNIx

// A short vertical line between groups of toolbar buttons.
Rectangle {
    Layout.preferredWidth: 1
    Layout.preferredHeight: 18
    Layout.alignment: Qt.AlignVCenter
    Layout.leftMargin: 5
    Layout.rightMargin: 5
    color: Theme.separator
}
