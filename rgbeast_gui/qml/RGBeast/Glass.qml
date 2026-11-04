import QtQuick
import QtQuick.Effects

// --glass: a translucent surface that blurs what is behind it (blur 28px, saturate 1.7) and carries the
// popover shadow. Where shader effects are unavailable it is the opaque-enough glass colour alone.
Item {
    id: g
    property real radius: 14
    property bool shadow: true
    readonly property bool canBlur: Theme.backdrop !== null && GraphicsInfo.api !== GraphicsInfo.Software
                                    && GraphicsInfo.api !== GraphicsInfo.Unknown

    Shadow {
        visible: g.shadow
        anchors.fill: parent
        radius: g.radius
        offsetY: 18
        spread: 48
        strength: Theme.dark ? 0.55 : 0.16
    }

    // Blurred copy of the window behind this item.
    Item {
        id: blurred
        anchors.fill: parent
        visible: g.canBlur && g.visible && g.width > 0
        layer.enabled: visible
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: mask
        }
        ShaderEffectSource {
            id: src
            anchors.fill: parent
            sourceItem: g.canBlur ? Theme.backdrop : null
            live: true
            hideSource: false
            visible: false
            sourceRect: {
                // Re-evaluated whenever this item moves or resizes.
                g.x; g.y; g.width; g.height; g.parent ? g.parent.x + g.parent.y : 0
                if (!g.canBlur)
                    return Qt.rect(0, 0, 0, 0)
                const p = g.mapToItem(Theme.backdrop, 0, 0)
                return Qt.rect(p.x, p.y, g.width, g.height)
            }
        }
        MultiEffect {
            anchors.fill: parent
            source: src
            blurEnabled: true
            blur: 1.0
            blurMax: 48
            saturation: 0.7
            autoPaddingEnabled: false
        }
    }
    Item {
        id: mask
        anchors.fill: parent
        visible: false
        layer.enabled: true
        Rectangle { anchors.fill: parent; radius: g.radius; color: "black" }
    }

    Rectangle {
        anchors.fill: parent
        radius: g.radius
        color: Theme.glassFill
        border.width: 1
        border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(0, 0, 0, 0.06)
    }
}
