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
                spacing: 2
                Text {
                    Layout.fillWidth: true
                    text: lighting.title
                    color: Theme.text
                    font.pixelSize: page.narrow ? 22 : Theme.fsDisplay
                    font.weight: Font.Bold
                    font.letterSpacing: -0.4
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
                Layout.fillWidth: true
                implicitHeight: preview.contentHeight
                radius: Theme.rMd + 2
                color: Theme.stage
                border.width: 1
                border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.10)
                Rectangle {     // inner glow from the top, like the audius device faceplate
                    anchors.fill: parent
                    anchors.margins: 1
                    radius: parent.radius - 1
                    gradient: Gradient {
                        GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.035) }
                        GradientStop { position: 0.6; color: "transparent" }
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

        // ============================================================ effect
        Card {
            title: "Effect"
            visible: (page.p.modes || []).length > 0
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
            visible: page.p.usesColors === true
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
                            border.width: 1
                            border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.2) : Qt.rgba(0, 0, 0, 0.12)
                            Accessible.ignored: true
                        }
                        TextField {
                            id: hex
                            Layout.preferredWidth: 130
                            font.family: "Adwaita Mono"
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
            visible: page.p.hasBrightness === true || page.p.hasMotion === true
            spacing: 0

            SettingRow {
                visible: page.p.hasBrightness === true
                divider: false
                title: "Brightness"
                description: brightness.value + "%"
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
                description: speed.value + "%"
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
    }
}
