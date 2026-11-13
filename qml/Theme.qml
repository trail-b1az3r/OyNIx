pragma Singleton
import QtQuick
import OyNIx

// Material 3 Expressive design tokens: dynamic colour roles generated from
// the accent (seed) colour, the M3 shape and type scales, and spring motion.
// Legacy token names (window, toolbar, field, …) map onto M3 roles so every
// component follows the same scheme.
QtObject {
    id: t

    readonly property SystemPalette sys: SystemPalette { colorGroup: SystemPalette.Active }
    readonly property bool systemDark: (0.299 * sys.window.r + 0.587 * sys.window.g + 0.114 * sys.window.b) < 0.5
    readonly property bool dark: Core.theme === "dark" || (Core.theme === "auto" && systemDark)

    readonly property var scheme: JSON.parse(Core.materialScheme(Core.accent, dark))
    function role(n) { return scheme[n] || "#ff00ff" }
    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }

    // ── Material 3 colour roles ──
    readonly property color primary: role("primary")
    readonly property color primaryFg: role("onPrimary")
    readonly property color primaryContainer: role("primaryContainer")
    readonly property color primaryContainerFg: role("onPrimaryContainer")
    readonly property color secondary: role("secondary")
    readonly property color secondaryFg: role("onSecondary")
    readonly property color secondaryContainer: role("secondaryContainer")
    readonly property color secondaryContainerFg: role("onSecondaryContainer")
    readonly property color tertiary: role("tertiary")
    readonly property color tertiaryFg: role("onTertiary")
    readonly property color tertiaryContainer: role("tertiaryContainer")
    readonly property color tertiaryContainerFg: role("onTertiaryContainer")
    readonly property color error: role("error")
    readonly property color errorFg: role("onError")
    readonly property color errorContainer: role("errorContainer")
    readonly property color errorContainerFg: role("onErrorContainer")
    readonly property color surface: role("surface")
    readonly property color surfaceDim: role("surfaceDim")
    readonly property color surfaceBright: role("surfaceBright")
    readonly property color surfaceContainerLowest: role("surfaceContainerLowest")
    readonly property color surfaceContainerLow: role("surfaceContainerLow")
    readonly property color surfaceContainer: role("surfaceContainer")
    readonly property color surfaceContainerHigh: role("surfaceContainerHigh")
    readonly property color surfaceContainerHighest: role("surfaceContainerHighest")
    readonly property color surfaceFg: role("onSurface")
    readonly property color surfaceVariantFg: role("onSurfaceVariant")
    readonly property color outline: role("outline")
    readonly property color outlineVariant: role("outlineVariant")
    readonly property color inverseSurface: role("inverseSurface")
    readonly property color inverseOnSurface: role("inverseOnSurface")
    readonly property color inversePrimary: role("inversePrimary")

    // State layers (M3: hover 8 %, focus/pressed 10 %)
    readonly property color hover: alpha(surfaceFg, 0.08)
    readonly property color pressed: alpha(surfaceFg, 0.10)
    readonly property color selection: alpha(primary, 0.28)

    // ── Legacy aliases used across the UI ──
    readonly property color accent: primary
    readonly property color accentText: primaryFg
    readonly property color window: surface
    readonly property color toolbar: surfaceContainer
    readonly property color toolbarPrivate: dark ? Qt.tint(surfaceContainer, alpha(tertiary, 0.18)) : Qt.tint(surfaceContainer, alpha(tertiary, 0.14))
    readonly property color tabActive: secondaryContainer
    readonly property color tabHover: hover
    readonly property color sidebar: surfaceContainerLow
    readonly property color field: surfaceContainerHighest
    readonly property color fieldFocus: surfaceContainerLowest
    readonly property color popover: surfaceContainer
    readonly property color card: surfaceContainerLow
    readonly property color separator: outlineVariant
    readonly property color shadow: alpha("#000000", dark ? 0.5 : 0.18)
    readonly property color text: surfaceFg
    readonly property color textDim: surfaceVariantFg
    readonly property color textFaint: outline
    readonly property color danger: error
    readonly property color success: dark ? "#7bdc8f" : "#1b7f3b"
    readonly property color warning: dark ? "#ffb95c" : "#a05a00"
    readonly property color purple: tertiary

    // ── Shape scale ──
    readonly property int shapeXS: 4
    readonly property int shapeS: 8
    readonly property int shapeM: 12
    readonly property int shapeL: 16
    readonly property int shapeXL: 28
    readonly property int radius: shapeM
    readonly property int radiusLarge: shapeXL

    // ── Metrics ──
    readonly property int toolbarHeight: 60
    readonly property int tabHeight: 38
    readonly property int sidebarWidth: 300

    // ── Type scale (Roboto Flex / Google Sans Flex when installed) ──
    readonly property string font: "Roboto Flex, Google Sans Flex, Google Sans Text, Roboto, Inter, Noto Sans, Cantarell, sans-serif"
    readonly property string displayFont: "Google Sans Flex, Roboto Flex, Google Sans, Roboto, Inter, Noto Sans, sans-serif"
    readonly property int fontSize: 14
    readonly property int smallFont: 12
    readonly property int titleSize: 16
    readonly property int headlineSize: 28
    readonly property int displaySize: 45

    // ── Motion: expressive springs ──
    readonly property int animFast: Core.lowPowerMode ? 0 : 150
    readonly property int anim: Core.lowPowerMode ? 0 : 300
    readonly property real springFast: Core.lowPowerMode ? 100 : 7.0
    readonly property real springDefault: Core.lowPowerMode ? 100 : 4.5
    readonly property real damping: 0.42

    readonly property var groupColors: [primary, secondary, tertiary, error]
}
