import QtQuick
import QtQuick.Templates as T

// GNOME-style window buttons (round, 24 px, on a quiet fill) in the order the desktop's
// button-layout asks for.
Row {
    id: wb
    property var buttons: ["close"]
    property Window window: null
    readonly property bool maximized: window && (window.visibility === Window.Maximized || window.visibility === Window.FullScreen)
    spacing: 10

    Repeater {
        model: wb.buttons
        delegate: T.AbstractButton {
            id: b
            required property string modelData
            width: 24; height: 24
            anchors.verticalCenter: parent ? parent.verticalCenter : undefined
            hoverEnabled: true
            focusPolicy: Qt.NoFocus
            Accessible.role: Accessible.Button
            Accessible.name: modelData === "close" ? "Close" : modelData === "minimize" ? "Minimize" : (wb.maximized ? "Restore" : "Maximize")
            onClicked: {
                if (modelData === "close") wb.window.close()
                else if (modelData === "minimize") wb.window.showMinimized()
                else wb.window.visibility = wb.maximized ? Window.Windowed : Window.Maximized
            }
            background: Rectangle {
                radius: 12
                color: b.down ? Theme.fill3 : b.hovered ? Theme.fill2 : Theme.fill
                Behavior on color { ColorAnimation { duration: Theme.fast } }
            }
            contentItem: Item {
                Icon {
                    anchors.centerIn: parent
                    size: 16
                    strokeWidth: 2
                    name: b.modelData === "maximize" ? (wb.maximized ? "restore" : "maximize") : b.modelData
                    color: Theme.text
                }
            }
        }
    }
}
