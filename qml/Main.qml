import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtMultimedia
import com.lainify
import com.lainify.native

ApplicationWindow {
    id: win

    width: 1280
    height: 760
    visible: true
    title: "lainify"
    color: "black"

    palette.window: "#18181b"
    palette.windowText: "#e4e4e7"
    palette.base: "#27272a"
    palette.alternateBase: "#3f3f46"
    palette.text: "#e4e4e7"
    palette.button: "#27272a"
    palette.buttonText: "#e4e4e7"
    palette.highlight: "#8b5cf6"
    palette.highlightedText: "#ffffff"
    palette.placeholderText: "#71717a"
    palette.mid: "#3f3f46"
    palette.dark: "#09090b"
    palette.light: "#52525b"

    // ── State ────────────────────────────────────────────────────────────────

    readonly property var devices: {
        try {
            return JSON.parse(capture.devicesJson || "[]");
        } catch (e) {
            return [];
        }
    }
    // The selection is derived from what the user asked for (want*), falling back to the closest
    // mode the device has, so switching format or device keeps as much of the choice as it can.
    readonly property int deviceIndex: devices.length ? Math.min(wantDevice, devices.length - 1) : -1
    readonly property var device: devices[deviceIndex] ?? null
    readonly property int formatIndex: device ? pick(device.formats, f => f.id === wantFormat) : -1
    readonly property var format: device ? (device.formats[formatIndex] ?? null) : null
    readonly property int sizeIndex: format ? pick(format.sizes, s => s.width === wantWidth && s.height === wantHeight) : -1
    readonly property var size: format ? (format.sizes[sizeIndex] ?? null) : null
    readonly property int rateIndex: size ? closestRate(size.rates) : -1
    readonly property var rate: size ? (size.rates[rateIndex] ?? null) : null

    readonly property bool live: capture.status === "live"
    readonly property bool running: capture.status === "live" || capture.status === "starting"

    // What the user last asked for; kept when switching format or device.
    property int wantDevice: 0
    property string wantFormat: "BGR"
    property int wantWidth: 1920
    property int wantHeight: 1080
    property real wantFps: 60
    property bool autoStarted: false

    function fpsOf(r) {
        return r.num / r.den;
    }
    function fpsLabel(r) {
        const f = fpsOf(r);
        return (Number.isInteger(f) ? f.toString() : f.toFixed(2)) + " fps";
    }
    function pick(list, pred) {
        const i = list.findIndex(pred);
        return i < 0 ? 0 : i;
    }
    function closestRate(rates) {
        let best = 0;
        for (let i = 1; i < rates.length; ++i) {
            if (Math.abs(fpsOf(rates[i]) - wantFps) < Math.abs(fpsOf(rates[best]) - wantFps))
                best = i;
        }
        return best;
    }

    function startCapture() {
        if (!device || !format || !size || !rate)
            return;
        capture.start(deviceIndex, format.id, size.width, size.height, rate.num, rate.den);
    }
    function autoStart() {
        if (!autoStarted && devices.length > 0) {
            autoStarted = true;
            startCapture();
        }
    }
    function restartIfRunning() {
        if (running)
            Qt.callLater(startCapture);
    }
    function toggleFullscreen() {
        win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen;
    }

    Capture {
        id: capture

        onNotice: (text, error) => toast.show(text, error)
        // Deferred: this handler can run before the `devices` binding has re-evaluated.
        onDevicesJsonChanged: Qt.callLater(win.autoStart)
        Component.onCompleted: refresh()
    }

    Timer {
        interval: 1000
        repeat: true
        running: win.running
        onTriggered: capture.tick()
    }

    // ── Picture ──────────────────────────────────────────────────────────────

    VideoOutput {
        id: video

        anchors.fill: parent
        fillMode: VideoOutput.PreserveAspectFit
        Component.onCompleted: FrameSink.videoSink = video.videoSink
    }

    Column {
        anchors.centerIn: parent
        spacing: 8
        visible: !win.live

        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            font.pixelSize: 20
            color: "#e4e4e7"
            text: win.devices.length === 0 ? "No capture device found"
                : capture.status === "starting" ? "Starting…"
                : capture.status === "error" ? "Capture failed"
                : "Stopped"
        }
        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            color: "#a1a1aa"
            text: win.devices.length === 0 ? "Plug in a card, then press Refresh." : "Pick a mode below and press Start."
        }
    }

    // ── Controls (hide after a few idle seconds while live) ─────────────────

    property bool idle: false

    Timer {
        id: idleTimer
        interval: 2500
        onTriggered: win.idle = true
    }

    MouseArea {
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.NoButton
        cursorShape: win.live && win.idle ? Qt.BlankCursor : Qt.ArrowCursor
        onPositionChanged: {
            win.idle = false;
            idleTimer.restart();
        }
    }

    Rectangle {
        id: bar

        readonly property bool shown: !win.live || !win.idle || barHover.hovered

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 16
        width: Math.min(parent.width - 32, controls.implicitWidth + 24)
        height: controls.implicitHeight + 20
        radius: 14
        color: "#d918181b"
        border.color: "#33ffffff"
        opacity: shown ? 1 : 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: 200
            }
        }

        HoverHandler {
            id: barHover
        }

        ColumnLayout {
            id: controls

            anchors.fill: parent
            anchors.margins: 10
            spacing: 8

            RowLayout {
                spacing: 8

                ComboBox {
                    id: deviceBox
                    Layout.preferredWidth: 200
                    model: win.devices.map(d => d.name)
                    currentIndex: win.deviceIndex
                    enabled: win.devices.length > 0
                    onActivated: index => {
                        win.wantDevice = index;
                        win.restartIfRunning();
                    }
                }
                ComboBox {
                    id: formatBox
                    Layout.preferredWidth: 270
                    model: win.device ? win.device.formats.map(f => f.label) : []
                    currentIndex: win.formatIndex
                    onActivated: index => {
                        win.wantFormat = win.device.formats[index].id;
                        win.restartIfRunning();
                    }
                }
                ComboBox {
                    id: sizeBox
                    Layout.preferredWidth: 130
                    model: win.format ? win.format.sizes.map(s => `${s.width}×${s.height}`) : []
                    currentIndex: win.sizeIndex
                    onActivated: index => {
                        win.wantWidth = win.format.sizes[index].width;
                        win.wantHeight = win.format.sizes[index].height;
                        win.restartIfRunning();
                    }
                }
                ComboBox {
                    id: rateBox
                    Layout.preferredWidth: 110
                    model: win.size ? win.size.rates.map(r => win.fpsLabel(r)) : []
                    currentIndex: win.rateIndex
                    onActivated: index => {
                        win.wantFps = win.fpsOf(win.size.rates[index]);
                        win.restartIfRunning();
                    }
                }
            }

            RowLayout {
                spacing: 8

                Button {
                    text: win.running ? "Stop" : "Start"
                    highlighted: !win.running
                    enabled: win.running || win.rate !== null
                    onClicked: win.running ? capture.stop() : win.startCapture()
                }
                Button {
                    text: "Screenshot"
                    enabled: win.live
                    onClicked: capture.screenshot()
                }
                Button {
                    text: win.visibility === Window.FullScreen ? "Exit fullscreen" : "Fullscreen"
                    onClicked: win.toggleFullscreen()
                }
                Button {
                    text: "Refresh"
                    enabled: !win.running
                    onClicked: capture.refresh()
                }
                Label {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignRight
                    color: "#a1a1aa"
                    font.features: { "tnum": 1 }
                    text: win.live ? `${capture.delivering}  ·  showing ${capture.fps.toFixed(1)} fps` : ""
                }
            }
        }
    }

    // ── Toast ────────────────────────────────────────────────────────────────

    Rectangle {
        id: toast

        property bool error: false

        function show(text, isError) {
            toastText.text = text;
            error = isError;
            opacity = 1;
            toastTimer.restart();
        }

        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: parent.top
        anchors.topMargin: 16
        width: Math.min(parent.width - 32, toastText.implicitWidth + 28)
        height: toastText.implicitHeight + 16
        radius: 10
        color: error ? "#e67f1d1d" : "#e618181b"
        border.color: "#33ffffff"
        opacity: 0
        visible: opacity > 0

        Behavior on opacity {
            NumberAnimation {
                duration: 200
            }
        }

        Label {
            id: toastText
            anchors.centerIn: parent
            width: Math.min(implicitWidth, parent.parent.width - 60)
            wrapMode: Text.Wrap
            color: "#f4f4f5"
        }

        Timer {
            id: toastTimer
            interval: 4000
            onTriggered: toast.opacity = 0
        }
    }

    // ── Shortcuts ────────────────────────────────────────────────────────────

    Shortcut {
        sequence: "F"
        onActivated: win.toggleFullscreen()
    }
    Shortcut {
        sequence: "Escape"
        enabled: win.visibility === Window.FullScreen
        onActivated: win.visibility = Window.Windowed
    }
    Shortcut {
        sequence: "S"
        onActivated: if (win.live) capture.screenshot()
    }
    Shortcut {
        sequence: "Space"
        onActivated: win.running ? capture.stop() : win.startCapture()
    }
}
