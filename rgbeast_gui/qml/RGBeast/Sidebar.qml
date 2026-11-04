import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import RGBeast.Widgets

// Devices (with "All Devices" first) and scenes, on the window background; the selection is a raised
// surface. The status line at the bottom says how many devices the service has.
Item {
    id: side
    signal deviceChosen(string id)
    signal menuRequested(Item anchor)
    signal saveSceneRequested()
    signal renameSceneRequested(string name)
    signal deleteSceneRequested(string name)
    property alias headerLeft: headerLeftSlot.data
    readonly property Item menuButton: menuButton

    // ------------------------------------------------------------------ header
    Item {
        width: parent.width
        height: Theme.titlebar
        DragHandler {
            target: null
            onActiveChanged: if (active) side.Window.window.startSystemMove()
        }
        TapHandler {
            onDoubleTapped: {
                const w = side.Window.window
                w.visibility = (w.visibility === Window.Maximized) ? Window.Windowed : Window.Maximized
            }
        }
    }
    RowLayout {
        id: header
        x: 14
        width: parent.width - 24
        height: Theme.titlebar
        spacing: 10
        Row { id: headerLeftSlot; spacing: 6; Layout.alignment: Qt.AlignVCenter }
        Image {
            Layout.preferredWidth: 26
            Layout.preferredHeight: 26
            source: iconUrl
            sourceSize: Qt.size(52, 52)
            smooth: true
            mipmap: true
            Accessible.ignored: true
        }
        Text {
            Layout.fillWidth: true
            text: "RGBeast"
            color: Theme.text
            font.pixelSize: Theme.fsTitle
            font.weight: Font.DemiBold
            elide: Text.ElideRight
            Accessible.role: Accessible.Heading
        }
        IconButton {
            id: menuButton
            glyph: "menu"
            tip: "Main Menu"
            size: 32
            onClicked: side.menuRequested(menuButton)
        }
    }

    // ------------------------------------------------------------------ lists
    Flickable {
        id: flick
        anchors.top: header.bottom
        anchors.bottom: status.top
        width: parent.width
        contentHeight: col.implicitHeight + 12
        clip: true
        T.ScrollBar.vertical: Scroller {}
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: col
            x: 10
            width: flick.width - 20
            spacing: 2

            SectionLabel { text: "Devices"; Layout.topMargin: 4 }
            Repeater {
                model: lighting.devices
                delegate: SideRow {
                    required property var modelData
                    Layout.fillWidth: true
                    title: modelData.name
                    subtitle: modelData.subtitle
                    glyph: modelData.icon
                    colors: modelData.colors
                    selected: modelData.selected
                    onClicked: side.deviceChosen(modelData.id)
                }
            }

            SectionLabel { text: "Scenes"; Layout.topMargin: 14 }
            SideRow {
                Layout.fillWidth: true
                title: "Lights Off"
                swatch: [lighting.lightsOffColor]
                onClicked: lighting.lightsOff()
            }
            Repeater {
                model: lighting.scenesList
                delegate: SideRow {
                    id: sceneRow
                    required property var modelData
                    Layout.fillWidth: true
                    title: modelData.name
                    swatch: modelData.colors
                    hasMenu: true
                    onClicked: lighting.applyScene(modelData.name)
                    onMenuClicked: (anchor) => {
                        sceneMenu.scene = modelData.name
                        sceneMenu.parent = anchor
                        sceneMenu.x = anchor.width - sceneMenu.implicitWidth
                        sceneMenu.y = anchor.height + 4
                        sceneMenu.open()
                    }
                }
            }
            SideRow {
                Layout.fillWidth: true
                title: "Save Current as Scene…"
                glyph: "plus"
                quiet: true
                enabled: lighting.status === "ready"
                onClicked: side.saveSceneRequested()
            }
        }
    }

    PopMenu {
        id: sceneMenu
        property string scene: ""
        items: [
            { text: "Rename…", icon: "pencil", action: "rename" },
            { text: "Delete", icon: "trash", action: "delete", danger: true }
        ]
        onTriggered: (action) => {
            if (action === "rename") side.renameSceneRequested(scene)
            else side.deleteSceneRequested(scene)
        }
    }

    // ------------------------------------------------------------------ status (.device-chip)
    Item {
        id: status
        anchors.bottom: parent.bottom
        width: parent.width
        height: 44
        Rectangle { width: parent.width - 28; x: 14; height: 1; color: Theme.hairline }
        Row {
            x: 18
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 1
            spacing: 9
            Item {
                width: 8; height: 8
                anchors.verticalCenter: parent.verticalCenter
                readonly property bool live: lighting.status === "ready" || lighting.status === "empty"
                Rectangle {
                    anchors.centerIn: parent
                    width: 14; height: 14; radius: 7
                    color: Theme.alpha(Theme.ok, 0.22)
                    visible: parent.live
                }
                Rectangle {
                    anchors.fill: parent
                    radius: 4
                    color: parent.live ? Theme.ok : Theme.text3
                }
            }
            Text {
                text: lighting.statusText
                color: Theme.text2
                font.pixelSize: Theme.fsCallout
                font.weight: Font.DemiBold
                Accessible.role: Accessible.StatusBar
                Accessible.name: text
            }
        }
    }

    component SectionLabel: Text {
        Layout.leftMargin: 10
        Layout.bottomMargin: 6
        color: Theme.text2
        font.pixelSize: Theme.fsCallout
        font.weight: Font.DemiBold
        font.letterSpacing: 0.12
    }

    component SideRow: T.AbstractButton {
        id: row
        property string title: ""
        property string subtitle: ""
        property string glyph: ""
        property var colors: []
        property var swatch: []
        property bool selected: false
        property bool quiet: false
        property bool hasMenu: false
        signal menuClicked(Item anchor)

        implicitHeight: subtitle !== "" ? 58 + (colors.length ? 8 : 0) : 48
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.ListItem
        Accessible.name: subtitle !== "" ? title + ", " + subtitle : title
        Accessible.selected: selected
        opacity: enabled ? 1 : 0.45

        background: Item {
            Shadow {
                anchors.fill: bgRect
                radius: Theme.rMd
                offsetY: 3
                spread: 10
                strength: Theme.dark ? 0.25 : 0.05
                visible: row.selected
            }
            Rectangle {
                id: bgRect
                anchors.fill: parent
                radius: Theme.rMd
                color: row.selected ? Theme.surface : row.down ? Theme.fill2 : row.hovered ? Theme.fill : "transparent"
                border.width: row.selected ? 1 : 0
                border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.05)
                Behavior on color { ColorAnimation { duration: Theme.fast } }
            }
            FocusRing { shown: row.visualFocus; targetRadius: Theme.rMd; offset: 0 }
        }

        contentItem: Item {
            RowLayout {
                id: line
                x: 10
                width: parent.width - 16
                height: 40
                y: row.subtitle !== "" ? 9 : 4
                spacing: 12

                // .glyph-tile, or a scene swatch
                Item {
                    Layout.preferredWidth: 34
                    Layout.preferredHeight: 34
                    Rectangle {
                        anchors.fill: parent
                        visible: row.swatch.length === 0
                        radius: 10
                        color: row.selected ? Theme.alpha(Theme.accent, 0.16) : Theme.fill
                        Icon {
                            anchors.centerIn: parent
                            name: row.glyph
                            size: 19
                            color: row.selected ? Theme.accentText : Theme.text
                        }
                    }
                    ColorStrip {
                        anchors.fill: parent
                        visible: row.swatch.length > 0
                        colors: row.swatch
                        radius: 10
                        ring: Theme.dark ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(0, 0, 0, 0.10)
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1
                    Text {
                        Layout.fillWidth: true
                        text: row.title
                        color: row.quiet ? Theme.text2 : Theme.text
                        font.pixelSize: Theme.fsBody
                        font.weight: row.quiet ? Font.Medium : Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        visible: row.subtitle !== ""
                        text: row.subtitle
                        color: Theme.text2
                        font.pixelSize: Theme.fsCallout
                        elide: Text.ElideRight
                    }
                }
                IconButton {
                    id: more
                    visible: row.hasMenu
                    glyph: "more"
                    tip: "Scene Actions"
                    opacity: row.hovered || activeFocus || down ? 1 : 0.0
                    Behavior on opacity { NumberAnimation { duration: Theme.fast } }
                    onClicked: row.menuClicked(more)
                }
            }
            ColorStrip {
                visible: row.colors.length > 0
                x: 56
                width: parent.width - 66
                y: line.y + line.height + 3
                height: 3
                radius: 1.5
                colors: row.colors
            }
        }
    }
}
