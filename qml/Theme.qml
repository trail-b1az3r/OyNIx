pragma Singleton
import QtQuick
import OyNIx

// Visual language: light, quiet chrome with hairline separators, rounded
// controls and a single accent colour — native-desktop feel, no heavy theming.
QtObject {
    id: t

    readonly property SystemPalette sys: SystemPalette { colorGroup: SystemPalette.Active }
    readonly property bool systemDark: (0.299 * sys.window.r + 0.587 * sys.window.g + 0.114 * sys.window.b) < 0.5
    readonly property bool dark: Core.theme === "dark" || (Core.theme === "auto" && systemDark)

    readonly property color accent: Core.accent
    readonly property color accentText: "#ffffff"

    // Surfaces
    readonly property color window:      dark ? "#1e1e20" : "#ffffff"
    readonly property color toolbar:     dark ? "#2a2a2d" : "#f3f3f4"
    readonly property color toolbarPrivate: dark ? "#2a2238" : "#ece6f7"
    readonly property color tabActive:   dark ? "#3a3a3e" : "#ffffff"
    readonly property color tabHover:    dark ? "#333336" : "#e7e7e9"
    readonly property color sidebar:     dark ? "#252527" : "#efeff1"
    readonly property color field:       dark ? "#1f1f21" : "#e4e4e7"
    readonly property color fieldFocus:  dark ? "#18181a" : "#ffffff"
    readonly property color popover:     dark ? "#2c2c2f" : "#fbfbfc"
    readonly property color card:        dark ? "#2b2b2e" : "#f5f5f7"
    readonly property color hover:       dark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(0, 0, 0, 0.06)
    readonly property color pressed:     dark ? Qt.rgba(1, 1, 1, 0.14) : Qt.rgba(0, 0, 0, 0.11)
    readonly property color separator:   dark ? Qt.rgba(1, 1, 1, 0.09) : Qt.rgba(0, 0, 0, 0.10)
    readonly property color shadow:      dark ? Qt.rgba(0, 0, 0, 0.55) : Qt.rgba(0, 0, 0, 0.18)
    readonly property color selection:   Qt.rgba(accent.r, accent.g, accent.b, dark ? 0.35 : 0.20)

    // Text
    readonly property color text:        dark ? "#ececee" : "#1d1d1f"
    readonly property color textDim:     dark ? "#a1a1a6" : "#6e6e73"
    readonly property color textFaint:   dark ? "#6c6c70" : "#a1a1a6"
    readonly property color danger:      "#ff453a"
    readonly property color success:     "#30d158"
    readonly property color warning:     "#ff9f0a"
    readonly property color purple:      "#bf5af2"

    // Window control dots
    readonly property color ctlClose: "#ff5f57"
    readonly property color ctlMin:   "#febc2e"
    readonly property color ctlMax:   "#28c840"

    // Metrics
    readonly property int radius: 7
    readonly property int radiusLarge: 12
    readonly property int toolbarHeight: 44
    readonly property int tabHeight: 30
    readonly property int sidebarWidth: 260
    readonly property string font: "Inter, -apple-system, 'SF Pro Text', 'Segoe UI', Cantarell, 'Noto Sans', sans-serif"
    readonly property int fontSize: 13
    readonly property int smallFont: 11

    readonly property int animFast: Core.lowPowerMode ? 0 : 120
    readonly property int anim: Core.lowPowerMode ? 0 : 200

    readonly property var groupColors: ["#8e8e93", "#0a84ff", "#30d158", "#ffd60a", "#ff9f0a", "#ff453a", "#bf5af2", "#64d2ff"]
}
