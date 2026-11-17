pragma Singleton
import QtQuick

// Design tokens, "Spectral" (docs/DESIGN.md): a deep near-black (or cool frost) canvas lit by the colours
// your lights show, glass cards with a lit top rim, and one accent, the desktop's, drawn as a short
// gradient that glows. The chrome stays neutral, so the saturated colour on screen is still the light
// you chose.
QtObject {
    id: t

    property bool dark: true
    property color systemAccent: "#3584e4"     // GNOME accent-color
    property bool reduceMotion: false
    // Liquid Glass theme tokens (backend.LiquidGlass); null or inactive keeps the audius look.
    property var glass: null
    readonly property bool glassy: glass !== null && glass.active
    // Set by the window: the item that glass surfaces blur, or null where blur is unavailable.
    property Item backdrop: null
    // Set by the window: where its visible frame sits, so overlays dim only the frame.
    property real windowMargin: 0
    property real windowRadius: 0

    function pick(role, fallback) {
        if (!glassy)
            return fallback
        const v = glass.color(dark, role, "")
        return v === "" ? fallback : v
    }
    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }
    // Turns a colour round the hue wheel by d (0..1), keeping saturation and lightness.
    function hue(c, d) {
        c = Qt.color(c)
        if (c.hslSaturation < 0.05)
            return c
        return Qt.hsla((c.hslHue + d + 1) % 1, c.hslSaturation, c.hslLightness, c.a)
    }
    // color-mix(in srgb, a p, b)
    function mix(a, b, p) {
        a = Qt.color(a); b = Qt.color(b)
        return Qt.rgba(a.r * p + b.r * (1 - p), a.g * p + b.g * (1 - p), a.b * p + b.b * (1 - p), a.a * p + b.a * (1 - p))
    }

    // ---------------------------------------------------------------- surfaces
    readonly property color bg:        pick("bg", dark ? "#07080c" : "#eceef4")
    // Cards are glass: translucent enough that the ambient light behind them tints them.
    readonly property color surface:   pick("surface", dark ? Qt.rgba(19 / 255, 22 / 255, 31 / 255, 0.80) : Qt.rgba(1, 1, 1, 0.84))
    readonly property color surface2:  pick("surface2", dark ? "#151822" : "#ffffff")  // raised controls, sheets
    readonly property color surface3:  pick("surface3", dark ? "#1f2330" : "#f4f5fa")  // hover
    readonly property color sunken:    pick("sunken", dark ? "#05060a" : "#e3e6ef")    // tracks, grooves
    readonly property color sidebar:   dark ? Qt.rgba(1, 1, 1, 0.018) : Qt.rgba(1, 1, 1, 0.45)
    readonly property color hairline:       pick("hairline", dark ? Qt.rgba(1, 1, 1, 0.07) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.08))
    readonly property color hairlineStrong: pick("hairlineStrong", dark ? Qt.rgba(1, 1, 1, 0.13) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.14))
    // The lit top edge of every glass surface.
    readonly property color rim: dark ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(1, 1, 1, 1)
    readonly property color fill:  dark ? Qt.rgba(1, 1, 1, 0.055) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.045)
    readonly property color fill2: dark ? Qt.rgba(1, 1, 1, 0.095) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.075)
    readonly property color fill3: dark ? Qt.rgba(1, 1, 1, 0.15) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.11)
    // --glass: legible even where blur is unavailable
    readonly property color glassFill: dark ? Qt.rgba(21 / 255, 24 / 255, 34 / 255, 0.90) : Qt.rgba(1, 1, 1, 0.92)
    readonly property color scrim: dark ? Qt.rgba(2 / 255, 3 / 255, 8 / 255, 0.62) : Qt.rgba(20 / 255, 24 / 255, 40 / 255, 0.28)
    // The LED preview is always a dark stage: lights are read against dark.
    readonly property color stage: "#050609"
    // How strongly the ambient light (the colours your devices show) washes the window.
    readonly property real ambient: dark ? 0.20 : 0.13

    // ---------------------------------------------------------------- text
    readonly property color text:  pick("text", dark ? "#f2f4fa" : "#0e1120")
    readonly property color text2: pick("text2", dark ? "#a1a9ba" : "#4f566a")
    readonly property color text3: pick("text3", dark ? "#737b8e" : "#646b7e")

    // ---------------------------------------------------------------- accent and status
    readonly property color accent: pick("accent", systemAccent)
    readonly property color accentText: pick("accentText", dark ? mix(accent, "#ffffff", 0.72) : mix(accent, "#000000", 0.88))
    // The accent's partner for gradients: the same colour turned a little round the wheel.
    readonly property color accent2: hue(accent, 0.075)
    readonly property color inkOnAccent: "#ffffff"
    readonly property color ok: pick("ok", dark ? "#32d74b" : "#1f8a33")
    readonly property color danger: pick("danger", dark ? "#ff453a" : "#d70015")
    readonly property color thumb: dark ? "#f1f3f6" : "#ffffff"

    // ---------------------------------------------------------------- metrics
    readonly property int rXs: 6
    readonly property int rSm: 10
    readonly property int rMd: 14
    readonly property int rLg: glassy ? glass.number(dark, "radius", 22) : 22
    readonly property int rXl: 28
    readonly property int gap: 16
    readonly property int titlebar: 52

    // ---------------------------------------------------------------- type (Adwaita Sans; weights 400/500/600/700)
    // Numbers and codes are monospaced, so values do not jitter while you drag.
    readonly property string mono: "Adwaita Mono"
    // Eyebrows: small uppercase labels with wide tracking (section and card titles).
    readonly property real eyebrowTracking: 1.3
    readonly property int fsDisplay: 30
    readonly property int fsTitle: 15
    readonly property int fsHeadline: 14
    readonly property int fsBody: 13
    readonly property int fsCallout: 12
    readonly property int fsCaption: 11

    // ---------------------------------------------------------------- motion
    readonly property int fast: reduceMotion ? 0 : 140
    readonly property int med: reduceMotion ? 0 : 240
    readonly property int slow: reduceMotion ? 0 : 420
    readonly property var easeOut: [0.2, 0.8, 0.2, 1, 1, 1]       // cubic-bezier(0.2, 0.8, 0.2, 1)
    readonly property var spring: [0.34, 1.36, 0.64, 1, 1, 1]     // cubic-bezier(0.34, 1.36, 0.64, 1)
}
