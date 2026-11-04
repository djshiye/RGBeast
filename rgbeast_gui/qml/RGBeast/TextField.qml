import QtQuick
import QtQuick.Templates as T

// .field: 30 px, radius 8, an accent ring and a soft halo while focused; red ring when invalid.
T.TextField {
    id: f
    property bool invalid: false
    implicitHeight: 30
    implicitWidth: 180
    leftPadding: 10
    rightPadding: 10
    verticalAlignment: TextInput.AlignVCenter
    color: Theme.text
    placeholderTextColor: Theme.text3
    selectionColor: Theme.alpha(Theme.accent, 0.45)
    selectedTextColor: Theme.text
    font.pixelSize: Theme.fsBody
    selectByMouse: true
    background: Rectangle {
        radius: 8
        color: Theme.fill
        border.width: 1
        border.color: f.invalid ? Theme.danger : f.activeFocus ? Theme.accent : Theme.hairline
        Rectangle {     // 0 0 0 3px halo
            anchors.fill: parent
            anchors.margins: -3
            radius: 11
            color: "transparent"
            border.width: 3
            border.color: Theme.alpha(f.invalid ? Theme.danger : Theme.accent, 0.30)
            visible: f.activeFocus
        }
    }
    Text {
        visible: f.text === "" && !f.preeditText
        text: f.placeholderText
        color: f.placeholderTextColor
        font: f.font
        x: f.leftPadding
        anchors.verticalCenter: parent.verticalCenter
    }
}
