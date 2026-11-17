import QtQuick
import QtQuick.Layouts

// A glass card: translucent surface, lit top rim, radius 22, eyebrow title with an accent tick.
Item {
    id: card
    default property alias content: body.data
    property string title: ""
    property string subtitle: ""
    property alias headerExtra: extra.data
    property alias spacing: body.spacing
    property real padTop: 18
    property real padSide: 20
    property real padBottom: 20
    property color color: Theme.surface

    implicitWidth: 320
    implicitHeight: column.implicitHeight + padTop + padBottom
    Layout.fillWidth: true

    // Light style: a soft lift. Dark style: no shadow (it would only muddy the glass); the rim does it.
    Shadow { anchors.fill: parent; radius: Theme.rLg; visible: !Theme.dark; strength: 0.06; offsetY: 12; spread: 34 }
    Rectangle {
        anchors.fill: parent
        radius: Theme.rLg
        color: card.color
        border.width: 1
        border.color: Theme.hairline
        Behavior on color { ColorAnimation { duration: Theme.med } }
        // A sheen from the top: the glass catching the light.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            gradient: Gradient {
                GradientStop { position: 0; color: Theme.dark ? Qt.rgba(1, 1, 1, 0.035) : Qt.rgba(1, 1, 1, 0.6) }
                GradientStop { position: Math.min(0.9, 140 / Math.max(1, card.height)); color: "transparent" }
            }
        }
    }
    // The lit rim along the top edge, brightest in the middle.
    Rectangle {
        x: Theme.rLg; width: parent.width - 2 * Theme.rLg; y: 0; height: 1
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: "transparent" }
            GradientStop { position: 0.5; color: Theme.rim }
            GradientStop { position: 1; color: "transparent" }
        }
    }

    ColumnLayout {
        id: column
        x: card.padSide
        y: card.padTop
        width: card.width - 2 * card.padSide
        spacing: 12

        RowLayout {
            visible: card.title !== "" || extra.children.length > 0
            Layout.fillWidth: true
            Layout.minimumHeight: 28
            spacing: 12
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                RowLayout {
                    visible: card.title !== ""
                    spacing: 9
                    // The accent tick: a short lit bar before the title.
                    Rectangle {
                        implicitWidth: 3; implicitHeight: 12; radius: 1.5
                        gradient: Gradient {
                            GradientStop { position: 0; color: Theme.accent2 }
                            GradientStop { position: 1; color: Theme.accent }
                        }
                        Glow { anchors.fill: parent; radius: 1.5; spread: 10; strength: Theme.dark ? 0.5 : 0.25 }
                    }
                    Eyebrow {
                        Layout.fillWidth: true
                        text: card.title
                        color: Theme.text2
                        Accessible.role: Accessible.Heading
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: card.subtitle
                    visible: text !== ""
                    color: Theme.text2
                    Layout.topMargin: 4
                    font.pixelSize: Theme.fsCallout
                    lineHeight: 1.25
                    wrapMode: Text.Wrap
                }
            }
            Row { id: extra; spacing: 6; Layout.alignment: Qt.AlignVCenter }
        }
        ColumnLayout {
            id: body
            Layout.fillWidth: true
            spacing: 12
        }
    }
}
