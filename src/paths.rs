//! Where screenshots and recordings go, and how they're named.

use std::path::PathBuf;

use gstreamer::glib;

fn special(dir: glib::UserDirectory) -> PathBuf {
    glib::user_special_dir(dir)
        .unwrap_or_else(glib::home_dir)
        .join("lainify")
}

pub fn screenshots_dir() -> PathBuf {
    special(glib::UserDirectory::Pictures)
}

pub fn recordings_dir() -> PathBuf {
    special(glib::UserDirectory::Videos)
}

/// `2026-11-17_18-04-05`, local time.
pub fn timestamp() -> String {
    glib::DateTime::now_local()
        .and_then(|t| t.format("%Y-%m-%d_%H-%M-%S"))
        .map(|s| s.to_string())
        .unwrap_or_else(|_| "capture".into())
}

pub fn clips_dir() -> PathBuf {
    recordings_dir().join("clips")
}
