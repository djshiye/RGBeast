import QtQuick
import QtQuick.Layouts

// A settings row: title and description on the left, the control on the right, a hairline between rows.
Item {
    id: row
    property string title: ""
    property string description: ""
    property bool divider: true
    default property alias control: slot.data
    property bool stacked: false        // control under the text (narrow cards)
    Layout.fillWidth: true
    implicitHeight: grid.implicitHeight + 24

    Rectangle {
        visible: row.divider
        width: parent.width
        height: 1
        color: Theme.hairline
    }
    GridLayout {
        id: grid
        y: 12
        width: parent.width
        columns: row.stacked ? 1 : 2
        columnSpacing: 18
        rowSpacing: 10
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text {
                Layout.fillWidth: true
                text: row.title
                color: Theme.text
                font.pixelSize: Theme.fsBody
                wrapMode: Text.Wrap
            }
            Text {
                Layout.fillWidth: true
                visible: text !== ""
                text: row.description
                color: Theme.text2
                font.pixelSize: Theme.fsCallout
                lineHeight: 1.2
                wrapMode: Text.Wrap
            }
        }
        Item {
            id: slot
            Layout.alignment: row.stacked ? Qt.AlignLeft : (Qt.AlignRight | Qt.AlignVCenter)
            Layout.fillWidth: row.stacked
            implicitWidth: children.length ? children[0].implicitWidth : 0
            implicitHeight: children.length ? children[0].implicitHeight : 0
        }
    }
}
