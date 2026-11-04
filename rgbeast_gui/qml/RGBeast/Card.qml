import QtQuick
import QtQuick.Layouts

// .card: surface, radius 20, layered shadow, hairline inset ring, padding 18 / 20 / 20.
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

    Shadow { anchors.fill: parent; radius: Theme.rLg }
    Rectangle {
        anchors.fill: parent
        radius: Theme.rLg
        color: card.color
        border.width: 1
        border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.055) : Qt.rgba(0, 0, 0, 0.045)
        Behavior on color { ColorAnimation { duration: Theme.med } }
    }
    // The top inner highlight of the dark style (inset 0 1px 0 rgba(255,255,255,.035)).
    Rectangle {
        visible: Theme.dark
        x: Theme.rLg; width: parent.width - 2 * Theme.rLg; y: 1; height: 1
        color: Qt.rgba(1, 1, 1, 0.035)
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
            Layout.minimumHeight: 34
            spacing: 12
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text {
                    Layout.fillWidth: true
                    text: card.title
                    visible: text !== ""
                    color: Theme.text
                    font.pixelSize: Theme.fsTitle
                    font.weight: Font.DemiBold
                    font.letterSpacing: -0.15
                    elide: Text.ElideRight
                }
                Text {
                    Layout.fillWidth: true
                    text: card.subtitle
                    visible: text !== ""
                    color: Theme.text2
                    font.pixelSize: Theme.fsCallout
                    lineHeight: 1.2
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
