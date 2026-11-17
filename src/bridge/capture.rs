//! `Capture` — the QML-facing controller: lists devices, starts/stops the pipeline, takes
//! screenshots, and reports what the card is delivering.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// JSON: `[{ name, formats: [{ id, label, sizes: [{ width, height, rates: [{ num, den }] }] }] }]`.
        #[qproperty(QString, devices_json, cxx_name = "devicesJson")]
        /// `idle`, `starting`, `live` or `error`.
        #[qproperty(QString, status)]
        /// The negotiated source mode, e.g. `1920×1080 BGR @ 60 fps`.
        #[qproperty(QString, delivering)]
        /// Frames per second actually arriving, measured over the last tick.
        #[qproperty(f64, fps)]
        type Capture = super::CaptureRust;
    }

    impl cxx_qt::Threading for Capture {}

    extern "RustQt" {
        /// Re-enumerate capture devices in the background.
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

        #[qinvokable]
        fn stop(self: Pin<&mut Capture>);

        /// Save the current frame as a lossless PNG under ~/Pictures/lainify.
        #[qinvokable]
        fn screenshot(self: Pin<&mut Capture>);

        /// Call about once a second to update `fps`.
        #[qinvokable]
        fn tick(self: Pin<&mut Capture>);

        /// A message for the user.
        #[qsignal]
        fn notice(self: Pin<&mut Capture>, text: QString, error: bool);
    }
}

use core::pin::Pin;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;

use crate::devices;
use crate::pipeline::{self, Event, Selection};

#[derive(Default)]
pub struct CaptureRust {
    devices_json: QString,
    status: QString,
    delivering: QString,
    fps: f64,

    devices: Vec<gst::Device>,
    capture: Option<pipeline::Capture>,
    /// Bumped on every start/stop so events from an old pipeline are ignored.
    generation: u64,
    last_frames: u64,
    last_tick: Option<Instant>,
}

impl qobject::Capture {
    pub fn refresh(self: Pin<&mut Self>) {
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let (handles, infos) = devices::enumerate();
            let json = serde_json::to_string(&infos).unwrap_or_else(|_| "[]".into());
            for info in &infos {
                let formats: Vec<&str> = info.formats.iter().map(|f| f.id.as_str()).collect();
                eprintln!("lainify: found {} ({})", info.name, formats.join(", "));
            }
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                this.as_mut().rust_mut().devices = handles;
                this.as_mut().set_devices_json(QString::from(&json));
            });
        });
    }

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
        let Some(device) = usize::try_from(device)
            .ok()
            .and_then(|i| self.rust().devices.get(i).cloned())
        else {
            self.as_mut().notice(QString::from("No such device"), true);
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
            "lainify: opening {} as {} {}x{} @ {}/{}",
            selection.device.display_name(),
            selection.format,
            width,
            height,
            fps_num,
            fps_den
        );
        self.as_mut().set_status(QString::from("starting"));
        let started = pipeline::Capture::start(selection, move |event| {
            let _ = qt.queue(move |mut this: Pin<&mut qobject::Capture>| {
                if this.rust().generation != generation {
                    return;
                }
                match event {
                    Event::Live(desc) => {
                        eprintln!("lainify: live, source delivers {desc}");
                        this.as_mut().set_delivering(QString::from(&desc));
                        this.as_mut().set_status(QString::from("live"));
                    }
                    Event::Error(msg) => {
                        eprintln!("lainify: capture error: {msg}");
                        this.as_mut().stop();
                        this.as_mut().set_status(QString::from("error"));
                        this.as_mut().notice(QString::from(&msg), true);
                    }
                }
            });
        });

        match started {
            Ok(capture) => {
                let mut rust = self.as_mut().rust_mut();
                rust.capture = Some(capture);
                rust.last_frames = 0;
                rust.last_tick = Some(Instant::now());
            }
            Err(msg) => {
                eprintln!("lainify: start failed: {msg}");
                self.as_mut().set_status(QString::from("error"));
                self.as_mut().notice(QString::from(&msg), true);
            }
        }
    }

    pub fn stop(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            rust.generation += 1;
            rust.capture = None; // drops the pipeline
        }
        self.as_mut().set_status(QString::from("idle"));
        self.as_mut().set_delivering(QString::default());
        self.as_mut().set_fps(0.0);
    }

    pub fn screenshot(mut self: Pin<&mut Self>) {
        let Some(sample) = self.rust().capture.as_ref().and_then(|c| c.last_sample()) else {
            self.as_mut()
                .notice(QString::from("Nothing to capture yet"), true);
            return;
        };
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let result = save_png(&sample);
            let _ = qt.queue(move |this: Pin<&mut qobject::Capture>| match result {
                Ok(path) => this.notice(QString::from(&format!("Saved {path}")), false),
                Err(msg) => this.notice(QString::from(&format!("Screenshot failed: {msg}")), true),
            });
        });
    }

    pub fn tick(mut self: Pin<&mut Self>) {
        let now = Instant::now();
        let fps = {
            let mut rust = self.as_mut().rust_mut();
            let Some(frames) = rust.capture.as_ref().map(|c| c.frames()) else {
                return;
            };
            let elapsed = rust
                .last_tick
                .map(|t| now.duration_since(t).as_secs_f64())
                .unwrap_or(0.0);
            let fps = if elapsed > 0.0 {
                (frames - rust.last_frames) as f64 / elapsed
            } else {
                0.0
            };
            rust.last_frames = frames;
            rust.last_tick = Some(now);
            fps
        };
        self.as_mut().set_fps(fps);
    }
}

fn save_png(sample: &gst::Sample) -> Result<String, String> {
    let png = pipeline::encode_png(sample)?;
    let dir = glib::user_special_dir(glib::UserDirectory::Pictures)
        .unwrap_or_else(glib::home_dir)
        .join("lainify");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = glib::DateTime::now_local()
        .and_then(|t| t.format("%Y-%m-%d_%H-%M-%S"))
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "frame".into());
    let path = dir.join(format!("lainify-{stamp}.png"));
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}
