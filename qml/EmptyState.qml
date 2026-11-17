import QtQuick
import com.lainify
import QtQuick.Effects
import QtQuick.Shapes

// What fills the window when there's no picture. `phase`: `welcome` (no device yet),
// `starting`, `stopped`, `error` or `lost` (the device was unplugged).
Item {
    id: empty

    property string phase
    property string deviceName
    property string errorTitle
    property string errorMessage

    signal startRequested
    signal refresh
    signal openSettings

    anchors.fill: parent

    // radial-gradient(60% 50% at 50% 45%, accent glow / 22%, transparent)
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeWidth: -1
            fillGradient: RadialGradient {
                centerX: empty.width / 2
                centerY: empty.height * 0.45
                centerRadius: Math.max(empty.width * 0.6, empty.height * 0.5)
                focalX: centerX
                focalY: centerY
                GradientStop {
                    position: 0
                    color: Qt.rgba(0x37 / 255, 0x27 / 255, 0x5b / 255, 0.22)
                }
                GradientStop {
                    position: 1
                    color: "transparent"
                }
            }
            PathRectangle {
                width: empty.width
                height: empty.height
            }
        }
    }

    component Title: Text {
        width: Math.min(implicitWidth, 384)
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        font.family: Theme.font
        font.pixelSize: 16
        font.weight: Font.Medium
        color: Theme.zinc100
    }
    component Detail: Text {
        width: Math.min(implicitWidth, 384)
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        lineHeight: 1.3
        font.family: Theme.font
        font.pixelSize: 14
        color: Theme.zinc400
    }
    component Tile: Rectangle {
        property alias iconName: tileIcon.name
        property alias iconColor: tileIcon.color
        width: 56
        height: 56
        radius: 16
        Icon {
            id: tileIcon
            anchors.centerIn: parent
            size: 24
        }
    }

    Column {
        anchors.centerIn: parent
        width: Math.min(384, parent.width - 48)
        spacing: 0

        // ── welcome ──
        Column {
            visible: empty.phase === "welcome"
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            Item {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 64
                height: 64
                RectangularShadow {
                    anchors.fill: logo
                    radius: logo.radius
                    blur: 24
                    color: Qt.rgba(Theme.accent400.r, Theme.accent400.g, Theme.accent400.b, 0.35)
                }
                Rectangle {
                    id: logo
                    anchors.fill: parent
                    radius: 16
                    color: "#15121d"
                    Rectangle {
                        x: 12
                        y: 17
                        width: 40
                        height: 30
                        radius: 7
                        color: "transparent"
                        border.color: "#b9a2ff"
                        border.width: 3.2
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        width: 13
                        height: 13
                        radius: 6.5
                        color: "#b9a2ff"
                    }
                }
            }
            Item {
                width: 1
                height: 24
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "lainify"
                font.family: Theme.font
                font.pixelSize: 24
                font.weight: Font.DemiBold
                font.letterSpacing: -0.4
                color: Theme.zinc100
            }
            Item {
                width: 1
                height: 8
            }
            Detail {
                anchors.horizontalCenter: parent.horizontalCenter
                width: 384
                text: "A clean, low-latency window into your capture card — picture and sound, straight from the device."
            }
            Item {
                width: 1
                height: 32
            }
            TextButton {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Look for devices"
                iconName: "plug"
                onClicked: empty.refresh()
            }
            Item {
                width: 1
                height: 16
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Plug in a capture card and lainify opens it automatically."
                font.family: Theme.font
                font.pixelSize: 12
                color: Theme.zinc500
            }
        }

        // ── starting ──
        Column {
            visible: empty.phase === "starting"
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            Spinner {
                anchors.horizontalCenter: parent.horizontalCenter
                size: 28
                color: Theme.accent400
            }
            Item {
                width: 1
                height: 20
            }
            Title {
                anchors.horizontalCenter: parent.horizontalCenter
                text: `Opening ${empty.deviceName || "device"}…`
            }
            Item {
                width: 1
                height: 6
            }
            Detail {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Negotiating format and frame rate."
            }
        }

        // ── stopped ──
        Column {
            visible: empty.phase === "stopped"
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            Tile {
                anchors.horizontalCenter: parent.horizontalCenter
                color: Theme.white(0.05)
                iconName: "power"
                iconColor: Theme.zinc400
            }
            Item {
                width: 1
                height: 20
            }
            Title {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Capture stopped"
            }
            Item {
                width: 1
                height: 6
            }
            Detail {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "The device is released and free for other apps."
            }
            Item {
                width: 1
                height: 32
            }
            TextButton {
                anchors.horizontalCenter: parent.horizontalCenter
                text: "Start capture"
                iconName: "power"
                onClicked: empty.startRequested()
            }
        }

        // ── error / lost ──
        Column {
            visible: empty.phase === "error" || empty.phase === "lost"
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            readonly property bool lost: empty.phase === "lost"

            Tile {
                anchors.horizontalCenter: parent.horizontalCenter
                color: parent.lost ? Qt.rgba(Theme.amber400.r, Theme.amber400.g, Theme.amber400.b, 0.10) : Qt.rgba(Theme.rose500.r, Theme.rose500.g, Theme.rose500.b, 0.10)
                iconName: parent.lost ? "unplug" : "triangle-alert"
                iconColor: parent.lost ? Theme.amber300 : Theme.rose400
            }
            Item {
                width: 1
                height: 20
            }
            Title {
                anchors.horizontalCenter: parent.horizontalCenter
                text: empty.errorTitle
            }
            Item {
                width: 1
                height: 6
            }
            Detail {
                anchors.horizontalCenter: parent.horizontalCenter
                text: empty.errorMessage
            }
            Item {
                width: 1
                height: 32
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 8
                TextButton {
                    visible: !parent.parent.lost
                    kind: "secondary"
                    text: "Retry"
                    iconName: "rotate-cw"
                    onClicked: empty.startRequested()
                }
                TextButton {
                    kind: "ghost"
                    text: "Choose device"
                    iconName: "settings-2"
                    onClicked: empty.openSettings()
                }
            }
        }
    }
}
