# lainify

A clean, low-latency browser viewer for capture cards and webcams. It shows the picture, plays the sound, takes screenshots and records.

Built with SvelteKit (static, client-only), Svelte 5 runes, TypeScript and Tailwind CSS v4.

## Develop

```sh
nix develop        # or: direnv allow
pnpm install
pnpm dev           # http://localhost:5173
```

The dev shell provides Node 22, pnpm and `v4l-utils`. Run `v4l2-ctl --list-formats-ext -d /dev/video0` to see the modes your card really supports.

| Command       | What it does              |
| ------------- | ------------------------- |
| `pnpm check`  | Type-check (svelte-check) |
| `pnpm test`   | Unit tests (vitest)       |
| `pnpm build`  | Static site in `build/`   |
| `pnpm format` | Prettier                  |

Camera access needs a secure context, so serve the build over `https://` or `localhost`.

## Docker

```sh
docker compose up -d --build     # http://localhost:8080
```

The image is a two-stage build: pnpm builds the static site, and unprivileged nginx serves it on port 8080. That nginx has SPA fallback, long-lived caching for hashed assets, and a `Permissions-Policy` that allows the camera and microphone for this origin only. Compose binds it to `127.0.0.1` because browsers only allow camera access on `localhost` or HTTPS. To reach it from other machines, put it behind an HTTPS reverse proxy.

To run the CI-built image instead of building locally:

```sh
LAINIFY_IMAGE=ghcr.io/<owner>/lainify:latest docker compose pull
LAINIFY_IMAGE=ghcr.io/<owner>/lainify:latest docker compose up -d
```

### CI

`.github/workflows/docker.yml` first runs `pnpm check` and `pnpm test`. It then builds a `linux/amd64` + `linux/arm64` image and pushes it to `ghcr.io/<owner>/<repo>`:

| Trigger        | Tags                            |
| -------------- | ------------------------------- |
| push to `main` | `latest`, `main`, `sha-<short>` |
| tag `v1.2.3`   | `1.2.3`, `1.2`, `sha-<short>`   |
| pull request   | built only, not pushed          |

New GHCR packages start private. To make it public, go to the repo → Packages → lainify → Package settings → Change visibility.

## Features

- Picks the capture device automatically and matches its audio input (by USB id, group or name). It never falls back to a microphone.
- Audio is raw: echo cancellation, noise suppression and AGC are off. It requests stereo at 48 kHz and plays through Web Audio with 0–150 % volume and a stereo peak meter.
- Resolution / FPS picker limited to the device's reported range. It requests native modes without browser scaling.
- Measured presentation fps is shown in the status pill.
- PNG screenshots at native resolution.
- Recording to MP4 (H.264/AAC) or WebM. In Chromium you can stream long recordings straight to a file.
- Fullscreen; controls and cursor auto-hide.
- Settings are remembered between visits.

### Shortcuts

`F` fullscreen · `M` mute · `↑`/`↓` volume · `S` screenshot · `R` record · `,` settings · `Esc` close

## Troubleshooting

- **"Device is busy"**: only one app can stream from a V4L2 device at a time. Close OBS or other tabs that use it.
- **No sound from the card (Linux)**: PipeWire may have the card's profile set to _Off_. Enable it in pavucontrol → Configuration, or run:

  ```sh
  pactl set-card-profile alsa_card.usb-Elgato_… input:analog-stereo
  ```

## Layout

```
src/lib/
  capture/   device discovery, stream session, constraints, mode presets, errors
  audio/     Web Audio engine + meter math
  record/    MediaRecorder wrapper, MIME selection, screenshots
  settings/  persisted preferences (schema + rune store)
  ui/        idle tracker, fps meter, toasts, keyboard shortcuts
  components/
src/routes/+page.svelte   wires everything together
```
