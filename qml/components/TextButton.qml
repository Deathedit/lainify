import QtQuick
import com.lainify
import QtQuick.Controls.Basic
import QtQuick.Effects

// Text + icon button. `kind`: `primary` (accent), `secondary` (white/10) or `ghost`.
AbstractButton {
    id: button

    property string kind: "primary"
    property string iconName

    readonly property bool primary: kind === "primary"

    implicitWidth: row.implicitWidth + (primary ? 40 : 32)
    implicitHeight: primary ? 40 : 36
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    scale: pressed ? 0.98 : 1

    Behavior on scale {
        NumberAnimation {
            duration: 100
        }
    }

    background: Item {
        RectangularShadow {
            anchors.fill: fill
            visible: button.primary
            radius: fill.radius
            offset.y: 8
            blur: 30
            spread: -8
            color: Theme.accent500
        }
        Rectangle {
            id: fill
            anchors.fill: parent
            radius: 12
            color: button.primary ? (button.hovered ? Theme.accent300 : Theme.accent400)
                : button.kind === "secondary" ? Theme.white(button.hovered ? 0.15 : 0.10)
                : Theme.white(button.hovered ? 0.08 : 0)
            Behavior on color {
                ColorAnimation {
                    duration: 150
                }
            }
        }
    }

    contentItem: Item {
        Row {
            id: row
            anchors.centerIn: parent
            spacing: 8

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                visible: button.iconName !== ""
                name: button.iconName
                size: button.primary ? 16 : 15
                color: label.color
            }
            Text {
                id: label
                anchors.verticalCenter: parent.verticalCenter
                text: button.text
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: button.primary ? Font.DemiBold : Font.Medium
                color: button.primary ? Theme.zinc950 : button.kind === "ghost" && !button.hovered ? Theme.zinc300 : "white"
            }
        }
    }
}
