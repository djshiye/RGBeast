import QtQuick
import QtQuick.Shapes

// Stroked vector icons on a 24 x 24 grid (the audius/Wheelguard icon style).
Item {
    id: root
    property string name: ""
    property color color: Theme.text
    property real size: 18
    property real strokeWidth: 1.8

    implicitWidth: size
    implicitHeight: size
    // The layer gives smooth edges in every renderer and obeys ancestor clipping.
    layer.enabled: true
    layer.samples: 4
    layer.smooth: true

    readonly property var paths: ({
        "grid": "M4.5 4.5 H10.5 V10.5 H4.5 Z M13.5 4.5 H19.5 V10.5 H13.5 Z M4.5 13.5 H10.5 V19.5 H4.5 Z M13.5 13.5 H19.5 V19.5 H13.5 Z",
        "board": "M4 4 H20 V20 H4 Z M8 8 H13 V13 H8 Z M16 8 V8.1 M16 12 V12.1 M8 16.5 H16",
        "memory": "M3 7.5 H21 V15.5 H3 Z M6.5 10 V13 M10 10 V13 M14 10 V13 M17.5 10 V13 M5 15.5 V18 M19 15.5 V18 M11.5 15.5 V18",
        "gpu": "M3 7 H21 V16 H3 Z M9.5 11.5 A2.5 2.5 0 1 0 14.5 11.5 A2.5 2.5 0 1 0 9.5 11.5 Z M17 9.5 V13.5 M5 16 V19 H12",
        "fan": "M12 12 C12 6 16.5 5 17.5 7 C18.5 9 15 11 12 12 C18 12 19 16.5 17 17.5 C15 18.5 13 15 12 12 C12 18 7.5 19 6.5 17 C5.5 15 9 13 12 12 C6 12 5 7.5 7 6.5 C9 5.5 11 9 12 12 Z",
        "strip": "M3 12 H21 M6 12 V12.1 M10 12 V12.1 M14 12 V12.1 M18 12 V12.1",
        "cooler": "M12 3.5 C12 3.5 6.5 9.5 6.5 14 A5.5 5.5 0 0 0 17.5 14 C17.5 9.5 12 3.5 12 3.5 Z",
        "question": "M3 12 A9 9 0 1 1 21 12 A9 9 0 1 1 3 12 Z M9.5 9.5 A2.5 2.5 0 1 1 13 11.8 C12.3 12.2 12 12.8 12 13.5 M12 16.8 V16.9",
        "menu": "M4.5 7 H19.5 M4.5 12 H19.5 M4.5 17 H19.5",
        "more": "M6 12 V12.1 M12 12 V12.1 M18 12 V12.1",
        "plus": "M12 5.5 V18.5 M5.5 12 H18.5",
        "minus": "M5.5 12 H18.5",
        "check": "M5 12.5 L10 17.5 L19 7",
        "store": "M12 4 V14 M7.5 9.5 L12 14 L16.5 9.5 M5 17 V19.5 H19 V17",
        "refresh": "M19.5 12 A7.5 7.5 0 1 1 17 6.4 M19.5 4 V8.5 H15",
        "close": "M7 7 L17 17 M17 7 L7 17",
        "minimize": "M7 12.5 H17",
        "maximize": "M7 7 H17 V17 H7 Z",
        "restore": "M7 9.5 H14.5 V17 H7 Z M9.5 7 H17 V14.5",
        "back": "M15 5.5 L8.5 12 L15 18.5",
        "chevron": "M9 6 L15 12 L9 18",
        "power": "M12 3.5 V11.5 M7 6.5 A7.5 7.5 0 1 0 17 6.5",
        "offline": "M3.5 9 A12 12 0 0 1 8 6.4 M20.5 9 A12 12 0 0 0 12.5 5.6 M7 12.6 A7 7 0 0 1 10 10.9 M17 12.6 A7 7 0 0 0 15.4 11.5 M12 17.2 V17.3 M4 4 L20 20",
        "sliders": "M5 7 H12 M16 7 H19 M5 17 H8 M12 17 H19 M14 4.5 V9.5 M10 14.5 V19.5",
        "info": "M3 12 A9 9 0 1 1 21 12 A9 9 0 1 1 3 12 Z M12 11 V16.5 M12 7.7 V7.8",
        "keyboard": "M3 6.5 H21 V17.5 H3 Z M6.5 10 V10.1 M10 10 V10.1 M14 10 V10.1 M17.5 10 V10.1 M7.5 14 H16.5",
        "paint": "M14.5 4.5 L19.5 9.5 L11 18 L6 13 Z M6 13 L4 20 L11 18",
        "scene": "M4 8 H20 V19 H4 Z M6.5 5 H17.5",
        "sparkle": "M12 3 V7 M12 17 V21 M3 12 H7 M17 12 H21 M5.6 5.6 L8.4 8.4 M15.6 15.6 L18.4 18.4 M5.6 18.4 L8.4 15.6 M15.6 8.4 L18.4 5.6",
        "pencil": "M15 5 L19 9 L9 19 H5 V15 Z M13 7 L17 11",
        "trash": "M5 7 H19 M9.5 7 V4.5 H14.5 V7 M7 7 L8 19.5 H16 L17 7"
    })

    Shape {
        width: 24
        height: 24
        scale: root.size / 24
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.GeometryRenderer

        ShapePath {
            strokeColor: root.color
            strokeWidth: root.strokeWidth
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.paths[root.name] || "" }
        }
    }
}
