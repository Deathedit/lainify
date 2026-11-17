import QtQuick
import com.lainify
import QtQuick.Effects

// The browser's `glass` surface: translucent dark fill, hairline border, soft drop shadow.
Item {
    id: glass

    property real radius: 16
    property bool shadow: true
    default property alias content: inner.data

    RectangularShadow {
        anchors.fill: surface
        visible: glass.shadow
        radius: glass.radius
        offset.y: 12
        blur: 40
        spread: -12
        color: Qt.rgba(0, 0, 0, 0.7)
    }

    Rectangle {
        id: surface
        anchors.fill: parent
        radius: glass.radius
        color: Theme.glass
        border.color: Theme.glassBorder
        border.width: 1
    }

    // inset 0 1px 0 white/5: a faint top highlight.
    Rectangle {
        x: glass.radius / 2
        width: parent.width - glass.radius
        y: 1
        height: 1
        color: Theme.white(0.05)
    }

    Item {
        id: inner
        anchors.fill: parent
    }
}
