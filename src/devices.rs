//! Capture devices: video sources with the exact modes (format × size × frame rate) each one
//! offers, and audio inputs. A watcher thread reports the lists again whenever a device is
//! plugged in or removed.
//!
//! Unlike a browser, GStreamer sees the card's real format list, so the user can pick
//! uncompressed RGB (4:4:4) or YUYV (4:2:2) instead of whatever the browser prefers.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gstreamer as gst;
use gstreamer::prelude::*;
use serde::Serialize;

/// Formats we know how to show, best quality first. The first field is GStreamer's caps name
/// (`MJPG` stands for `image/jpeg`); the second is the short label used in the status pill.
const FORMATS: &[(&str, &str, &str)] = &[
    ("BGR", "RGB 4:4:4", "RGB 4:4:4 — uncompressed"),
    ("YUY2", "YUV 4:2:2", "YUV 4:2:2 — uncompressed"),
    (
        "P010_10LE",
        "10-bit 4:2:0",
        "YUV 4:2:0 10-bit — uncompressed",
    ),
    ("NV12", "YUV 4:2:0", "YUV 4:2:0 — uncompressed"),
    ("I420", "YUV 4:2:0", "YUV 4:2:0 planar — uncompressed"),
    ("MJPG", "MJPEG", "MJPEG — compressed"),
];

pub const MJPEG: &str = "MJPG";

/// Short label for a format id, e.g. `BGR` → `RGB 4:4:4`.
pub fn short_label(id: &str) -> &'static str {
    FORMATS
        .iter()
        .find(|(f, _, _)| *f == id)
        .map(|(_, short, _)| *short)
        .unwrap_or("")
}

#[derive(Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub formats: Vec<FormatInfo>,
}

#[derive(Serialize)]
pub struct FormatInfo {
    pub id: String,
    pub label: String,
    pub sizes: Vec<SizeInfo>,
}

#[derive(Serialize)]
pub struct SizeInfo {
    pub width: i32,
    pub height: i32,
    pub rates: Vec<RateInfo>,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
pub struct RateInfo {
    pub num: i32,
    pub den: i32,
}

/// Every device at one moment. Handles and descriptions share indices.
pub struct Snapshot {
    pub video: Vec<gst::Device>,
    pub video_info: Vec<DeviceInfo>,
    pub audio: Vec<gst::Device>,
    pub audio_names: Vec<String>,
}

/// `Elgato 4K X: Elgato 4K X` → `Elgato 4K X`, and drops a trailing USB id like `(0fd9:009c)`.
pub fn clean_label(label: &str) -> String {
    let mut s = label.trim();
    if let Some(open) = s.rfind(" (") {
        let id = &s[open + 2..];
        if id.len() == 10 && id.ends_with(')') && id.as_bytes()[4] == b':' {
            s = s[..open].trim_end();
        }
    }
    if let Some((a, b)) = s.split_once(": ")
        && a == b
    {
        return a.to_string();
    }
    s.to_string()
}

/// Watch for devices in the background. `on_change` runs on the watcher thread once at start,
/// then after every plug or unplug (debounced), and whenever `force` is set.
pub fn watch(force: Arc<AtomicBool>, on_change: impl Fn(Snapshot) + Send + 'static) {
    std::thread::Builder::new()
        .name("lainify-devices".into())
        .spawn(move || {
            let monitor = gst::DeviceMonitor::new();
            monitor.add_filter(Some("Video/Source"), None);
            monitor.add_filter(Some("Audio/Source"), None);
            if monitor.start().is_err() {
                on_change(Snapshot {
                    video: Vec::new(),
                    video_info: Vec::new(),
                    audio: Vec::new(),
                    audio_names: Vec::new(),
                });
                return;
            }
            let bus = monitor.bus();
            on_change(snapshot(&monitor));

            loop {
                let mut changed = force.swap(false, Ordering::Relaxed);
                if let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(200)) {
                    use gst::MessageView;
                    if matches!(
                        msg.view(),
                        MessageView::DeviceAdded(_) | MessageView::DeviceRemoved(_)
                    ) {
                        // A card brings up several nodes (video, audio) in quick succession.
                        std::thread::sleep(Duration::from_millis(400));
                        while bus.pop().is_some() {}
                        changed = true;
                    }
                }
                if changed {
                    on_change(snapshot(&monitor));
                }
            }
        })
        .expect("spawn the device watcher");
}

fn snapshot(monitor: &gst::DeviceMonitor) -> Snapshot {
    let mut snap = Snapshot {
        video: Vec::new(),
        video_info: Vec::new(),
        audio: Vec::new(),
        audio_names: Vec::new(),
    };
    for device in monitor.devices() {
        if device.has_classes("Video/Source") {
            let Some(caps) = device.caps() else { continue };
            let formats = describe(&caps);
            if formats.is_empty() {
                continue;
            }
            let name = unique(
                &snap
                    .video_info
                    .iter()
                    .map(|d| d.name.clone())
                    .collect::<Vec<_>>(),
                clean_label(&device.display_name()),
            );
            snap.video_info.push(DeviceInfo { name, formats });
            snap.video.push(device);
        } else if device.has_classes("Audio/Source") {
            // Skip "Monitor of …" sources: they record an output, not an input.
            let is_monitor = device
                .properties()
                .and_then(|p| p.get::<String>("device.class").ok())
                .is_some_and(|c| c == "monitor");
            if is_monitor {
                continue;
            }
            let name = unique(&snap.audio_names, clean_label(&device.display_name()));
            snap.audio_names.push(name);
            snap.audio.push(device);
        }
    }
    snap
}

/// `name`, or `name (2)`, `name (3)`… if it's taken.
fn unique(taken: &[String], name: String) -> String {
    if !taken.contains(&name) {
        return name;
    }
    (2..)
        .map(|n| format!("{name} ({n})"))
        .find(|candidate| !taken.contains(candidate))
        .expect("an unused name")
}

fn describe(caps: &gst::CapsRef) -> Vec<FormatInfo> {
    // format → (width, height) → rates
    let mut found: BTreeMap<&str, BTreeMap<(i32, i32), Vec<RateInfo>>> = BTreeMap::new();

    // Only system-memory caps: the DMABuf duplicates describe the same modes.
    for (s, features) in caps.iter_with_features() {
        if !features.contains(gst::CAPS_FEATURE_MEMORY_SYSTEM_MEMORY) {
            continue;
        }
        let formats: Vec<&str> = match s.name().as_str() {
            "image/jpeg" => vec![MJPEG],
            "video/x-raw" => strings(s, "format")
                .iter()
                .filter_map(|f| {
                    FORMATS
                        .iter()
                        .find(|(id, _, _)| id == f)
                        .map(|(id, _, _)| *id)
                })
                .collect(),
            _ => continue,
        };
        let rates_here = frame_rates(s);
        for width in ints(s, "width") {
            for height in ints(s, "height") {
                for format in &formats {
                    let rates = found
                        .entry(format)
                        .or_default()
                        .entry((width, height))
                        .or_default();
                    for rate in &rates_here {
                        if !rates.contains(rate) {
                            rates.push(*rate);
                        }
                    }
                }
            }
        }
    }

    FORMATS
        .iter()
        .filter_map(|(id, _, label)| {
            let sizes = found.remove(id)?;
            let mut sizes: Vec<SizeInfo> = sizes
                .into_iter()
                .filter(|(_, rates)| !rates.is_empty())
                .map(|((width, height), mut rates)| {
                    rates.sort_by(|a, b| {
                        (b.num as i64 * a.den as i64).cmp(&(a.num as i64 * b.den as i64))
                    });
                    SizeInfo {
                        width,
                        height,
                        rates,
                    }
                })
                .collect();
            sizes.sort_by_key(|s| std::cmp::Reverse(s.width as i64 * s.height as i64));
            Some(FormatInfo {
                id: id.to_string(),
                label: label.to_string(),
                sizes,
            })
        })
        .collect()
}

/// A field that is either one value or a list of them.
fn list_of<'a, T: for<'b> gst::glib::value::FromValue<'b> + 'a>(
    s: &'a gst::StructureRef,
    field: &str,
) -> Vec<T> {
    if let Ok(v) = s.get::<T>(field) {
        return vec![v];
    }
    s.get::<gst::List>(field)
        .map(|list| list.iter().filter_map(|v| v.get::<T>().ok()).collect())
        .unwrap_or_default()
}

fn strings(s: &gst::StructureRef, field: &str) -> Vec<String> {
    list_of::<String>(s, field)
}

fn ints(s: &gst::StructureRef, field: &str) -> Vec<i32> {
    list_of::<i32>(s, field)
}

fn frame_rates(s: &gst::StructureRef) -> Vec<RateInfo> {
    let to_rate = |f: gst::Fraction| RateInfo {
        num: f.numer(),
        den: f.denom(),
    };
    if let Ok(f) = s.get::<gst::Fraction>("framerate") {
        return vec![to_rate(f)];
    }
    if let Ok(list) = s.get::<gst::List>("framerate") {
        return list
            .iter()
            .filter_map(|v| v.get::<gst::Fraction>().ok())
            .map(to_rate)
            .collect();
    }
    if let Ok(range) = s.get::<gst::FractionRange>("framerate") {
        return vec![to_rate(range.max())];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_labels() {
        assert_eq!(clean_label("Elgato 4K X: Elgato 4K X"), "Elgato 4K X");
        assert_eq!(clean_label("USB Video (0fd9:009c)"), "USB Video");
        assert_eq!(clean_label("Cam: Front"), "Cam: Front");
    }

    #[test]
    fn makes_names_unique() {
        let taken = vec!["Cam".to_string(), "Cam (2)".to_string()];
        assert_eq!(unique(&taken, "Cam".into()), "Cam (3)");
        assert_eq!(unique(&taken, "Other".into()), "Other");
    }
}
