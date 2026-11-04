import QtQuick
import QtQuick.Templates as T

// .switch: 40 x 24, accent when on.
T.Switch {
    id: sw
    implicitWidth: 40
    implicitHeight: 24
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: 0
    Accessible.role: Accessible.CheckBox
    indicator: Rectangle {
        width: 40; height: 24; radius: 12
        color: sw.checked ? Theme.accent : Theme.fill3
        border.width: sw.checked ? 0 : 1
        border.color: Theme.hairlineStrong
        Behavior on color { ColorAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
        Rectangle {
            x: sw.checked ? 18 : 2
            y: 2
            width: 20; height: 20; radius: 10
            color: "#ffffff"
            Behavior on x { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
            Rectangle { z: -1; anchors.fill: parent; anchors.topMargin: 1; anchors.bottomMargin: -1; radius: 10; color: "black"; opacity: 0.25 }
        }
        FocusRing { shown: sw.visualFocus; targetRadius: 12 }
    }
    contentItem: Item {}
}
