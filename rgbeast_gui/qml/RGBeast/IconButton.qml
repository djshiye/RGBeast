import QtQuick
import QtQuick.Templates as T

// .icon-btn: 28 px, radius 8, quiet until hovered.
T.AbstractButton {
    id: b
    property string glyph: ""
    property string tip: ""
    property real size: 28
    property real iconSize: 16
    property color tint: Theme.text2
    implicitWidth: size
    implicitHeight: size
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.role: Accessible.Button
    Accessible.name: tip !== "" ? tip : glyph
    background: Rectangle {
        radius: b.size >= 32 ? 9 : 8
        color: b.down ? Theme.fill2 : (b.hovered || b.checked) ? Theme.fill : "transparent"
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { shown: b.visualFocus; targetRadius: parent.radius }
    }
    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: b.glyph
            size: b.iconSize
            color: b.hovered || b.checked ? Theme.text : b.tint
            Behavior on color { ColorAnimation { duration: Theme.fast } }
        }
    }
    opacity: enabled ? 1 : 0.38
    Tip { text: b.tip; shown: b.hovered && b.tip !== "" }
}
