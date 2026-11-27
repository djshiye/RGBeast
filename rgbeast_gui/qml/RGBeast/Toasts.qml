import QtQuick
import QtQuick.Layouts

// .toasts: glass pills at the bottom centre. Info stays 4 s, errors 7 s, a toast with an action
// (Undo) 6 s; pointing at a toast holds it. The same message again restarts the one on screen
// instead of stacking a copy (a slider dragged while the service says no sends many).
Item {
    id: root
    signal action()
    property int serial: 0
    // A toast on screen still offers its action (Ctrl+Z follows the Undo button: gone with it).
    property bool offersAction: false

    function refreshAction() {
        let any = false
        for (let i = 0; i < model.count; i++)
            if (model.get(i).actionLabel !== "") any = true
        offersAction = any
    }

    function show(text, kind, actionLabel) {
        kind = kind || "info"
        actionLabel = actionLabel || ""
        // Only the newest toast can offer an action: the backend keeps one undo.
        if (actionLabel !== "")
            for (let i = 0; i < model.count; i++)
                model.setProperty(i, "actionLabel", "")
        for (let i = 0; i < model.count; i++) {
            if (model.get(i).text === text && model.get(i).kind === kind) {
                model.setProperty(i, "actionLabel", actionLabel)
                model.setProperty(i, "serial", ++serial)
                refreshAction()
                return
            }
        }
        model.append({ "text": text, "kind": kind, "actionLabel": actionLabel, "serial": ++serial })
        if (model.count > 3)
            model.remove(0)
        refreshAction()
    }
    function dismiss(serial) {
        for (let i = 0; i < model.count; i++)
            if (model.get(i).serial === serial) { model.remove(i); break }
        refreshAction()
    }
    ListModel { id: model }
    Column {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 22
        spacing: 8
        Repeater {
            model: model
            delegate: Item {
                id: toast
                required property string text
                required property string kind
                required property string actionLabel
                required property int serial
                readonly property bool error: kind === "error"
                width: Math.min(row.implicitWidth + 36, root.width - 48)
                height: 42
                anchors.horizontalCenter: parent ? parent.horizontalCenter : undefined
                opacity: 0
                Accessible.role: Accessible.AlertMessage
                Accessible.name: (error ? "Error: " : "") + text
                Glass { anchors.fill: parent; radius: 21 }
                Rectangle {     // errors carry a thin danger edge, so they read as errors at a glance
                    visible: toast.error
                    anchors.fill: parent
                    radius: 21
                    color: "transparent"
                    border.width: 1
                    border.color: Theme.alpha(Theme.danger, 0.55)
                }
                HoverHandler { id: hover }
                RowLayout {
                    id: row
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, root.width - 84)
                    spacing: 10
                    Icon {
                        visible: toast.error
                        name: "alert"
                        size: 16
                        color: Theme.danger
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.maximumWidth: root.width - 84 - (act.visible ? act.implicitWidth + 10 : 0) - (toast.error ? 26 : 0)
                        text: toast.text
                        color: Theme.text
                        font.pixelSize: Theme.fsBody
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    Button {
                        id: act
                        visible: toast.actionLabel !== ""
                        text: toast.actionLabel
                        small: true
                        plain: true
                        Layout.rightMargin: -8
                        onClicked: { root.dismiss(toast.serial); root.action() }
                    }
                }
                transform: Translate { id: tr; y: 12 }
                Component.onCompleted: inAnim.start()
                onSerialChanged: { outAnim.stop(); opacity = 1; tr.y = 0; life.restart() }
                ParallelAnimation {
                    id: inAnim
                    NumberAnimation { target: toast; property: "opacity"; to: 1; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
                    NumberAnimation { target: tr; property: "y"; to: 0; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
                }
                Timer {
                    id: life
                    interval: toast.error ? 7000 : toast.actionLabel !== "" ? 6000 : 4000
                    running: !hover.hovered && !act.activeFocus
                    onTriggered: outAnim.start()
                }
                SequentialAnimation {
                    id: outAnim
                    ParallelAnimation {
                        NumberAnimation { target: toast; property: "opacity"; to: 0; duration: 180 }
                        NumberAnimation { target: tr; property: "y"; to: 8; duration: 180 }
                    }
                    ScriptAction { script: root.dismiss(toast.serial) }
                }
            }
        }
    }
}
