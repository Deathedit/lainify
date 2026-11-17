import QtQuick
import com.lainify
import QtQuick.Controls.Basic
import QtQuick.Effects

// Thin track filled in zinc-100, white thumb that grows on hover.
Slider {
    id: slider

    implicitWidth: 96
    implicitHeight: 20
    padding: 0
    focusPolicy: Qt.NoFocus
    hoverEnabled: true

    background: Item {
        x: slider.leftPadding
        width: slider.availableWidth
        height: slider.height

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width
            height: 4
            radius: 2
            color: Theme.white(0.16)

            Rectangle {
                width: slider.visualPosition * parent.width
                height: parent.height
                radius: 2
                color: Theme.zinc100
            }
        }
    }

    handle: Item {
        x: slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
        y: (slider.height - height) / 2
        width: 12
        height: 12
        scale: slider.hovered || slider.pressed ? 1.2 : 1

        Behavior on scale {
            NumberAnimation {
                duration: 150
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.easeSoft
            }
        }

        RectangularShadow {
            anchors.fill: knob
            radius: 6
            offset.y: 1
            blur: 4
            color: Qt.rgba(0, 0, 0, 0.5)
        }
        Rectangle {
            id: knob
            anchors.fill: parent
            radius: 6
            color: "white"
        }
    }
}
