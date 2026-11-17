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
    property alias headerRight: headerRightSlot.data
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
            font.pixelSize: 16
            font.weight: Font.Bold
            font.letterSpacing: -0.2
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
        Row { id: headerRightSlot; spacing: 6; Layout.alignment: Qt.AlignVCenter; Layout.leftMargin: 4 }
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

            SectionLabel { text: "Devices"; Layout.topMargin: 8 }
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

            SectionLabel { text: "Scenes"; Layout.topMargin: 20 }
            SideRow {
                Layout.fillWidth: true
                title: "Lights Off"
                swatch: [lighting.lightsOffColor]
                enabled: lighting.status === "ready"
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
                    tipText: applicable ? "Apply “" + modelData.name + "”" : "Can be applied once the lighting service is running"
                    // Rename and delete still work while the service is away; applying does not.
                    applicable: lighting.status === "ready"
                    onClicked: if (applicable) lighting.applyScene(modelData.name)
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
                // Until there is a scene, say what scenes are for.
                subtitle: lighting.scenesList.length === 0 ? "Come back to this look in one click" : ""
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

    // ------------------------------------------------------------------ status chip
    Item {
        id: status
        anchors.bottom: parent.bottom
        width: parent.width
        height: 58
        readonly property bool live: lighting.status === "ready" || lighting.status === "empty"
        Rectangle {
            x: 14
            width: parent.width - 28
            height: 36
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: -3
            radius: 18
            color: Theme.fill
            border.width: 1
            border.color: Theme.hairline
            Row {
                x: 14
                anchors.verticalCenter: parent.verticalCenter
                spacing: 10
                Item {
                    width: 8; height: 8
                    anchors.verticalCenter: parent.verticalCenter
                    Rectangle {     // a slow heartbeat while the service is there
                        id: pulse
                        anchors.centerIn: parent
                        width: 8; height: 8; radius: 4
                        color: Theme.ok
                        visible: status.live
                        opacity: 0
                        SequentialAnimation on scale {
                            running: status.live && !Theme.reduceMotion && side.visible
                            loops: Animation.Infinite
                            NumberAnimation { from: 1; to: 2.6; duration: 1800; easing.type: Easing.OutCubic }
                            PauseAnimation { duration: 600 }
                        }
                        SequentialAnimation on opacity {
                            running: status.live && !Theme.reduceMotion && side.visible
                            loops: Animation.Infinite
                            NumberAnimation { from: 0.5; to: 0; duration: 1800; easing.type: Easing.OutCubic }
                            PauseAnimation { duration: 600 }
                        }
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        width: 14; height: 14; radius: 7
                        color: Theme.alpha(Theme.ok, 0.22)
                        visible: status.live
                    }
                    Rectangle {
                        anchors.fill: parent
                        radius: 4
                        color: status.live ? Theme.ok : Theme.text3
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
    }

    component SectionLabel: Eyebrow {
        Layout.leftMargin: 12
        Layout.bottomMargin: 8
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
        property string tipText: ""
        property bool applicable: true
        readonly property bool isSideRow: true
        signal menuClicked(Item anchor)

        implicitHeight: subtitle !== "" ? 58 + (colors.length ? 8 : 0) : 48
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        Accessible.role: Accessible.ListItem
        Accessible.name: subtitle !== "" ? title + ", " + subtitle : title
        Accessible.selected: selected
        Accessible.description: hasMenu ? "Press the Menu key for rename and delete" : ""
        opacity: enabled ? 1 : 0.45

        // Up and Down move between rows (Tab still reaches the scene buttons).
        function step(forward) {
            let it = row.nextItemInFocusChain(forward)
            while (it && it !== row && !(it.isSideRow && it.visible && it.enabled))
                it = it.nextItemInFocusChain(forward)
            if (it && it !== row)
                it.forceActiveFocus(forward ? Qt.TabFocusReason : Qt.BacktabFocusReason)
        }
        Keys.onDownPressed: step(true)
        Keys.onUpPressed: step(false)
        Keys.onMenuPressed: if (hasMenu) row.menuClicked(more)
        Keys.onPressed: (e) => {
            if (hasMenu && e.key === Qt.Key_F10 && (e.modifiers & Qt.ShiftModifier)) {
                row.menuClicked(more)
                e.accepted = true
            }
        }
        // Right-click opens the scene menu, as it does everywhere else on the desktop.
        TapHandler {
            enabled: row.hasMenu
            acceptedButtons: Qt.RightButton
            onTapped: row.menuClicked(more)
        }
        // Long names are cut short in a 288 px sidebar: show the whole name on hover.
        Tip {
            text: row.tipText !== "" ? row.tipText : (titleText.truncated || subText.truncated) ? row.title + (row.subtitle !== "" ? "\n" + row.subtitle : "") : ""
            shown: row.hovered && !more.hovered
        }

        background: Item {
            Shadow {
                anchors.fill: bgRect
                radius: Theme.rMd
                offsetY: 4
                spread: 14
                strength: Theme.dark ? 0.3 : 0.06
                visible: row.selected
            }
            Rectangle {
                id: bgRect
                anchors.fill: parent
                radius: Theme.rMd
                color: row.selected ? Theme.surface : row.down ? Theme.fill2 : row.hovered ? Theme.fill : "transparent"
                border.width: row.selected ? 1 : 0
                border.color: Theme.hairlineStrong
                Behavior on color { ColorAnimation { duration: Theme.fast } }
                // The selected row's lit rim.
                Rectangle {
                    visible: row.selected
                    x: Theme.rMd; width: parent.width - 2 * Theme.rMd; height: 1
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop { position: 0; color: "transparent" }
                        GradientStop { position: 0.5; color: Theme.rim }
                        GradientStop { position: 1; color: "transparent" }
                    }
                }
            }
            // The selection beam: a short glowing accent bar on the left edge.
            Item {
                x: -1
                width: 3
                height: 22
                anchors.verticalCenter: parent.verticalCenter
                anchors.verticalCenterOffset: row.colors.length ? -4 : 0
                opacity: row.selected ? 1 : 0
                scale: row.selected ? 1 : 0.3
                Behavior on opacity { NumberAnimation { duration: Theme.med } }
                Behavior on scale { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.spring } }
                Glow { anchors.fill: parent; radius: 1.5; spread: 12; strength: Theme.dark ? 0.7 : 0.4 }
                Rectangle {
                    anchors.fill: parent
                    radius: 1.5
                    gradient: Gradient {
                        GradientStop { position: 0; color: Theme.accent2 }
                        GradientStop { position: 1; color: Theme.accent }
                    }
                }
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
                        radius: 11
                        color: row.selected ? "transparent" : Theme.fill
                        border.width: 1
                        border.color: row.selected ? Theme.alpha(Theme.accent, 0.45) : Theme.hairline
                        gradient: row.selected ? tileLit : null
                        Gradient {
                            id: tileLit
                            GradientStop { position: 0; color: Theme.alpha(Theme.accent2, 0.30) }
                            GradientStop { position: 1; color: Theme.alpha(Theme.accent, 0.14) }
                        }
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
                        radius: 11
                        ring: Theme.dark ? Qt.rgba(1, 1, 1, 0.16) : Qt.rgba(0, 0, 0, 0.10)
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1
                    Text {
                        id: titleText
                        Layout.fillWidth: true
                        text: row.title
                        color: row.quiet || !row.applicable ? Theme.text2 : Theme.text
                        font.pixelSize: Theme.fsBody
                        font.weight: row.quiet ? Font.Medium : Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        id: subText
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
                    opacity: row.hovered || row.visualFocus || activeFocus || down ? 1 : 0.0
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
