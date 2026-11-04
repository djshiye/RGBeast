import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T

Sheet {
    id: prefs
    title: "Preferences"
    preferredWidth: 560
    onAboutToShow: lighting.loadPreferences()

    Flickable {
        id: fl
        Layout.fillWidth: true
        Layout.fillHeight: true
        Layout.preferredHeight: Math.min(col.implicitHeight, Math.max(240, (prefs.parent ? prefs.parent.height : 800) - 200))
        contentHeight: col.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        T.ScrollBar.vertical: Scroller {}

        ColumnLayout {
            id: col
            width: fl.width - 10
            spacing: 0

            Group { text: "Preview" }
            SettingRow {
                divider: false
                title: "Animate Effects"
                description: "Play the selected effect in the preview. Always off when the system asks for reduced motion."
                Switch {
                    checked: lighting.animate
                    Accessible.name: "Animate Effects"
                    onToggled: lighting.setAnimate(checked)
                }
            }

            Group { text: "Lighting Service" }
            SettingRow {
                divider: false
                title: "Restore After Sleep"
                description: "Re-apply the lighting when the computer wakes up."
                Switch {
                    checked: lighting.restoreOnResume
                    enabled: lighting.status !== "disconnected"
                    Accessible.name: "Restore After Sleep"
                    onToggled: lighting.setRestoreOnResume(checked)
                }
            }
            SettingRow {
                title: "Service Version"
                Text {
                    text: lighting.status === "disconnected" ? "Not connected" : lighting.version
                    color: Theme.text2
                    font.pixelSize: Theme.fsBody
                }
            }
            SettingRow {
                id: logRow
                property bool open: false
                title: "Detection Log"
                description: "What the service found and skipped."
                IconButton {
                    glyph: "chevron"
                    tip: logRow.open ? "Hide" : "Show"
                    rotation: logRow.open ? 90 : 0
                    Behavior on rotation { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
                    onClicked: logRow.open = !logRow.open
                }
            }
            Rectangle {
                Layout.fillWidth: true
                visible: logRow.open
                implicitHeight: logText.implicitHeight + 20
                radius: Theme.rSm
                color: Theme.sunken
                TextEdit {
                    id: logText
                    x: 12; y: 10
                    width: parent.width - 24
                    readOnly: true
                    selectByMouse: true
                    text: lighting.discoveryLog.length ? lighting.discoveryLog.join("\n") : "Nothing reported."
                    color: Theme.text2
                    font.family: "Adwaita Mono"
                    font.pixelSize: Theme.fsCallout
                    wrapMode: TextEdit.Wrap
                    selectionColor: Theme.alpha(Theme.accent, 0.45)
                }
            }

            Group { text: "Addressable Headers"; visible: lighting.headerZones.length > 0 }
            Text {
                Layout.fillWidth: true
                Layout.bottomMargin: 4
                visible: lighting.headerZones.length > 0
                text: "Effects and colours reach every header whatever its length; the controller cannot count the LEDs on a chain. Set the count to paint single lights and see them in the preview. Arctic P12 A-RGB fans have 12 LEDs each, so three daisy-chained fans are 36."
                color: Theme.text2
                font.pixelSize: Theme.fsCallout
                lineHeight: 1.2
                wrapMode: Text.Wrap
            }
            Repeater {
                model: lighting.headerZones
                delegate: SettingRow {
                    required property var modelData
                    required property int index
                    divider: index > 0
                    title: modelData.title
                    description: "LEDs connected"
                    SpinBox {
                        from: modelData.min
                        to: modelData.max
                        value: modelData.leds
                        Accessible.name: modelData.title + " LED count"
                        onValueModified: lighting.resizeZone(modelData.device, modelData.zone, value)
                    }
                }
            }
        }
    }

    buttons: [ Button { text: "Done"; accent: true; onClicked: prefs.close() } ]

    component Group: Text {
        Layout.fillWidth: true
        Layout.topMargin: 16
        Layout.bottomMargin: 2
        color: Theme.text2
        font.pixelSize: Theme.fsCallout
        font.weight: Font.DemiBold
        font.letterSpacing: 0.12
    }
}
