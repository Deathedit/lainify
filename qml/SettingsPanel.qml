import QtQuick
import com.lainify
import QtQuick.Controls.Basic

// Slide-in panel on the right with every setting, grouped like the browser version.
Item {
    id: settings

    property bool open: false

    property var videoOptions: []
    property var formatOptions: []
    property var sizeOptions: []
    property var rateOptions: []
    property var audioOptions: []
    property var encoderOptions: []
    property var bitrateOptions: []
    property var chromaOptions: []
    property var containerOptions: []
    property var hideOptions: []
    property var replayOptions: []
    property var replayLengthOptions: []
    property var replayBitrateOptions: []

    property string videoValue
    property string formatValue
    property string sizeValue
    property string rateValue
    property string audioValue
    property string encoderValue
    property string bitrateValue
    property string chromaValue
    property string containerValue
    property string hideValue
    property string replayValue
    property string replayLengthValue
    property string replayBitrateValue
    property string replayHint

    /// Mode and recording settings are locked while recording.
    property bool recording: false
    property bool chromaAvailable: true
    property string videoInfo
    property string audioInfo
    property string audioWarning
    property string screenshotDir
    property string recordingDir
    property string clipsDir

    signal closeRequested
    signal videoChanged(string value)
    signal formatChanged(string value)
    signal sizeChanged(string value)
    signal rateChanged(string value)
    signal audioChanged(string value)
    signal encoderChanged(string value)
    signal bitrateChanged(string value)
    signal chromaChanged(string value)
    signal containerChanged(string value)
    signal hideChanged(string value)
    signal replayChanged(string value)
    signal replayLengthChanged(string value)
    signal replayBitrateChanged(string value)

    anchors.fill: parent
    visible: open || panel.opacity > 0

    readonly property var shortcuts: [
        {
            "keys": ["F"],
            "label": "Fullscreen"
        },
        {
            "keys": ["M"],
            "label": "Mute"
        },
        {
            "keys": ["↑", "↓"],
            "label": "Volume"
        },
        {
            "keys": ["S"],
            "label": "Screenshot"
        },
        {
            "keys": ["R"],
            "label": "Start / stop recording"
        },
        {
            "keys": ["C"],
            "label": "Save replay clip"
        },
        {
            "keys": [","],
            "label": "Settings"
        },
        {
            "keys": ["Esc"],
            "label": "Close panel"
        }
    ]

    function home(path) {
        return path.replace(/^\/home\/[^/]+/, "~");
    }

    // Click outside closes.
    MouseArea {
        anchors.fill: parent
        enabled: settings.open
        onClicked: settings.closeRequested()
    }

    component Heading: Row {
        property alias text: headingText.text
        property alias iconName: headingIcon.name
        spacing: 8
        Icon {
            id: headingIcon
            anchors.verticalCenter: parent.verticalCenter
            size: 13
            color: Theme.zinc500
        }
        Text {
            id: headingText
            anchors.verticalCenter: parent.verticalCenter
            font.family: Theme.font
            font.pixelSize: 11
            font.weight: Font.DemiBold
            font.letterSpacing: 0.9
            font.capitalization: Font.AllUppercase
            color: Theme.zinc500
        }
    }

    component InfoBox: Rectangle {
        property alias text: infoText.text
        visible: text !== ""
        width: parent.width
        height: infoText.implicitHeight + 16
        radius: 8
        color: Theme.white(0.03)
        Text {
            id: infoText
            x: 12
            width: parent.width - 24
            anchors.verticalCenter: parent.verticalCenter
            wrapMode: Text.Wrap
            lineHeight: 1.25
            font.family: Theme.font
            font.pixelSize: 12
            font.features: Theme.tabular
            color: Theme.zinc400
        }
    }

    component WarningBox: Rectangle {
        property alias text: warningText.text
        visible: text !== ""
        width: parent.width
        height: warningText.implicitHeight + 20
        radius: 8
        color: Qt.rgba(Theme.amber400.r, Theme.amber400.g, Theme.amber400.b, 0.08)
        border.color: Qt.rgba(Theme.amber400.r, Theme.amber400.g, Theme.amber400.b, 0.20)
        Icon {
            id: warningIcon
            x: 10
            y: 11
            name: "triangle-alert"
            size: 14
            color: Theme.amber300
        }
        Text {
            id: warningText
            x: 32
            width: parent.width - 44
            anchors.verticalCenter: parent.verticalCenter
            wrapMode: Text.Wrap
            lineHeight: 1.25
            font.family: Theme.font
            font.pixelSize: 12
            font.features: Theme.tabular
            color: Theme.zinc200
        }
    }

    component FolderRow: Item {
        property string label
        property string path
        width: parent.width
        height: 36
        Column {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - openButton.width - 8
            spacing: 2
            Text {
                text: parent.parent.label
                font.family: Theme.font
                font.pixelSize: 12
                font.weight: Font.Medium
                color: Theme.zinc400
            }
            Text {
                width: parent.width
                elide: Text.ElideMiddle
                text: settings.home(parent.parent.path)
                font.family: Theme.font
                font.pixelSize: 12
                color: Theme.zinc500
            }
        }
        IconButton {
            id: openButton
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: 32
            height: 32
            tooltip: "none"
            label: "Open folder"
            iconName: "folder-open"
            iconSize: 16
            onClicked: Qt.openUrlExternally("file://" + parent.path)
        }
    }

    Glass {
        id: panel

        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.margins: 12
        width: Math.min(352, parent.width - 24)
        x: parent.width - width - 12 + (settings.open ? 0 : 24)
        opacity: settings.open ? 1 : 0
        radius: 16
        clip: true

        Behavior on x {
            NumberAnimation {
                duration: 220
                easing.type: Easing.Bezier
                easing.bezierCurve: Theme.easeSoft
            }
        }
        Behavior on opacity {
            NumberAnimation {
                duration: 220
            }
        }

        // Swallow clicks so they don't reach the close-catcher behind.
        MouseArea {
            anchors.fill: parent
        }

        Item {
            id: header
            width: parent.width
            height: 56
            Text {
                x: 20
                anchors.verticalCenter: parent.verticalCenter
                text: "Settings"
                font.family: Theme.font
                font.pixelSize: 14
                font.weight: Font.DemiBold
                color: Theme.zinc100
            }
            IconButton {
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                width: 32
                height: 32
                tooltip: "none"
                label: "Close settings"
                iconName: "x"
                iconSize: 16
                onClicked: settings.closeRequested()
            }
        }

        Flickable {
            anchors.top: header.bottom
            anchors.bottom: parent.bottom
            width: parent.width
            contentHeight: body.implicitHeight + 28
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {
                policy: ScrollBar.AsNeeded
                contentItem: Rectangle {
                    implicitWidth: 4
                    radius: 2
                    color: Theme.white(0.15)
                }
            }

            Column {
                id: body
                x: 20
                y: 4
                width: parent.width - 40
                spacing: 28

                // ── Video ──
                Column {
                    width: parent.width
                    spacing: 12
                    Heading {
                        text: "Video"
                        iconName: "video"
                    }
                    Select {
                        label: "Device"
                        options: settings.videoOptions
                        value: settings.videoValue
                        onChanged: v => settings.videoChanged(v)
                    }
                    Select {
                        label: "Format"
                        options: settings.formatOptions
                        value: settings.formatValue
                        enabled: !settings.recording
                        onChanged: v => settings.formatChanged(v)
                    }
                    Select {
                        label: "Resolution"
                        options: settings.sizeOptions
                        value: settings.sizeValue
                        enabled: !settings.recording
                        onChanged: v => settings.sizeChanged(v)
                    }
                    Select {
                        label: "Frame rate"
                        options: settings.rateOptions
                        value: settings.rateValue
                        enabled: !settings.recording
                        hint: settings.recording ? "Stop recording to change the mode." : ""
                        onChanged: v => settings.rateChanged(v)
                    }
                    InfoBox {
                        text: settings.videoInfo
                    }
                }

                // ── Audio ──
                Column {
                    width: parent.width
                    spacing: 12
                    Heading {
                        text: "Audio"
                        iconName: "audio-lines"
                    }
                    Select {
                        label: "Device"
                        options: settings.audioOptions
                        value: settings.audioValue
                        onChanged: v => settings.audioChanged(v)
                    }
                    InfoBox {
                        text: settings.audioInfo
                    }
                    WarningBox {
                        text: settings.audioWarning
                    }
                }

                // ── Recording ──
                Column {
                    width: parent.width
                    spacing: 12
                    Heading {
                        text: "Recording"
                        iconName: "monitor-play"
                    }
                    Select {
                        label: "Encoder"
                        options: settings.encoderOptions
                        value: settings.encoderValue
                        enabled: !settings.recording
                        onChanged: v => settings.encoderChanged(v)
                    }
                    Select {
                        label: "Video bitrate"
                        options: settings.bitrateOptions
                        value: settings.bitrateValue
                        enabled: !settings.recording
                        onChanged: v => settings.bitrateChanged(v)
                    }
                    Select {
                        label: "Colour"
                        options: settings.chromaOptions
                        value: settings.chromaAvailable ? settings.chromaValue : "420"
                        enabled: !settings.recording && settings.chromaAvailable
                        hint: !settings.chromaAvailable ? "This encoder only records 4:2:0." : settings.chromaValue === "444" ? "Full colour detail. Plays in mpv and VLC; browsers and most editors can't decode it." : ""
                        onChanged: v => settings.chromaChanged(v)
                    }
                    Select {
                        label: "File format"
                        options: settings.containerOptions
                        value: settings.containerValue
                        enabled: !settings.recording
                        onChanged: v => settings.containerChanged(v)
                    }
                }

                // ── Replay buffer ──
                Column {
                    width: parent.width
                    spacing: 12
                    Heading {
                        text: "Replay buffer"
                        iconName: "history"
                    }
                    Select {
                        label: "Keep recent gameplay"
                        options: settings.replayOptions
                        value: settings.replayValue
                        onChanged: v => settings.replayChanged(v)
                    }
                    Select {
                        visible: settings.replayValue === "on"
                        label: "Clip length"
                        options: settings.replayLengthOptions
                        value: settings.replayLengthValue
                        onChanged: v => settings.replayLengthChanged(v)
                    }
                    Select {
                        visible: settings.replayValue === "on"
                        label: "Video bitrate"
                        options: settings.replayBitrateOptions
                        value: settings.replayBitrateValue
                        onChanged: v => settings.replayBitrateChanged(v)
                    }
                    InfoBox {
                        text: settings.replayHint
                    }
                }

                // ── Files ──
                Column {
                    width: parent.width
                    spacing: 8
                    Heading {
                        text: "Files"
                        iconName: "folder-open"
                    }
                    FolderRow {
                        label: "Recordings"
                        path: settings.recordingDir
                    }
                    FolderRow {
                        label: "Clips"
                        path: settings.clipsDir
                    }
                    FolderRow {
                        label: "Screenshots"
                        path: settings.screenshotDir
                    }
                }

                // ── Interface ──
                Column {
                    width: parent.width
                    spacing: 12
                    Heading {
                        text: "Interface"
                        iconName: "panel-top"
                    }
                    Select {
                        label: "Hide controls"
                        options: settings.hideOptions
                        value: settings.hideValue
                        onChanged: v => settings.hideChanged(v)
                    }
                }

                // ── Shortcuts ──
                Column {
                    width: parent.width
                    spacing: 8
                    Heading {
                        text: "Shortcuts"
                        iconName: "keyboard"
                    }
                    Repeater {
                        model: settings.shortcuts
                        Item {
                            required property var modelData
                            width: parent.width
                            height: 22
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: modelData.label
                                font.family: Theme.font
                                font.pixelSize: 14
                                color: Theme.zinc300
                            }
                            Row {
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4
                                Repeater {
                                    model: modelData.keys
                                    Kbd {
                                        required property string modelData
                                        text: modelData
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
