import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T

// A modal sheet over a dimmed window: title, body, and buttons at the bottom right.
T.Popup {
    id: d
    property string title: ""
    property string body: ""
    property real preferredWidth: 420
    default property alias content: extra.data
    property alias buttons: btns.data
    property bool scrollable: false

    parent: T.Overlay.overlay
    modal: true
    focus: true
    dim: true
    anchors.centerIn: parent
    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset, implicitContentHeight + topPadding + bottomPadding)
    width: Math.min(preferredWidth, (parent ? parent.width : 600) - 48)
    height: Math.min(implicitHeight, (parent ? parent.height : 600) - 64)
    padding: 24
    topPadding: 22
    closePolicy: T.Popup.CloseOnEscape | T.Popup.CloseOnPressOutside

    T.Overlay.modal: Item {
        Behavior on opacity { NumberAnimation { duration: Theme.med } }
        // Dims the window's frame only, so nothing shows past its rounded corners.
        Rectangle {
            anchors.fill: parent
            anchors.margins: Theme.windowMargin
            radius: Theme.windowRadius
            color: Theme.scrim
        }
    }
    background: Item {
        Shadow { anchors.fill: parent; radius: Theme.rXl; offsetY: 22; spread: 60; strength: Theme.dark ? 0.7 : 0.2 }
        // A wash of the accent behind the glass, so the sheet floats in light.
        Glow { anchors.fill: parent; radius: Theme.rXl; spread: 70; strength: Theme.dark ? 0.16 : 0.08 }
        Rectangle {
            anchors.fill: parent
            radius: Theme.rXl
            color: Theme.floating
            border.width: 1
            border.color: Theme.hairlineStrong
            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: parent.radius - 1
                gradient: Gradient {
                    GradientStop { position: 0; color: Theme.dark ? Qt.rgba(1, 1, 1, 0.04) : Qt.rgba(1, 1, 1, 0) }
                    GradientStop { position: 0.4; color: "transparent" }
                }
            }
            Rectangle {
                x: parent.radius; width: parent.width - 2 * parent.radius; height: 1
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop { position: 0; color: "transparent" }
                    GradientStop { position: 0.5; color: Theme.rim }
                    GradientStop { position: 1; color: "transparent" }
                }
            }
        }
    }
    enter: Transition {
        ParallelAnimation {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
            NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring }
        }
    }
    exit: Transition {
        ParallelAnimation {
            NumberAnimation { property: "opacity"; to: 0; duration: 140 }
            NumberAnimation { property: "scale"; to: 0.97; duration: 140 }
        }
    }

    contentItem: ColumnLayout {
        spacing: 14
        Text {
            Layout.fillWidth: true
            text: d.title
            visible: text !== ""
            color: Theme.text
            font.pixelSize: 19
            font.weight: Font.Bold
            font.letterSpacing: -0.3
            wrapMode: Text.Wrap
            Accessible.role: Accessible.Heading
        }
        Text {
            Layout.fillWidth: true
            text: d.body
            visible: text !== ""
            color: Theme.text2
            font.pixelSize: Theme.fsBody
            lineHeight: 1.25
            wrapMode: Text.Wrap
        }
        ColumnLayout {
            id: extra
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12
        }
        RowLayout {
            id: btns
            Layout.alignment: Qt.AlignRight
            Layout.topMargin: 6
            spacing: 8
        }
    }
}
