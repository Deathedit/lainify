//! lainify (native) — a capture-card viewer that reads the card's real formats, including
//! uncompressed RGB 4:4:4, which browsers can't.

mod audio;
mod bridge;
mod devices;
mod paths;
mod recorder;
mod replay;
mod video;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

unsafe extern "C" {
    /// Registers the `com.lainify.native` FrameSink singleton. Defined in `cpp/frame_sink.cpp`.
    fn lainify_register_frame_sink();
}

fn main() {
    if let Err(e) = gstreamer::init() {
        eprintln!("lainify: GStreamer failed to initialise: {e}");
        std::process::exit(1);
    }

    // SAFETY: single-threaded here; nothing has spawned and Qt is not initialised yet.
    unsafe {
        if std::env::var_os("QT_QUICK_CONTROLS_STYLE").is_none() {
            std::env::set_var("QT_QUICK_CONTROLS_STYLE", "Basic");
        }
    }

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        // Where QML's Settings keeps preferences: ~/.config/lainify/lainify.conf.
        app.as_mut()
            .set_organization_name(&QString::from("lainify"));
        app.as_mut().set_application_name(&QString::from("lainify"));
    }
    QGuiApplication::set_desktop_file_name(&QString::from("lainify"));
    // SAFETY: plain registration call; the application exists and no QML has loaded yet.
    unsafe { lainify_register_frame_sink() };
    let mut engine = QQmlApplicationEngine::new();

    // Turn a QML load failure into a non-zero exit instead of an invisible running process.
    let failed = Arc::new(AtomicBool::new(false));
    let _qml_failure = engine.as_mut().map(|mut engine| {
        let failed = Arc::clone(&failed);
        engine.as_mut().on_object_created(move |_, object, _url| {
            if object.is_null() {
                failed.store(true, Ordering::Relaxed);
            }
        })
    });

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/com/lainify/qml/Main.qml"));
    }
    if failed.load(Ordering::Relaxed) {
        eprintln!("lainify: the QML errors above are fatal; not starting.");
        std::process::exit(1);
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
