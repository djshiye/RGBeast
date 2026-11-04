import QtQuick
import QtQuick.Templates as T

// .pill with its LED: a choice that lights up in the accent when it is the active one.
T.AbstractButton {
    id: p
    checkable: false
    property bool on: false
    implicitHeight: 32
    implicitWidth: row.implicitWidth + 28
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.role: Accessible.RadioButton
    Accessible.name: text
    Accessible.checked: on
    Accessible.checkable: true

    background: Rectangle {
        radius: height / 2
        color: p.on ? Theme.alpha(Theme.accent, p.hovered ? 0.22 : 0.16)
                    : (p.down ? Theme.fill3 : p.hovered ? Theme.fill2 : Theme.fill)
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { shown: p.visualFocus; targetRadius: parent.radius }
    }
    contentItem: Item {
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 8
            Rectangle {
                id: led
                width: 7; height: 7; radius: 3.5
                anchors.verticalCenter: parent.verticalCenter
                color: p.on ? Theme.accent : Theme.fill3
                Behavior on color { ColorAnimation { duration: Theme.fast } }
                Rectangle {     // the lit halo
                    anchors.centerIn: parent
                    width: 13; height: 13; radius: 6.5
                    color: Theme.alpha(Theme.accent, 0.25)
                    visible: p.on
                }
            }
            Text {
                text: p.text
                anchors.verticalCenter: parent.verticalCenter
                font.pixelSize: Theme.fsBody
                font.weight: Font.Medium
                color: p.on ? Theme.accentText : p.hovered ? Theme.text : Theme.text2
                Behavior on color { ColorAnimation { duration: Theme.fast } }
            }
        }
    }
    scale: down ? 0.97 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast } }
}
