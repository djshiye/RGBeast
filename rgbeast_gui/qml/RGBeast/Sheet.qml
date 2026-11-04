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
    padding: 22
    topPadding: 20
    closePolicy: T.Popup.CloseOnEscape | T.Popup.CloseOnPressOutside

    T.Overlay.modal: Rectangle {
        color: Theme.scrim
        Behavior on opacity { NumberAnimation { duration: Theme.med } }
    }
    background: Item {
        Shadow { anchors.fill: parent; radius: Theme.rLg; offsetY: 18; spread: 50; strength: Theme.dark ? 0.6 : 0.18 }
        Rectangle {
            anchors.fill: parent
            radius: Theme.rLg
            color: Theme.surface2
            border.width: 1
            border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(0, 0, 0, 0.06)
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
            font.pixelSize: 17
            font.weight: Font.Bold
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
