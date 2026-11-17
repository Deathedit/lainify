# lainify

A clean, low-latency desktop viewer for capture cards. It opens the card in its real formats, including uncompressed RGB 4:4:4, and takes lossless screenshots.

Built with Rust, Qt 6 / QML (through [cxx-qt](https://github.com/KDAB/cxx-qt)) and GStreamer. Linux only.

## Why native

lainify started as a browser app, and browsers set a hard limit on picture quality. Chromium opens any capture card as MJPEG once the mode is larger than 640×480, processes video as 4:2:0 internally, and gives a web page no way to change either. A native app can open the card in whatever format it offers.

With an Elgato 4K X, that means:

| Mode         | Formats the card offers                       |
| ------------ | --------------------------------------------- |
| 1080p60      | RGB 4:4:4, YUV 4:2:2, 10-bit P010, NV12, MJPEG |
| 4K30         | NV12 / I420 (uncompressed 4:2:0), MJPEG       |
| 4K60 and up  | MJPEG only                                    |

## Run

```sh
./run.sh            # build (release) and start
```

`run.sh` enters the Nix shell from `shell.nix` (Rust, Qt 6 with Qt Multimedia, GStreamer, `v4l-utils`) and runs the command you asked for:

| Command            | What it does                                        |
| ------------------ | --------------------------------------------------- |
| `./run.sh run`     | Release build, then start (default)                 |
| `./run.sh debug`   | Debug build, then start with Qt logging on stderr   |
| `./run.sh build`   | Release build into `target/release/lainify`         |
| `./run.sh check`   | Clippy                                              |
| `./run.sh fmt`     | Format the Rust code                                |
| `./run.sh package` | `nix build`, a standalone binary in `./result`      |

For an interactive shell, use `nix-shell`, `nix develop` or `direnv allow`; they're the same environment. Without the repo, `nix run github:Deathedit/lainify` works too.

Run `v4l2-ctl --list-formats-ext -d /dev/video0` to see the modes your card really supports.

## Using it

- On launch it opens the first capture device in the best format it offers, preferring RGB 4:4:4 at 1080p60.
- Device, Format, Resolution and Frame rate are listed from the card's real format list, so every combination shown is one the card offers.
- The bar at the bottom shows the mode the card is actually delivering and the frame rate reaching the screen.
- Screenshots are lossless PNGs in `~/Pictures/lainify`.
- Shortcuts: <kbd>F</kbd> fullscreen, <kbd>Esc</kbd> leave fullscreen, <kbd>S</kbd> screenshot, <kbd>Space</kbd> start/stop.

Not there yet: audio, recording, remembering your choices, hot-plug detection (press Refresh).

## How it works

```
device ! capsfilter [! jpegdec | ! videoflip] ! videoconvert ! appsink  →  QVideoFrame  →  VideoOutput
```

- `src/devices.rs` lists devices and their exact modes through GStreamer's device monitor.
- `src/pipeline.rs` builds the pipeline for the chosen mode. `videoconvert` passes through every format Qt can draw and only converts the rest (24-bit BGR → BGRx, losslessly), so colour detail is never reduced. UVC cards send RGB bottom-up, so RGB frames go through `videoflip`.
- `cpp/frame_sink.*` copies each frame once into a `QVideoFrame` for the QML `VideoOutput`, keeping only the newest so a busy UI drops frames instead of lagging. It's the only hand-written C++, because cxx-qt-lib has no Qt Multimedia bindings.
- `src/bridge/capture.rs` is the `Capture` object QML talks to; `qml/Main.qml` is the window.

Qt sends its log to the systemd journal when stderr isn't a terminal; set `QT_FORCE_STDERR_LOGGING=1` (as `./run.sh debug` does) to see it.
