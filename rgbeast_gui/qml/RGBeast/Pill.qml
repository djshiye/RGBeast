import QtQuick
import QtQuick.Templates as T

// An effect chip with its LED: quiet glass when off; when it is the active one, an accent gradient
// that glows from within and an LED that is lit.
T.AbstractButton {
    id: p
    checkable: false
    property bool on: false
    implicitHeight: 34
    implicitWidth: row.implicitWidth + 30
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.role: Accessible.RadioButton
    Accessible.name: text
    Accessible.checked: on
    Accessible.checkable: true

    background: Item {
        Glow { anchors.fill: parent; radius: height / 2; on: p.on; spread: 16; strength: Theme.dark ? 0.42 : 0.22 }
        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: p.down ? Theme.fill3 : p.hovered ? Theme.fill2 : Theme.fill
            border.width: 1
            border.color: p.hovered ? Theme.hairlineStrong : Theme.hairline
            opacity: p.on ? 0 : 1
            Behavior on opacity { NumberAnimation { duration: Theme.fast } }
            Behavior on color { ColorAnimation { duration: Theme.fast } }
        }
        Rectangle {     // the lit state
            anchors.fill: parent
            radius: height / 2
            opacity: p.on ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
            border.width: 1
            border.color: Theme.alpha(Theme.accent, Theme.dark ? 0.7 : 0.55)
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: Theme.alpha(Theme.accent, Theme.dark ? (p.hovered ? 0.36 : 0.28) : (p.hovered ? 0.22 : 0.16)) }
                GradientStop { position: 1; color: Theme.alpha(Theme.accent2, Theme.dark ? (p.hovered ? 0.26 : 0.18) : (p.hovered ? 0.16 : 0.10)) }
            }
        }
        FocusRing { shown: p.visualFocus; targetRadius: parent.height / 2 }
    }
    contentItem: Item {
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 9
            Item {
                width: 7; height: 7
                anchors.verticalCenter: parent.verticalCenter
                Rectangle {     // the lit halo
                    anchors.centerIn: parent
                    width: 15; height: 15; radius: 7.5
                    color: Theme.alpha(Theme.accent, 0.28)
                    opacity: p.on ? 1 : 0
                    scale: p.on ? 1 : 0.4
                    Behavior on opacity { NumberAnimation { duration: Theme.med } }
                    Behavior on scale { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring } }
                }
                Rectangle {
                    anchors.fill: parent
                    radius: 3.5
                    color: p.on ? (Theme.dark ? Theme.mix(Theme.accent, "#ffffff", 0.6) : Theme.accent) : Theme.fill3
                    Behavior on color { ColorAnimation { duration: Theme.fast } }
                }
            }
            Text {
                text: p.text
                anchors.verticalCenter: parent.verticalCenter
                font.pixelSize: Theme.fsBody
                font.weight: p.on ? Font.DemiBold : Font.Medium
                color: p.on ? Theme.accentText : p.hovered ? Theme.text : Theme.text2
                Behavior on color { ColorAnimation { duration: Theme.fast } }
            }
        }
    }
    scale: down ? 0.96 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast } }
}
