import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T

// A glass popover menu (.popover + .menu-item): items are {text, icon, danger, action}.
T.Popup {
    id: m
    property var items: []
    signal triggered(string action)

    padding: 6
    modal: false
    focus: true
    closePolicy: T.Popup.CloseOnEscape | T.Popup.CloseOnPressOutside
    implicitWidth: Math.max(220, col.implicitWidth + 12)
    implicitHeight: col.implicitHeight + 12
    transformOrigin: T.Popup.Top

    background: Glass { radius: 14 }
    enter: Transition {
        ParallelAnimation {
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Theme.fast }
            NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: Theme.fast; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut }
        }
    }
    exit: Transition {
        ParallelAnimation {
            NumberAnimation { property: "opacity"; to: 0; duration: 110 }
            NumberAnimation { property: "scale"; to: 0.97; duration: 110 }
        }
    }
    onOpened: if (rep.count) rep.itemAt(0).forceActiveFocus(Qt.TabFocusReason)

    contentItem: ColumnLayout {
        id: col
        spacing: 0
        Repeater {
            id: rep
            model: m.items
            delegate: T.AbstractButton {
                id: it
                required property var modelData
                required property int index
                readonly property bool sep: modelData.separator === true
                Layout.fillWidth: true
                implicitHeight: sep ? 11 : 32
                implicitWidth: sep ? 0 : r.implicitWidth + 20
                enabled: !sep && modelData.enabled !== false
                hoverEnabled: true
                focusPolicy: Qt.StrongFocus
                Accessible.role: Accessible.MenuItem
                Accessible.name: sep ? "" : modelData.text
                Keys.onDownPressed: m.focusAt(index + 1, 1)
                Keys.onUpPressed: m.focusAt(index - 1, -1)
                onClicked: { m.close(); m.triggered(modelData.action) }
                readonly property bool hot: hovered || activeFocus
                background: Item {
                    Rectangle {
                        visible: it.sep
                        anchors.verticalCenter: parent.verticalCenter
                        x: 8; width: parent.width - 16; height: 1
                        color: Theme.hairline
                    }
                    Rectangle {
                        visible: !it.sep
                        anchors.fill: parent
                        radius: 8
                        color: it.hot ? (it.modelData.danger ? Theme.danger : Theme.accent) : "transparent"
                    }
                }
                contentItem: Item {
                    visible: !it.sep
                    RowLayout {
                        id: r
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        spacing: 10
                        Icon {
                            visible: !!it.modelData.icon
                            name: it.modelData.icon || ""
                            size: 16
                            color: it.hot ? Theme.inkOnAccent : it.modelData.danger ? Theme.danger : Theme.text2
                        }
                        Text {
                            Layout.fillWidth: true
                            text: it.modelData.text || ""
                            font.pixelSize: Theme.fsBody
                            color: it.hot ? Theme.inkOnAccent : it.modelData.danger ? Theme.danger : Theme.text
                            elide: Text.ElideRight
                        }
                        Text {
                            visible: !!it.modelData.shortcut
                            text: it.modelData.shortcut || ""
                            font.pixelSize: Theme.fsCaption
                            color: it.hot ? Theme.alpha(Theme.inkOnAccent, 0.75) : Theme.text3
                        }
                    }
                }
            }
        }
    }

    function focusAt(i, step) {
        while (i >= 0 && i < rep.count) {
            const item = rep.itemAt(i)
            if (item && item.enabled) { item.forceActiveFocus(Qt.TabFocusReason); return }
            i += step
        }
    }
}
