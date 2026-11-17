// Hands decoded GStreamer frames to a QML VideoOutput.
//
// The project's only hand-written C++: cxx-qt-lib has no QVideoSink / QVideoFrame bindings.
// QML gives FrameSink the VideoOutput's sink; Rust pushes frames with lainify_present() from the
// GStreamer streaming thread. Frames are copied once into a QVideoFrame, and only the newest one
// is kept, so a slow UI thread drops frames instead of falling behind.

#pragma once

#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtMultimedia/QVideoSink>

#include <cstdint>

/// QML singleton (`import com.lainify.native`) holding the VideoOutput's sink.
class FrameSink : public QObject
{
    Q_OBJECT
    Q_PROPERTY(QVideoSink *videoSink READ videoSink WRITE setVideoSink NOTIFY videoSinkChanged)

public:
    explicit FrameSink(QObject *parent = nullptr);

    QVideoSink *videoSink() const;
    void setVideoSink(QVideoSink *sink);

Q_SIGNALS:
    void videoSinkChanged();
};

// Pixel formats, matching `pipeline::PixelFormat` in Rust.
enum LainifyFormat : int32_t {
    LainifyBgrx = 0,
    LainifyNv12 = 1,
    LainifyI420 = 2,
    LainifyYuyv = 3,
    LainifyYuv422p = 4,
    LainifyP010 = 5,
};

extern "C" {
/// Copy one frame and show it. Safe to call from any thread.
///
/// `range`: 0 unknown, 1 limited (16–235), 2 full. `matrix`: 0 unknown, 1 BT.601, 2 BT.709,
/// 3 BT.2020. `planes`, `strides` and `rows` each have `n_planes` entries.
void lainify_present(int32_t format, int32_t width, int32_t height, int32_t range, int32_t matrix,
                     const uint8_t *const *planes, const int32_t *strides, const int32_t *rows,
                     int32_t n_planes);

/// Blank the picture (capture stopped).
void lainify_clear();

/// Register the FrameSink singleton. Call before loading QML.
void lainify_register_frame_sink();
}
