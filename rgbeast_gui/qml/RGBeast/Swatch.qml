import QtQuick
import QtQuick.Templates as T

// A round colour dot with a hairline ring; a double accent ring when selected.
T.AbstractButton {
    id: sw
    property color swatch: "white"
    property bool selected: false
    property string label: ""
    property real size: 28
    implicitWidth: size
    implicitHeight: size
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.role: Accessible.Button
    Accessible.name: label

    background: Item {
        Rectangle {     // its own light, glowing around it
            anchors.centerIn: parent
            width: sw.size + 10; height: width; radius: width / 2
            color: Theme.alpha(sw.swatch, Theme.dark ? 0.22 : 0.16)
            opacity: sw.hovered || sw.selected ? 1 : (Theme.dark ? 0.55 : 0)
            Behavior on opacity { NumberAnimation { duration: Theme.fast } }
        }
        Rectangle {     // selection: card-coloured gap, then the accent ring
            anchors.centerIn: parent
            width: sw.size + 8; height: width; radius: width / 2
            color: "transparent"
            border.width: 2
            border.color: Theme.accent
            visible: sw.selected
        }
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: sw.swatch
            border.width: 1
            border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.22) : Qt.rgba(0, 0, 0, 0.14)
        }
        FocusRing { shown: sw.visualFocus; targetRadius: sw.size / 2; offset: sw.selected ? 6 : 2 }
    }
    scale: down ? 0.94 : hovered ? 1.08 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring } }
    Tip { text: sw.label; shown: sw.hovered && sw.label !== "" }
}
