//! The GStreamer capture pipeline:
//! `device ! capsfilter [! jpegdec | ! videoflip] ! videoconvert ! appsink`.
//!
//! The capsfilter pins the exact mode the user picked. `videoconvert` is a passthrough for every
//! format Qt can draw directly and only converts the rest (24-bit BGR → BGRx, losslessly), so
//! chroma resolution is never reduced on the way to the screen.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use gstreamer_video::prelude::*;

use crate::devices::MJPEG;
use crate::recorder::Tap;

/// Pixel formats Qt can draw, matching `LainifyFormat` in `cpp/frame_sink.h`.
#[derive(Clone, Copy)]
#[repr(i32)]
enum PixelFormat {
    Bgrx = 0,
    Nv12 = 1,
    I420 = 2,
    Yuyv = 3,
    Yuv422p = 4,
    P010 = 5,
}

impl PixelFormat {
    fn from_gst(format: gst_video::VideoFormat) -> Option<Self> {
        use gst_video::VideoFormat as F;
        Some(match format {
            F::Bgrx => Self::Bgrx,
            F::Nv12 => Self::Nv12,
            F::I420 => Self::I420,
            F::Yuy2 => Self::Yuyv,
            F::Y42b => Self::Yuv422p,
            F::P01010le => Self::P010,
            _ => return None,
        })
    }

    /// Rows in `plane` for a frame `height` rows tall.
    fn plane_rows(self, plane: usize, height: i32) -> i32 {
        match (self, plane) {
            (_, 0) => height,
            (Self::Nv12 | Self::I420 | Self::P010, _) => (height + 1) / 2,
            _ => height,
        }
    }
}

const SINK_FORMATS: &[&str] = &["BGRx", "NV12", "I420", "YUY2", "Y42B", "P010_10LE"];

unsafe extern "C" {
    fn lainify_present(
        format: i32,
        width: i32,
        height: i32,
        range: i32,
        matrix: i32,
        planes: *const *const u8,
        strides: *const i32,
        rows: *const i32,
        n_planes: i32,
    );
    fn lainify_clear();
}

/// What the user picked.
pub struct Selection {
    pub device: gst::Device,
    pub format: String,
    pub width: i32,
    pub height: i32,
    pub fps: gst::Fraction,
}

/// Something the UI should hear about, sent from GStreamer threads.
pub enum Event {
    /// The device accepted the mode and is streaming.
    Live(SourceMode),
    Error(String),
}

/// The mode the device is actually delivering.
pub struct SourceMode {
    pub width: i32,
    pub height: i32,
    /// Format id as in `devices` (`BGR`, `YUY2`, … or `MJPG`).
    pub format: String,
    pub fps: f64,
}

impl SourceMode {
    /// e.g. `1920×1080 RGB 4:4:4 @ 60 fps`.
    pub fn describe(&self) -> String {
        format!(
            "{}×{} {} @ {} fps",
            self.width,
            self.height,
            crate::devices::short_label(&self.format),
            format_fps(self.fps)
        )
    }
}

/// 60 → `60`, 59.94006 → `59.94`.
pub fn format_fps(fps: f64) -> String {
    let s = format!("{fps:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A running capture. Stops when dropped.
pub struct Capture {
    pipeline: gst::Pipeline,
    last: Arc<Mutex<Option<gst::Sample>>>,
    frames: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    bus_thread: Option<JoinHandle<()>>,
}

impl Capture {
    pub fn start(
        sel: Selection,
        tap: Tap,
        on_event: impl Fn(Event) + Send + 'static,
    ) -> Result<Self, String> {
        let src = sel
            .device
            .create_element(None)
            .map_err(|e| format!("Could not open device: {e}"))?;

        let caps = if sel.format == MJPEG {
            gst::Caps::builder("image/jpeg")
        } else {
            gst::Caps::builder("video/x-raw").field("format", sel.format.as_str())
        }
        .field("width", sel.width)
        .field("height", sel.height)
        .field("framerate", sel.fps)
        .build();

        let filter = gst::ElementFactory::make("capsfilter")
            .property("caps", &caps)
            .build()
            .map_err(|e| e.to_string())?;
        let convert = gst::ElementFactory::make("videoconvert")
            .property("n-threads", 4u32)
            .build()
            .map_err(|e| e.to_string())?;
        let sink = gst_app::AppSink::builder()
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", gst::List::new(SINK_FORMATS.iter().copied()))
                    .build(),
            )
            .sync(false)
            .max_buffers(1)
            .drop(true)
            .build();

        let pipeline = gst::Pipeline::new();
        // Shared time base with the audio pipeline, so recordings stay in sync.
        pipeline.use_clock(Some(&gst::SystemClock::obtain()));
        let mut chain = vec![src, filter.clone()];
        if sel.format == MJPEG {
            chain.push(
                gst::ElementFactory::make("jpegdec")
                    .build()
                    .map_err(|e| format!("jpegdec missing: {e}"))?,
            );
        }
        if sel.format == "BGR" {
            // UVC sends RGB bottom-up (the Windows DIB convention); v4l2src passes it as is.
            chain.push(
                gst::ElementFactory::make("videoflip")
                    .property_from_str("video-direction", "vert")
                    .build()
                    .map_err(|e| format!("videoflip missing: {e}"))?,
            );
        }
        chain.push(convert);
        chain.push(sink.clone().upcast());
        pipeline.add_many(&chain).map_err(|e| e.to_string())?;
        gst::Element::link_many(&chain).map_err(|e| e.to_string())?;

        let last = Arc::new(Mutex::new(None));
        let frames = Arc::new(AtomicU64::new(0));
        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample({
                    let last = Arc::clone(&last);
                    let frames = Arc::clone(&frames);
                    move |sink| {
                        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                        present(&sample);
                        tap.push_video(&sample, sink.base_time());
                        frames.fetch_add(1, Ordering::Relaxed);
                        *last.lock().unwrap() = Some(sample);
                        Ok(gst::FlowSuccess::Ok)
                    }
                })
                .build(),
        );

        let stop = Arc::new(AtomicBool::new(false));
        let bus = pipeline.bus().expect("pipeline has a bus");
        let bus_thread = std::thread::Builder::new()
            .name("lainify-bus".into())
            .spawn({
                let stop = Arc::clone(&stop);
                let pipeline = pipeline.downgrade();
                move || watch_bus(bus, pipeline, filter, stop, on_event)
            })
            .map_err(|e| e.to_string())?;

        let capture = Self {
            pipeline,
            last,
            frames,
            stop,
            bus_thread: Some(bus_thread),
        };
        capture
            .pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| "The device refused this mode".to_string())?;
        Ok(capture)
    }

    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    pub fn last_sample(&self) -> Option<gst::Sample> {
        self.last.lock().unwrap().clone()
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.bus_thread.take() {
            let _ = t.join();
        }
        unsafe { lainify_clear() };
    }
}

fn watch_bus(
    bus: gst::Bus,
    pipeline: gst::glib::WeakRef<gst::Pipeline>,
    filter: gst::Element,
    stop: Arc<AtomicBool>,
    on_event: impl Fn(Event),
) {
    use gst::MessageView;
    while !stop.load(Ordering::Relaxed) {
        let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match msg.view() {
            MessageView::Error(err) => {
                let detail = err.debug().map(|d| format!(" ({d})")).unwrap_or_default();
                on_event(Event::Error(format!("{}{detail}", err.error())));
                return;
            }
            MessageView::Eos(_) => {
                on_event(Event::Error("The device stopped sending video".into()));
                return;
            }
            MessageView::StateChanged(change) => {
                let from_pipeline = pipeline
                    .upgrade()
                    .is_some_and(|p| msg.src() == Some(p.upcast_ref::<gst::Object>()));
                if from_pipeline && change.current() == gst::State::Playing {
                    on_event(Event::Live(describe_source(&filter)));
                }
            }
            _ => {}
        }
    }
}

fn describe_source(filter: &gst::Element) -> SourceMode {
    let caps = filter.static_pad("src").and_then(|p| p.current_caps());
    let s = caps.as_ref().and_then(|c| c.structure(0));
    SourceMode {
        width: s.and_then(|s| s.get::<i32>("width").ok()).unwrap_or(0),
        height: s.and_then(|s| s.get::<i32>("height").ok()).unwrap_or(0),
        format: match s {
            Some(s) if s.name() == "image/jpeg" => MJPEG.to_string(),
            Some(s) => s.get::<&str>("format").unwrap_or("").to_string(),
            None => String::new(),
        },
        fps: s
            .and_then(|s| s.get::<gst::Fraction>("framerate").ok())
            .map(|f| f.numer() as f64 / f.denom().max(1) as f64)
            .unwrap_or(0.0),
    }
}

fn present(sample: &gst::Sample) {
    let (Some(buffer), Some(caps)) = (sample.buffer(), sample.caps()) else {
        return;
    };
    let Ok(info) = gst_video::VideoInfo::from_caps(caps) else {
        return;
    };
    let Some(format) = PixelFormat::from_gst(info.format()) else {
        return;
    };
    let Ok(frame) = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info) else {
        return;
    };

    let n = frame.n_planes() as usize;
    let mut planes = [std::ptr::null::<u8>(); 4];
    let mut strides = [0i32; 4];
    let mut rows = [0i32; 4];
    for i in 0..n.min(4) {
        let Ok(data) = frame.plane_data(i as u32) else {
            return;
        };
        planes[i] = data.as_ptr();
        strides[i] = frame.plane_stride()[i];
        rows[i] = format.plane_rows(i, info.height() as i32);
    }

    let colorimetry = info.colorimetry();
    let (range, matrix) = if matches!(format, PixelFormat::Bgrx) {
        (0, 0)
    } else {
        let range = match colorimetry.range() {
            gst_video::VideoColorRange::Range16_235 => 1,
            gst_video::VideoColorRange::Range0_255 => 2,
            _ => 0,
        };
        let matrix = match colorimetry.matrix() {
            gst_video::VideoColorMatrix::Bt601 => 1,
            gst_video::VideoColorMatrix::Bt709 => 2,
            gst_video::VideoColorMatrix::Bt2020 => 3,
            _ => 0,
        };
        (range, matrix)
    };

    // SAFETY: every pointer stays valid while `frame` is mapped, which outlives this call; the
    // C++ side copies the data before returning.
    unsafe {
        lainify_present(
            format as i32,
            info.width() as i32,
            info.height() as i32,
            range,
            matrix,
            planes.as_ptr(),
            strides.as_ptr(),
            rows.as_ptr(),
            n as i32,
        );
    }
}

/// Encode `sample` as a lossless PNG. Blocking; call off the UI thread.
pub fn encode_png(sample: &gst::Sample) -> Result<Vec<u8>, String> {
    let png = gst_video::convert_sample(
        sample,
        &gst::Caps::builder("image/png").build(),
        Some(gst::ClockTime::from_seconds(5)),
    )
    .map_err(|e| e.to_string())?;
    let buffer = png.buffer().ok_or("No PNG data")?;
    let map = buffer.map_readable().map_err(|e| e.to_string())?;
    Ok(map.as_slice().to_vec())
}

#[cfg(test)]
mod tests {
    use super::format_fps;

    #[test]
    fn formats_fps() {
        assert_eq!(format_fps(60.0), "60");
        assert_eq!(format_fps(60000.0 / 1001.0), "59.94");
        assert_eq!(format_fps(29.5), "29.5");
    }
}
