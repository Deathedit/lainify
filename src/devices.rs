//! Capture devices and the exact modes (format × size × frame rate) each one offers.
//!
//! Unlike a browser, GStreamer sees the card's real format list, so the user can pick
//! uncompressed RGB (4:4:4) or YUYV (4:2:2) instead of whatever the browser prefers.

use std::collections::BTreeMap;

use gstreamer as gst;
use gstreamer::prelude::*;
use serde::Serialize;

/// Formats we know how to show, best quality first. The first field is GStreamer's caps name
/// (`MJPG` stands for `image/jpeg`).
const FORMATS: &[(&str, &str)] = &[
    ("BGR", "RGB 4:4:4 — uncompressed"),
    ("YUY2", "YUV 4:2:2 — uncompressed"),
    ("P010_10LE", "YUV 4:2:0 10-bit — uncompressed"),
    ("NV12", "YUV 4:2:0 — uncompressed"),
    ("I420", "YUV 4:2:0 planar — uncompressed"),
    ("MJPG", "MJPEG — compressed"),
];

pub const MJPEG: &str = "MJPG";

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

/// All video capture devices, with their GStreamer handles (used to open them) and a
/// description for the UI. Both lists share indices.
pub fn enumerate() -> (Vec<gst::Device>, Vec<DeviceInfo>) {
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Video/Source"), None);
    if monitor.start().is_err() {
        return (Vec::new(), Vec::new());
    }
    let devices = monitor.devices();
    monitor.stop();

    let mut handles = Vec::new();
    let mut infos = Vec::new();
    for device in devices {
        let Some(caps) = device.caps() else { continue };
        let formats = describe(&caps);
        if formats.is_empty() {
            continue;
        }
        infos.push(DeviceInfo {
            name: device.display_name().to_string(),
            formats,
        });
        handles.push(device);
    }
    (handles, infos)
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
                .filter_map(|f| FORMATS.iter().find(|(id, _)| id == f).map(|(id, _)| *id))
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
        .filter_map(|(id, label)| {
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
