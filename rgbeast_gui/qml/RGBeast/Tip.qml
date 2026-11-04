import QtQuick
import QtQuick.Templates as T

// A small glass tooltip below its parent.
T.ToolTip {
    id: tip
    property bool shown: false
    visible: shown && text !== ""
    delay: 600
    timeout: 6000
    x: (parent.width - width) / 2
    y: parent.height + 6
    padding: 0
    implicitWidth: label.implicitWidth + 20
    implicitHeight: 26
    contentItem: Text {
        id: label
        text: tip.text
        color: Theme.text
        font.pixelSize: Theme.fsCallout
        font.weight: Font.Medium
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Glass { radius: 8 }
    enter: Transition { NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast } }
    exit: Transition { NumberAnimation { property: "opacity"; from: 1; to: 0; duration: 110 } }
}
