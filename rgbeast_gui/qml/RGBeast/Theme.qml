pragma Singleton
import QtQuick

// Design tokens from audius (audius/ui/css/tokens.css): one accent for the values you set, quiet
// neutral surfaces, so the only saturated colour on screen is the light you chose.
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

    function pick(role, fallback) {
        if (!glassy)
            return fallback
        const v = glass.color(dark, role, "")
        return v === "" ? fallback : v
    }
    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }
    // color-mix(in srgb, a p, b)
    function mix(a, b, p) {
        a = Qt.color(a); b = Qt.color(b)
        return Qt.rgba(a.r * p + b.r * (1 - p), a.g * p + b.g * (1 - p), a.b * p + b.b * (1 - p), a.a * p + b.a * (1 - p))
    }

    // ---------------------------------------------------------------- surfaces
    readonly property color bg:        pick("bg", dark ? "#0d0f12" : "#f2f2f5")
    readonly property color surface:   pick("surface", dark ? "#16191e" : "#ffffff")   // cards
    readonly property color surface2:  pick("surface2", dark ? "#1e2228" : "#ffffff")  // raised controls, popovers
    readonly property color surface3:  pick("surface3", dark ? "#262b33" : "#f4f4f7")  // hover
    readonly property color sunken:    pick("sunken", dark ? "#0b0d10" : "#e7e7ec")    // tracks, grooves
    readonly property color hairline:       pick("hairline", dark ? Qt.rgba(1, 1, 1, 0.07) : Qt.rgba(0, 0, 0, 0.075))
    readonly property color hairlineStrong: pick("hairlineStrong", dark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.12))
    readonly property color fill:  dark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.045)
    readonly property color fill2: dark ? Qt.rgba(1, 1, 1, 0.10) : Qt.rgba(0, 0, 0, 0.075)
    readonly property color fill3: dark ? Qt.rgba(1, 1, 1, 0.15) : Qt.rgba(0, 0, 0, 0.11)
    // --glass: legible even where blur is unavailable
    readonly property color glassFill: dark ? Qt.rgba(30 / 255, 34 / 255, 40 / 255, 0.88) : Qt.rgba(1, 1, 1, 0.90)
    readonly property color scrim: dark ? Qt.rgba(0, 0, 0, 0.45) : Qt.rgba(0, 0, 0, 0.20)
    // The LED preview is always a dark stage: lights are read against dark.
    readonly property color stage: "#0b0d10"

    // ---------------------------------------------------------------- text
    readonly property color text:  pick("text", dark ? "#eceff4" : "#1d1d1f")
    readonly property color text2: pick("text2", dark ? "#a3aab5" : "#5c5d66")
    readonly property color text3: pick("text3", dark ? "#7c8490" : "#6e6f78")

    // ---------------------------------------------------------------- accent and status
    readonly property color accent: pick("accent", systemAccent)
    readonly property color accentText: pick("accentText", dark ? mix(accent, "#ffffff", 0.72) : mix(accent, "#000000", 0.88))
    readonly property color inkOnAccent: "#ffffff"
    readonly property color ok: pick("ok", dark ? "#32d74b" : "#1f8a33")
    readonly property color danger: pick("danger", dark ? "#ff453a" : "#d70015")
    readonly property color thumb: dark ? "#f1f3f6" : "#ffffff"

    // ---------------------------------------------------------------- metrics
    readonly property int rXs: 6
    readonly property int rSm: 9
    readonly property int rMd: 13
    readonly property int rLg: glassy ? glass.number(dark, "radius", 20) : 20
    readonly property int rXl: 26
    readonly property int gap: 18
    readonly property int titlebar: 52

    // ---------------------------------------------------------------- type (Adwaita Sans; weights 400/500/600/700)
    readonly property int fsDisplay: 28
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
