use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("com.lainify")
            // Each file becomes a type named after its stem, in one flat namespace; the
            // directories only group source.
            .qml_files([
                "qml/Main.qml",
                "qml/ControlBar.qml",
                "qml/StatusPill.qml",
                "qml/EmptyState.qml",
                "qml/SettingsPanel.qml",
                "qml/Toasts.qml",
                "qml/components/Glass.qml",
                "qml/components/Icon.qml",
                "qml/components/IconButton.qml",
                "qml/components/Kbd.qml",
                "qml/components/LevelMeter.qml",
                "qml/components/Select.qml",
                "qml/components/Spinner.qml",
                "qml/components/TextButton.qml",
                "qml/components/VolumeSlider.qml",
            ])
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true))
            .qml_file(QmlFile::from("qml/Icons.qml").singleton(true))
            .depend("QtQuick")
            .depend("QtQuick.Controls")
            .depend("QtQuick.Effects")
            .depend("QtQuick.Shapes")
            .depend("QtMultimedia")
            .depend("QtCore"),
    )
    .qt_module("Quick")
    .qt_module("Multimedia")
    // Served at qrc:/qt/qml/com/lainify/assets/…
    .qrc_resources(["assets/InterVariable.ttf"])
    // Hand-written C++: QVideoSink/QVideoFrame have no cxx-qt-lib bindings. The header is moc'd.
    .cpp_file("cpp/frame_sink.h")
    .cpp_file("cpp/frame_sink.cpp")
    // Exactly the files containing a `#[cxx_qt::bridge]`.
    .files(["src/bridge/capture.rs"])
    .build();
}
