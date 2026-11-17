//! Audio: play the chosen input with a volume control, meter it, and feed recordings.
//!
//! `src ! audioconvert ! audioresample ! F32 48 kHz stereo ! tee`
//!   → `queue ! level ! volume ! pulsesink` (what you hear, with peak levels for the meter)
//!   → `queue ! appsink` (into the recording tap)
//!
//! It is its own pipeline so switching inputs never interrupts the picture. Both pipelines use
//! the system clock, which is what lets the recorder line the two up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::recorder::Tap;

/// The format audio is converted to before monitoring and recording.
pub const AUDIO_CAPS: &str = "audio/x-raw,format=F32LE,rate=48000,channels=2,layout=interleaved";

/// Bottom of the meter scale, in dBFS.
const METER_FLOOR_DB: f64 = -60.0;

pub enum AudioEvent {
    /// Playing; describes the input, e.g. `48 kHz · stereo`.
    Live(String),
    /// Peak levels for the meter, left and right, each 0…1 on a dBFS scale.
    Levels(f64, f64),
    Error(String),
}

pub struct AudioCapture {
    pipeline: gst::Pipeline,
    volume: gst::Element,
    stop: Arc<AtomicBool>,
    bus_thread: Option<JoinHandle<()>>,
}

impl AudioCapture {
    pub fn start(
        device: &gst::Device,
        tap: Tap,
        volume: f64,
        muted: bool,
        on_event: impl Fn(AudioEvent) + Send + 'static,
    ) -> Result<Self, String> {
        let make = |factory: &str| {
            gst::ElementFactory::make(factory)
                .build()
                .map_err(|_| format!("GStreamer element '{factory}' is missing"))
        };

        let src = device
            .create_element(None)
            .map_err(|e| format!("Could not open audio input: {e}"))?;
        // Small buffers: this is a monitor, not music playback (values in µs).
        set_if_present(&src, "latency-time", 10_000i64);
        set_if_present(&src, "buffer-time", 40_000i64);

        let caps = gst::ElementFactory::make("capsfilter")
            .property("caps", AUDIO_CAPS.parse::<gst::Caps>().expect("valid caps"))
            .build()
            .map_err(|e| e.to_string())?;
        let tee = make("tee")?;

        let leaky_queue = || {
            gst::ElementFactory::make("queue")
                .property_from_str("leaky", "downstream")
                .property("max-size-time", 200_000_000u64)
                .property("max-size-buffers", 0u32)
                .property("max-size-bytes", 0u32)
                .build()
                .map_err(|e| e.to_string())
        };

        let level = gst::ElementFactory::make("level")
            .property("interval", 50_000_000u64)
            .property("post-messages", true)
            .build()
            .map_err(|e| e.to_string())?;
        let volume_el = gst::ElementFactory::make("volume")
            .property("volume", volume)
            .property("mute", muted)
            .build()
            .map_err(|e| e.to_string())?;
        let out = make("pulsesink").or_else(|_| make("autoaudiosink"))?;
        set_if_present(&out, "sync", false);
        set_if_present(&out, "latency-time", 10_000i64);
        set_if_present(&out, "buffer-time", 40_000i64);

        let appsink = gst_app::AppSink::builder().sync(false).build();
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    tap.push_audio(&sample, sink.base_time());
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        let pipeline = gst::Pipeline::new();
        // Shared time base with the video pipeline; see the module docs.
        pipeline.use_clock(Some(&gst::SystemClock::obtain()));

        let convert = make("audioconvert")?;
        let resample = make("audioresample")?;
        let monitor_queue = leaky_queue()?;
        let record_queue = leaky_queue()?;
        pipeline
            .add_many([
                &src,
                &convert,
                &resample,
                &caps,
                &tee,
                &monitor_queue,
                &level,
                &volume_el,
                &out,
                &record_queue,
                appsink.upcast_ref(),
            ])
            .map_err(|e| e.to_string())?;
        gst::Element::link_many([&src, &convert, &resample, &caps, &tee])
            .map_err(|e| format!("Audio input format not supported: {e}"))?;
        gst::Element::link_many([&tee, &monitor_queue, &level, &volume_el, &out])
            .map_err(|e| e.to_string())?;
        gst::Element::link_many([&tee, &record_queue, appsink.upcast_ref()])
            .map_err(|e| e.to_string())?;

        let stop = Arc::new(AtomicBool::new(false));
        let bus = pipeline.bus().expect("pipeline has a bus");
        let bus_thread = std::thread::Builder::new()
            .name("lainify-audio".into())
            .spawn({
                let stop = Arc::clone(&stop);
                let pipeline = pipeline.downgrade();
                move || watch_bus(bus, pipeline, src, stop, on_event)
            })
            .map_err(|e| e.to_string())?;

        let capture = Self {
            pipeline,
            volume: volume_el,
            stop,
            bus_thread: Some(bus_thread),
        };
        capture
            .pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| "The audio input could not be opened".to_string())?;
        Ok(capture)
    }

    pub fn set_volume(&self, volume: f64, muted: bool) {
        self.volume.set_property("volume", volume);
        self.volume.set_property("mute", muted);
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.bus_thread.take() {
            let _ = t.join();
        }
    }
}

fn set_if_present<V: Into<glib::Value>>(element: &gst::Element, name: &str, value: V) {
    if let Some(spec) = element.find_property(name) {
        let value: glib::Value = value.into();
        // Tolerate int/int64 differences between elements.
        if let Ok(v) = value.transform_with_type(spec.value_type()) {
            element.set_property_from_value(name, &v);
        }
    }
}

fn watch_bus(
    bus: gst::Bus,
    pipeline: glib::WeakRef<gst::Pipeline>,
    src: gst::Element,
    stop: Arc<AtomicBool>,
    on_event: impl Fn(AudioEvent),
) {
    use gst::MessageView;
    while !stop.load(Ordering::Relaxed) {
        let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match msg.view() {
            MessageView::Element(el) => {
                if let Some(s) = el.structure().filter(|s| s.name() == "level") {
                    let peaks = peaks_db(s);
                    let left = peaks.first().copied().unwrap_or(f64::NEG_INFINITY);
                    let right = peaks.get(1).copied().unwrap_or(left);
                    on_event(AudioEvent::Levels(to_meter(left), to_meter(right)));
                }
            }
            MessageView::Error(err) => {
                on_event(AudioEvent::Error(err.error().to_string()));
                return;
            }
            MessageView::StateChanged(change) => {
                let from_pipeline = pipeline
                    .upgrade()
                    .is_some_and(|p| msg.src() == Some(p.upcast_ref::<gst::Object>()));
                if from_pipeline && change.current() == gst::State::Playing {
                    on_event(AudioEvent::Live(describe(&src)));
                }
            }
            _ => {}
        }
    }
}

/// `peak` is a GValueArray of per-channel dB values.
fn peaks_db(s: &gst::StructureRef) -> Vec<f64> {
    #[allow(deprecated)]
    if let Ok(arr) = s.get::<glib::ValueArray>("peak") {
        return arr.iter().filter_map(|v| v.get::<f64>().ok()).collect();
    }
    if let Ok(arr) = s.get::<gst::Array>("peak") {
        return arr.iter().filter_map(|v| v.get::<f64>().ok()).collect();
    }
    Vec::new()
}

fn to_meter(db: f64) -> f64 {
    if !db.is_finite() {
        return 0.0;
    }
    ((db - METER_FLOOR_DB) / -METER_FLOOR_DB).clamp(0.0, 1.0)
}

fn describe(src: &gst::Element) -> String {
    let caps = src.static_pad("src").and_then(|p| p.current_caps());
    let Some(s) = caps.as_ref().and_then(|c| c.structure(0)) else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Ok(rate) = s.get::<i32>("rate") {
        let khz = rate as f64 / 1000.0;
        parts.push(format!(
            "{} kHz",
            format!("{khz:.1}").trim_end_matches(".0")
        ));
    }
    match s.get::<i32>("channels") {
        Ok(1) => parts.push("mono".into()),
        Ok(2) => parts.push("stereo".into()),
        Ok(n) => parts.push(format!("{n} channels")),
        Err(_) => {}
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::to_meter;

    #[test]
    fn meter_scale() {
        assert_eq!(to_meter(0.0), 1.0);
        assert_eq!(to_meter(-60.0), 0.0);
        assert_eq!(to_meter(-90.0), 0.0);
        assert_eq!(to_meter(f64::NEG_INFINITY), 0.0);
        assert!((to_meter(-30.0) - 0.5).abs() < 1e-9);
    }
}
