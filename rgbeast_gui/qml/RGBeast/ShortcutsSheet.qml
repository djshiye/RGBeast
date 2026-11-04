import QtQuick
import QtQuick.Layouts

Sheet {
    id: sh
    title: "Keyboard Shortcuts"
    preferredWidth: 420
    readonly property var keys: [
        ["Scan for devices", "Ctrl+R  ·  F5"],
        ["Preferences", "Ctrl+,"],
        ["Keyboard shortcuts", "Ctrl+?"],
        ["Close the window", "Ctrl+W"],
        ["Quit", "Ctrl+Q"],
        ["Colour wheel: hue / brightness", "← →  /  ↑ ↓  (Shift: ×5)"]
    ]
    Repeater {
        model: sh.keys
        delegate: RowLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: 12
            Text { Layout.fillWidth: true; text: modelData[0]; color: Theme.text; font.pixelSize: Theme.fsBody; wrapMode: Text.Wrap }
            Rectangle {     // .kbd
                implicitWidth: k.implicitWidth + 14
                implicitHeight: 22
                radius: 5
                color: Theme.fill2
                Text { id: k; anchors.centerIn: parent; text: modelData[1]; color: Theme.text2; font.pixelSize: Theme.fsCaption; font.weight: Font.DemiBold }
            }
        }
    }
    buttons: [ Button { text: "Close"; onClicked: sh.close() } ]
}
