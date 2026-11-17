import QtQuick

// A soft coloured halo around its parent's shape: the light-from-within of lit controls. Stacked
// rectangles, like Shadow, so it looks the same in every renderer.
Shadow {
    property bool on: true
    offsetY: 0
    fade: true
    contact: false
    color: Theme.accent
    spread: 18
    strength: Theme.dark ? 0.55 : 0.32
    visible: on && opacity > 0
    opacity: on ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: Theme.med } }
}
