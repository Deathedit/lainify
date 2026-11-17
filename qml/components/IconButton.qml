import QtQuick
import com.lainify
import QtQuick.Controls.Basic

// Square (or pill, when `wide`) toolbar button with a hover tooltip showing its shortcut.
AbstractButton {
    id: button

    property string label
    property string shortcut
    property bool active: false
    property bool wide: false
    /// `top`, `bottom` or `none`.
    property string tooltip: "top"
    property string iconName
    property color iconColor: active || hovered ? "white" : Theme.zinc300
    property real iconSize: 18

    implicitWidth: wide ? contentItem.implicitWidth + 24 : 40
    implicitHeight: 40
    hoverEnabled: true
    focusPolicy: Qt.NoFocus
    opacity: enabled ? 1 : 0.35
    Accessible.name: label

    background: Rectangle {
        radius: 12
        color: button.active ? Theme.white(0.12) : button.hovered ? Theme.white(0.08) : "transparent"
        Behavior on color {
            ColorAnimation {
                duration: 150
            }
        }
    }

    contentItem: Item {
        implicitWidth: glyph.width
        implicitHeight: glyph.height
        Icon {
            id: glyph
            anchors.centerIn: parent
            name: button.iconName
            size: button.iconSize
            color: button.iconColor
        }
    }

    Glass {
        id: tip

        readonly property bool shown: button.tooltip !== "none" && button.hovered && button.enabled

        z: 10
        radius: 8
        width: tipRow.implicitWidth + 20
        height: tipRow.implicitHeight + 12
        x: (button.width - width) / 2
        y: button.tooltip === "bottom" ? button.height + 12 + (shown ? 0 : -4) : -height - 12 + (shown ? 0 : 4)
        opacity: shown ? 1 : 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: 150
            }
        }
        Behavior on y {
            NumberAnimation {
                duration: 150
            }
        }

        Row {
            id: tipRow
            anchors.centerIn: parent
            spacing: 8

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: button.label
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Medium
                color: Theme.zinc200
            }
            Kbd {
                anchors.verticalCenter: parent.verticalCenter
                visible: button.shortcut !== ""
                text: button.shortcut
            }
        }
    }
}
