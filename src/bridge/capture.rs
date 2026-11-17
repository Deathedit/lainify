//! `Capture` — the QML-facing controller. It owns the video and audio pipelines and the
//! recorder, follows devices as they're plugged in and out, and reports everything the UI shows.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        // ── Devices ──
        /// JSON: `[{ name, formats: [{ id, label, sizes: [{ width, height, rates: [{ num, den }] }] }] }]`.
        #[qproperty(QString, devices_json, cxx_name = "devicesJson")]
        /// JSON array of audio input names.
        #[qproperty(QString, audio_devices_json, cxx_name = "audioDevicesJson")]
        /// JSON: `[{ id, label, supports444, hardware }]`, the encoders this machine has.
        #[qproperty(QString, encoders_json, cxx_name = "encodersJson")]
        /// True once the first device list has arrived.
        #[qproperty(bool, ready)]
        // ── Video ──
        /// `idle`, `starting`, `live`, `error` or `lost` (the device was unplugged).
        #[qproperty(QString, status)]
        #[qproperty(QString, error_title, cxx_name = "errorTitle")]
        #[qproperty(QString, error_message, cxx_name = "errorMessage")]
        /// The device being captured.
        #[qproperty(QString, device_name, cxx_name = "deviceName")]
        /// The negotiated mode, e.g. `1920×1080 RGB 4:4:4 @ 60 fps`.
        #[qproperty(QString, delivering)]
        #[qproperty(i32, video_width, cxx_name = "videoWidth")]
        #[qproperty(i32, video_height, cxx_name = "videoHeight")]
        /// Frames per second actually arriving, measured over the last tick.
        #[qproperty(f64, fps)]
        // ── Audio ──
        /// `none` (turned off), `missing` (no input found), `starting`, `live` or `error`.
        #[qproperty(QString, audio_status, cxx_name = "audioStatus")]
        #[qproperty(QString, audio_name, cxx_name = "audioName")]
        /// e.g. `48 kHz · stereo`.
        #[qproperty(QString, audio_info, cxx_name = "audioInfo")]
        #[qproperty(f64, level_left, cxx_name = "levelLeft")]
        #[qproperty(f64, level_right, cxx_name = "levelRight")]
        /// Percent of the input's audio that never arrives (0 when it's intact). Some capture
        /// cards drop audio in heavy video modes, which is heard as crackling.
        #[qproperty(f64, audio_loss, cxx_name = "audioLoss")]
        // ── Recording ──
        #[qproperty(bool, recording)]
        /// Closing the file after a recording stopped.
        #[qproperty(bool, saving)]
        #[qproperty(f64, record_seconds, cxx_name = "recordSeconds")]
        #[qproperty(QString, screenshot_dir, cxx_name = "screenshotDir")]
        #[qproperty(QString, recording_dir, cxx_name = "recordingDir")]
        // ── Replay buffer ──
        /// The buffer is running (enabled and capturing).
        #[qproperty(bool, replay_active, cxx_name = "replayActive")]
        /// Seconds of gameplay currently held.
        #[qproperty(f64, replay_buffered, cxx_name = "replayBuffered")]
        /// A clip is being written.
        #[qproperty(bool, clip_saving, cxx_name = "clipSaving")]
        #[qproperty(QString, clips_dir, cxx_name = "clipsDir")]
        type Capture = super::CaptureRust;
    }

    impl cxx_qt::Threading for Capture {}
    impl cxx_qt::Initialize for Capture {}

    extern "RustQt" {
        /// Re-enumerate devices now (they're also watched for hot-plug).
        #[qinvokable]
        fn refresh(self: Pin<&mut Capture>);

        /// Open `device` (index into the devices list) in exactly this mode.
        #[qinvokable]
        fn start(
            self: Pin<&mut Capture>,
            device: i32,
            format: &QString,
            width: i32,
            height: i32,
            fps_num: i32,
            fps_den: i32,
        );

        /// Stop capturing and release the device.
        #[qinvokable]
        fn stop(self: Pin<&mut Capture>);

        /// `auto` (match the video device), `none`, or an audio input's name.
        #[qinvokable]
        #[cxx_name = "setAudio"]
        fn set_audio(self: Pin<&mut Capture>, choice: &QString);

        #[qinvokable]
        #[cxx_name = "setVolume"]
        fn set_volume(self: Pin<&mut Capture>, volume: f64, muted: bool);

        /// Save the current frame as a lossless PNG.
        #[qinvokable]
        fn screenshot(self: Pin<&mut Capture>);

        #[qinvokable]
        #[cxx_name = "startRecording"]
        fn start_recording(
            self: Pin<&mut Capture>,
            encoder: &QString,
            bitrate_kbps: i32,
            chroma_444: bool,
            container: &QString,
        );

        #[qinvokable]
        #[cxx_name = "stopRecording"]
        fn stop_recording(self: Pin<&mut Capture>);

        /// Turn the replay buffer on or off and set what it keeps. It runs whenever capture is
        /// live and `enabled` is set; changing anything restarts it (emptying the buffer).
        #[qinvokable]
        #[cxx_name = "configureReplay"]
        fn configure_replay(
            self: Pin<&mut Capture>,
            enabled: bool,
            seconds: i32,
            encoder: &QString,
            bitrate_kbps: i32,
            chroma_444: bool,
            container: &QString,
        );

        /// Save the replay buffer as a clip.
        #[qinvokable]
        #[cxx_name = "saveClip"]
        fn save_clip(self: Pin<&mut Capture>);

        /// Call about once a second to update `fps`, `recordSeconds` and `replayBuffered`.
        #[qinvokable]
        fn tick(self: Pin<&mut Capture>);

        /// A message for the user. `tone` is `info`, `success` or `error`.
        #[qsignal]
        fn notice(self: Pin<&mut Capture>, text: QString, tone: QString);
    }
}

use core::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use cxx_qt::{CxxQtType, Initialize, Threading};
use cxx_qt_lib::QString;
use gstreamer as gst;
use serde::Serialize;

use crate::audio::{AudioCapture, AudioEvent};
use crate::devices::{self, Snapshot};
use crate::recorder::{Container, Encoder, Options, RecordEvent, Recording, Tap};
use crate::replay::Replay;
use crate::video::{self, Event, Selection};

#[derive(Default)]
pub struct CaptureRust {
    devices_json: QString,
    audio_devices_json: QString,
    encoders_json: QString,
    ready: bool,
    status: QString,
    error_title: QString,
    error_message: QString,
    device_name: QString,
    delivering: QString,
    video_width: i32,
    video_height: i32,
    fps: f64,
    audio_status: QString,
    audio_name: QString,
    audio_info: QString,
    level_left: f64,
    level_right: f64,
    audio_loss: f64,
    recording: bool,
    saving: bool,
    record_seconds: f64,
    screenshot_dir: QString,
    recording_dir: QString,
    replay_active: bool,
    replay_buffered: f64,
    clip_saving: bool,
    clips_dir: QString,

    video_devices: Vec<gst::Device>,
    video_names: Vec<String>,
    audio_devices: Vec<gst::Device>,
    audio_names: Vec<String>,
    force_refresh: Option<Arc<AtomicBool>>,

    video: Option<video::Capture>,
    /// Bumped on every video start/stop so events from an old pipeline are ignored.
    generation: u64,
    source_fps: f64,
    last_frames: u64,
    last_tick: Option<Instant>,

    audio: Option<AudioCapture>,
    audio_generation: u64,
    audio_choice: String,
    volume: f64,
    muted: bool,
    /// The audio-loss toast was shown for this capture session.
    loss_warned: bool,

    tap: Tap,
    record: Option<Recording>,
    record_started: Option<Instant>,

    /// `None` while the replay buffer is turned off.
    replay_config: Option<ReplayConfig>,
    replay: Option<Replay>,
    replay_generation: u64,
}

#[derive(Clone, PartialEq)]
struct ReplayConfig {
    seconds: u32,
    encoder: String,
    bitrate_kbps: i32,
    chroma_444: bool,
    container: String,
}

#[derive(Serialize)]
struct EncoderInfo {
    id: &'static str,
    label: &'static str,
    #[serde(rename = "supports444")]
    supports_444: bool,
    hardware: bool,
}

impl Initialize for qobject::Capture {
    fn initialize(mut self: Pin<&mut Self>) {
        let encoders: Vec<EncoderInfo> = Encoder::available()
            .into_iter()
            .map(|e| EncoderInfo {
                id: e.id(),
                label: e.label(),
                supports_444: e.supports_444(),
                hardware: e.is_hardware(),
            })
            .collect();
        let json = serde_json::to_string(&encoders).unwrap_or_else(|_| "[]".into());
        self.as_mut().set_encoders_json(QString::from(&json));
        self.as_mut().set_status(QString::from("idle"));
        self.as_mut().set_audio_status(QString::from("none"));
        self.as_mut().set_screenshot_dir(QString::from(
            &crate::paths::screenshots_dir().display().to_string(),
        ));
        self.as_mut().set_recording_dir(QString::from(
            &crate::paths::recordings_dir().display().to_string(),
        ));
        self.as_mut().set_clips_dir(QString::from(
            &crate::paths::clips_dir().display().to_string(),
        ));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.audio_choice = "auto".into();
            rust.volume = 1.0;
        }

        let force = Arc::new(AtomicBool::new(false));
        self.as_mut().rust_mut().force_refresh = Some(Arc::clone(&force));
        let qt = self.qt_thread();
        devices::watch(force, move |snap| {
            let _ = qt.queue(move |this: Pin<&mut qobject::Capture>| this.apply_snapshot(snap));
        });
    }
}

impl qobject::Capture {
    // ── Devices ──────────────────────────────────────────────────────────────

    pub fn refresh(self: Pin<&mut Self>) {
        if let Some(force) = &self.rust().force_refresh {
            force.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn apply_snapshot(mut self: Pin<&mut Self>, snap: Snapshot) {
        let video_json = serde_json::to_string(&snap.video_info).unwrap_or_else(|_| "[]".into());
        let audio_json = serde_json::to_string(&snap.audio_names).unwrap_or_else(|_| "[]".into());
        {
            let mut rust = self.as_mut().rust_mut();
            rust.video_names = snap.video_info.iter().map(|d| d.name.clone()).collect();
            rust.video_devices = snap.video;
            rust.audio_names = snap.audio_names;
            rust.audio_devices = snap.audio;
        }
        self.as_mut().set_devices_json(QString::from(&video_json));
        self.as_mut()
            .set_audio_devices_json(QString::from(&audio_json));

        // The device we were capturing from is gone: stop, and let the UI wait for it.
        let active = self.rust().device_name.to_string();
        let status = self.rust().status.to_string();
        let capturing = matches!(status.as_str(), "live" | "starting" | "error");
        if capturing && !active.is_empty() && !self.rust().video_names.contains(&active) {
            self.as_mut().stop();
            self.as_mut().set_device_name(QString::from(&active));
            self.as_mut().set_status(QString::from("lost"));
            self.as_mut()
                .set_error_title(QString::from("Capture device disconnected"));
            self.as_mut().set_error_message(QString::from(
                "Plug it back in and lainify picks up where it left off.",
            ));
        } else if self.rust().video.is_some() {
            // An audio input may have appeared or vanished.
            let audio_status = self.rust().audio_status.to_string();
            let audio_gone = self.rust().audio.is_some()
                && !self
                    .rust()
                    .audio_names
                    .contains(&self.rust().audio_name.to_string());
            if audio_gone || audio_status == "missing" || audio_status == "error" {
                self.as_mut().start_audio();
            }
        }

        if !self.rust().ready {
            self.as_mut().set_ready(true);
        }
    }

    // ── Video ────────────────────────────────────────────────────────────────

    pub fn start(
        mut self: Pin<&mut Self>,
        device: i32,
        format: &QString,
        width: i32,
        height: i32,
        fps_num: i32,
        fps_den: i32,
    ) {
        self.as_mut().stop();
        let index = usize::try_from(device).ok();
        let Some((device, name)) = index.and_then(|i| {
            let rust = self.rust();
            Some((
                rust.video_devices.get(i)?.clone(),
                rust.video_names.get(i)?.clone(),
            ))
        }) else {
            self.as_mut().notify("No such device", "error");
            return;
        };

        let generation = self.rust().generation;
        let qt = self.qt_thread();
        let selection = Selection {
            device,
            format: format.to_string(),
            width,
            height,
            fps: gst::Fraction::new(fps_num, fps_den.max(1)),
        };
        eprintln!(
            "lainify: opening {name} as {} {width}x{height} @ {fps_num}/{fps_den}",
            selection.format
        );

        self.as_mut().set_device_name(QString::from(&name));
        self.as_mut().set_status(QString::from("starting"));
        self.as_mut().rust_mut().loss_warned = false;
        let tap = self.rust().tap.clone();
        let started = video::Capture::start(selection, tap, move |event| {
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                if this.rust().generation != generation {
                    return;
                }
                match event {
                    Event::Live(mode) => {
                        eprintln!("lainify: live, source delivers {}", mode.describe());
                        this.as_mut()
                            .set_delivering(QString::from(&mode.describe()));
                        this.as_mut().set_video_width(mode.width);
                        this.as_mut().set_video_height(mode.height);
                        this.as_mut().rust_mut().source_fps = mode.fps;
                        this.as_mut().set_status(QString::from("live"));
                        this.as_mut().ensure_replay();
                    }
                    Event::Error(msg) => {
                        eprintln!("lainify: capture error: {msg}");
                        this.as_mut().fail(&msg);
                    }
                }
            });
        });

        match started {
            Ok(capture) => {
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.video = Some(capture);
                    rust.last_frames = 0;
                    rust.last_tick = Some(Instant::now());
                }
                self.as_mut().start_audio();
            }
            Err(msg) => {
                eprintln!("lainify: start failed: {msg}");
                self.as_mut().fail(&msg);
            }
        }
    }

    pub fn stop(mut self: Pin<&mut Self>) {
        if self.rust().record.is_some() && self.rust().recording {
            self.as_mut().stop_recording();
        }
        self.as_mut().stop_replay();
        {
            let mut rust = self.as_mut().rust_mut();
            rust.generation += 1;
            rust.video = None; // drops the pipeline
        }
        self.as_mut().stop_audio();
        self.as_mut().set_status(QString::from("idle"));
        self.as_mut().set_delivering(QString::default());
        self.as_mut().set_video_width(0);
        self.as_mut().set_video_height(0);
        self.as_mut().set_fps(0.0);
    }

    /// Stop with an error the empty state explains.
    fn fail(mut self: Pin<&mut Self>, detail: &str) {
        let name = self.rust().device_name.clone();
        self.as_mut().stop();
        self.as_mut().set_device_name(name);
        let (title, message) = explain(detail);
        self.as_mut().set_error_title(QString::from(title));
        self.as_mut().set_error_message(QString::from(&message));
        self.as_mut().set_status(QString::from("error"));
    }

    // ── Audio ────────────────────────────────────────────────────────────────

    pub fn set_audio(mut self: Pin<&mut Self>, choice: &QString) {
        self.as_mut().rust_mut().audio_choice = choice.to_string();
        if self.rust().video.is_some() {
            self.as_mut().start_audio();
        }
    }

    pub fn set_volume(mut self: Pin<&mut Self>, volume: f64, muted: bool) {
        let mut rust = self.as_mut().rust_mut();
        rust.volume = volume;
        rust.muted = muted;
        if let Some(audio) = &rust.audio {
            audio.set_volume(volume, muted);
        }
    }

    fn stop_audio(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.audio_generation += 1;
            rust.audio = None;
        }
        self.as_mut().set_audio_name(QString::default());
        self.as_mut().set_audio_info(QString::default());
        self.as_mut().set_level_left(0.0);
        self.as_mut().set_level_right(0.0);
        self.as_mut().set_audio_loss(0.0);
        self.as_mut().set_audio_status(QString::from("none"));
    }

    /// (Re)open the audio input the current choice points at.
    fn start_audio(mut self: Pin<&mut Self>) {
        self.as_mut().stop_audio();
        let choice = self.rust().audio_choice.clone();
        if choice == "none" {
            return;
        }

        let found = {
            let rust = self.rust();
            let index = if choice == "auto" {
                match_audio(&rust.device_name.to_string(), &rust.audio_names)
            } else {
                rust.audio_names.iter().position(|n| *n == choice)
            };
            index.map(|i| (rust.audio_devices[i].clone(), rust.audio_names[i].clone()))
        };
        let Some((device, name)) = found else {
            self.as_mut().set_audio_status(QString::from("missing"));
            return;
        };

        let generation = self.rust().audio_generation;
        let qt = self.qt_thread();
        let (tap, volume, muted) = {
            let rust = self.rust();
            (rust.tap.clone(), rust.volume, rust.muted)
        };
        self.as_mut().set_audio_name(QString::from(&name));
        self.as_mut().set_audio_status(QString::from("starting"));
        let started = AudioCapture::start(&device, tap, volume, muted, move |event| {
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                if this.rust().audio_generation != generation {
                    return;
                }
                match event {
                    AudioEvent::Live(info) => {
                        this.as_mut().set_audio_info(QString::from(&info));
                        this.as_mut().set_audio_status(QString::from("live"));
                    }
                    AudioEvent::Levels(left, right) => {
                        this.as_mut().set_level_left(left);
                        this.as_mut().set_level_right(right);
                    }
                    AudioEvent::Rate { ppm, losing } => {
                        eprintln!("lainify: audio input rate {ppm:+.0} ppm");
                        let loss = if losing { -ppm / 10_000.0 } else { 0.0 };
                        this.as_mut().set_audio_loss(loss.max(0.0));
                        if losing && !this.rust().loss_warned {
                            this.as_mut().rust_mut().loss_warned = true;
                            this.notify(
                                "Audio is dropping out (crackling) — see Settings → Audio",
                                "error",
                            );
                        }
                    }
                    AudioEvent::Error(msg) => {
                        eprintln!("lainify: audio error: {msg}");
                        this.as_mut().audio_failed();
                    }
                }
            });
        });
        match started {
            Ok(audio) => self.as_mut().rust_mut().audio = Some(audio),
            Err(msg) => {
                eprintln!("lainify: audio start failed: {msg}");
                self.as_mut().audio_failed();
            }
        }
    }

    fn audio_failed(mut self: Pin<&mut Self>) {
        let name = self.rust().audio_name.clone();
        self.as_mut().stop_audio();
        self.as_mut().set_audio_name(name);
        self.as_mut().set_audio_status(QString::from("error"));
        self.as_mut()
            .notify("Audio input could not be opened — video only", "error");
    }

    // ── Screenshots ──────────────────────────────────────────────────────────

    pub fn screenshot(mut self: Pin<&mut Self>) {
        let Some(sample) = self.rust().video.as_ref().and_then(|c| c.last_sample()) else {
            self.as_mut().notify("Nothing to capture yet", "error");
            return;
        };
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let result = save_png(&sample);
            let _ = qt.queue(move |this: Pin<&mut qobject::Capture>| match result {
                Ok(name) => this.notify(&format!("Saved {name}"), "success"),
                Err(msg) => this.notify(&format!("Screenshot failed: {msg}"), "error"),
            });
        });
    }

    // ── Recording ────────────────────────────────────────────────────────────

    pub fn start_recording(
        mut self: Pin<&mut Self>,
        encoder: &QString,
        bitrate_kbps: i32,
        chroma_444: bool,
        container: &QString,
    ) {
        if self.rust().record.is_some() {
            return;
        }
        let Some(caps) = self
            .rust()
            .video
            .as_ref()
            .and_then(|c| c.last_sample())
            .and_then(|s| s.caps_owned())
        else {
            self.as_mut().notify("Nothing to record yet", "error");
            return;
        };
        let Some(encoder) = pick_encoder(&encoder.to_string()) else {
            self.as_mut()
                .notify("No video encoder is available", "error");
            return;
        };

        let opts = Options {
            encoder,
            bitrate_kbps: bitrate_kbps.clamp(1_000, 400_000) as u32,
            chroma_444,
            container: Container::from_id(&container.to_string()),
            fps: self.rust().source_fps,
            keyframe_seconds: 2.0,
        };
        let with_audio = self.rust().audio.is_some();
        let qt = self.qt_thread();
        let tap = self.rust().tap.clone();
        let started = Recording::start(opts, caps, with_audio, tap, move |event| {
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                {
                    let mut rust = this.as_mut().rust_mut();
                    rust.record = None;
                    rust.record_started = None;
                }
                this.as_mut().set_recording(false);
                this.as_mut().set_saving(false);
                this.as_mut().set_record_seconds(0.0);
                match event {
                    RecordEvent::Saved(path) => {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        this.notify(&format!("Saved {name}"), "success");
                    }
                    RecordEvent::Error(msg) => {
                        eprintln!("lainify: recording error: {msg}");
                        this.notify(&format!("Recording failed: {msg}"), "error");
                    }
                }
            });
        });
        match started {
            Ok(record) => {
                eprintln!("lainify: recording to {}", record.path.display());
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.record = Some(record);
                    rust.record_started = Some(Instant::now());
                }
                self.as_mut().set_record_seconds(0.0);
                self.as_mut().set_recording(true);
            }
            Err(msg) => {
                self.as_mut()
                    .notify(&format!("Could not start recording: {msg}"), "error");
            }
        }
    }

    pub fn stop_recording(mut self: Pin<&mut Self>) {
        if !self.rust().recording {
            return;
        }
        if let Some(record) = &self.rust().record {
            record.finish();
        }
        self.as_mut().set_recording(false);
        self.as_mut().set_saving(true);
    }

    // ── Replay buffer ────────────────────────────────────────────────────────

    pub fn configure_replay(
        mut self: Pin<&mut Self>,
        enabled: bool,
        seconds: i32,
        encoder: &QString,
        bitrate_kbps: i32,
        chroma_444: bool,
        container: &QString,
    ) {
        let config = enabled.then(|| ReplayConfig {
            seconds: seconds.clamp(5, 600) as u32,
            encoder: encoder.to_string(),
            bitrate_kbps: bitrate_kbps.clamp(1_000, 400_000),
            chroma_444,
            container: container.to_string(),
        });
        if config == self.rust().replay_config {
            return;
        }
        self.as_mut().rust_mut().replay_config = config;
        self.as_mut().stop_replay();
        self.as_mut().ensure_replay();
    }

    /// Start the buffer if it's wanted, capture is live and the first frame has arrived.
    /// Called on going live, on configuration changes, and every tick (to catch the first frame).
    fn ensure_replay(mut self: Pin<&mut Self>) {
        if self.rust().replay.is_some() || self.rust().status.to_string() != "live" {
            return;
        }
        let Some(config) = self.rust().replay_config.clone() else {
            return;
        };
        let Some(caps) = self
            .rust()
            .video
            .as_ref()
            .and_then(|c| c.last_sample())
            .and_then(|s| s.caps_owned())
        else {
            return;
        };
        let Some(encoder) = pick_encoder(&config.encoder) else {
            return;
        };
        let opts = Options {
            encoder,
            bitrate_kbps: config.bitrate_kbps as u32,
            chroma_444: config.chroma_444,
            container: Container::from_id(&config.container),
            fps: self.rust().source_fps,
            // Clips start on a keyframe, so this is how much longer than asked a clip can be.
            keyframe_seconds: 1.0,
        };
        let generation = self.rust().replay_generation;
        let qt = self.qt_thread();
        let tap = self.rust().tap.clone();
        let started = Replay::start(opts, config.seconds, caps, tap, move |msg| {
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                if this.rust().replay_generation != generation {
                    return;
                }
                eprintln!("lainify: replay buffer error: {msg}");
                this.as_mut().stop_replay();
                this.notify(&format!("Replay buffer stopped: {msg}"), "error");
            });
        });
        match started {
            Ok(replay) => {
                eprintln!(
                    "lainify: replay buffer on, last {} s with {}",
                    config.seconds,
                    encoder.id()
                );
                self.as_mut().rust_mut().replay = Some(replay);
                self.as_mut().set_replay_buffered(0.0);
                self.as_mut().set_replay_active(true);
            }
            Err(msg) => {
                eprintln!("lainify: replay buffer failed: {msg}");
                // Don't retry every tick with a setup that can't work.
                self.as_mut().rust_mut().replay_config = None;
                self.as_mut()
                    .notify(&format!("Replay buffer could not start: {msg}"), "error");
            }
        }
    }

    fn stop_replay(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.replay_generation += 1;
            rust.replay = None;
        }
        self.as_mut().set_replay_active(false);
        self.as_mut().set_replay_buffered(0.0);
    }

    pub fn save_clip(mut self: Pin<&mut Self>) {
        if self.rust().clip_saving {
            return;
        }
        let qt = self.qt_thread();
        let saved = match &self.rust().replay {
            None => Err("Replay buffer is off — turn it on in Settings".to_string()),
            Some(replay) => replay.save(move |result| {
                let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                    this.as_mut().set_clip_saving(false);
                    match result {
                        Ok((path, seconds)) => {
                            eprintln!("lainify: saved clip {}", path.display());
                            let name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_default();
                            this.notify(
                                &format!("Saved last {} s — {name}", seconds.round()),
                                "success",
                            );
                        }
                        Err(msg) => {
                            eprintln!("lainify: clip failed: {msg}");
                            this.notify(&format!("Clip failed: {msg}"), "error");
                        }
                    }
                });
            }),
        };
        match saved {
            Ok(()) => self.as_mut().set_clip_saving(true),
            Err(msg) => self.as_mut().notify(&msg, "info"),
        }
    }

    // ── Housekeeping ─────────────────────────────────────────────────────────

    pub fn tick(mut self: Pin<&mut Self>) {
        let now = Instant::now();
        let (fps, seconds) = {
            let mut rust = self.as_mut().rust_mut();
            let fps = match rust.video.as_ref().map(|c| c.frames()) {
                Some(frames) => {
                    let elapsed = rust
                        .last_tick
                        .map(|t| now.duration_since(t).as_secs_f64())
                        .unwrap_or(0.0);
                    let fps = if elapsed > 0.0 {
                        frames.saturating_sub(rust.last_frames) as f64 / elapsed
                    } else {
                        0.0
                    };
                    rust.last_frames = frames;
                    rust.last_tick = Some(now);
                    fps
                }
                None => 0.0,
            };
            let seconds = rust
                .record_started
                .map(|t| now.duration_since(t).as_secs_f64());
            (fps, seconds)
        };
        self.as_mut().set_fps(fps);
        if let Some(seconds) = seconds
            && self.rust().recording
        {
            self.as_mut().set_record_seconds(seconds);
        }
        self.as_mut().ensure_replay();
        let buffered = self.rust().replay.as_ref().map(|r| r.buffered_seconds());
        if let Some(buffered) = buffered {
            self.as_mut().set_replay_buffered(buffered);
        }
    }

    fn notify(self: Pin<&mut Self>, text: &str, tone: &str) {
        self.notice(QString::from(text), QString::from(tone));
    }
}

/// The encoder with this id if the machine has it, else the first one it has.
fn pick_encoder(id: &str) -> Option<Encoder> {
    let available = Encoder::available();
    Encoder::from_id(id)
        .filter(|e| available.contains(e))
        .or_else(|| available.first().copied())
}

/// The audio input that belongs to the video device, e.g. `Elgato 4K X Analog Stereo` for
/// `Elgato 4K X`.
fn match_audio(video_name: &str, audio_names: &[String]) -> Option<usize> {
    let video = video_name.to_lowercase();
    if video.is_empty() {
        return None;
    }
    audio_names.iter().position(|a| {
        let a = a.to_lowercase();
        a.contains(&video) || video.contains(&a)
    })
}

/// A title and explanation for a GStreamer error.
fn explain(detail: &str) -> (&'static str, String) {
    let lower = detail.to_lowercase();
    if lower.contains("busy") {
        (
            "Device is busy",
            "Another app is using the capture card. Close it, then retry.".into(),
        )
    } else if lower.contains("not-negotiated") || lower.contains("refused") {
        (
            "Mode not supported",
            "The device refused this format or frame rate. Pick another one in Settings.".into(),
        )
    } else if lower.contains("no such device") || lower.contains("stopped sending") {
        (
            "Capture device disconnected",
            "Plug it back in and lainify picks up where it left off.".into(),
        )
    } else {
        ("Capture failed", detail.to_string())
    }
}

/// Write `sample` as a lossless PNG and return its file name.
fn save_png(sample: &gst::Sample) -> Result<String, String> {
    let png = video::encode_png(sample)?;
    let dir = crate::paths::screenshots_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("lainify-{}.png", crate::paths::timestamp());
    std::fs::write(dir.join(&name), png).map_err(|e| e.to_string())?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::{explain, match_audio};

    #[test]
    fn matches_audio_to_video_device() {
        let inputs = vec![
            "UMC204HD 192k Analog Stereo".to_string(),
            "Elgato 4K X Analog Stereo".to_string(),
        ];
        assert_eq!(match_audio("Elgato 4K X", &inputs), Some(1));
        assert_eq!(match_audio("Webcam", &inputs), None);
        assert_eq!(match_audio("", &inputs), None);
    }

    #[test]
    fn explains_errors() {
        assert_eq!(explain("Device '/dev/video0' is busy").0, "Device is busy");
        assert_eq!(explain("something odd").0, "Capture failed");
    }
}
