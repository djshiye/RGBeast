import QtQuick
import QtQuick.Layouts

// Loading, disconnected and empty states: a big glyph tile, a title, a sentence, one action.
Item {
    id: sp
    property string icon: ""
    property string title: ""
    property string description: ""
    property string action: ""
    property bool busy: false
    signal activated()

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(460, sp.width - 48)
        spacing: 14

        Item {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 72
            Layout.preferredHeight: 72
            Rectangle {
                anchors.fill: parent
                radius: 22
                color: Theme.fill
                visible: !sp.busy
                Icon { anchors.centerIn: parent; name: sp.icon; size: 34; strokeWidth: 1.6; color: Theme.text2 }
            }
            // A quiet spinner: an accent arc turning on a faint ring.
            Item {
                anchors.centerIn: parent
                width: 36; height: 36
                visible: sp.busy
                Rectangle { anchors.fill: parent; radius: 18; color: "transparent"; border.width: 3; border.color: Theme.fill2 }
                Canvas {
                    id: arc
                    anchors.fill: parent
                    property color c: Theme.accent
                    onCChanged: requestPaint()
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.reset()
                        ctx.lineWidth = 3
                        ctx.lineCap = "round"
                        ctx.strokeStyle = c
                        ctx.beginPath()
                        ctx.arc(18, 18, 16.5, -Math.PI / 2, Math.PI / 4)
                        ctx.stroke()
                    }
                    RotationAnimator on rotation {
                        from: 0; to: 360; duration: 900; loops: Animation.Infinite
                        running: sp.busy && sp.visible && !Theme.reduceMotion
                    }
                }
            }
        }
        Text {
            Layout.fillWidth: true
            Layout.topMargin: 6
            text: sp.title
            color: Theme.text
            font.pixelSize: 20
            font.weight: Font.Bold
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            Accessible.role: Accessible.Heading
        }
        Text {
            Layout.fillWidth: true
            visible: text !== ""
            text: sp.description
            color: Theme.text2
            font.pixelSize: Theme.fsBody
            lineHeight: 1.25
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            textFormat: Text.StyledText
        }
        Button {
            Layout.alignment: Qt.AlignHCenter
            Layout.topMargin: 8
            visible: sp.action !== ""
            text: sp.action
            accent: true
            onClicked: sp.activated()
        }
    }
}
