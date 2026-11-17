//! Audio: play the chosen input with a volume control, meter it, and feed recordings.
//!
//! `src ! audioconvert ! audioresample ! F32 48 kHz stereo ! tee`
//!   → `queue ! level ! volume ! pulsesink` (what you hear, with peak levels for the meter)
//!   → `queue ! appsink` (into the recording tap)
//!
//! It is its own pipeline so switching inputs never interrupts the picture. Both pipelines use
//! the system clock, which is what lets the recorder line the two up.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

use gstreamer as gst;
use gstreamer::glib;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::recorder::Tap;

/// The format audio is converted to before monitoring and recording.
pub const AUDIO_CAPS: &str = "audio/x-raw,format=F32LE,rate=48000,channels=2,layout=interleaved";

/// Bottom of the meter scale, in dBFS.
const METER_FLOOR_DB: f64 = -60.0;

/// Name of the bus message the rate meter posts.
const RATE_MESSAGE: &str = "lainify-audio-rate";

pub enum AudioEvent {
    /// Playing; describes the input, e.g. `48 kHz · stereo`.
    Live(String),
    /// Peak levels for the meter, left and right, each 0…1 on a dBFS scale.
    Levels(f64, f64),
    /// The input is (or stopped) delivering fewer samples than real time; `ppm` is the latest
    /// measured shortfall (negative), e.g. -856 for 0.0856% missing.
    Rate {
        ppm: f64,
        losing: bool,
    },
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
        // Small chunks for low latency, but a deep buffer: while lainify is busy with 4K video
        // and encoding, the input can be read 30-90 ms late, and a short buffer would drop
        // audio (values in µs).
        set_if_present(&src, "latency-time", 10_000i64);
        set_if_present(&src, "buffer-time", 200_000i64);

        let caps = gst::ElementFactory::make("capsfilter")
            .property("caps", AUDIO_CAPS.parse::<gst::Caps>().expect("valid caps"))
            .build()
            .map_err(|e| e.to_string())?;
        let tee = make("tee")?;

        // Count what actually arrives (after conversion: F32 stereo, 8 bytes a frame). Some
        // capture cards silently drop audio in heavy video modes, which is heard as crackling.
        let meter = Mutex::new(RateMeter::new(48_000.0));
        let started = Instant::now();
        caps.static_pad("src")
            .expect("capsfilter has a src pad")
            .add_probe(gst::PadProbeType::BUFFER, move |pad, info| {
                if let Some(buffer) = info.buffer() {
                    let frames = buffer.size() as u64 / 8;
                    let now = started.elapsed().as_secs_f64();
                    if let Some(report) = meter.lock().unwrap().observe(now, frames)
                        && let Some(element) = pad.parent_element()
                    {
                        let s = gst::Structure::builder(RATE_MESSAGE)
                            .field("ppm", report.ppm)
                            .field("losing", report.losing)
                            .build();
                        let _ = element.post_message(gst::message::Element::new(s));
                    }
                }
                gst::PadProbeReturn::Ok
            });

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
        // 100 ms of headroom rides out those late reads; with 40 ms the output ran dry every
        // few seconds under load, which is heard as crackling.
        set_if_present(&out, "latency-time", 20_000i64);
        set_if_present(&out, "buffer-time", 100_000i64);

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
                if let Some(s) = el.structure().filter(|s| s.name() == RATE_MESSAGE) {
                    on_event(AudioEvent::Rate {
                        ppm: s.get::<f64>("ppm").unwrap_or(0.0),
                        losing: s.get::<bool>("losing").unwrap_or(false),
                    });
                } else if let Some(s) = el.structure().filter(|s| s.name() == "level") {
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

/// Detects an input that delivers fewer samples than real time.
///
/// Arrival times jitter by tens of milliseconds (more while lainify is busy), so a plain count
/// would need many minutes to be trustworthy. Instead each window fits a straight line through
/// (arrival time, frames so far); its slope is the delivered rate, and the jitter averages out.
/// Two bad windows in a row raise the flag, two good ones clear it.
struct RateMeter {
    nominal: f64,
    start: Option<f64>,
    window: Option<Window>,
    /// Positive: consecutive bad windows; negative: consecutive good ones.
    streak: i32,
    losing: bool,
}

#[derive(Default)]
struct Window {
    t0: f64,
    frames: f64,
    n: f64,
    st: f64,
    sf: f64,
    stt: f64,
    stf: f64,
}

#[derive(Debug, Clone, Copy)]
struct RateReport {
    ppm: f64,
    losing: bool,
}

impl RateMeter {
    /// Ignore the start-up, when buffers fill and arrive in bursts.
    const WARMUP_S: f64 = 5.0;
    const WINDOW_S: f64 = 20.0;
    /// Clean inputs measure within ±40 ppm; one dropping audio, hundreds.
    const LOSS_PPM: f64 = -150.0;

    fn new(nominal: f64) -> Self {
        Self {
            nominal,
            start: None,
            window: None,
            streak: 0,
            losing: false,
        }
    }

    /// Record `frames` arriving at `now` (seconds). Returns a report at the end of each window.
    fn observe(&mut self, now: f64, frames: u64) -> Option<RateReport> {
        let start = *self.start.get_or_insert(now);
        if now - start < Self::WARMUP_S {
            return None;
        }
        let w = self.window.get_or_insert_with(|| Window {
            t0: now,
            ..Default::default()
        });
        w.frames += frames as f64;
        let t = now - w.t0;
        w.n += 1.0;
        w.st += t;
        w.sf += w.frames;
        w.stt += t * t;
        w.stf += t * w.frames;
        if t < Self::WINDOW_S {
            return None;
        }

        let denominator = w.n * w.stt - w.st * w.st;
        let w = self.window.take().expect("window exists");
        if denominator <= 0.0 {
            return None;
        }
        let rate = (w.n * w.stf - w.st * w.sf) / denominator;
        let ppm = (rate / self.nominal - 1.0) * 1e6;
        if ppm < Self::LOSS_PPM {
            self.streak = self.streak.max(0) + 1;
        } else {
            self.streak = self.streak.min(0) - 1;
        }
        if self.streak >= 2 {
            self.losing = true;
        } else if self.streak <= -2 {
            self.losing = false;
        }
        Some(RateReport {
            ppm,
            losing: self.losing,
        })
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
    use super::{RateMeter, to_meter};

    /// Feed `seconds` of an input delivering `ppm` off nominal in 10 ms chunks, arriving up to
    /// `jitter` seconds late (deterministic pseudo-random), with an occasional long stall.
    fn simulate(ppm: f64, jitter: f64, seconds: f64) -> Vec<(f64, bool)> {
        let mut meter = RateMeter::new(48_000.0);
        let rate = 48_000.0 * (1.0 + ppm / 1e6);
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut random = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % 10_000) as f64 / 10_000.0
        };
        let (mut produced, mut sent, mut reports) = (0.0f64, 0u64, Vec::new());
        let mut t = 0.0;
        while t < seconds {
            t += 0.01;
            produced += rate * 0.01;
            let stall = if random() < 0.005 { 0.09 } else { 0.0 };
            let arrival = t + random() * jitter + stall;
            let frames = produced as u64 - sent;
            sent += frames;
            if let Some(r) = meter.observe(arrival, frames) {
                reports.push((r.ppm, r.losing));
            }
        }
        reports
    }

    #[test]
    fn rate_meter_flags_a_lossy_input() {
        let reports = simulate(-856.0, 0.04, 100.0);
        assert!(reports.len() >= 3);
        // Each window is within ~100 ppm under heavy jitter, well clear of the threshold.
        assert!(
            reports.iter().all(|(ppm, _)| (ppm + 856.0).abs() < 150.0),
            "{reports:?}"
        );
        assert!(!reports[0].1, "one window is not enough");
        assert!(reports.last().unwrap().1, "flagged after two windows");
    }

    #[test]
    fn rate_meter_trusts_a_clean_input() {
        for ppm in [-40.0, 0.0, 30.0] {
            let reports = simulate(ppm, 0.04, 200.0);
            assert!(
                reports.iter().all(|(_, losing)| !losing),
                "{ppm}: {reports:?}"
            );
        }
    }

    #[test]
    fn meter_scale() {
        assert_eq!(to_meter(0.0), 1.0);
        assert_eq!(to_meter(-60.0), 0.0);
        assert_eq!(to_meter(-90.0), 0.0);
        assert_eq!(to_meter(f64::NEG_INFINITY), 0.0);
        assert!((to_meter(-30.0) - 0.5).abs() < 1e-9);
    }
}
