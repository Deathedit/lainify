import QtQuick
import QtQuick.Controls.Basic
import QtCore
import QtMultimedia
import com.lainify
import com.lainify.native

ApplicationWindow {
    id: win

    width: 1280
    height: 760
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    color: "black"
    title: live && capture.deviceName ? `${capture.deviceName} · lainify` : "lainify"
    font.family: Theme.font
    font.features: Theme.fontFeatures

    FontLoader {
        id: inter
        source: "qrc:/qt/qml/com/lainify/assets/InterVariable.ttf"
        onStatusChanged: if (status === FontLoader.Ready)
            Theme.font = name
    }

    // Remembered between runs (~/.config/lainify/lainify.conf).
    Settings {
        id: prefs
        property string videoDevice: ""
        property string format: "BGR"
        property int width: 1920
        property int height: 1080
        property real fps: 60
        property string audio: "auto"
        property real volume: 1
        property bool muted: false
        property string encoder: ""
        property int bitrate: 12000
        property bool chroma444: false
        property string container: "mkv"
        property int hideDelay: 2500
        property bool replayEnabled: false
        property int replaySeconds: 60
        property int replayBitrate: 20000
    }

    // ── Devices and the current selection ────────────────────────────────────

    function parse(json, fallback) {
        try {
            return JSON.parse(json || "null") ?? fallback;
        } catch (e) {
            return fallback;
        }
    }
    function pick(list, pred) {
        const i = list.findIndex(pred);
        return i < 0 ? 0 : i;
    }
    function fpsOf(r) {
        return r.num / r.den;
    }

    readonly property var devices: parse(capture.devicesJson, [])
    readonly property var audioDevices: parse(capture.audioDevicesJson, [])
    readonly property var encoders: parse(capture.encodersJson, [])

    // Derived from what the user asked for (prefs), falling back to the closest mode the device
    // has, so switching device or format keeps as much of the choice as it can.
    readonly property int deviceIndex: devices.length ? pick(devices, d => d.name === prefs.videoDevice) : -1
    readonly property var device: devices[deviceIndex] ?? null
    readonly property var format: device ? device.formats[pick(device.formats, f => f.id === prefs.format)] : null
    readonly property var size: format ? format.sizes[pick(format.sizes, s => s.width === prefs.width && s.height === prefs.height)] : null
    readonly property var rate: {
        if (!size)
            return null;
        let best = size.rates[0];
        for (const r of size.rates)
            if (Math.abs(fpsOf(r) - prefs.fps) < Math.abs(fpsOf(best) - prefs.fps))
                best = r;
        return best;
    }
    readonly property var encoder: encoders.find(e => e.id === prefs.encoder) ?? encoders[0] ?? null

    // ── State ────────────────────────────────────────────────────────────────

    readonly property bool live: capture.status === "live"
    readonly property bool running: live || capture.status === "starting"
    readonly property bool hasAudio: capture.audioStatus === "live"
    property bool userStopped: false
    property bool panelOpen: false
    property bool idle: false
    readonly property bool fullscreen: visibility === Window.FullScreen
    readonly property bool uiVisible: !live || !idle || panelOpen || controls.hovered

    readonly property string phase: {
        if (live || !capture.ready)
            return "";
        switch (capture.status) {
        case "starting":
            return "starting";
        case "error":
            return "error";
        case "lost":
            return "lost";
        }
        if (devices.length === 0)
            return "welcome";
        return userStopped ? "stopped" : "starting";
    }

    function startCapture() {
        if (!device || !format || !size || !rate)
            return;
        userStopped = false;
        prefs.videoDevice = device.name;
        capture.setAudio(prefs.audio);
        capture.setVolume(prefs.volume, prefs.muted);
        capture.start(deviceIndex, format.id, size.width, size.height, rate.num, rate.den);
    }
    function stopCapture() {
        userStopped = true;
        capture.stop();
    }
    /// After a selection change: reopen if we were capturing (or trying to).
    function reopen() {
        if (["live", "starting", "error"].includes(capture.status) || (capture.status === "idle" && !userStopped))
            Qt.callLater(startCapture);
    }
    function onDevicesUpdated() {
        if (capture.status === "lost" && devices.some(d => d.name === capture.deviceName)) {
            toasts.show("Capture device reconnected", "success");
            prefs.videoDevice = capture.deviceName;
            startCapture();
        } else if (capture.status === "idle" && !userStopped && devices.length > 0) {
            startCapture();
        }
    }
    function setVolume(value) {
        prefs.volume = Math.round(Math.max(0, Math.min(1.5, value)) * 100) / 100;
        prefs.muted = false;
        capture.setVolume(prefs.volume, prefs.muted);
    }
    function toggleMute() {
        prefs.muted = !prefs.muted;
        capture.setVolume(prefs.volume, prefs.muted);
    }
    function screenshot() {
        if (!live)
            return;
        flash.restart();
        capture.screenshot();
    }
    function toggleRecording() {
        if (capture.recording)
            capture.stopRecording();
        else if (live && !capture.saving && encoder)
            capture.startRecording(encoder.id, prefs.bitrate, prefs.chroma444, prefs.container);
    }
    function saveClip() {
        if (!prefs.replayEnabled)
            toasts.show("Replay buffer is off — turn it on in Settings", "info");
        else if (live)
            capture.saveClip();
    }
    // The replay buffer shares the recording's encoder, colour and file format.
    readonly property var replayConfig: [prefs.replayEnabled, prefs.replaySeconds, encoder ? encoder.id : "", prefs.replayBitrate, prefs.chroma444, prefs.container]
    onReplayConfigChanged: Qt.callLater(syncReplay)
    function syncReplay() {
        capture.configureReplay(prefs.replayEnabled, prefs.replaySeconds, encoder ? encoder.id : "", prefs.replayBitrate, prefs.chroma444, prefs.container);
    }
    function toggleFullscreen() {
        win.visibility = fullscreen ? Window.Windowed : Window.FullScreen;
    }
    function poke() {
        idle = false;
        if (prefs.hideDelay > 0)
            idleTimer.restart();
        else
            idleTimer.stop();
    }

    Capture {
        id: capture

        onNotice: (text, tone) => toasts.show(text, tone)
        // Deferred: the handler can run before bindings on devicesJson have re-evaluated.
        onDevicesJsonChanged: Qt.callLater(win.onDevicesUpdated)
        onReadyChanged: Qt.callLater(win.onDevicesUpdated)
    }

    Timer {
        interval: 1000
        repeat: true
        running: win.running || capture.recording || capture.replayActive
        onTriggered: capture.tick()
    }

    Timer {
        id: idleTimer
        interval: Math.max(500, prefs.hideDelay)
        onTriggered: win.idle = true
    }

    Rectangle {
        id: stage
        anchors.fill: parent
        color: "black"

        // ── Picture ──────────────────────────────────────────────────────────────

        VideoOutput {
            id: video
            anchors.fill: parent
            fillMode: VideoOutput.PreserveAspectFit
            opacity: win.live ? 1 : 0
            Behavior on opacity {
                NumberAnimation {
                    duration: 500
                }
            }
            Component.onCompleted: FrameSink.videoSink = video.videoSink
        }

        // Movement shows the controls; double-click toggles fullscreen.
        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: win.uiVisible ? Qt.ArrowCursor : Qt.BlankCursor
            onPositionChanged: win.poke()
            onPressed: win.poke()
            onDoubleClicked: if (win.live)
                win.toggleFullscreen()
        }

        Rectangle {
            id: flashRect
            anchors.fill: parent
            color: "white"
            opacity: 0
            NumberAnimation {
                id: flash
                target: flashRect
                property: "opacity"
                from: 0.55
                to: 0
                duration: 450
                easing.type: Easing.OutQuad
            }
        }

        EmptyState {
            visible: win.phase !== ""
            phase: win.phase
            deviceName: capture.deviceName || (win.device ? win.device.name : "")
            errorTitle: capture.errorTitle
            errorMessage: capture.errorMessage
            onStartRequested: win.startCapture()
            onRefresh: capture.refresh()
            onOpenSettings: win.panelOpen = true
        }

        StatusPill {
            shown: win.uiVisible
            live: win.live
            recording: capture.recording
            elapsed: capture.recordSeconds
            replay: capture.replayActive
            replayBuffered: capture.replayBuffered
            replayLength: prefs.replaySeconds
            deviceName: capture.deviceName
            details: {
                const parts = [];
                if (capture.videoWidth)
                    parts.push(`${capture.videoWidth}×${capture.videoHeight}`);
                const fps = capture.fps > 0 ? Math.round(capture.fps * 10) / 10 : (win.rate ? win.fpsOf(win.rate) : 0);
                if (fps)
                    parts.push(`${Theme.fps(fps)} fps`);
                if (win.hasAudio && capture.audioInfo)
                    parts.push(capture.audioInfo);
                return parts.join(" · ");
            }
        }

        ControlBar {
            id: controls
            visible: capture.ready
            shown: win.uiVisible
            live: win.live
            running: win.running
            hasAudio: win.hasAudio
            volume: prefs.volume
            muted: prefs.muted
            levelLeft: capture.levelLeft
            levelRight: capture.levelRight
            recording: capture.recording
            saving: capture.saving
            elapsed: capture.recordSeconds
            replayOn: prefs.replayEnabled
            replayActive: capture.replayActive
            replayBuffered: capture.replayBuffered
            replayLength: prefs.replaySeconds
            clipSaving: capture.clipSaving
            onClip: win.saveClip()
            fullscreen: win.fullscreen
            panelOpen: win.panelOpen
            onPower: win.running ? win.stopCapture() : win.startCapture()
            onScreenshot: win.screenshot()
            onRecord: win.toggleRecording()
            onSettings: win.panelOpen = !win.panelOpen
            onToggleFullscreen: win.toggleFullscreen()
            onVolumeMoved: value => win.setVolume(value)
            onMuteToggled: win.toggleMute()
        }

        SettingsPanel {
            id: panel

            open: win.panelOpen
            onCloseRequested: win.panelOpen = false
            recording: capture.recording

            videoOptions: win.devices.map(d => ({
                        "value": d.name,
                        "label": d.name
                    }))
            videoValue: win.device ? win.device.name : ""
            onVideoChanged: v => {
                prefs.videoDevice = v;
                win.reopen();
            }

            formatOptions: win.device ? win.device.formats.map(f => ({
                        "value": f.id,
                        "label": f.label
                    })) : []
            formatValue: win.format ? win.format.id : ""
            onFormatChanged: v => {
                prefs.format = v;
                win.reopen();
            }

            sizeOptions: win.format ? win.format.sizes.map(s => ({
                        "value": `${s.width}x${s.height}`,
                        "label": `${s.width}×${s.height}`
                    })) : []
            sizeValue: win.size ? `${win.size.width}x${win.size.height}` : ""
            onSizeChanged: v => {
                const [w, h] = v.split("x").map(Number);
                prefs.width = w;
                prefs.height = h;
                win.reopen();
            }

            rateOptions: win.size ? win.size.rates.map(r => ({
                        "value": `${r.num}/${r.den}`,
                        "label": `${Theme.fps(win.fpsOf(r))} fps`
                    })) : []
            rateValue: win.rate ? `${win.rate.num}/${win.rate.den}` : ""
            onRateChanged: v => {
                const [n, d] = v.split("/").map(Number);
                prefs.fps = n / d;
                win.reopen();
            }

            videoInfo: win.live ? `Delivering ${capture.delivering}${capture.fps > 0 ? ` · showing ${capture.fps.toFixed(1)} fps` : ""}` : ""

            audioOptions: [
                {
                    "value": "auto",
                    "label": "Match video device"
                },
                {
                    "value": "none",
                    "label": "No audio"
                }
            ].concat(win.audioDevices.map(n => ({
                        "value": n,
                        "label": n
                    })))
            audioValue: prefs.audio
            onAudioChanged: v => {
                prefs.audio = v;
                capture.setAudio(v);
            }
            audioInfo: {
                switch (capture.audioStatus) {
                case "live":
                    return [capture.audioName, capture.audioInfo].filter(Boolean).join(" · ");
                case "starting":
                    return `Opening ${capture.audioName}…`;
                case "error":
                    return `${capture.audioName || "The audio input"} could not be opened.`;
                case "missing":
                    return prefs.audio === "auto" ? "No audio input matches this device. Pick one above, or choose No audio." : "That audio input isn't connected.";
                }
                return "";
            }

            encoderOptions: win.encoders.map(e => ({
                        "value": e.id,
                        "label": e.label
                    }))
            encoderValue: win.encoder ? win.encoder.id : ""
            onEncoderChanged: v => prefs.encoder = v

            readonly property int recommended: win.size && win.size.height >= 2160 ? 40 : win.size && win.size.height >= 1440 ? 20 : 12
            bitrateOptions: [6, 12, 20, 40, 80].map(m => ({
                        "value": String(m * 1000),
                        "label": `${m} Mbps${m === recommended ? " (recommended)" : ""}`
                    }))
            bitrateValue: String(prefs.bitrate)
            onBitrateChanged: v => prefs.bitrate = Number(v)

            chromaAvailable: win.encoder ? win.encoder.supports444 : false
            chromaOptions: [
                {
                    "value": "420",
                    "label": "4:2:0 — plays everywhere"
                },
                {
                    "value": "444",
                    "label": "4:4:4 — full colour"
                }
            ]
            chromaValue: prefs.chroma444 ? "444" : "420"
            onChromaChanged: v => prefs.chroma444 = v === "444"

            containerOptions: [
                {
                    "value": "mkv",
                    "label": "MKV — survives crashes"
                },
                {
                    "value": "mp4",
                    "label": "MP4"
                }
            ]
            containerValue: prefs.container
            onContainerChanged: v => prefs.container = v

            hideOptions: [
                {
                    "value": "1500",
                    "label": "After 1.5 s"
                },
                {
                    "value": "2500",
                    "label": "After 2.5 s"
                },
                {
                    "value": "5000",
                    "label": "After 5 s"
                },
                {
                    "value": "0",
                    "label": "Never"
                }
            ]
            hideValue: String(prefs.hideDelay)
            onHideChanged: v => {
                prefs.hideDelay = Number(v);
                win.poke();
            }

            replayOptions: [
                {
                    "value": "off",
                    "label": "Off"
                },
                {
                    "value": "on",
                    "label": "On — press C to save a clip"
                }
            ]
            replayValue: prefs.replayEnabled ? "on" : "off"
            onReplayChanged: v => prefs.replayEnabled = v === "on"
            replayLengthOptions: [30, 60, 120, 300].map(n => ({
                        "value": String(n),
                        "label": n < 60 ? `Last ${n} seconds` : n === 60 ? "Last minute" : `Last ${n / 60} minutes`
                    }))
            replayLengthValue: String(prefs.replaySeconds)
            onReplayLengthChanged: v => prefs.replaySeconds = Number(v)
            replayBitrateOptions: [12, 20, 40, 80].map(m => ({
                        "value": String(m * 1000),
                        "label": `${m} Mbps${m === recommended ? " (recommended)" : ""}`
                    }))
            replayBitrateValue: String(prefs.replayBitrate)
            onReplayBitrateChanged: v => prefs.replayBitrate = Number(v)
            replayHint: {
                if (!prefs.replayEnabled)
                    return "Keeps the last moments of gameplay in memory, so you can save a clip after something happens.";
                const mb = Math.round(prefs.replayBitrate * prefs.replaySeconds / 8 / 1000 * 1.05);
                const parts = [`Uses about ${mb >= 1000 ? (mb / 1000).toFixed(1) + " GB" : mb + " MB"} of memory. Encoder, colour and file format follow Recording.`];
                if (win.encoder && !win.encoder.hardware)
                    parts.push("x264 encodes on the CPU the whole time the buffer is on.");
                if (capture.replayActive)
                    parts.push(`Holding ${Theme.duration(Math.min(capture.replayBuffered, prefs.replaySeconds))}.`);
                return parts.join(" ");
            }

            screenshotDir: capture.screenshotDir
            recordingDir: capture.recordingDir
            clipsDir: capture.clipsDir
        }

        Toasts {
            id: toasts
        }
    }

    // ── Shortcuts (same keys as the browser version) ─────────────────────────

    Shortcut {
        sequence: "F"
        onActivated: win.toggleFullscreen()
    }
    Shortcut {
        sequence: "M"
        enabled: win.hasAudio
        onActivated: win.toggleMute()
    }
    Shortcut {
        sequence: "Up"
        autoRepeat: true
        onActivated: win.setVolume(prefs.volume + 0.05)
    }
    Shortcut {
        sequence: "Down"
        autoRepeat: true
        onActivated: win.setVolume(prefs.volume - 0.05)
    }
    Shortcut {
        sequence: "S"
        onActivated: win.screenshot()
    }
    Shortcut {
        sequence: "R"
        onActivated: win.toggleRecording()
    }
    Shortcut {
        sequence: "C"
        onActivated: win.saveClip()
    }
    Shortcut {
        sequence: ","
        onActivated: win.panelOpen = !win.panelOpen
    }
    Shortcut {
        sequence: "Escape"
        onActivated: {
            if (win.panelOpen)
                win.panelOpen = false;
            else if (win.fullscreen)
                win.visibility = Window.Windowed;
        }
    }

    Component.onCompleted: poke()
}
