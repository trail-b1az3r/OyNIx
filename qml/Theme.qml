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
    // "neumorphic": the built-in Neumorphic theme (graphite) — dark, monochrome,
    // with soft raised and pressed-in surfaces instead of glass (Neu.qml).
    readonly property bool neu: Core.theme === "neumorphic"
    readonly property bool dark: neu || Core.theme === "dark" || (Core.theme === "auto" && systemDark)

    // A browser style from an extension's theme.json: its accent, colour
    // role overrides for light and dark, fonts and corner scale.
    readonly property var custom: { Core.settingsJson; Core.extensionsRevision; return JSON.parse(Core.browserStyleJson()) }
    readonly property var scheme: {
        var s = JSON.parse(Core.materialScheme(custom.accent || Core.accent, dark))
        if (neu) for (var g in graphite) s[g] = graphite[g]
        var o = dark ? custom.dark : custom.light
        if (o) for (var k in o) s[k] = o[k]
        return s
    }
    // Neumorphic (graphite): every surface is (nearly) the same graphite grey,
    // so depth comes from shadows, not colour; one muted steel-blue accent.
    // Text keeps ≥ 4.5:1 contrast on the base (onSurface ~11:1, variant ~6.5:1).
    readonly property var graphite: ({
        surface: "#2b2d31", surfaceDim: "#242629", surfaceBright: "#36383d",
        surfaceContainerLowest: "#26282b", surfaceContainerLow: "#2b2d31", surfaceContainer: "#2b2d31",
        surfaceContainerHigh: "#2f3135", surfaceContainerHighest: "#33353a",
        onSurface: "#e3e5e8", onSurfaceVariant: "#a9adb3", outline: "#858a92", outlineVariant: "#3c3f45",
        primary: "#a9bdd3", onPrimary: "#1a2330", primaryContainer: "#3a4552", onPrimaryContainer: "#d6e3f1",
        secondary: "#b4bcc6", onSecondary: "#1f252c", secondaryContainer: "#34373c", onSecondaryContainer: "#e1e5ea",
        tertiary: "#c9b9a7", onTertiary: "#2c241b", tertiaryContainer: "#463d33", onTertiaryContainer: "#efe0cf",
        inverseSurface: "#e3e5e8", inverseOnSurface: "#2b2d31", inversePrimary: "#4a5d72"
    })
    // The two neumorphic shadows: light from the top-left, dark below-right.
    readonly property color neuHighlight: alpha("#ffffff", 0.11)
    readonly property color neuShadow: alpha("#000000", 0.5)
    readonly property real cornerScale: custom.corners !== undefined ? custom.corners : 1
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
    readonly property color toolbar: glassBar
    readonly property color toolbarPrivate: glassBarPrivate
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

    // ── 5.4: Liquid Glass on the classic layout. Controls are clear until
    // used, then a translucent glass capsule with a light rim and a bright
    // top edge (specular line); the address field and the selected tab are
    // glass at rest; bars are translucent with a specular top line. No
    // gradient backdrops. ──
    // In Neumorphic these are solid graphite with a faint rim: depth comes from Neu.
    readonly property color glassFill: neu ? surface : alpha(surfaceContainerHighest, dark ? 0.55 : 0.62)
    readonly property color glassFillStrong: neu ? surface : alpha(surfaceContainerHighest, dark ? 0.78 : 0.86)
    readonly property color glassRim: neu ? alpha("#ffffff", 0.04) : dark ? alpha("#ffffff", 0.14) : alpha(outlineVariant, 0.65)
    readonly property color glassSpecular: neu ? "transparent" : dark ? alpha("#ffffff", 0.20) : alpha("#ffffff", 0.95)
    readonly property color glassShadow: alpha("#000000", dark ? 0.35 : 0.10)
    readonly property color glassBar: neu ? surface : alpha(surfaceContainer, dark ? 0.88 : 0.86)
    readonly property color glassBarPrivate: alpha(Qt.tint(surfaceContainer, alpha(tertiary, dark ? 0.18 : 0.14)), 0.9)
    // Smallest click target for icon buttons (Apple / M3 guidance).
    readonly property int target: 40

    // ── Shape scale ──
    readonly property int shapeXS: Math.round(4 * cornerScale)
    readonly property int shapeS: Math.round(8 * cornerScale)
    readonly property int shapeM: Math.round(12 * cornerScale)
    readonly property int shapeL: Math.round(16 * cornerScale)
    readonly property int shapeXL: Math.round(28 * cornerScale)
    readonly property int radius: shapeM
    readonly property int radiusLarge: shapeXL

    // ── Metrics ──
    readonly property int toolbarHeight: 60
    readonly property int tabHeight: 38
    readonly property int sidebarWidth: 300

    // ── Type scale (Roboto Flex / Google Sans Flex when installed) ──
    readonly property string font: (custom.font ? custom.font + ", " : "") + "Roboto Flex, Google Sans Flex, Google Sans Text, Roboto, Inter, Noto Sans, Cantarell, sans-serif"
    readonly property string displayFont: (custom.displayFont ? custom.displayFont + ", " : "") + "Google Sans Flex, Roboto Flex, Google Sans, Roboto, Inter, Noto Sans, sans-serif"
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
