import QtQuick
import QtQuick.Templates as T

// .slider: a 6 px groove with a glowing gradient fill, and a 22 px bead thumb that grows while dragged.
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
            color: Theme.sunken
            border.width: 1
            border.color: Theme.hairline
            Item {
                id: fillArea
                width: Math.max(6, s.visualPosition * parent.width)
                height: parent.height
                Glow { anchors.fill: parent; radius: 3; spread: 14; on: s.enabled; strength: Theme.dark ? (s.pressed || s.hovered ? 0.55 : 0.38) : 0.22 }
                Rectangle {
                    anchors.fill: parent
                    radius: 3
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0; color: s.enabled ? Theme.accent : Theme.text3 }
                        GradientStop { position: 1; color: s.enabled ? Theme.accent2 : Theme.text3 }
                    }
                }
            }
        }
    }
    handle: Item {
        x: s.inset + s.visualPosition * (s.width - 2 * s.inset) - 11
        y: (s.height - 22) / 2
        width: 22; height: 22
        Rectangle {     // a soft halo while you touch it
            anchors.centerIn: parent
            width: 36; height: 36; radius: 18
            color: Theme.alpha(Theme.accent, 0.18)
            opacity: s.pressed || s.hovered ? 1 : 0
            scale: s.pressed ? 1.1 : 1
            Behavior on opacity { NumberAnimation { duration: Theme.fast } }
            Behavior on scale { NumberAnimation { duration: Theme.fast } }
        }
        Rectangle { anchors.fill: parent; anchors.topMargin: 2; anchors.bottomMargin: -2; radius: 11; color: "black"; opacity: Theme.dark ? 0.4 : 0.16 }
        Rectangle {
            anchors.fill: parent
            radius: 11
            color: Theme.thumb
            border.width: 0.5
            border.color: Qt.rgba(0, 0, 0, 0.18)
            scale: s.pressed ? 1.1 : 1
            Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring } }
            // The accent core: the thumb is a lit bead.
            Rectangle {
                anchors.centerIn: parent
                width: 8; height: 8; radius: 4
                color: s.enabled ? Theme.accent : Theme.text3
            }
        }
        FocusRing { shown: s.visualFocus; targetRadius: 11 }
    }
}
