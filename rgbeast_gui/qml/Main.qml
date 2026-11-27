import QtQuick
import QtQuick.Layouts
import QtQuick.Window
import RGBeast

Window {
    id: win
    readonly property bool maximized: visibility === Window.Maximized || visibility === Window.FullScreen
    readonly property int margin: maximized ? 0 : 14
    readonly property int cornerRadius: maximized ? 0 : 15
    // Narrow windows show the sidebar or the editor, with a back button (like a navigation split view).
    readonly property bool collapsed: frame.width < 760
    property bool showContent: false

    width: 1120
    height: 800
    minimumWidth: 560 + 2 * margin
    minimumHeight: 520 + 2 * margin
    color: "transparent"
    flags: Qt.Window | Qt.FramelessWindowHint
    title: "RGBeast"

    Binding { target: Theme; property: "dark"; value: appearance.dark }
    Binding { target: Theme; property: "systemAccent"; value: appearance.accent }
    Binding { target: Theme; property: "reduceMotion"; value: appearance.reduceMotion }
    Binding { target: Theme; property: "glass"; value: liquidGlass }
    Binding { target: Theme; property: "backdrop"; value: backdrop }
    Binding { target: Theme; property: "windowMargin"; value: win.margin }
    Binding { target: Theme; property: "windowRadius"; value: win.cornerRadius }

    Shortcut { sequences: [StandardKey.Quit, "Ctrl+Q"]; onActivated: Qt.quit() }
    Shortcut { sequences: ["Ctrl+W"]; onActivated: win.close() }
    Shortcut { sequences: ["Ctrl+R", "F5"]; onActivated: lighting.rescan() }
    Shortcut { sequences: ["Ctrl+,"]; onActivated: prefsSheet.open() }
    Shortcut { sequences: ["Ctrl+?", "Ctrl+Shift+/"]; onActivated: shortcutsSheet.open() }
    Shortcut {
        sequences: ["Ctrl+S"]
        enabled: lighting.status === "ready"
        onActivated: saveSheet.openFor("")
    }
    // Undo the last Lights Off, scene or scene delete (text fields keep Ctrl+Z for themselves).
    // Only while its toast still shows the Undo button, so an old undo never fires minutes later.
    Shortcut { sequences: ["Ctrl+Z"]; enabled: toasts.offersAction; onActivated: lighting.toastAction() }
    Shortcut {
        sequences: ["Alt+Left", "Escape"]
        enabled: win.collapsed && win.showContent
        onActivated: win.showContent = false
    }

    Connections {
        target: lighting
        function onToast(text, kind, action) { toasts.show(text, kind, action) }
        function onApplied() { applied.flash() }
    }

    // Development: --sheet menu|toast opens these for screenshots.
    property string demo: ""
    onDemoChanged: {
        if (demo === "menu") sidebar.menuRequested(sidebar.menuButton)
        else if (demo === "toast") {
            toasts.show("Not allowed to change the lighting from this session", "error", "")
            toasts.show("Scene “Movie Night” applied", "info", "Undo")
        }
    }

    // Up to three distinct, visible colours from the page's preview: the ambient light's palette.
    readonly property var ambient: {
        const seen = {}
        const all = []
        for (const z of (lighting.preview || []))
            for (const h of (z.colors || [])) {
                const c = Qt.color(h)
                if (Math.max(c.r, c.g, c.b) < 0.16 || seen[h])
                    continue
                seen[h] = true
                all.push(h)
            }
        if (all.length <= 3)
            return all
        return [all[0], all[Math.floor(all.length / 2)], all[all.length - 1]]
    }

    function chooseDevice(id) {
        lighting.selectDevice(id)
        showContent = true
        page.contentY = 0
    }

    // ------------------------------------------------------------------ window shadow
    Shadow {
        visible: !win.maximized
        anchors.fill: frame
        radius: win.cornerRadius
        // All of it inside the window's margin (spread / 2 + offsetY <= margin): a shadow cut off
        // at the window's edge shows as a hard line over a bright background.
        offsetY: 3
        spread: 20
        fade: true
        strength: Theme.dark ? 0.5 : 0.18
    }

    // ------------------------------------------------------------------ frame
    Rectangle {
        id: frame
        anchors.fill: parent
        anchors.margins: win.margin
        radius: win.cornerRadius
        color: Theme.bg
        border.width: win.maximized ? 0 : 1
        border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.10) : Qt.rgba(15 / 255, 18 / 255, 40 / 255, 0.14)
        Behavior on color { ColorAnimation { duration: Theme.med } }

        Item {
            id: backdrop      // what glass blurs
            anchors.fill: parent

            // The ambient light: the colours the current page shows, washed across the window.
            Aurora {
                anchors.fill: parent
                colors: win.ambient
                cornerRadius: win.cornerRadius
                running: win.active
            }
            // A faint top-down sheen so the canvas reads as depth, not flat paint.
            Rectangle {
                anchors.fill: parent
                radius: win.cornerRadius
                gradient: Gradient {
                    GradientStop { position: 0; color: Theme.dark ? Qt.rgba(1, 1, 1, 0.025) : Qt.rgba(1, 1, 1, 0.35) }
                    GradientStop { position: 0.5; color: "transparent" }
                }
            }

            // ---------------------------------------------------------- sidebar
            Item {          // the sidebar's glass panel: rounded on the window's side only
                visible: sidebar.visible
                width: sidebar.width
                height: frame.height
                clip: true
                Rectangle {
                    width: parent.width + (win.collapsed ? 0 : win.cornerRadius)
                    height: parent.height
                    radius: win.cornerRadius
                    color: Theme.sidebar
                }
            }
            Sidebar {
                id: sidebar
                visible: !win.collapsed || !win.showContent
                x: 0
                width: win.collapsed ? frame.width : 288
                height: frame.height
                onDeviceChosen: (id) => win.chooseDevice(id)
                onMenuRequested: (anchor) => {
                    mainMenu.parent = anchor
                    mainMenu.x = 0
                    mainMenu.y = anchor.height + 6
                    mainMenu.open()
                }
                onSaveSceneRequested: saveSheet.openFor("")
                onRenameSceneRequested: (name) => saveSheet.openFor(name)
                // Deleting is undoable from the toast, so it does not ask first.
                onDeleteSceneRequested: (name) => lighting.deleteScene(name)
                headerLeft: [
                    WindowButtons {
                        visible: appearance.buttonsLeft.length > 0
                        buttons: appearance.buttonsLeft
                        window: win
                    }
                ]
                // A narrow window shows only the sidebar, so it carries the close button too.
                headerRight: [
                    WindowButtons {
                        visible: win.collapsed && appearance.buttonsRight.length > 0
                        buttons: appearance.buttonsRight
                        window: win
                    }
                ]
            }
            Rectangle {
                visible: !win.collapsed
                x: sidebar.width
                width: 1
                height: frame.height
                gradient: Gradient {
                    GradientStop { position: 0; color: "transparent" }
                    GradientStop { position: 0.12; color: Theme.hairline }
                    GradientStop { position: 0.88; color: Theme.hairline }
                    GradientStop { position: 1; color: "transparent" }
                }
            }

            // ---------------------------------------------------------- content
            Item {
                id: content
                visible: !win.collapsed || win.showContent
                x: win.collapsed ? 0 : sidebar.width + 1
                width: frame.width - x
                height: frame.height

                Item {
                    id: header
                    width: parent.width
                    height: Theme.titlebar
                    DragHandler {
                        target: null
                        onActiveChanged: if (active) win.startSystemMove()
                    }
                    TapHandler {
                        onDoubleTapped: win.visibility = win.maximized ? Window.Windowed : Window.Maximized
                    }
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 14
                        anchors.rightMargin: 14
                        spacing: 10

                        WindowButtons {
                            visible: win.collapsed && appearance.buttonsLeft.length > 0
                            buttons: appearance.buttonsLeft
                            window: win
                        }
                        Button {
                            visible: win.collapsed
                            glyph: "back"
                            text: "Devices"
                            plain: true
                            onClicked: win.showContent = false
                        }
                        Item { Layout.fillWidth: true }

                        // The quiet "Applied" mark: in at once, out 1.2 s later.
                        Row {
                            id: applied
                            spacing: 7
                            opacity: 0
                            Accessible.role: Accessible.StaticText
                            Accessible.name: opacity > 0 ? "Applied" : ""
                            function flash() {
                                fadeOut.stop()
                                opacity = 1
                                hold.restart()
                            }
                            Behavior on opacity { enabled: applied.opacity === 0; NumberAnimation { duration: 150 } }
                            Timer { id: hold; interval: 1200; onTriggered: fadeOut.start() }
                            NumberAnimation { id: fadeOut; target: applied; property: "opacity"; to: 0; duration: 400; easing.type: Easing.OutCubic }
                            Rectangle { width: 8; height: 8; radius: 4; color: Theme.ok; anchors.verticalCenter: parent.verticalCenter }
                            Text { text: "Applied"; color: Theme.text2; font.pixelSize: Theme.fsCallout; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter }
                        }
                        Button {
                            visible: lighting.status === "ready" && lighting.canSave
                            glyph: "store"
                            text: content.width > 640 ? "Store as Power-On" : ""
                            tip: "Store as Power-On Lighting"
                            small: false
                            onClicked: lighting.storeOnDevice()
                        }
                        WindowButtons {
                            Layout.leftMargin: 4
                            visible: appearance.buttonsRight.length > 0
                            buttons: appearance.buttonsRight
                            window: win
                        }
                    }
                }

                Item {
                    id: body
                    anchors.top: header.bottom
                    anchors.bottom: parent.bottom
                    width: parent.width

                    DevicePage {
                        id: page
                        anchors.fill: parent
                        visible: lighting.status === "ready"
                        opacity: visible ? 1 : 0
                        Behavior on opacity { NumberAnimation { duration: Theme.med; easing.type: Easing.Bezier; easing.bezierCurve: Theme.easeOut } }
                    }
                    StatusPage {
                        anchors.fill: parent
                        visible: lighting.status === "loading"
                        busy: true
                        title: "Connecting to the Lighting Service"
                    }
                    StatusPage {
                        anchors.fill: parent
                        visible: lighting.status === "disconnected"
                        icon: "offline"
                        title: "Lighting Service Not Running"
                        description: "RGBeast talks to a small system service, rgbeastd, which is the only part that touches the hardware. Start it with <b>sudo systemctl enable --now rgbeastd</b> in a terminal. RGBeast connects by itself as soon as it starts."
                        action: "Try Again"
                        secondaryAction: "Copy Command"
                        onActivated: lighting.retry()
                        onSecondaryActivated: lighting.copyText("sudo systemctl enable --now rgbeastd")
                    }
                    StatusPage {
                        anchors.fill: parent
                        visible: lighting.status === "empty"
                        icon: "question"
                        title: "No Lighting Devices Found"
                        description: "Nothing answered on USB or SMBus. Check that the i2c-dev module is loaded (<b>sudo modprobe i2c-dev</b>) and that the service can open the device nodes, then scan again. Preferences › Detection Log shows what the service tried."
                        action: lighting.scanning ? "Scanning…" : "Scan Again"
                        actionEnabled: !lighting.scanning
                        secondaryAction: "Open Detection Log"
                        onActivated: lighting.rescan()
                        onSecondaryActivated: prefsSheet.openLog()
                    }
                }
            }
        }

        Toasts {
            id: toasts
            anchors.fill: parent
            z: 20
            onAction: lighting.toastAction()
        }

        // ---------------------------------------------------------- resize edges (frameless)
        Repeater {
            model: win.maximized ? [] : [
                { e: Qt.TopEdge, x: 8, y: -6, w: -16, h: 8, cur: Qt.SizeVerCursor },
                { e: Qt.BottomEdge, x: 8, y: -2, w: -16, h: 8, cur: Qt.SizeVerCursor, bottom: true },
                { e: Qt.LeftEdge, x: -6, y: 8, w: 8, h: -16, cur: Qt.SizeHorCursor },
                { e: Qt.RightEdge, x: -2, y: 8, w: 8, h: -16, cur: Qt.SizeHorCursor, right: true },
                { e: Qt.TopEdge | Qt.LeftEdge, x: -6, y: -6, w: 14, h: 14, cur: Qt.SizeFDiagCursor },
                { e: Qt.TopEdge | Qt.RightEdge, x: -8, y: -6, w: 14, h: 14, cur: Qt.SizeBDiagCursor, right: true },
                { e: Qt.BottomEdge | Qt.LeftEdge, x: -6, y: -8, w: 14, h: 14, cur: Qt.SizeBDiagCursor, bottom: true },
                { e: Qt.BottomEdge | Qt.RightEdge, x: -8, y: -8, w: 14, h: 14, cur: Qt.SizeFDiagCursor, right: true, bottom: true }
            ]
            delegate: Item {
                required property var modelData
                x: modelData.right ? frame.width + modelData.x : modelData.x
                y: modelData.bottom ? frame.height + modelData.y : modelData.y
                width: modelData.w > 0 ? modelData.w : frame.width + modelData.w
                height: modelData.h > 0 ? modelData.h : frame.height + modelData.h
                z: 30
                HoverHandler { cursorShape: modelData.cur }
                DragHandler {
                    target: null
                    onActiveChanged: if (active) win.startSystemResize(modelData.e)
                }
            }
        }
    }

    // ------------------------------------------------------------------ menus and sheets
    PopMenu {
        id: mainMenu
        items: [
            { text: lighting.scanning ? "Scanning…" : "Scan for Devices", icon: "refresh", action: "rescan",
              shortcut: "Ctrl+R", enabled: !lighting.scanning },
            { text: "Save as Scene…", icon: "plus", action: "save", shortcut: "Ctrl+S",
              enabled: lighting.status === "ready" },
            { text: "Preferences", icon: "sliders", action: "prefs", shortcut: "Ctrl+," },
            { separator: true },
            { text: "Keyboard Shortcuts", icon: "keyboard", action: "shortcuts", shortcut: "Ctrl+?" },
            { text: "About RGBeast", icon: "info", action: "about" }
        ]
        onTriggered: (action) => {
            if (action === "rescan") lighting.rescan()
            else if (action === "prefs") prefsSheet.open()
            else if (action === "save") saveSheet.openFor("")
            else if (action === "shortcuts") shortcutsSheet.open()
            else aboutSheet.open()
        }
    }

    PreferencesSheet { id: prefsSheet; objectName: "prefsSheet" }
    ShortcutsSheet { id: shortcutsSheet; objectName: "shortcutsSheet" }
    AboutSheet { id: aboutSheet; objectName: "aboutSheet" }

    Sheet {
        id: saveSheet
        property string renaming: ""
        readonly property bool replaces: nameField.text.trim() !== "" && nameField.text.trim() !== renaming
                                         && lighting.scenesList.some(s => s.name === nameField.text.trim())
        function openFor(name) {
            renaming = name
            nameField.text = name
            nameField.invalid = false
            open()
        }
        title: renaming === "" ? "Save Scene" : "Rename Scene"
        body: renaming === "" ? "The current lighting of every device will be saved under this name." : ""
        onOpened: { nameField.forceActiveFocus(); nameField.selectAll() }
        function commit() {
            const name = nameField.text.trim()
            if (name === "")
                return
            if (renaming === "")
                lighting.saveScene(name)
            else
                lighting.renameScene(renaming, name)
            close()
        }
        TextField {
            id: nameField
            Layout.fillWidth: true
            placeholderText: "Scene name"
            Accessible.name: "Scene name"
            onAccepted: saveSheet.commit()
        }
        Text {
            Layout.fillWidth: true
            visible: saveSheet.replaces
            text: "A scene with this name exists and will be replaced."
            wrapMode: Text.Wrap
            color: Theme.text2
            font.pixelSize: Theme.fsCallout
        }
        buttons: [
            Button { text: "Cancel"; onClicked: saveSheet.close() },
            Button {
                text: saveSheet.replaces ? "Replace" : saveSheet.renaming === "" ? "Save" : "Rename"
                accent: true
                enabled: nameField.text.trim() !== ""
                onClicked: saveSheet.commit()
            }
        ]
    }
}
