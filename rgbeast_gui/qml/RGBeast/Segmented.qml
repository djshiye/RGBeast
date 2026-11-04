import QtQuick
import QtQuick.Templates as T

// .seg: a segmented control with a sliding thumb.
Item {
    id: seg
    property var model: []          // labels
    property int currentIndex: 0
    property bool small: false
    signal activated(int index)

    readonly property real itemH: small ? 24 : 28
    implicitHeight: itemH + 6
    implicitWidth: row.implicitWidth + 6
    Accessible.role: Accessible.PageTabList

    Rectangle {
        anchors.fill: parent
        radius: seg.small ? 10 : 12
        color: Theme.fill
        border.width: 1
        border.color: Theme.hairline
    }
    Rectangle {
        id: thumb
        readonly property Item target: rep.count > seg.currentIndex && seg.currentIndex >= 0 ? rep.itemAt(seg.currentIndex) : null
        visible: target !== null
        y: 3
        height: seg.itemH
        x: target ? target.x + 3 : 3
        width: target ? target.width : 0
        radius: seg.small ? 7 : 9
        color: Theme.dark ? Theme.fill3 : "#ffffff"
        border.width: Theme.dark ? 0 : 0.5
        border.color: Qt.rgba(0, 0, 0, 0.05)
        Behavior on x { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
        Behavior on width { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
        Rectangle {     // drop shadow under the thumb
            z: -1
            anchors.fill: parent
            anchors.topMargin: 1
            anchors.bottomMargin: -1
            radius: parent.radius
            color: "black"
            opacity: Theme.dark ? 0.32 : 0.10
        }
    }
    Row {
        id: row
        x: 3; y: 3
        Repeater {
            id: rep
            model: seg.model
            delegate: T.AbstractButton {
                id: btn
                required property var modelData
                required property int index
                readonly property bool on: index === seg.currentIndex
                height: seg.itemH
                width: Math.max(seg.small ? 0 : 64, lbl.implicitWidth + (seg.small ? 20 : 28))
                hoverEnabled: true
                focusPolicy: Qt.StrongFocus
                Accessible.role: Accessible.PageTab
                Accessible.name: String(modelData)
                Accessible.checked: on
                onClicked: if (!on) seg.activated(index)
                Keys.onLeftPressed: if (index > 0) { seg.activated(index - 1); rep.itemAt(index - 1).forceActiveFocus(Qt.TabFocusReason) }
                Keys.onRightPressed: if (index < rep.count - 1) { seg.activated(index + 1); rep.itemAt(index + 1).forceActiveFocus(Qt.TabFocusReason) }
                background: Item { FocusRing { shown: btn.visualFocus; targetRadius: 9; offset: 1 } }
                contentItem: Text {
                    id: lbl
                    text: String(btn.modelData)
                    font.pixelSize: seg.small ? Theme.fsCallout : Theme.fsBody
                    font.weight: Font.Medium
                    color: btn.on || btn.hovered ? Theme.text : Theme.text2
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    Behavior on color { ColorAnimation { duration: Theme.fast } }
                }
            }
        }
    }
}
