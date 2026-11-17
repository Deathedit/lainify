import QtQuick
import com.lainify

// The floating toolbar at the bottom: power, volume, screenshot, record, settings, fullscreen.
Item {
    id: bar

    property bool shown: true
    property bool live: false
    property bool running: false
    property bool hasAudio: false
    property real volume: 1
    property bool muted: false
    property real levelLeft: 0
    property real levelRight: 0
    property bool recording: false
    property bool saving: false
    /// The replay buffer is turned on in settings (the clip button only shows then).
    property bool replayOn: false
    property bool replayActive: false
    property real replayBuffered: 0
    property int replayLength: 60
    property bool clipSaving: false
    property real elapsed: 0
    property bool fullscreen: false
    property bool panelOpen: false
    readonly property bool hovered: hover.hovered

    signal power
    signal screenshot
    signal clip
    signal record
    signal settings
    signal toggleFullscreen
    signal volumeMoved(real value)
    signal muteToggled

    anchors.fill: parent

    component Divider: Item {
        width: 9
        height: 40
        Rectangle {
            anchors.centerIn: parent
            width: 1
            height: 24
            color: Theme.white(0.10)
        }
    }

    Glass {
        id: panel

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 20 - (bar.shown ? 0 : 12)
        width: Math.min(parent.width - 32, row.implicitWidth + 12)
        height: row.implicitHeight + 12
        radius: 16
        opacity: bar.shown ? 1 : 0
        visible: opacity > 0
        enabled: bar.shown

        Behavior on opacity {
            NumberAnimation {
                duration: 300
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.easeSoft
            }
        }
        Behavior on anchors.bottomMargin {
            NumberAnimation {
                duration: 300
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.easeSoft
            }
        }

        HoverHandler {
            id: hover
        }

        Row {
            id: row
            anchors.centerIn: parent
            spacing: 2

            IconButton {
                label: bar.running ? "Stop capture" : "Start capture"
                iconName: "power"
                iconColor: bar.live ? Theme.emerald400 : (hovered ? "white" : Theme.zinc300)
                onClicked: bar.power()
            }

            Divider {}

            // Volume: mute button, slider, percentage, meter.
            Row {
                spacing: 6
                rightPadding: 8

                readonly property bool silent: bar.muted || bar.volume === 0 || !bar.hasAudio

                IconButton {
                    label: !bar.hasAudio ? "No audio input" : bar.muted ? "Unmute" : "Mute"
                    shortcut: bar.hasAudio ? "M" : ""
                    enabled: bar.hasAudio
                    iconName: parent.silent ? "volume-x" : bar.volume < 0.35 ? "volume" : bar.volume < 0.8 ? "volume-1" : "volume-2"
                    onClicked: bar.muteToggled()
                }
                VolumeSlider {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: bar.width > 640
                    from: 0
                    to: 1.5
                    stepSize: 0.01
                    enabled: bar.hasAudio
                    opacity: bar.muted || !bar.hasAudio ? 0.5 : 1
                    value: bar.volume
                    onMoved: bar.volumeMoved(value)
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: bar.width > 640
                    width: 36
                    horizontalAlignment: Text.AlignRight
                    text: Math.round(bar.volume * 100) + "%"
                    font.family: Theme.font
                    font.pixelSize: 12
                    font.features: Theme.tabular
                    color: Theme.zinc400
                }
                LevelMeter {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: bar.width > 640
                    levelLeft: bar.levelLeft
                    levelRight: bar.levelRight
                    active: bar.hasAudio && !bar.muted
                }
            }

            Divider {}

            IconButton {
                label: "Screenshot"
                shortcut: "S"
                iconName: "camera"
                enabled: bar.live
                onClicked: bar.screenshot()
            }

            IconButton {
                id: clipButton

                readonly property bool full: bar.replayBuffered >= bar.replayLength - 0.5

                visible: bar.replayOn
                label: bar.clipSaving ? "Saving clip…"
                    : full ? `Save last ${Theme.duration(bar.replayLength)}`
                    : `Save clip (${Theme.duration(bar.replayBuffered)} so far)`
                shortcut: "C"
                iconName: "history"
                enabled: bar.replayActive && bar.replayBuffered >= 1 && !bar.clipSaving
                onClicked: bar.clip()

                // Fill indicator: a thin bar under the icon while the buffer builds up.
                Rectangle {
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 5
                    visible: bar.replayActive && !clipButton.full
                    width: 16
                    height: 2
                    radius: 1
                    color: Theme.white(0.15)
                    Rectangle {
                        width: parent.width * Math.min(1, bar.replayBuffered / Math.max(1, bar.replayLength))
                        height: parent.height
                        radius: 1
                        color: Theme.accent400
                    }
                }
                Spinner {
                    anchors.centerIn: parent
                    visible: bar.clipSaving
                    size: 18
                    color: Theme.zinc300
                }
                iconColor: bar.clipSaving ? "transparent" : (hovered ? "white" : Theme.zinc300)
            }

            IconButton {
                id: rec

                wide: true
                label: bar.recording ? "Stop recording" : "Record"
                shortcut: "R"
                enabled: !bar.saving && (bar.recording || bar.live)
                onClicked: bar.record()

                background: Rectangle {
                    radius: 12
                    color: bar.recording ? Qt.rgba(Theme.rec.r, Theme.rec.g, Theme.rec.b, rec.hovered ? 1 : 0.9)
                        : rec.hovered ? Theme.white(0.08) : "transparent"
                    Behavior on color {
                        ColorAnimation {
                            duration: 150
                        }
                    }
                }

                contentItem: Item {
                    implicitWidth: recRow.implicitWidth
                    implicitHeight: recRow.implicitHeight
                    Row {
                        id: recRow
                        anchors.centerIn: parent
                        spacing: 8

                        // Idle: red dot with a soft ring.
                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: !bar.recording && !bar.saving
                            width: 16
                            height: 16
                            radius: 8
                            color: Qt.rgba(Theme.rec.r, Theme.rec.g, Theme.rec.b, 0.25)
                            Rectangle {
                                anchors.centerIn: parent
                                width: 10
                                height: 10
                                radius: 5
                                color: Theme.rec
                            }
                        }
                        Icon {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: bar.recording
                            name: "square"
                            size: 11
                            filled: true
                            color: "white"
                        }
                        Spinner {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: bar.saving
                            size: 16
                            color: Theme.zinc300
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: bar.recording ? Theme.duration(bar.elapsed) : bar.saving ? "Saving" : "Rec"
                            font.family: Theme.font
                            font.pixelSize: 14
                            font.weight: Font.Medium
                            font.features: Theme.tabular
                            color: bar.recording || rec.hovered ? "white" : Theme.zinc300
                        }
                    }
                }
            }

            Divider {}

            IconButton {
                label: "Settings"
                shortcut: ","
                iconName: "settings-2"
                active: bar.panelOpen
                onClicked: bar.settings()
            }
            IconButton {
                label: bar.fullscreen ? "Exit fullscreen" : "Fullscreen"
                shortcut: "F"
                iconName: bar.fullscreen ? "minimize" : "maximize"
                onClicked: bar.toggleFullscreen()
            }
        }
    }
}
