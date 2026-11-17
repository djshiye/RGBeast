import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import RGBeast.Widgets

// The editor for one device (or All Devices): the live preview, effect, colour, adjustments, zones.
// Every change applies at once (70 ms debounce in the backend).
Flickable {
    id: page
    readonly property var p: lighting.page
    readonly property bool narrow: width < 640
    contentHeight: inner.implicitHeight + 6 + 36
    boundsBehavior: Flickable.StopAtBounds
    clip: true
    T.ScrollBar.vertical: Scroller {}
    // Same right edge on every page (.page padding 6 18 36 28), centred up to 980 px.
    readonly property real side: narrow ? 14 : 24
    // The sidebar row of the page on show: its kind names the hero's eyebrow, its summary the readout.
    readonly property var row: {
        for (const d of (lighting.devices || []))
            if (d.selected)
                return d
        return ({})
    }
    readonly property string kind: ({ grid: "Every Device", board: "Motherboard", memory: "Memory",
                                       gpu: "Graphics Card", keyboard: "Keyboard", fan: "Fans",
                                       strip: "Light Strip", cooler: "Cooler" })[row.icon] || "Device"

    ColumnLayout {
        id: inner
        y: 6
        x: Math.max(page.side, (page.width - width) / 2)
        width: Math.min(980, page.width - 2 * page.side)
        spacing: Theme.gap

        // ============================================================ hero: name + preview
        Card {
            padTop: 22
            padSide: page.narrow ? 16 : 24
            padBottom: 22
            spacing: 16

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 3
                RowLayout {
                    Layout.fillWidth: true
                    Layout.bottomMargin: 4
                    spacing: 10
                    Rectangle {
                        implicitWidth: 3; implicitHeight: 12; radius: 1.5
                        gradient: Gradient {
                            GradientStop { position: 0; color: Theme.accent2 }
                            GradientStop { position: 1; color: Theme.accent }
                        }
                        Glow { anchors.fill: parent; radius: 1.5; spread: 10; strength: Theme.dark ? 0.5 : 0.25 }
                    }
                    Eyebrow {
                        Layout.fillWidth: true
                        text: page.kind
                        color: Theme.accentText
                    }
                    // A readout of the effect and the light count, monospaced like an instrument.
                    Rectangle {
                        visible: !lighting.isGroup && (page.row.subtitle || "") !== "" && !page.narrow
                        implicitWidth: readout.implicitWidth + 20
                        implicitHeight: 22
                        radius: 11
                        color: Theme.fill
                        border.width: 1
                        border.color: Theme.hairline
                        Text {
                            id: readout
                            anchors.centerIn: parent
                            text: (page.row.subtitle || "").toUpperCase()
                            color: Theme.text2
                            font.family: Theme.mono
                            font.pixelSize: Theme.fsCaption
                            font.letterSpacing: 0.6
                        }
                    }
                }
                Text {
                    Layout.fillWidth: true
                    text: lighting.title
                    color: Theme.text
                    font.pixelSize: page.narrow ? 24 : Theme.fsDisplay
                    font.weight: Font.Bold
                    font.letterSpacing: -0.7
                    elide: Text.ElideRight
                    Accessible.role: Accessible.Heading
                }
                Text {
                    Layout.fillWidth: true
                    text: lighting.subtitle
                    visible: text !== ""
                    color: Theme.text2
                    font.pixelSize: Theme.fsBody
                    elide: Text.ElideRight
                }
            }

            // The stage: always dark, lights glow on it.
            Rectangle {
                visible: page.p.locked !== true
                Layout.fillWidth: true
                implicitHeight: preview.contentHeight
                radius: Theme.rMd + 2
                color: Theme.stage
                border.width: 1
                border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.07) : Qt.rgba(0, 0, 0, 0.14)
                // A faint dot grid: the bench the lights sit on.
                Canvas {
                    id: grid
                    anchors.fill: parent
                    anchors.margins: 1
                    renderStrategy: Canvas.Cooperative
                    onWidthChanged: requestPaint()
                    onHeightChanged: requestPaint()
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.reset()
                        const step = 18
                        ctx.fillStyle = Qt.rgba(1, 1, 1, 0.055)
                        for (let y = step / 2; y < height; y += step)
                            for (let x = (width % step) / 2; x < width; x += step)
                                ctx.fillRect(x, y, 1.2, 1.2)
                    }
                }
                Rectangle {     // inner light from the top, fading into the dark
                    anchors.fill: parent
                    anchors.margins: 1
                    radius: parent.radius - 1
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.04) }
                        GradientStop { position: 0.55; color: "transparent" }
                        GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0.35) }
                    }
                }
                // Corner marks, like the viewfinder of an instrument.
                Repeater {
                    model: 4
                    delegate: Item {
                        required property int index
                        id: corner
                        readonly property bool atRight: index % 2 === 1
                        readonly property bool atBottom: index >= 2
                        width: 12; height: 12
                        x: atRight ? parent.width - width - 10 : 10
                        y: atBottom ? parent.height - height - 10 : 10
                        Rectangle { width: 12; height: 1.5; y: corner.atBottom ? 10.5 : 0; color: Qt.rgba(1, 1, 1, 0.22) }
                        Rectangle { width: 1.5; height: 12; x: corner.atRight ? 10.5 : 0; color: Qt.rgba(1, 1, 1, 0.22) }
                    }
                }
                LedPreview {
                    id: preview
                    anchors.fill: parent
                    zones: lighting.preview
                    effect: lighting.effect
                    animate: lighting.animate && !Theme.reduceMotion && page.visible
                    labelColor: Qt.rgba(1, 1, 1, 0.55)
                    Accessible.role: Accessible.Graphic
                    Accessible.name: "Lighting preview"
                    onLedClicked: (zone, led) => lighting.paintLed(zone, led)
                    TapHandler {
                        enabled: page.p.paintBar === true
                        onTapped: (ev) => preview.clickAt(ev.position.x, ev.position.y)
                    }
                    HoverHandler { cursorShape: page.p.paintBar ? Qt.CrossCursor : Qt.ArrowCursor }
                }
            }

            RowLayout {
                visible: page.p.paintBar === true
                Layout.alignment: Qt.AlignHCenter
                spacing: 10
                Icon { name: "paint"; size: 15; color: Theme.text3 }
                Text {
                    text: "Click a light to paint it with the current colour."
                    color: Theme.text2
                    font.pixelSize: Theme.fsCallout
                }
                Button { text: "Fill All"; small: true; plain: true; onClicked: lighting.fillAll() }
            }
        }

        // ============================================================ untested device, still locked
        Card {
            visible: page.p.locked === true
            spacing: 14
            RowLayout {
                Layout.fillWidth: true
                spacing: 14
                Rectangle {
                    Layout.alignment: Qt.AlignTop
                    implicitWidth: 40; implicitHeight: 40
                    radius: Theme.rMd
                    color: Theme.fill
                    Icon { anchors.centerIn: parent; name: "keyboard"; size: 22; color: Theme.text2 }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 4
                    Text {
                        Layout.fillWidth: true
                        text: "Not Tried on a Real Keyboard Yet"
                        color: Theme.text
                        font.pixelSize: Theme.fsHeadline
                        font.weight: Font.DemiBold
                        wrapMode: Text.WordWrap
                        Accessible.role: Accessible.Heading
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "RGBeast drives the " + (page.p.deviceName || "keyboard") + " the way Glorious’s own app does, "
                              + "but nobody has tried it on one yet. Until you turn lighting control on, RGBeast sends "
                              + "nothing to the keyboard and it keeps the lighting it has."
                        color: Theme.text2
                        font.pixelSize: Theme.fsBody
                        wrapMode: Text.WordWrap
                        lineHeight: 1.15
                    }
                }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Item { Layout.preferredWidth: page.narrow ? 0 : 40 }
                Button {
                    text: "Turn On Lighting Control"
                    accent: true
                    onClicked: lighting.setUnlocked(true)
                }
                Text {
                    Layout.fillWidth: true
                    text: "You can turn it off again here."
                    color: Theme.text3
                    font.pixelSize: Theme.fsCallout
                    wrapMode: Text.WordWrap
                }
            }
        }

        // ============================================================ effect
        Card {
            title: "Effect"
            visible: (page.p.modes || []).length > 0 && page.p.locked !== true
            Flow {
                Layout.fillWidth: true
                spacing: 8
                Repeater {
                    model: page.p.modes || []
                    delegate: Pill {
                        required property var modelData
                        text: modelData.name
                        on: modelData.id === page.p.mode
                        onClicked: lighting.selectMode(modelData.id)
                    }
                }
            }
        }

        // ============================================================ colour
        Card {
            id: colorCard
            title: "Colour"
            visible: page.p.usesColors === true && page.p.locked !== true
            readonly property bool stacked: width < 600
            headerExtra: [
                Row {
                    spacing: 8
                    visible: page.p.multi === true
                    Repeater {
                        model: page.p.slots || []
                        delegate: Swatch {
                            required property var modelData
                            required property int index
                            anchors.verticalCenter: parent.verticalCenter
                            size: 26
                            swatch: modelData.color
                            label: modelData.label
                            selected: index === page.p.selectedSlot
                            onClicked: lighting.selectSlot(index)
                        }
                    }
                    IconButton {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: page.p.canAddSlot === true
                        glyph: "plus"; tip: "Add Colour"
                        onClicked: lighting.addSlot()
                    }
                    IconButton {
                        anchors.verticalCenter: parent.verticalCenter
                        visible: page.p.canRemoveSlot === true
                        glyph: "minus"; tip: "Remove Colour"
                        onClicked: lighting.removeSlot()
                    }
                }
            ]

            GridLayout {
                Layout.fillWidth: true
                columns: colorCard.stacked ? 1 : 2
                columnSpacing: 28
                rowSpacing: 18

                ColorWheel {
                    id: wheel
                    Layout.preferredWidth: 220
                    Layout.preferredHeight: 220
                    Layout.alignment: Qt.AlignHCenter | Qt.AlignVCenter
                    color: page.p.color || "#FFFFFF"
                    activeFocusOnTab: true
                    property bool keyboard: true
                    Keys.onPressed: (e) => { keyboard = true; e.accepted = false }
                    Accessible.role: Accessible.Slider
                    Accessible.name: "Colour wheel, " + wheel.color
                    onPicked: (hex) => lighting.pickColor(hex)
                    DragHandler {
                        target: null
                        dragThreshold: 0
                        onActiveChanged: if (!active) wheel.release()
                        onCentroidChanged: if (active) wheel.move(centroid.position.x, centroid.position.y)
                    }
                    TapHandler {
                        gesturePolicy: TapHandler.DragThreshold
                        onPressedChanged: if (pressed) { wheel.keyboard = false; wheel.forceActiveFocus(Qt.MouseFocusReason); wheel.press(point.position.x, point.position.y) } else wheel.release()
                    }
                    HoverHandler { cursorShape: Qt.CrossCursor }
                    Keys.onLeftPressed: (e) => wheel.nudge((e.modifiers & Qt.ShiftModifier) ? -10 : -2, 0)
                    Keys.onRightPressed: (e) => wheel.nudge((e.modifiers & Qt.ShiftModifier) ? 10 : 2, 0)
                    Keys.onUpPressed: (e) => wheel.nudge(0, (e.modifiers & Qt.ShiftModifier) ? 10 : 2)
                    Keys.onDownPressed: (e) => wheel.nudge(0, (e.modifiers & Qt.ShiftModifier) ? -10 : -2)
                    FocusRing { shown: wheel.activeFocus && wheel.keyboard; targetRadius: 110; offset: 2 }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 18

                    GridLayout {
                        Layout.fillWidth: true
                        columns: 6
                        columnSpacing: 8
                        rowSpacing: 12
                        Repeater {
                            model: lighting.palette
                            delegate: Item {
                                required property string modelData
                                Layout.fillWidth: true
                                implicitHeight: 30
                                Swatch {
                                    anchors.centerIn: parent
                                    size: 28
                                    swatch: parent.modelData
                                    label: parent.modelData
                                    onClicked: lighting.pickColor(parent.modelData)
                                }
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        Rectangle {
                            Layout.preferredWidth: 44
                            Layout.preferredHeight: 44
                            radius: Theme.rMd
                            color: page.p.color || "#FFFFFF"
                            Glow { anchors.fill: parent; radius: parent.radius; color: parent.color; spread: 22; strength: Theme.dark ? 0.6 : 0.35 }
                            border.width: 1
                            border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.2) : Qt.rgba(0, 0, 0, 0.12)
                            Accessible.ignored: true
                        }
                        TextField {
                            id: hex
                            Layout.preferredWidth: 130
                            font.family: Theme.mono
                            font.pixelSize: Theme.fsHeadline
                            maximumLength: 7
                            placeholderText: "#RRGGBB"
                            text: page.p.color || ""
                            Accessible.name: "Hex colour"
                            function commit() {
                                if (text.toUpperCase() === (page.p.color || "")) { invalid = false; return }
                                invalid = !lighting.pickColor(text)
                            }
                            onAccepted: commit()
                            onActiveFocusChanged: if (!activeFocus) { commit(); if (invalid) { text = page.p.color; invalid = false } }
                            onTextEdited: invalid = false
                        }
                        Item { Layout.fillWidth: true }
                    }
                }
            }
        }

        // ============================================================ adjust: brightness and motion
        Card {
            title: "Adjust"
            visible: page.p.locked !== true
                     && (page.p.hasBrightness === true || page.p.hasMotion === true || (page.p.switches || []).length > 0)
            spacing: 0

            SettingRow {
                visible: page.p.hasBrightness === true
                divider: false
                title: "Brightness"
                value: brightness.value + "%"
                stacked: page.narrow
                Slider {
                    id: brightness
                    anchors.fill: parent
                    implicitWidth: page.narrow ? 200 : 300
                    value: page.p.brightness !== undefined ? page.p.brightness : 100
                    Accessible.name: "Brightness"
                    onMoved: lighting.setBrightness(Math.round(value))
                }
            }
            SettingRow {
                visible: page.p.hasSpeed === true
                divider: page.p.hasBrightness === true
                title: "Speed"
                value: speed.value + "%"
                stacked: page.narrow
                Slider {
                    id: speed
                    anchors.fill: parent
                    implicitWidth: page.narrow ? 200 : 300
                    value: page.p.speed !== undefined ? page.p.speed : 50
                    Accessible.name: "Speed"
                    onMoved: lighting.setSpeed(Math.round(value))
                }
            }
            SettingRow {
                visible: (page.p.directions || []).length >= 2
                divider: page.p.hasBrightness === true || page.p.hasSpeed === true
                title: "Direction"
                description: "Which way the effect travels."
                Segmented {
                    model: page.p.directions || []
                    currentIndex: page.p.directionIndex || 0
                    onActivated: (i) => lighting.setDirection(i)
                }
            }
            SettingRow {
                visible: page.p.hasRandom === true
                divider: page.p.hasBrightness === true || page.p.hasSpeed === true || (page.p.directions || []).length >= 2
                title: "Random Colours"
                description: "Let the effect pick its own colours."
                Switch {
                    checked: page.p.random === true
                    Accessible.name: "Random Colours"
                    onToggled: lighting.setRandom(checked)
                }
            }
            Repeater {
                model: page.p.switches || []
                delegate: SettingRow {
                    required property var modelData
                    required property int index
                    divider: index > 0 || page.p.hasBrightness === true || page.p.hasMotion === true
                    title: modelData.name
                    description: modelData.description
                    Switch {
                        checked: modelData.on
                        Accessible.name: modelData.name
                        onToggled: lighting.setSwitch(modelData.id, checked)
                    }
                }
            }
        }

        // ============================================================ zones
        Card {
            title: "Zones"
            subtitle: (page.p.zones || []).some(z => z.sizable)
                      ? "Effects and colours reach every header whatever its length. Set the LED count to paint single lights and see them in the preview (12 per Arctic fan)."
                      : ""
            visible: page.p.showZones === true
            spacing: 0
            Repeater {
                model: page.p.zones || []
                delegate: SettingRow {
                    required property var modelData
                    required property int index
                    divider: index > 0
                    title: modelData.name
                    description: modelData.sizable ? "LEDs connected to this header" : modelData.leds + (modelData.leds === 1 ? " LED" : " LEDs")
                    SpinBox {
                        visible: modelData.sizable
                        from: modelData.min
                        to: modelData.max
                        value: modelData.leds
                        Accessible.name: modelData.name + " LED count"
                        onValueModified: lighting.resizeZone(lighting.currentId, modelData.id, value)
                    }
                }
            }
        }

        // ============================================================ untested device, turned on
        RowLayout {
            visible: page.p.untested === true && page.p.locked !== true
            Layout.fillWidth: true
            Layout.leftMargin: 6
            spacing: 6
            Text {
                Layout.fillWidth: true
                text: "Support for this keyboard has not been confirmed on real hardware yet."
                color: Theme.text3
                font.pixelSize: Theme.fsCallout
                wrapMode: Text.WordWrap
            }
            Button {
                text: "Turn Off Lighting Control"
                small: true
                plain: true
                onClicked: lighting.setUnlocked(false)
            }
        }
    }
}
