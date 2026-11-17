import QtQuick
import QtQuick.Layouts

Sheet {
    id: sh
    title: "Keyboard Shortcuts"
    preferredWidth: 420
    readonly property var keys: [
        ["Scan for devices", "Ctrl+R  ·  F5"],
        ["Save the current lighting as a scene", "Ctrl+S"],
        ["Undo Lights Off, a scene or a delete", "Ctrl+Z"],
        ["Preferences", "Ctrl+,"],
        ["Keyboard shortcuts", "Ctrl+?"],
        ["Close the window", "Ctrl+W"],
        ["Quit", "Ctrl+Q"],
        ["Move between devices and scenes", "↑ ↓"],
        ["Scene menu (rename, delete)", "Menu  ·  Shift+F10"],
        ["Back to the device list (narrow window)", "Esc  ·  Alt+←"],
        ["Colour wheel: hue / brightness", "← →  /  ↑ ↓  (Shift: ×5)"]
    ]
    Repeater {
        model: sh.keys
        delegate: RowLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: 12
            Text { Layout.fillWidth: true; text: modelData[0]; color: Theme.text; font.pixelSize: Theme.fsBody; wrapMode: Text.Wrap }
            Rectangle {     // a keycap: lit top, deeper bottom edge
                implicitWidth: k.implicitWidth + 16
                implicitHeight: 24
                radius: 6
                color: Theme.fill2
                border.width: 1
                border.color: Theme.hairlineStrong
                Rectangle { anchors.bottom: parent.bottom; x: 1; width: parent.width - 2; height: 2; radius: 1; color: Theme.fill2 }
                Text { id: k; anchors.centerIn: parent; text: modelData[1]; color: Theme.text2; font.family: Theme.mono; font.pixelSize: Theme.fsCaption; font.weight: Font.DemiBold }
            }
        }
    }
    buttons: [ Button { text: "Close"; onClicked: sh.close() } ]
}
