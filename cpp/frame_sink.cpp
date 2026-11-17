#include "frame_sink.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QMutex>
#include <QtMultimedia/QVideoFrame>
#include <QtMultimedia/QVideoFrameFormat>
#include <QtQml/QQmlEngine>

#include <algorithm>
#include <atomic>
#include <cstring>

namespace {

QMutex g_mutex;
QPointer<QVideoSink> g_sink; // guarded by g_mutex
QVideoFrame g_latest;        // guarded by g_mutex
std::atomic_bool g_pending{false};

QVideoFrameFormat::PixelFormat toQt(int32_t format)
{
    switch (format) {
    case LainifyBgrx: return QVideoFrameFormat::Format_BGRX8888;
    case LainifyNv12: return QVideoFrameFormat::Format_NV12;
    case LainifyI420: return QVideoFrameFormat::Format_YUV420P;
    case LainifyYuyv: return QVideoFrameFormat::Format_YUYV;
    case LainifyYuv422p: return QVideoFrameFormat::Format_YUV422P;
    case LainifyP010: return QVideoFrameFormat::Format_P010;
    default: return QVideoFrameFormat::Format_Invalid;
    }
}

// Runs on the GUI thread. Takes whatever frame is newest at that moment.
void deliver(const QVideoFrame &frame)
{
    QMutexLocker lock(&g_mutex);
    g_latest = frame;
    if (g_pending.exchange(true)) {
        return; // a delivery is already queued; it will pick up this frame
    }
    lock.unlock();

    QMetaObject::invokeMethod(
        QCoreApplication::instance(),
        [] {
            QMutexLocker lock(&g_mutex);
            const QVideoFrame frame = g_latest;
            g_latest = QVideoFrame();
            g_pending.store(false);
            const QPointer<QVideoSink> sink = g_sink;
            lock.unlock();
            if (sink) {
                sink->setVideoFrame(frame);
            }
        },
        Qt::QueuedConnection);
}

} // namespace

FrameSink::FrameSink(QObject *parent)
    : QObject(parent)
{
}

QVideoSink *FrameSink::videoSink() const
{
    QMutexLocker lock(&g_mutex);
    return g_sink.data();
}

void FrameSink::setVideoSink(QVideoSink *sink)
{
    {
        QMutexLocker lock(&g_mutex);
        if (g_sink == sink) {
            return;
        }
        g_sink = sink;
    }
    Q_EMIT videoSinkChanged();
}

extern "C" void lainify_present(int32_t format, int32_t width, int32_t height, int32_t range,
                                int32_t matrix, const uint8_t *const *planes,
                                const int32_t *strides, const int32_t *rows, int32_t n_planes)
{
    const auto pixelFormat = toQt(format);
    if (pixelFormat == QVideoFrameFormat::Format_Invalid || width <= 0 || height <= 0) {
        return;
    }

    QVideoFrameFormat frameFormat(QSize(width, height), pixelFormat);
    if (range == 1) {
        frameFormat.setColorRange(QVideoFrameFormat::ColorRange_Video);
    } else if (range == 2) {
        frameFormat.setColorRange(QVideoFrameFormat::ColorRange_Full);
    }
    switch (matrix) {
    case 1: frameFormat.setColorSpace(QVideoFrameFormat::ColorSpace_BT601); break;
    case 2: frameFormat.setColorSpace(QVideoFrameFormat::ColorSpace_BT709); break;
    case 3: frameFormat.setColorSpace(QVideoFrameFormat::ColorSpace_BT2020); break;
    default: break;
    }

    QVideoFrame frame(frameFormat);
    if (!frame.map(QVideoFrame::WriteOnly)) {
        return;
    }
    const int count = std::min<int>(n_planes, frame.planeCount());
    for (int i = 0; i < count; ++i) {
        uchar *dst = frame.bits(i);
        const int dstStride = frame.bytesPerLine(i);
        if (dst == nullptr || dstStride <= 0) {
            continue;
        }
        const int dstRows = frame.mappedBytes(i) / dstStride;
        const int rowCount = std::min(rows[i], dstRows);
        const int rowBytes = std::min(dstStride, strides[i]);
        for (int r = 0; r < rowCount; ++r) {
            std::memcpy(dst + qsizetype(r) * dstStride, planes[i] + qsizetype(r) * strides[i],
                        rowBytes);
        }
    }
    frame.unmap();
    deliver(frame);
}

extern "C" void lainify_clear()
{
    deliver(QVideoFrame());
}

// Registered imperatively: this file is not part of the cxx-qt QML module, so it never reaches
// qmltyperegistrar. main.rs calls it, which is also what keeps the object linked.
extern "C" void lainify_register_frame_sink()
{
    qmlRegisterSingletonType<FrameSink>(
        "com.lainify.native", 1, 0, "FrameSink",
        [](QQmlEngine *, QJSEngine *) -> QObject * { return new FrameSink; });
}
