import QtQuick

// .toasts: glass pills at the bottom centre, three seconds each.
Item {
    id: root
    function show(text) {
        model.append({ "text": text })
        if (model.count > 3)
            model.remove(0)
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
                required property int index
                width: Math.min(label.implicitWidth + 36, root.width - 48)
                height: 42
                anchors.horizontalCenter: parent ? parent.horizontalCenter : undefined
                opacity: 0
                Accessible.role: Accessible.AlertMessage
                Accessible.name: text
                Glass { anchors.fill: parent; radius: 21 }
                Text {
                    id: label
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, root.width - 84)
                    text: toast.text
                    color: Theme.text
                    font.pixelSize: Theme.fsBody
                    font.weight: Font.Medium
                    elide: Text.ElideRight
                }
                transform: Translate { id: tr; y: 12 }
                Component.onCompleted: { inAnim.start(); life.start() }
                ParallelAnimation {
                    id: inAnim
                    NumberAnimation { target: toast; property: "opacity"; to: 1; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
                    NumberAnimation { target: tr; property: "y"; to: 0; duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
                }
                Timer { id: life; interval: 3000; onTriggered: outAnim.start() }
                SequentialAnimation {
                    id: outAnim
                    ParallelAnimation {
                        NumberAnimation { target: toast; property: "opacity"; to: 0; duration: 180 }
                        NumberAnimation { target: tr; property: "y"; to: 8; duration: 180 }
                    }
                    ScriptAction { script: { for (let i = 0; i < model.count; i++) if (model.get(i).text === toast.text) { model.remove(i); break } } }
                }
            }
        }
    }
}
