import QtQuick
import QtQuick.Templates as T

// .slider: 6 px track, accent fill, 20 px thumb that grows a little while dragged.
T.Slider {
    id: s
    from: 0
    to: 100
    stepSize: 1
    snapMode: T.Slider.SnapAlways
    implicitWidth: 200
    implicitHeight: 30
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    Accessible.role: Accessible.Slider
    readonly property real inset: 11

    background: Item {
        Rectangle {
            id: track
            x: s.inset
            width: s.width - 2 * s.inset
            y: (s.height - 6) / 2
            height: 6
            radius: 3
            color: Theme.fill2
            Rectangle {
                width: s.visualPosition * parent.width
                height: parent.height
                radius: 3
                color: s.enabled ? Theme.accent : Theme.text3
            }
        }
    }
    handle: Rectangle {
        x: s.inset + s.visualPosition * (s.width - 2 * s.inset) - 10
        y: (s.height - 20) / 2
        width: 20; height: 20; radius: 10
        color: Theme.thumb
        border.width: 0.5
        border.color: Qt.rgba(0, 0, 0, 0.18)
        scale: s.pressed ? 1.12 : 1
        Behavior on scale { NumberAnimation { duration: Theme.fast } }
        Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 2; anchors.bottomMargin: -2; radius: 10; color: "black"; opacity: Theme.dark ? 0.32 : 0.18 }
        FocusRing { shown: s.visualFocus; targetRadius: 10 }
    }
}
