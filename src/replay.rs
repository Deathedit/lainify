//! Replay buffer: keeps the last N seconds of gameplay encoded in memory, ready to be saved as a
//! clip at the press of a key.
//!
//! An always-on encoder (the recorder's chains, fed from the same [`Tap`]) ends in appsinks
//! instead of a muxer. Encoded buffers are kept grouped by keyframe, and whole groups fall off the
//! front once the rest still covers the length asked for, so the buffer always starts on a
//! keyframe. Saving copies that ring into a fresh `appsrc ! mux ! filesink` pipeline without
//! re-encoding: near-instant, and the live picture never notices.
//!
//! Raw frames would never fit (a minute of 1080p RGB is ~22 GB); encoded, a minute at 20 Mbps
//! is ~150 MB.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;

use crate::recorder::{
    Container, Feed, Options, Tap, add_audio_encoder, add_mux, add_video_encoder, wait_for_eos,
};

/// Encoded buffers covering at least `length`, starting on a keyframe.
struct Ring {
    length: gst::ClockTime,
    gops: VecDeque<Gop>,
    audio: VecDeque<gst::Buffer>,
    video_caps: Option<gst::Caps>,
    audio_caps: Option<gst::Caps>,
    newest: Option<gst::ClockTime>,
}

/// A keyframe and the frames that depend on it.
struct Gop {
    start: gst::ClockTime,
    buffers: Vec<gst::Buffer>,
}

/// What a save writes out: owned references to the buffers, so the ring keeps filling meanwhile.
struct Clip {
    video_caps: gst::Caps,
    audio_caps: Option<gst::Caps>,
    video: Vec<gst::Buffer>,
    audio: Vec<gst::Buffer>,
    start: gst::ClockTime,
    duration: gst::ClockTime,
}

impl Ring {
    fn new(length: gst::ClockTime) -> Self {
        Self {
            length,
            gops: VecDeque::new(),
            audio: VecDeque::new(),
            video_caps: None,
            audio_caps: None,
            newest: None,
        }
    }

    fn push_video(&mut self, buffer: gst::Buffer) {
        let Some(pts) = buffer.pts() else { return };
        if buffer.flags().contains(gst::BufferFlags::DELTA_UNIT) {
            match self.gops.back_mut() {
                Some(gop) => gop.buffers.push(buffer),
                None => return, // nothing decodable until the first keyframe
            }
        } else {
            self.gops.push_back(Gop {
                start: pts,
                buffers: vec![buffer],
            });
        }
        self.newest = Some(self.newest.map_or(pts, |n| n.max(pts)));

        // Drop the oldest group once the next one alone still reaches back far enough.
        while self.gops.len() >= 2 && pts.saturating_sub(self.gops[1].start) >= self.length {
            self.gops.pop_front();
        }
        if let Some(start) = self.gops.front().map(|g| g.start) {
            while self
                .audio
                .front()
                .and_then(|b| b.pts())
                .is_some_and(|p| p < start)
            {
                self.audio.pop_front();
            }
        }
    }

    fn push_audio(&mut self, buffer: gst::Buffer) {
        let Some(pts) = buffer.pts() else { return };
        self.audio.push_back(buffer);
        // Without video (yet), still keep audio bounded.
        if self.gops.is_empty() {
            while self
                .audio
                .front()
                .and_then(|b| b.pts())
                .is_some_and(|p| pts.saturating_sub(p) > self.length)
            {
                self.audio.pop_front();
            }
        }
    }

    /// How much is buffered.
    fn buffered(&self) -> gst::ClockTime {
        match (self.gops.front(), self.newest) {
            (Some(gop), Some(newest)) => newest.saturating_sub(gop.start),
            _ => gst::ClockTime::ZERO,
        }
    }

    fn snapshot(&self) -> Option<Clip> {
        let start = self.gops.front()?.start;
        let video: Vec<gst::Buffer> = self
            .gops
            .iter()
            .flat_map(|g| g.buffers.iter().cloned())
            .collect();
        let audio: Vec<gst::Buffer> = if self.audio_caps.is_some() {
            self.audio
                .iter()
                .filter(|b| b.pts().is_some_and(|p| p >= start))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        Some(Clip {
            video_caps: self.video_caps.clone()?,
            audio_caps: if audio.is_empty() {
                None
            } else {
                self.audio_caps.clone()
            },
            video,
            audio,
            start,
            duration: self.buffered(),
        })
    }
}

/// A running replay buffer. Dropping it stops the encoder and frees the memory.
pub struct Replay {
    feed: Arc<Feed>,
    tap: Tap,
    pipeline: gst::Pipeline,
    ring: Arc<Mutex<Ring>>,
    container: Container,
    stop: Arc<AtomicBool>,
}

impl Replay {
    /// Start encoding into memory. `on_error` is called (from another thread) if the encoder
    /// stops working.
    pub fn start(
        opts: Options,
        seconds: u32,
        video_caps: gst::Caps,
        tap: Tap,
        on_error: impl FnOnce(String) + Send + 'static,
    ) -> Result<Self, String> {
        let ring = Arc::new(Mutex::new(Ring::new(gst::ClockTime::from_seconds(
            seconds.max(5).into(),
        ))));
        let pipeline = gst::Pipeline::new();

        let (video_src, video_out) = add_video_encoder(&pipeline, &opts, &video_caps)?;
        let video_sink = store_into(&ring, |ring, sample| {
            if ring.video_caps.is_none() {
                ring.video_caps = sample.caps_owned();
            }
            if let Some(buffer) = running_time(&sample) {
                ring.push_video(buffer);
            }
        });
        pipeline.add(&video_sink).map_err(|e| e.to_string())?;
        video_out
            .link(&video_sink)
            .map_err(|e| format!("video → buffer: {e}"))?;

        // Always there: audio may start (or change device) after the buffer does. When no
        // audio arrives, clips are simply video-only.
        let (audio_src, audio_out) = add_audio_encoder(&pipeline, opts.container)?;
        let audio_sink = store_into(&ring, |ring, sample| {
            if ring.audio_caps.is_none() {
                ring.audio_caps = sample.caps_owned();
            }
            if let Some(buffer) = running_time(&sample) {
                ring.push_audio(buffer);
            }
        });
        pipeline.add(&audio_sink).map_err(|e| e.to_string())?;
        audio_out
            .link(&audio_sink)
            .map_err(|e| format!("audio → buffer: {e}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let bus = pipeline.bus().expect("pipeline has a bus");
        std::thread::Builder::new()
            .name("lainify-replay".into())
            .spawn({
                let stop = Arc::clone(&stop);
                move || {
                    if let Some(result) = wait_for_eos(&bus, &stop) {
                        on_error(result.err().unwrap_or_else(|| "the encoder stopped".into()));
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| "The replay encoder could not start".to_string())?;

        let feed = Feed::new(video_src, Some(audio_src), video_caps);
        tap.add(Arc::clone(&feed));
        Ok(Self {
            feed,
            tap,
            pipeline,
            ring,
            container: opts.container,
            stop,
        })
    }

    /// Seconds currently buffered (up to a keyframe interval over the length asked for).
    pub fn buffered_seconds(&self) -> f64 {
        self.ring.lock().unwrap().buffered().nseconds() as f64 / 1e9
    }

    /// Write what's buffered to a new file in the background. `on_done` gets the path and the
    /// clip's length in seconds.
    pub fn save(
        &self,
        on_done: impl FnOnce(Result<(PathBuf, f64), String>) + Send + 'static,
    ) -> Result<(), String> {
        let clip = self
            .ring
            .lock()
            .unwrap()
            .snapshot()
            .ok_or_else(|| "Nothing buffered yet".to_string())?;
        let container = self.container;
        std::thread::Builder::new()
            .name("lainify-clip".into())
            .spawn(move || {
                let seconds = clip.duration.nseconds() as f64 / 1e9;
                on_done(write_clip(clip, container).map(|path| (path, seconds)));
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Drop for Replay {
    fn drop(&mut self) {
        self.tap.remove(&self.feed);
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// An appsink that hands every sample to `store` with the ring locked.
fn store_into(
    ring: &Arc<Mutex<Ring>>,
    store: impl Fn(&mut Ring, gst::Sample) + Send + Sync + 'static,
) -> gst::Element {
    let sink = gst_app::AppSink::builder().sync(false).build();
    let ring = Arc::clone(ring);
    sink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                store(&mut ring.lock().unwrap(), sample);
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    sink.upcast()
}

/// The sample's buffer with its timestamps converted to running time. Encoders shift their
/// output by 1000 hours (so decode timestamps never go negative) and say so in the segment; the
/// audio encoder doesn't, so raw timestamps from the two streams aren't comparable.
fn running_time(sample: &gst::Sample) -> Option<gst::Buffer> {
    let buffer = sample.buffer_owned()?;
    let segment = sample.segment()?.downcast_ref::<gst::ClockTime>()?.clone();
    let pts = segment.to_running_time(buffer.pts()?)?;
    let dts = buffer.dts().and_then(|d| segment.to_running_time(d));
    let mut out = buffer.copy();
    {
        let out = out.get_mut().expect("fresh copy is writable");
        out.set_pts(pts);
        out.set_dts(dts);
    }
    Some(out)
}

/// Mux `clip` into `~/Videos/lainify/clips/` and return the file's path.
fn write_clip(clip: Clip, container: Container) -> Result<PathBuf, String> {
    let dir = crate::paths::clips_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!(
        "lainify-clip-{}.{}",
        crate::paths::timestamp(),
        container.extension()
    ));

    let pipeline = gst::Pipeline::new();
    let result = (|| {
        let (mux, _sink) = add_mux(&pipeline, container, &path)?;
        // Request the mux pad by kind: an appsrc's pad reports ANY caps until it's running, so
        // automatic linking would happily put video on an audio pad.
        let source = |caps: &gst::Caps, template: &str| -> Result<gst_app::AppSrc, String> {
            let src = gst_app::AppSrc::builder()
                .caps(caps)
                .format(gst::Format::Time)
                .build();
            pipeline.add(&src).map_err(|e| e.to_string())?;
            let sink_pad = mux
                .request_pad_simple(template)
                .ok_or_else(|| format!("the muxer has no {template} pad"))?;
            src.static_pad("src")
                .expect("appsrc has a src pad")
                .link(&sink_pad)
                .map_err(|e| format!("clip → mux: {e:?}"))?;
            Ok(src)
        };
        let video = source(&clip.video_caps, "video_%u")?;
        let audio = clip
            .audio_caps
            .as_ref()
            .map(|caps| source(caps, "audio_%u"))
            .transpose()?;

        let stop = AtomicBool::new(false);
        let bus = pipeline.bus().expect("pipeline has a bus");
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| "The clip writer could not start".to_string())?;

        // Interleave by time, so the muxer never waits on one stream with the other far ahead.
        let mut v = clip.video.iter().peekable();
        let mut a = clip.audio.iter().peekable();
        loop {
            let next_video = v.peek().and_then(|b| b.pts());
            let next_audio = a.peek().and_then(|b| b.pts());
            let (src, buffer) = match (next_video, next_audio, &audio) {
                (Some(tv), Some(ta), Some(asrc)) if ta < tv => (asrc, a.next()),
                (Some(_), _, _) => (&video, v.next()),
                (None, Some(_), Some(asrc)) => (asrc, a.next()),
                _ => break,
            };
            let Some(buffer) = buffer else { break };
            src.push_buffer(retime(buffer, clip.start))
                .map_err(|e| format!("writing the clip: {e:?}"))?;
        }
        let _ = video.end_of_stream();
        if let Some(audio) = &audio {
            let _ = audio.end_of_stream();
        }
        wait_for_eos(&bus, &stop).unwrap_or(Ok(()))
    })();
    let _ = pipeline.set_state(gst::State::Null);
    if result.is_err() {
        let _ = std::fs::remove_file(&path);
    }
    result.map(|()| path)
}

/// Shallow copy with timestamps relative to the clip's first keyframe.
fn retime(buffer: &gst::Buffer, start: gst::ClockTime) -> gst::Buffer {
    let mut out = buffer.copy();
    {
        let out = out.get_mut().expect("fresh copy is writable");
        out.set_pts(buffer.pts().map(|p| p.saturating_sub(start)));
        out.set_dts(buffer.dts().map(|d| d.saturating_sub(start)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ms: u64, key: bool) -> gst::Buffer {
        let mut buffer = gst::Buffer::new();
        {
            let b = buffer.get_mut().unwrap();
            b.set_pts(gst::ClockTime::from_mseconds(ms));
            if !key {
                b.set_flags(gst::BufferFlags::DELTA_UNIT);
            }
        }
        buffer
    }

    #[test]
    fn ring_keeps_whole_groups_covering_the_length() {
        gst::init().unwrap();
        let mut ring = Ring::new(gst::ClockTime::from_seconds(10));
        ring.push_video(frame(0, false)); // before any keyframe: dropped
        assert!(ring.gops.is_empty());
        // 25 s at 10 fps with a keyframe every second.
        for i in 0..250u64 {
            ring.push_video(frame(i * 100, i % 10 == 0));
        }
        let newest = 24_900;
        let start = ring.gops.front().unwrap().start.mseconds();
        assert_eq!(start % 1000, 0, "starts on a keyframe");
        assert!(newest - start >= 10_000, "covers the length");
        assert!(newest - start < 11_000, "but not more than one group extra");
        assert_eq!(ring.buffered().mseconds(), newest - start);
    }

    #[test]
    fn ring_trims_audio_to_the_video() {
        gst::init().unwrap();
        let mut ring = Ring::new(gst::ClockTime::from_seconds(2));
        for i in 0..100u64 {
            ring.push_audio(frame(i * 50, true));
            if i % 2 == 0 {
                ring.push_video(frame(i * 50, i % 20 == 0));
            }
        }
        let start = ring.gops.front().unwrap().start;
        assert!(ring.audio.iter().all(|b| b.pts().unwrap() >= start));
    }
}
