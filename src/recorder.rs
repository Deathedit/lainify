//! Recording: a separate pipeline fed with the frames and audio the capture pipelines already
//! produce, so starting or stopping a recording never touches the live picture.
//!
//! `appsrc ! queue ! videoconvert ! encoder ! parser ! mux ! filesink`, plus an audio branch.
//! Both capture pipelines run on the system clock; each buffer's absolute time
//! (`pts + base_time`) minus the consumer's start time becomes its timestamp, which keeps
//! picture and sound in sync even though they come from different pipelines.
//!
//! The encoder chains and the [`Tap`] are shared with the replay buffer (`replay.rs`).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::audio::AUDIO_CAPS;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoder {
    NvH264,
    NvH265,
    NvAv1,
    X264,
}

impl Encoder {
    pub const ALL: [Encoder; 4] = [Self::NvH264, Self::NvH265, Self::NvAv1, Self::X264];

    pub fn id(self) -> &'static str {
        match self {
            Self::NvH264 => "nvh264",
            Self::NvH265 => "nvh265",
            Self::NvAv1 => "nvav1",
            Self::X264 => "x264",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::NvH264 => "H.264 — NVIDIA (NVENC)",
            Self::NvH265 => "HEVC — NVIDIA (NVENC)",
            Self::NvAv1 => "AV1 — NVIDIA (NVENC)",
            Self::X264 => "H.264 — CPU (x264)",
        }
    }

    fn factory(self) -> &'static str {
        match self {
            Self::NvH264 => "nvh264enc",
            Self::NvH265 => "nvh265enc",
            Self::NvAv1 => "nvav1enc",
            Self::X264 => "x264enc",
        }
    }

    fn parser(self) -> &'static str {
        match self {
            Self::NvH264 | Self::X264 => "h264parse",
            Self::NvH265 => "h265parse",
            Self::NvAv1 => "av1parse",
        }
    }

    /// What the parser hands the muxer. Both matroskamux and mp4mux take exactly these, so
    /// encoded buffers can be stored and muxed later (the replay buffer relies on it).
    fn output_caps(self) -> gst::Caps {
        match self {
            Self::NvH264 | Self::X264 => gst::Caps::builder("video/x-h264")
                .field("stream-format", "avc")
                .field("alignment", "au")
                .build(),
            Self::NvH265 => gst::Caps::builder("video/x-h265")
                .field("stream-format", "hvc1")
                .field("alignment", "au")
                .build(),
            Self::NvAv1 => gst::Caps::builder("video/x-av1")
                .field("stream-format", "obu-stream")
                .field("alignment", "tu")
                .build(),
        }
    }

    pub fn is_hardware(self) -> bool {
        self != Self::X264
    }

    /// Caps after the encoder that pin its profile. Left open, nvh264enc settles on
    /// constrained-baseline (no CABAC, no B-frames), which wastes bitrate.
    fn profile(self, chroma_444: bool) -> Option<gst::Caps> {
        let (media, profile) = match (self, chroma_444) {
            (Self::NvH264 | Self::X264, false) => ("video/x-h264", "high"),
            (Self::NvH264 | Self::X264, true) => ("video/x-h264", "high-4:4:4"),
            (Self::NvH265, false) => ("video/x-h265", "main"),
            (Self::NvH265, true) => ("video/x-h265", "main-444"),
            (Self::NvAv1, _) => return None,
        };
        Some(gst::Caps::builder(media).field("profile", profile).build())
    }

    pub fn supports_444(self) -> bool {
        self.profile(true).is_some()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.id() == id)
    }

    /// Encoders whose GStreamer element exists. The NVENC elements only register when an
    /// NVIDIA GPU and driver are present, so this doubles as a hardware check.
    pub fn available() -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|e| gst::ElementFactory::find(e.factory()).is_some())
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Mkv,
    Mp4,
}

impl Container {
    pub fn from_id(id: &str) -> Self {
        if id == "mp4" { Self::Mp4 } else { Self::Mkv }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Mkv => "mkv",
            Self::Mp4 => "mp4",
        }
    }
}

pub struct Options {
    pub encoder: Encoder,
    pub bitrate_kbps: u32,
    pub chroma_444: bool,
    pub container: Container,
    pub fps: f64,
    /// Distance between keyframes. Clips can only start on one.
    pub keyframe_seconds: f64,
}

/// Where captured buffers go: every running consumer (a recording, the replay buffer).
/// Cloned into both capture pipelines.
#[derive(Clone, Default)]
pub struct Tap(Arc<Mutex<Vec<Arc<Feed>>>>);

/// One consumer's inputs.
pub struct Feed {
    video: gst_app::AppSrc,
    audio: Option<gst_app::AppSrc>,
    video_caps: gst::Caps,
    t0: gst::ClockTime,
}

impl Feed {
    /// Timestamps start from now.
    pub fn new(
        video: gst_app::AppSrc,
        audio: Option<gst_app::AppSrc>,
        video_caps: gst::Caps,
    ) -> Arc<Self> {
        Arc::new(Self {
            video,
            audio,
            video_caps,
            t0: gst::SystemClock::obtain().time(),
        })
    }

    /// Tell the encoders no more input is coming.
    pub fn end(&self) {
        let _ = self.video.end_of_stream();
        if let Some(audio) = &self.audio {
            let _ = audio.end_of_stream();
        }
    }
}

impl Tap {
    pub fn add(&self, feed: Arc<Feed>) {
        self.0.lock().unwrap().push(feed);
    }

    pub fn remove(&self, feed: &Arc<Feed>) {
        self.0.lock().unwrap().retain(|f| !Arc::ptr_eq(f, feed));
    }

    pub fn push_video(&self, sample: &gst::Sample, base_time: Option<gst::ClockTime>) {
        for feed in self.0.lock().unwrap().iter() {
            if sample.caps().is_some_and(|c| *c != feed.video_caps) {
                continue; // mode changed; the consumer is restarted or the UI prevents it
            }
            push(&feed.video, sample, base_time, feed.t0);
        }
    }

    pub fn push_audio(&self, sample: &gst::Sample, base_time: Option<gst::ClockTime>) {
        for feed in self.0.lock().unwrap().iter() {
            if let Some(audio) = &feed.audio {
                push(audio, sample, base_time, feed.t0);
            }
        }
    }
}

fn push(
    src: &gst_app::AppSrc,
    sample: &gst::Sample,
    base_time: Option<gst::ClockTime>,
    t0: gst::ClockTime,
) {
    let (Some(buffer), Some(base)) = (sample.buffer(), base_time) else {
        return;
    };
    let Some(pts) = buffer.pts() else { return };
    let absolute = pts + base;
    if absolute < t0 {
        return;
    }
    // Shallow copy: shares the frame memory, only the timestamps differ.
    let mut out = buffer.copy();
    {
        let out = out.get_mut().expect("fresh copy is writable");
        out.set_pts(absolute - t0);
        out.set_dts(gst::ClockTime::NONE);
    }
    let _ = src.push_buffer(out);
}

pub enum RecordEvent {
    Saved(PathBuf),
    Error(String),
}

/// A running recording. Call [`Recording::finish`] to close the file; the result arrives as a
/// [`RecordEvent`].
pub struct Recording {
    feed: Arc<Feed>,
    tap: Tap,
    pipeline: gst::Pipeline,
    pub path: PathBuf,
    stop: Arc<AtomicBool>,
}

impl Recording {
    pub fn start(
        opts: Options,
        video_caps: gst::Caps,
        with_audio: bool,
        tap: Tap,
        on_event: impl Fn(RecordEvent) + Send + 'static,
    ) -> Result<Self, String> {
        let dir = crate::paths::recordings_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!(
            "lainify-{}.{}",
            crate::paths::timestamp(),
            opts.container.extension()
        ));

        let pipeline = gst::Pipeline::new();
        let (mux, _sink) = add_mux(&pipeline, opts.container, &path)?;
        let (video_src, video_out) = add_video_encoder(&pipeline, &opts, &video_caps)?;
        video_out
            .link(&mux)
            .map_err(|e| format!("video → mux: {e}"))?;
        let audio_src = if with_audio {
            let (src, out) = add_audio_encoder(&pipeline, opts.container)?;
            out.link(&mux).map_err(|e| format!("audio → mux: {e}"))?;
            Some(src)
        } else {
            None
        };

        let stop = Arc::new(AtomicBool::new(false));
        let bus = pipeline.bus().expect("pipeline has a bus");
        std::thread::Builder::new()
            .name("lainify-record".into())
            .spawn({
                let pipeline = pipeline.clone();
                let path = path.clone();
                let stop = Arc::clone(&stop);
                move || match wait_for_eos(&bus, &stop) {
                    Some(Ok(())) => {
                        let _ = pipeline.set_state(gst::State::Null);
                        on_event(RecordEvent::Saved(path));
                    }
                    Some(Err(msg)) => {
                        let _ = pipeline.set_state(gst::State::Null);
                        on_event(RecordEvent::Error(msg));
                    }
                    None => {}
                }
            })
            .map_err(|e| e.to_string())?;

        pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| "The recorder could not start".to_string())?;

        let feed = Feed::new(video_src, audio_src, video_caps);
        tap.add(Arc::clone(&feed));
        Ok(Self {
            feed,
            tap,
            pipeline,
            path,
            stop,
        })
    }

    /// Stop feeding and close the file. Returns at once; `Saved` follows when it's written.
    pub fn finish(&self) {
        self.tap.remove(&self.feed);
        self.feed.end();
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.tap.remove(&self.feed);
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// Block until the pipeline finishes (`Ok`), fails (`Err`), or `stop` is set (`None`).
pub(crate) fn wait_for_eos(bus: &gst::Bus, stop: &AtomicBool) -> Option<Result<(), String>> {
    use gst::MessageView;
    while !stop.load(Ordering::Relaxed) {
        let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match msg.view() {
            MessageView::Eos(_) => return Some(Ok(())),
            MessageView::Error(err) => return Some(Err(err.error().to_string())),
            _ => {}
        }
    }
    None
}

fn make(factory: &str) -> Result<gst::Element, String> {
    gst::ElementFactory::make(factory)
        .build()
        .map_err(|_| format!("GStreamer element '{factory}' is missing"))
}

/// `mux ! filesink` writing to `path`.
pub(crate) fn add_mux(
    pipeline: &gst::Pipeline,
    container: Container,
    path: &std::path::Path,
) -> Result<(gst::Element, gst::Element), String> {
    let mux = make(match container {
        Container::Mkv => "matroskamux",
        Container::Mp4 => "mp4mux",
    })?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", path.to_string_lossy().as_ref())
        .property("sync", false)
        .build()
        .map_err(|e| e.to_string())?;
    pipeline
        .add_many([&mux, &sink])
        .map_err(|e| e.to_string())?;
    mux.link(&sink).map_err(|e| e.to_string())?;
    Ok((mux, sink))
}

/// Raw frames in, an encoded, mux-ready stream out:
/// `appsrc ! queue ! videoconvert ! capsfilter ! encoder ! capsfilter ! parser ! capsfilter ! queue`.
/// Returns the appsrc and the last element, for the caller to link onward.
pub(crate) fn add_video_encoder(
    pipeline: &gst::Pipeline,
    opts: &Options,
    raw_caps: &gst::Caps,
) -> Result<(gst_app::AppSrc, gst::Element), String> {
    let src = gst_app::AppSrc::builder()
        .caps(raw_caps)
        .format(gst::Format::Time)
        .build();
    // If the encoder falls behind, drop frames rather than pile up raw ones in memory
    // (a second of 1080p RGB is ~370 MB).
    src.set_property("max-buffers", 30u64);
    if src.find_property("leaky-type").is_some() {
        src.set_property_from_str("leaky-type", "downstream");
    }
    let queue = gst::ElementFactory::make("queue")
        .property("max-size-buffers", 0u32)
        .property("max-size-bytes", 0u32)
        .property("max-size-time", 2_000_000_000u64)
        .build()
        .map_err(|e| e.to_string())?;
    let convert = gst::ElementFactory::make("videoconvert")
        .property("n-threads", 4u32)
        .build()
        .map_err(|e| e.to_string())?;
    let chroma_444 = opts.chroma_444 && opts.encoder.supports_444();
    let format = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("format", if chroma_444 { "Y444" } else { "NV12" })
                .build(),
        )
        .build()
        .map_err(|e| e.to_string())?;
    let encoder = make(opts.encoder.factory())?;
    configure_encoder(&encoder, opts);
    let profile = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            opts.encoder
                .profile(chroma_444)
                .unwrap_or_else(gst::Caps::new_any),
        )
        .build()
        .map_err(|e| e.to_string())?;
    let parser = make(opts.encoder.parser())?;
    let output = gst::ElementFactory::make("capsfilter")
        .property("caps", opts.encoder.output_caps())
        .build()
        .map_err(|e| e.to_string())?;
    let out = make("queue")?;

    let chain: Vec<gst::Element> = vec![
        src.clone().upcast(),
        queue,
        convert,
        format,
        encoder,
        profile,
        parser,
        output,
        out.clone(),
    ];
    pipeline.add_many(&chain).map_err(|e| e.to_string())?;
    gst::Element::link_many(&chain).map_err(|e| format!("video: {e}"))?;
    Ok((src, out))
}

/// Captured audio in, encoded audio out: Opus for MKV, AAC for MP4.
pub(crate) fn add_audio_encoder(
    pipeline: &gst::Pipeline,
    container: Container,
) -> Result<(gst_app::AppSrc, gst::Element), String> {
    let src = gst_app::AppSrc::builder()
        .caps(&AUDIO_CAPS.parse::<gst::Caps>().expect("valid caps"))
        .format(gst::Format::Time)
        .build();
    let encoder = match container {
        // AAC is what MP4 players expect; Opus is better and standard in Matroska.
        Container::Mp4 if gst::ElementFactory::find("avenc_aac").is_some() => {
            gst::ElementFactory::make("avenc_aac")
                .property("bitrate", 192_000i32)
                .build()
                .map_err(|e| e.to_string())?
        }
        _ => gst::ElementFactory::make("opusenc")
            .property("bitrate", 192_000i32)
            .build()
            .map_err(|e| e.to_string())?,
    };
    let out = make("queue")?;
    let chain: Vec<gst::Element> = vec![
        src.clone().upcast(),
        make("queue")?,
        make("audioconvert")?,
        make("audioresample")?,
        encoder,
        out.clone(),
    ];
    pipeline.add_many(&chain).map_err(|e| e.to_string())?;
    gst::Element::link_many(&chain).map_err(|e| format!("audio: {e}"))?;
    Ok((src, out))
}

fn configure_encoder(encoder: &gst::Element, opts: &Options) {
    let gop = (opts.fps.max(1.0) * opts.keyframe_seconds.max(0.25)).round() as i32;
    let set_str = |name: &str, value: &str| {
        if encoder.find_property(name).is_some() {
            encoder.set_property_from_str(name, value);
        }
    };
    match opts.encoder {
        Encoder::X264 => {
            encoder.set_property("bitrate", opts.bitrate_kbps);
            encoder.set_property("key-int-max", gop as u32);
            encoder.set_property("bframes", 0u32);
            set_str("speed-preset", "veryfast");
        }
        _ => {
            encoder.set_property("bitrate", opts.bitrate_kbps);
            encoder.set_property("gop-size", gop);
            set_str("rc-mode", "vbr");
            set_str("preset", "p4");
            set_str("tune", "high-quality");
            // No B-frames: every buffer's decode order is its display order, so stored packets
            // can be cut at any keyframe and re-timed without DTS bookkeeping.
            if encoder.find_property("bframes").is_some() {
                encoder.set_property("bframes", 0u32);
            }
        }
    }
}
