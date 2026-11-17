# lainify

A clean, low-latency desktop viewer for capture cards. It opens the card in its real formats, including uncompressed RGB 4:4:4, plays its audio, takes lossless screenshots, records with the GPU encoder (in 4:4:4 if you want) and can keep a replay buffer of the last minutes of gameplay.

Built with Rust, Qt 6 / QML (through [cxx-qt](https://github.com/KDAB/cxx-qt)) and GStreamer. Linux only.

## Why native

lainify started as a browser app, and browsers set a hard limit on picture quality. Chromium opens any capture card as MJPEG once the mode is larger than 640×480, processes video as 4:2:0 internally, and gives a web page no way to change either. A native app can open the card in whatever format it offers.

With an Elgato 4K X on a 10 Gbps USB port, that means:

| Format                    | Largest modes                      |
| ------------------------- | ---------------------------------- |
| RGB 4:4:4 (uncompressed)  | 1080p120, 1440p60                  |
| NV12 4:2:0 (uncompressed) | 4K60                               |
| YUV 4:2:2 (uncompressed)  | 4K50                               |
| 10-bit P010               | 4K30                               |
| MJPEG                     | 4K144                              |

On a 5 Gbps port the card offers far fewer uncompressed modes; check `lsusb -t` if the list looks short.

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

For an interactive shell, use `nix-shell`, `nix develop` or `direnv allow`; they're the same environment. Without the repo, `nix run github:Deathedit/lainify` works too. The package also installs a desktop entry and icon, so `nix profile install` puts lainify in your app launcher.

Run `v4l2-ctl --list-formats-ext -d /dev/video0` to see the modes your card really supports.

## Using it

- On launch it opens the capture device and mode you used last, or the first device at 1080p60 RGB 4:4:4. Plugging the card in later opens it automatically; unplugging shows a notice and resumes when it's back.
- Settings (<kbd>,</kbd>) lists Device, Format, Resolution and Frame rate from the card's real format list, so every combination shown is one the card offers.
- Audio follows the video device ("Match video device") or any input you pick, with volume up to 150%, mute and a level meter. PipeWire usually labels a capture card's input "Analog Stereo"; that is its HDMI audio. lainify measures how much audio actually arrives, and warns under Settings → Audio if the card is dropping some (heard as crackling). A common cause on Linux is the USB controller missing the card's audio packets while it carries heavy video: an Elgato 4K X on an AMD 800-series chipset port lost about 0.1% of its audio in 4K uncompressed modes, and none at all on a port wired to the CPU's own USB controller. If you see the warning, try another 10 Gbps port first; MJPEG or a lower resolution also avoids it.
- Screenshots are lossless PNGs in `~/Pictures/lainify`.
- Recordings go to `~/Videos/lainify`. Pick the encoder (NVENC H.264, HEVC or AV1 when there's an NVIDIA GPU, otherwise x264), bitrate, 4:2:0 or 4:4:4 colour (H.264/HEVC) and MKV or MP4. MKV stays playable if lainify crashes mid-recording; 4:4:4 files play in mpv and VLC but not in browsers or most editors.
- The replay buffer (off by default; turn it on under Settings → Replay buffer) keeps the last 30 s to 5 min of gameplay encoded in memory. Press <kbd>C</kbd> or the clip button to save it to `~/Videos/lainify/clips`, instantly and without interrupting anything, even while recording. It uses the Recording encoder, colour and file format with its own bitrate. A minute at 20 Mbps takes about 150 MB of RAM. Clips start on a keyframe (one per second), so they can be up to a second longer than the length you picked. Changing the capture mode or the buffer's settings empties it.
- Every choice is remembered in `~/.config/lainify/lainify.conf`.

| Key                          | Action                 |
| ---------------------------- | ---------------------- |
| <kbd>F</kbd> / double-click  | Fullscreen             |
| <kbd>M</kbd>                 | Mute                   |
| <kbd>↑</kbd> <kbd>↓</kbd>    | Volume                 |
| <kbd>S</kbd>                 | Screenshot             |
| <kbd>R</kbd>                 | Start / stop recording |
| <kbd>C</kbd>                 | Save replay clip       |
| <kbd>,</kbd>                 | Settings               |
| <kbd>Esc</kbd>               | Close panel / leave fullscreen |

## How it works

```
video:  device ! capsfilter [! jpegdec | ! videoflip] ! videoconvert ! appsink ─→ QVideoFrame → VideoOutput
audio:  device ! audioconvert ! audioresample ! tee ─┬→ level ! volume ! pulsesink
                                                     └→ appsink
record: appsrc (video) ! videoconvert ! encoder ! parser ─┬→ mux ! filesink
        appsrc (audio) ! opusenc | avenc_aac ─────────────┘
replay: same encoders → appsink → ring of keyframe groups in memory
        on save: appsrc ×2 → mux ! filesink (no re-encode)
```

- `src/devices.rs` lists devices and their exact modes through GStreamer's device monitor, and keeps watching it for hot-plug.
- `src/video.rs` builds the capture pipeline for the chosen mode. `videoconvert` passes through every format Qt can draw and only converts the rest (24-bit BGR → BGRx, losslessly), so colour detail is never reduced. UVC cards send RGB bottom-up, so RGB frames go through `videoflip`.
- `src/audio.rs` plays the audio input and reports peak levels.
- `src/recorder.rs` runs recording as its own pipeline, fed with the frames and samples the viewer already has, so starting or stopping a recording never interrupts the picture. Stopping sends end-of-stream so the file is finalised properly.
- `src/replay.rs` is the replay buffer. It stores encoded buffers grouped by keyframe and drops whole groups from the front once the rest still covers the length, so it always starts on a keyframe. Timestamps are stored as running time, because encoders shift theirs by 1000 hours and the audio encoder doesn't.
- `cpp/frame_sink.*` copies each frame once into a `QVideoFrame` for the QML `VideoOutput`, keeping only the newest so a busy UI drops frames instead of lagging. It's the only hand-written C++, because cxx-qt-lib has no Qt Multimedia bindings.
- `src/bridge/capture.rs` is the `Capture` object QML talks to. `qml/` is the UI, a port of the browser version's design: `Main.qml` wires it together, `components/` holds the small building blocks, and icons are [Lucide](https://lucide.dev) (ISC) with the Inter font (OFL), both in `assets/`.

Qt sends its log to the systemd journal when stderr isn't a terminal; set `QT_FORCE_STDERR_LOGGING=1` (as `./run.sh debug` does) to see it.
