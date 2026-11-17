use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.lainify")
            .qml_files(["qml/Main.qml"])
            .depend("QtQuick")
            .depend("QtMultimedia"),
    )
    .qt_module("Quick")
    .qt_module("Multimedia")
    // Hand-written C++: QVideoSink/QVideoFrame have no cxx-qt-lib bindings. The header is moc'd.
    .cpp_file("cpp/frame_sink.h")
    .cpp_file("cpp/frame_sink.cpp")
    // Exactly the files containing a `#[cxx_qt::bridge]`.
    .files(["src/bridge/capture.rs"])
    .build();
}
