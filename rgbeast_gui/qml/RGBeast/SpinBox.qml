import QtQuick
import QtQuick.Templates as T

// A compact number field with − and + (wheel and arrow keys work too).
T.SpinBox {
    id: sb
    editable: true
    implicitWidth: 118
    implicitHeight: 30
    leftPadding: 30
    rightPadding: 30
    font.pixelSize: Theme.fsBody
    Accessible.role: Accessible.SpinBox
    wheelEnabled: true
    validator: IntValidator { bottom: Math.min(sb.from, sb.to); top: Math.max(sb.from, sb.to) }

    contentItem: TextInput {
        text: sb.displayText
        font: sb.font
        color: Theme.text
        horizontalAlignment: Qt.AlignHCenter
        verticalAlignment: Qt.AlignVCenter
        readOnly: !sb.editable
        validator: sb.validator
        inputMethodHints: Qt.ImhDigitsOnly
        selectByMouse: true
        selectionColor: Theme.alpha(Theme.accent, 0.45)
    }
    background: Rectangle {
        radius: 8
        color: Theme.fill
        border.width: 1
        border.color: sb.activeFocus ? Theme.accent : Theme.hairline
    }
    down.indicator: Rectangle {
        x: 2; y: 2; width: 26; height: sb.height - 4; radius: 6
        color: sb.down.pressed ? Theme.fill2 : sb.down.hovered ? Theme.fill : "transparent"
        Icon { anchors.centerIn: parent; name: "minus"; size: 14; color: sb.value > sb.from ? Theme.text2 : Theme.text3 }
    }
    up.indicator: Rectangle {
        x: sb.width - 28; y: 2; width: 26; height: sb.height - 4; radius: 6
        color: sb.up.pressed ? Theme.fill2 : sb.up.hovered ? Theme.fill : "transparent"
        Icon { anchors.centerIn: parent; name: "plus"; size: 14; color: sb.value < sb.to ? Theme.text2 : Theme.text3 }
    }
}
