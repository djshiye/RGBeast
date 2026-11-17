import QtQuick
import QtQuick.Templates as T

// .btn: 32 px, radius 10, 600 weight; the accent variant is a glowing gradient. Variants: accent, plain, danger, small; icon-only when text is empty.
T.AbstractButton {
    id: b
    property string glyph: ""
    property bool accent: false
    property bool plain: false
    property bool danger: false
    property bool small: false
    readonly property bool iconOnly: text === "" && glyph !== ""

    implicitHeight: small ? 28 : 34
    implicitWidth: iconOnly ? implicitHeight : row.implicitWidth + (small ? 20 : 26)
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    Accessible.role: Accessible.Button
    property string tip: ""
    Accessible.name: text !== "" ? text : (tip !== "" ? tip : glyph)

    background: Item {
        // The primary button glows in its own colour.
        Glow {
            anchors.fill: parent
            radius: b.small ? 8 : 10
            on: b.accent && b.enabled
            color: Theme.accent
            spread: b.hovered ? 22 : 16
            strength: Theme.dark ? (b.hovered ? 0.6 : 0.45) : (b.hovered ? 0.4 : 0.3)
        }
        Rectangle {
            anchors.fill: parent
            radius: b.small ? 8 : 10
            visible: !b.accent || !b.enabled
            color: !b.enabled ? Theme.fill
                 : b.plain ? (b.down ? Theme.fill2 : b.hovered ? Theme.fill : "transparent")
                 : (b.down ? Theme.fill3 : b.hovered ? Theme.fill2 : Theme.fill)
            border.width: b.plain ? 0 : 1
            border.color: b.danger && b.hovered ? Theme.alpha(Theme.danger, 0.5) : b.hovered ? Theme.hairlineStrong : Theme.hairline
            Behavior on color { ColorAnimation { duration: Theme.fast } }
        }
        Rectangle {
            anchors.fill: parent
            radius: b.small ? 8 : 10
            visible: b.accent && b.enabled
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: b.hovered ? Theme.mix(Theme.accent, "#ffffff", 0.88) : Theme.accent }
                GradientStop { position: 1; color: b.hovered ? Theme.mix(Theme.accent2, "#ffffff", 0.88) : Theme.accent2 }
            }
            // The lit top edge.
            Rectangle {
                x: parent.radius; width: parent.width - 2 * parent.radius; height: 1
                color: Qt.rgba(1, 1, 1, 0.35)
            }
        }
        FocusRing { shown: b.visualFocus; targetRadius: b.small ? 8 : 10 }
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
