import QtQuick
import QtQuick.Templates as T

// .switch: 42 x 24, a glowing accent gradient when on.
T.Switch {
    id: sw
    implicitWidth: 42
    implicitHeight: 24
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: 0
    Accessible.role: Accessible.CheckBox
    indicator: Item {
        width: 42; height: 24
        Glow { anchors.fill: parent; radius: 12; on: sw.checked && sw.enabled; spread: 14; strength: Theme.dark ? 0.45 : 0.25 }
        Rectangle {
            anchors.fill: parent
            radius: 12
            color: Theme.fill3
            border.width: 1
            border.color: Theme.hairlineStrong
        }
        Rectangle {
            anchors.fill: parent
            radius: 12
            opacity: sw.checked ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: Theme.accent }
                GradientStop { position: 1; color: Theme.accent2 }
            }
        }
        Rectangle {
            x: sw.checked ? 20 : 2
            y: 2
            width: 20; height: 20; radius: 10
            color: "#ffffff"
            Behavior on x { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring } }
            Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 1; anchors.bottomMargin: -1; radius: 10; color: "black"; opacity: 0.28 }
        }
        FocusRing { shown: sw.visualFocus; targetRadius: 12 }
    }
    contentItem: Item {}
}
