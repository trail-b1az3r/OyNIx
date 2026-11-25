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
    // Mode (Core.theme): auto, light, dark, oled (pure black), midnight
    // (deep blue-black). Style (settings.uiStyle): normal (Liquid Glass),
    // classic (flat), neumorphic and graphite (soft raised and pressed-in
    // surfaces, Neu.qml). The palettes are a Z#++ program: assets/themes.zsp.
    // ("neumorphic" as a mode is from a 5.4 preview: graphite, dark.)
    readonly property string mode: Core.theme === "auto" ? (systemDark ? "dark" : "light")
                                 : Core.theme === "neumorphic" ? "dark" : Core.theme
    readonly property string style: { var s = JSON.parse(Core.settingsJson); return hyperOn ? "hyper" : Core.theme === "neumorphic" ? "graphite" : (s.uiStyle || "normal") }
    readonly property bool neu: style === "neumorphic" || style === "graphite"
    // A hypertheme without glass draws like Classic (solid surfaces), with
    // its own borders, corners and hard shadows on top.
    readonly property bool classic: style === "classic" || (hyperOn && !ht("glass", false))
    readonly property bool glass: style === "normal" || (hyperOn && ht("glass", false) === true)
    readonly property bool dark: mode !== "light"

    // ── 5.4.2: hyperthemes (.hypertheme, made in Theme Studio). The chosen
    // one's tokens for this mode: { name, tokens, panels, warnings }, or {}.
    readonly property var hyper: { Core.settingsJson; return JSON.parse(Core.hyperthemeJson(mode !== "light")) }
    readonly property bool hyperOn: hyper.tokens !== undefined
    function ht(k, d) { return hyperOn && hyper.tokens[k] !== undefined ? hyper.tokens[k] : d }
    // A panel's style key (toolbar, tab, card, button, field, popup).
    function panel(name, key, d) { var p = hyperOn && hyper.panels ? hyper.panels[name] : undefined; return p && p[key] !== undefined ? p[key] : d }
    // Borders: the theme's width (0 = each control's own) and colour.
    readonly property real edge: hyperOn ? ht("border_width", 0) : 0
    function edgeOr(w) { return edge > 0 ? edge : w }
    // A border that's drawn at all takes the theme's width.
    function edgeW(w) { return w > 0 && edge > 0 ? edge : w }
    readonly property color edgeColor: hyperOn ? ht("border", outline) : outlineVariant
    // Hard offset shadows (Neubrutalism): x, y and colour, under cards,
    // buttons, menus, fields and the selected tab.
    readonly property real hardX: hyperOn ? ht("shadow_x", 0) : 0
    readonly property real hardY: hyperOn ? ht("shadow_y", 0) : 0
    readonly property bool hard: hardX !== 0 || hardY !== 0
    readonly property color hardColor: hyperOn ? ht("shadow", "#000000") : "#000000"
    // Corners: any radius scaled by the theme; a pill keeps to the theme's
    // corner when it's squarer than a pill.
    function r(px) { return Math.round(px * cornerScale) }
    function pill(h) { return hyperOn ? Math.min(h / 2, ht("radius", h / 2)) : h / 2 }
    readonly property int heavy: hyperOn && ht("font_weight", 400) >= 600 ? Font.Bold : Font.Normal

    // ── 5.5: power hyperthemes (.phypertheme) ──
    // global.zxx's settings (glass, background_image, overlay_opacity, …).
    readonly property var hyperGlobal: hyperOn && hyper.global ? hyper.global : ({})
    function globalSetting(k, d) { var v = hyperGlobal[k]; return v === undefined ? d : v }
    function glassSetting(k, d) { var g = hyperGlobal.glass; return g && g[k] !== undefined ? g[k] : d }
    // global.zxx `window: { transparent: true, opacity, surfaces }`: the
    // window itself is see-through (the desktop shows behind OyNIx, and a
    // compositor such as Hyprland or KWin blurs it). Needs the window's
    // alpha channel, which OyNIx sets up at start-up for such a theme.
    readonly property var windowSetting: hyperGlobal.window || ({})
    readonly property bool wantsSeeThrough: hyperOn && windowSetting.transparent === true
    readonly property bool seeThrough: wantsSeeThrough && Core.windowAlpha()
    readonly property real windowOpacity: windowSetting.opacity !== undefined ? windowSetting.opacity : 0.2
    // A full page's background: clear in a see-through theme (the window's
    // own tint is the one layer over the desktop), the surface otherwise.
    readonly property color page: seeThrough ? "transparent" : surface
    readonly property real surfaceOpacity: windowSetting.surfaces !== undefined ? windowSetting.surfaces : 0.4
    // What the window draws: the theme's backdrop (background.zsg drawn, or
    // a picture), the global.zsg overlay, and the power theme's QML parts.
    readonly property string backdrop: hyperOn && hyper.background ? hyper.background : ""
    readonly property string overlay: hyperOn && hyper.overlay ? hyper.overlay : ""
    readonly property string globalQml: hyperOn && hyper.globalQml ? hyper.globalQml : ""
    readonly property string effectQml: hyperOn && hyper.effectQml ? hyper.effectQml : ""
    readonly property string formatQml: hyperOn && hyper.formatQml ? hyper.formatQml : ""
    // effect.qml as a layer effect for the whole interface, and
    // format-panel.qt's object (null: none, or it didn't load). Made when
    // the theme changes, not in a binding.
    property var effectComponent: null
    property var format: null
    function loadEffect() {
        var c = null
        if (effectQml) {
            c = Qt.createComponent(effectQml)
            if (c.status === Component.Error) { console.warn("effect.qml: " + c.errorString()); c = null }
        }
        effectComponent = c
    }
    function loadFormat() {
        var o = null
        if (formatQml) {
            var c = Qt.createComponent(formatQml)
            if (c.status === Component.Ready) o = c.createObject(t)
            else console.warn("format-panel.qt: " + c.errorString())
        }
        if (format) format.destroy()
        format = o
    }
    onEffectQmlChanged: loadEffect()
    onFormatQmlChanged: loadFormat()
    Component.onCompleted: { loadEffect(); loadFormat() }
    function fmt(k, d) { var o = format; return o && o[k] !== undefined && o[k] !== null ? o[k] : d }
    // styles.tsx: per-component styles, e.g. sx("button", "radius", 20).
    readonly property var styles: {
        if (!hyperOn || !hyper.stylesJs) return ({})
        try { return (new Function(hyper.stylesJs))() || ({}) }
        catch (e) { console.warn("styles.tsx: " + e); return ({}) }
    }
    function sx(component, key, d) {
        var o = styles[component]
        if (!o && styles["default"]) o = styles["default"][component]
        return o && o[key] !== undefined ? o[key] : d
    }

    // A browser style from an extension's theme.json: its accent, colour
    // role overrides for light and dark, fonts and corner scale.
    readonly property var custom: { Core.settingsJson; Core.extensionsRevision; return JSON.parse(Core.browserStyleJson()) }
    readonly property var scheme: {
        var s = JSON.parse(Core.themeScheme(custom.accent || Core.accent, mode, style))
        var o = dark ? custom.dark : custom.light
        if (o) for (var k in o) s[k] = o[k]
        if (hyperOn) {
            // The hypertheme's colours as Material roles.
            var t = hyper.tokens
            var bg = t.background, sf = t.surface || bg, alt = t.surface_alt || sf, fg = t.text, acc = t.accent
            var mix = function (a, b, x) { var p = Qt.color(a), q = Qt.color(b); return Qt.rgba(p.r + (q.r - p.r) * x, p.g + (q.g - p.g) * x, p.b + (q.b - p.b) * x, 1).toString() }
            s.surface = bg; s.surfaceDim = bg; s.surfaceBright = sf
            s.surfaceContainerLowest = sf; s.surfaceContainerLow = sf; s.surfaceContainer = sf
            s.surfaceContainerHigh = alt; s.surfaceContainerHighest = alt
            s.onSurface = fg; s.onSurfaceVariant = t.muted || fg
            s.outline = t.border || fg; s.outlineVariant = t.border || fg
            s.primary = acc; s.onPrimary = t.on_accent || fg
            s.primaryContainer = mix(acc, sf, 0.45); s.onPrimaryContainer = fg
            s.secondaryContainer = mix(acc, sf, 0.55); s.onSecondaryContainer = fg
            s.tertiaryContainer = alt; s.onTertiaryContainer = fg
            s.error = t.danger || s.error
        }
        return s
    }
    // The two neumorphic shadows: light from the top-left, dark below-right.
    readonly property color neuHighlight: scheme.neuHighlight || alpha("#ffffff", 0.11)
    readonly property color neuShadow: scheme.neuShadow || alpha("#000000", 0.5)
    // The extruded side of neumorphic headings (NeuTextEffect): between the text and the surface.
    readonly property color neuSide: Qt.rgba(text.r * 0.5 + surface.r * 0.5, text.g * 0.5 + surface.g * 0.5, text.b * 0.5 + surface.b * 0.5, 1)
    // OyNIx Neu: the round monoline typeface of the neumorphic styles'
    // headings (assets/fonts, made by scripts/make-neu-font.py).
    readonly property FontLoader neuFont: FontLoader { source: "qrc:/qt/qml/OyNIx/assets/fonts/OyNIxNeu-Regular.ttf" }
    readonly property string neuFamily: neuFont.status === FontLoader.Ready ? neuFont.name : ""
    readonly property real cornerScale: (hyperOn ? ht("radius", 12) / 12 : custom.corners !== undefined ? custom.corners : 1) * fmt("cornerScale", 1)
    function role(n) {
        var c = scheme[n] || "#ff00ff"
        // A see-through theme: surfaces let the desktop show through.
        if (seeThrough && n.indexOf("surface") === 0) return alpha(c, n === "surface" ? windowOpacity : surfaceOpacity)
        // Coloured containers (primaryContainer, …) are glass too, a little denser.
        if (seeThrough && n.indexOf("on") !== 0 && /Container$/.test(n)) return alpha(c, Math.min(1, surfaceOpacity + 0.2))
        return c
    }
    // An opaque version of a colour (web pages keep a solid background).
    function opaque(c) { c = Qt.color(c); return Qt.rgba(c.r, c.g, c.b, 1) }
    // `c` at opacity `a` (multiplied with c's own, so translucent stays translucent).
    function alpha(c, a) { c = Qt.color(c); return Qt.rgba(c.r, c.g, c.b, c.a * a) }

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
    readonly property color tabActive: panel("tab", "fill", secondaryContainer)
    readonly property color tabHover: hover
    readonly property color sidebar: surfaceContainerLow
    readonly property color field: panel("field", "fill", surfaceContainerHighest)
    readonly property color fieldFocus: surfaceContainerLowest
    readonly property color popover: panel("popup", "fill", surfaceContainer)
    readonly property color card: panel("card", "fill", surfaceContainerLow)
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
    // Neumorphic styles: solid surface with a faint rim (depth comes from Neu).
    // Classic: OyNIx's flat look — solid containers, no rim or specular line.
    readonly property color glassFill: neu ? surface : classic ? surfaceContainerHighest : alpha(glassSetting("tint", surfaceContainerHighest), glassSetting("opacity", dark ? 0.55 : 0.62))
    readonly property color glassFillStrong: neu ? surface : classic ? surfaceContainerHighest : alpha(surfaceContainerHighest, dark ? 0.78 : 0.86)
    readonly property color glassRim: edge > 0 ? edgeColor : neu ? alpha(dark ? "#ffffff" : "#000000", 0.04) : classic ? "transparent" : dark ? alpha("#ffffff", 0.14) : alpha(outlineVariant, 0.65)
    readonly property color glassSpecular: !glass ? "transparent" : dark ? alpha("#ffffff", 0.20) : alpha("#ffffff", 0.95)
    readonly property color glassShadow: alpha("#000000", dark ? 0.35 : 0.10)
    readonly property color glassBar: panel("toolbar", "fill", "") !== "" ? panel("toolbar", "fill", "") : neu ? surface : classic ? surfaceContainer : alpha(surfaceContainer, dark ? 0.88 : 0.86)
    readonly property color glassBarPrivate: alpha(Qt.tint(surfaceContainer, alpha(tertiary, dark ? 0.18 : 0.14)), 0.9)
    // Smallest click target for icon buttons (Apple / M3 guidance).
    readonly property int target: 40
    // Space between controls (format-panel.qt can change it).
    readonly property int spacing: fmt("spacing", 8)

    // ── Shape scale ──
    readonly property int shapeXS: Math.round(4 * cornerScale)
    readonly property int shapeS: Math.round(8 * cornerScale)
    readonly property int shapeM: Math.round(12 * cornerScale)
    readonly property int shapeL: Math.round(16 * cornerScale)
    readonly property int shapeXL: Math.round(28 * cornerScale)
    readonly property int radius: shapeM
    readonly property int radiusLarge: shapeXL

    // ── Metrics ──
    readonly property int toolbarHeight: fmt("toolbarHeight", 60)
    readonly property int tabHeight: fmt("tabHeight", 38)
    readonly property int sidebarWidth: fmt("sidebarWidth", 300)

    // ── Type scale (Roboto Flex / Google Sans Flex when installed) ──
    readonly property string font: (hyperOn && ht("font_family", "") ? ht("font_family", "") + ", " : "") + (custom.font ? custom.font + ", " : "") + "Roboto Flex, Google Sans Flex, Google Sans Text, Roboto, Inter, Noto Sans, Cantarell, sans-serif"
    // (Qt takes one family name, not a CSS list: the neumorphic styles name OyNIx Neu alone.)
    readonly property string displayFont: neu && neuFamily ? neuFamily : ((hyperOn && ht("font_family", "") ? ht("font_family", "") + ", " : "") + (custom.displayFont ? custom.displayFont + ", " : "") + "Google Sans Flex, Roboto Flex, Google Sans, Roboto, Inter, Noto Sans, sans-serif")
    readonly property int fontSize: fmt("fontSize", 14)
    readonly property int smallFont: fmt("smallFont", 12)
    readonly property int titleSize: fmt("titleSize", 16)
    readonly property int headlineSize: fmt("headlineSize", 28)
    readonly property int displaySize: fmt("displaySize", 45)

    // ── Motion: expressive springs ──
    readonly property int animFast: Core.lowPowerMode ? 0 : 150
    readonly property int anim: Core.lowPowerMode ? 0 : 300
    readonly property real springFast: Core.lowPowerMode ? 100 : 7.0
    readonly property real springDefault: Core.lowPowerMode ? 100 : 4.5
    readonly property real damping: 0.42

    readonly property var groupColors: [primary, secondary, tertiary, error]
}
