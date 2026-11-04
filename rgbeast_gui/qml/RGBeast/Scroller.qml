import QtQuick
import QtQuick.Templates as T

// A thin overlay scroll bar: shows while scrolling or hovered, hides otherwise (.scroll in audius).
T.ScrollBar {
    id: bar
    orientation: Qt.Vertical
    implicitWidth: 12
    padding: 3
    minimumSize: 0.08
    hoverEnabled: true
    visible: size < 1
    policy: T.ScrollBar.AsNeeded

    contentItem: Rectangle {
        implicitWidth: bar.hovered || bar.pressed ? 6 : 4
        radius: width / 2
        color: bar.pressed ? Theme.fill3 : Theme.dark ? Qt.rgba(1, 1, 1, 0.22) : Qt.rgba(0, 0, 0, 0.22)
        opacity: bar.active || bar.hovered ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.med } }
        Behavior on implicitWidth { NumberAnimation { duration: Theme.fast } }
    }
}
