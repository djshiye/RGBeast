import QtQuick

// The ambient light behind everything: three soft clouds of the colours your devices show, drifting
// slowly. One canvas clipped to the window's rounded shape (no shaders, so it looks the same in every
// renderer). The drift is a few pixels a second, so four repaints a second are smooth enough and
// cost almost nothing; it holds still when the system asks for reduced motion.
Canvas {
    id: au
    property var colors: []         // up to three colours; fewer repeat, none falls back to the accent
    property real strength: Theme.ambient
    property real cornerRadius: 0
    property bool running: true
    property real t: 0

    readonly property var clouds: [
        { fx: 0.80, fy: -0.06, size: 0.62, dx: -0.10, dy: 0.07, period: 46 },
        { fx: 0.10, fy: 0.16, size: 0.50, dx: 0.07, dy: 0.10, period: 58 },
        { fx: 0.52, fy: 1.02, size: 0.58, dx: -0.12, dy: -0.06, period: 68 }
    ]
    property color c0: pick(0)
    property color c1: pick(1)
    property color c2: pick(2)
    Behavior on c0 { ColorAnimation { duration: Theme.slow * 2 } }
    Behavior on c1 { ColorAnimation { duration: Theme.slow * 2 } }
    Behavior on c2 { ColorAnimation { duration: Theme.slow * 2 } }
    function pick(i) {
        const n = colors.length
        return n === 0 ? Theme.accent : colors[i % n]
    }

    onC0Changed: requestPaint()
    onC1Changed: requestPaint()
    onC2Changed: requestPaint()
    onStrengthChanged: requestPaint()
    onTChanged: requestPaint()
    renderStrategy: Canvas.Cooperative

    Timer {
        interval: 250
        repeat: true
        running: au.running && au.visible && !Theme.reduceMotion
        onTriggered: au.t += interval / 1000
    }

    onPaint: {
        const ctx = getContext("2d")
        ctx.reset()
        if (width <= 0 || height <= 0)
            return
        ctx.save()
        ctx.beginPath()
        ctx.roundedRect(0, 0, width, height, cornerRadius, cornerRadius)
        ctx.clip()
        const cs = [c0, c1, c2]
        const span = Math.max(width, height)
        for (let i = 0; i < clouds.length; i++) {
            const k = clouds[i]
            const ph = 2 * Math.PI * t / k.period
            const x = width * (k.fx + k.dx * Math.sin(ph))
            const y = height * (k.fy + k.dy * Math.sin(ph * 0.8 + i))
            const r = span * k.size
            const c = cs[i]
            const a = strength * (i === 2 ? 0.75 : 1)
            const g = ctx.createRadialGradient(x, y, 0, x, y, r)
            g.addColorStop(0, Qt.rgba(c.r, c.g, c.b, a))
            g.addColorStop(0.35, Qt.rgba(c.r, c.g, c.b, a * 0.55))
            g.addColorStop(0.7, Qt.rgba(c.r, c.g, c.b, a * 0.12))
            g.addColorStop(1, Qt.rgba(c.r, c.g, c.b, 0))
            ctx.fillStyle = g
            ctx.fillRect(x - r, y - r, 2 * r, 2 * r)
        }
        ctx.restore()
    }
}
