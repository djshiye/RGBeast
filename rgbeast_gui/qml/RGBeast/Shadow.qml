import QtQuick

// A soft drop shadow from stacked rounded rectangles: cheap, and identical in every renderer
// (the software renderer used headless and on some VMs has no shader effects).
Item {
    id: s
    property real radius: Theme.rLg
    property real offsetY: Theme.dark ? 14 : 10
    property real spread: Theme.dark ? 30 : 26        // ~ the CSS blur radius
    property real strength: Theme.dark ? 0.28 : 0.07
    readonly property int steps: 9

    // The tight contact shadow (0 1px 2px).
    Rectangle {
        anchors.fill: parent
        anchors.topMargin: 1
        anchors.bottomMargin: -1
        radius: s.radius
        color: "black"
        opacity: Theme.dark ? 0.38 : 0.06
    }
    Repeater {
        model: s.steps
        delegate: Rectangle {
            required property int index
            readonly property real grow: (index + 1) * s.spread / s.steps / 2
            x: -grow
            y: -grow + s.offsetY * (index + 1) / s.steps
            width: s.width + grow * 2
            height: s.height + grow * 2
            radius: s.radius + grow
            color: "black"
            opacity: s.strength / s.steps * 1.25
        }
    }
}
