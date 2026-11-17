import QtQuick
import com.lainify
import QtQuick.Effects

// Top-left pills: REC with the elapsed time, and the live device with its mode.
Row {
    id: pills

    property bool shown: true
    property bool live: false
    property string deviceName
    property string details
    property bool recording: false
    property real elapsed: 0
    /// Replay buffer running: show how much it holds.
    property bool replay: false
    property real replayBuffered: 0
    property int replayLength: 60

    x: 16
    y: 16
    spacing: 8

    Glass {
        visible: pills.recording
        radius: height / 2
        width: recRow.implicitWidth + 24
        height: 28

        Row {
            id: recRow
            anchors.centerIn: parent
            spacing: 8

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 8
                height: 8
                radius: 4
                color: Theme.rec
                SequentialAnimation on opacity {
                    running: pills.recording
                    loops: Animation.Infinite
                    NumberAnimation {
                        to: 0.4
                        duration: 1000
                        easing.type: Easing.InOutSine
                    }
                    NumberAnimation {
                        to: 1
                        duration: 1000
                        easing.type: Easing.InOutSine
                    }
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "REC"
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.DemiBold
                font.letterSpacing: 0.3
                color: "white"
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: Theme.duration(pills.elapsed)
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Medium
                font.features: Theme.tabular
                color: "white"
            }
        }
    }

    Glass {
        visible: pills.live && opacity > 0
        radius: height / 2
        width: liveRow.implicitWidth + 26
        height: 28
        opacity: pills.shown ? 1 : 0
        transform: Translate {
            y: pills.shown ? 0 : -4
            Behavior on y {
                NumberAnimation {
                    duration: 300
                    easing.type: Easing.Bezier
                    easing.bezierCurve: Theme.easeSoft
                }
            }
        }

        Behavior on opacity {
            NumberAnimation {
                duration: 300
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.easeSoft
            }
        }

        Row {
            id: liveRow
            anchors.centerIn: parent
            spacing: 10

            Item {
                anchors.verticalCenter: parent.verticalCenter
                width: 8
                height: 8
                RectangularShadow {
                    anchors.fill: dot
                    radius: 4
                    blur: 8
                    color: Theme.emerald400
                }
                Rectangle {
                    id: dot
                    anchors.fill: parent
                    radius: 4
                    color: Theme.emerald400
                }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.min(implicitWidth, 208)
                elide: Text.ElideRight
                text: pills.deviceName || "Live"
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Medium
                color: Theme.zinc200
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: pills.details !== ""
                width: 1
                height: 12
                color: Theme.white(0.15)
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: pills.details !== ""
                text: pills.details
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Medium
                font.features: Theme.tabular
                color: Theme.zinc400
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                visible: pills.replay
                width: 1
                height: 12
                color: Theme.white(0.15)
            }
            Row {
                anchors.verticalCenter: parent.verticalCenter
                visible: pills.replay
                spacing: 5
                Icon {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "history"
                    size: 12
                    color: Theme.accent300
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Theme.duration(Math.min(pills.replayBuffered, pills.replayLength))
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.weight: Font.Medium
                    font.features: Theme.tabular
                    color: Theme.zinc400
                }
            }
        }
    }
}
