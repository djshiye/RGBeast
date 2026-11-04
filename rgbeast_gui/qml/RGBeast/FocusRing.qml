import QtQuick

// The 2 px accent focus ring (outline-offset 2px), shown for keyboard focus only.
Rectangle {
    property Item target: parent
    property bool shown: false
    property real offset: 2
    property real targetRadius: 9
    x: -offset - 2; y: -offset - 2
    width: target.width + 2 * (offset + 2); height: target.height + 2 * (offset + 2)
    radius: targetRadius + offset + 2
    color: "transparent"
    border.width: 2
    border.color: Theme.accent
    visible: shown
    z: 10
}
