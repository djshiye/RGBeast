import QtQuick
import QtQuick.Templates as T

// .btn: 32 px, radius 9, 600 weight. Variants: accent, plain, danger, small; icon-only when text is empty.
T.AbstractButton {
    id: b
    property string glyph: ""
    property bool accent: false
    property bool plain: false
    property bool danger: false
    property bool small: false
    readonly property bool iconOnly: text === "" && glyph !== ""

    implicitHeight: small ? 26 : 32
    implicitWidth: iconOnly ? implicitHeight : row.implicitWidth + (small ? 20 : 26)
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    Accessible.role: Accessible.Button
    property string tip: ""
    Accessible.name: text !== "" ? text : (tip !== "" ? tip : glyph)

    background: Rectangle {
        radius: b.small ? 8 : 9
        color: !b.enabled ? Theme.fill
             : b.accent ? (b.hovered ? Theme.mix(Theme.accent, "#ffffff", 0.88) : Theme.accent)
             : b.plain ? (b.down ? Theme.fill2 : b.hovered ? Theme.fill : "transparent")
             : (b.down ? Theme.fill3 : b.hovered ? Theme.fill2 : Theme.fill)
        Behavior on color { ColorAnimation { duration: Theme.fast } }
        FocusRing { shown: b.visualFocus; targetRadius: parent.radius }
    }
    contentItem: Item {
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 7
            Icon {
                visible: b.glyph !== ""
                name: b.glyph
                size: b.small ? 14 : 16
                anchors.verticalCenter: parent.verticalCenter
                color: lbl.color
            }
            Text {
                id: lbl
                visible: b.text !== ""
                text: b.text
                anchors.verticalCenter: parent.verticalCenter
                font.pixelSize: b.small ? Theme.fsCallout : Theme.fsBody
                font.weight: Font.DemiBold
                color: b.accent ? Theme.inkOnAccent : b.danger ? Theme.danger : Theme.text
            }
        }
    }
    opacity: enabled ? 1 : 0.38
    scale: down ? 0.97 : 1
    Behavior on scale { NumberAnimation { duration: Theme.fast; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }

    Tip { text: b.tip; shown: b.hovered && b.tip !== "" }
}
