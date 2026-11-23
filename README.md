<div align="center">

<img src="assets/mark.png" alt="Pookye Download Manager" width="180" />

# Pookye Download Manager

**PDM · Pookye DM** — a fast, multi-segment download manager written in Rust.

Files are split into byte ranges and fetched over several connections at once,
with a queue, per-connection telemetry and a browser handoff.

<sub>Rust · egui · reqwest · alpha</sub>

</div>

---

> **Alpha:** `0.1.0-alpha.2` is a source release. Build it with the tools below;
> there are no prebuilt binaries yet. APIs, storage format and UI may still change.

## Features

**Download engine**

- **Multi-segment transfers** — up to 32 parallel range requests per file, each
  written straight to its own offset in the destination file.
- **Pause and resume** — every stream keeps its own position, so resuming continues
  from the last byte rather than starting the file again.
- **Automatic retry** — failed or stalled streams back off and retry, and a server
  without `Range` support falls back to a single stream instead of failing.
- **Link probing** — before a download starts, Pookye DM asks the server for the
  size, the suggested file name, media type and range support.
- **Live telemetry** — total and per-stream speed, percentage, bytes and ETA.
- **System-wide speed limit** — one shared bandwidth budget for every stream of every
  download, not a per-file cap.

**Managing downloads**

- **Queue with a concurrency limit** — only a set number of downloads run at once;
  the rest wait and start automatically as slots free up.
- **Weekly schedule** — downloads may be limited to a time window on chosen days;
  starts outside it wait in the queue until the window opens.
- **Slow mode** — one switch holds every download to a gentler cap without changing
  the configured speed limit.
- **Categories** — Videos, Music, Documents, Programs, Archives, Other, derived from
  the file name, with instant search across names and URLs.
- **Batch actions** — pause all, resume all, clear finished, and multi-select rows to
  pause, resume or remove them together.
- **Per-download actions** — pause, resume, stop, retry, open file, show in folder,
  copy link, remove, and delete the partial file.
- **History** — the last 40 completed downloads are kept in the settings file.
- **Clipboard watch** — copied links are picked up and offered in the paste bar.

**Integration**

- **Browser handoff** — a Manifest V3 extension forwards browser downloads and
  right-clicked links to the app over a loopback API. The app hands back a token,
  the extension confirms it, and the cookies, referer and user-agent the browser
  held for the request are replayed so signed-in and hotlink-protected files work.
- **Flat dark interface** — a frameless window with its own title bar and squared-off
  window edges, painted flat black. Panels and cards are solid near-black fills with a
  single hairline; buttons are flat rectangles with no gradients, gloss or shadows.
- **Brand-native** — mostly black, with a deep rose for secondary controls and one
  bright pink reserved for the primary action, progress and live figures. The mark
  ships as an embedded PNG (with the traced `assets/logo.svg` as its vector source).
  Text is set in Inter (400/500/600), bundled with the binary.

## Pages

| Page | What it holds |
| --- | --- |
| **Downloads** | Paste bar, search and category filters, download rows with per-stream bars, batch actions |
| **Speed** | System-wide limit, presets, average speed and each download's share |
| **Networks** | Per-connection contribution: range, bytes carried and completion |
| **Schedule** | Concurrency limit and the waiting queue, with *Start now* |
| **Settings** | Folder, segments per download, concurrency, clipboard watch, history |
| **About** | Version, licence, source and lineage |

## Build from source

Requirements: Rust 1.75 or newer (`rustup`) and a C toolchain for the
`windows-gnu` or `windows-msvc` target.

```bash
git clone https://github.com/DesiAmbani/pookye-dm.git
cd pookye-dm
cargo build --release
./target/release/pookye-dm
```

The binary is self-contained: the logo is embedded as `assets/mark.png`, the Inter
faces as `assets/fonts/*.ttf`, and the window needs no decorations or external theme.

## Using it

1. Paste a link into the bar on **Downloads** and press **Start download** (or Enter),
   or use **+ New download** to check the link first and set the name, folder and
   stream count.
2. The download joins the queue; up to *Downloads at once* run in parallel.
3. Watch the block bar and the per-stream tracks; open **Networks** to see what each
   connection carries.
4. Pause, resume, retry or reveal the file from the row.

**Shortcuts:** `Ctrl+N` opens the New download dialog, `Esc` clears the selection
or closes the dialog, `Enter` in the paste bar starts the download.

## Settings

Stored in `%APPDATA%\PookyeDM\settings.json`.

| Option | Default | Description |
| --- | --- | --- |
| Download folder | your Downloads folder | Where files are written |
| Segments per download | 8 | Parallel connections per file (1 = safe, no splitting) |
| Downloads at once | 3 | Concurrent downloads (1–10) |
| Speed limit | 0 (unlimited) | System-wide cap in KB/s |
| Clipboard watch | on | Offer copied links in the paste bar |

## Browser extension

`extension/` is an unpacked Manifest V3 extension:

1. Open `chrome://extensions` and enable **Developer mode**.
2. Choose **Load unpacked** and select the `extension` folder.
3. Start Pookye DM. Browser downloads are handed over, and right-clicking a link
   offers *Download with Pookye DM*.

The extension talks to the app on `127.0.0.1:19330`, falling back to `19331`/`19332`
if that port is taken.

## Local API

Loopback only, JSON, CORS open for extension clients:

| Method | Path | Body | Answer |
| --- | --- | --- | --- |
| `GET` | `/ping` | — | `{ "app": "pookye", "api": 1, "version": "0.1.0-alpha.2", "name": "Pookye Download Manager" }` |
| `POST` | `/add` | `{ "url": "...", "fileName": "optional.bin" }` | `{ "ok": true, "queued": true }` |
| `GET` | `/jobs` | — | `{ "app": "pookye", "api": 1, "averageSpeedBps": 0, "active": 0, "queued": 0, "jobs": [ ... ] }` |

Each job carries `id`, `url`, `fileName`, `category`, `status`, `totalBytes`,
`bytesDownloaded`, `speedBps`, `connections`, `folder` and `error`.

The browser extension sends `POST /add` with a JSON body carrying `url` and an
optional `fileName`; any other client can do the same. Read endpoints are easy to
try from a shell:

```bash
curl -s http://127.0.0.1:19330/ping
curl -s http://127.0.0.1:19330/jobs
```

## Brand palette

Sampled from the logo and weighted 60 / 30 / 10 across the UI:

| Role | Colour |
| --- | --- |
| Window background (60%) | `#0A0A0C` |
| Card / panel surface | `#151416` |
| Raised control / tile | `#1E1B1D` |
| Input well | `#0E0D0F` |
| Deep rose, secondary buttons and selections (30%) | `#7C2D48` |
| Selected row / quiet fill | `#321722` |
| Bright pink, primary action and live figures (10%) | `#FF2E74` |
| Hairline | `#2B2428` |
| Primary text | `#F7F3F5` |
| Secondary text | `#C3A6B1` |
| Labels, hints | `#8A6E79` |

## Project layout

```
src/main.rs          app shell: window, sidebar, header, status bar, update loop
src/pages.rs         Downloads, Speed, Networks, Schedule, Settings, About
src/dialog.rs        New download dialog and link probing
src/widgets.rs       Cards, chips, navigation, download rows, file actions
src/actions.rs       Row and batch actions applied after drawing
src/engine/mod.rs    shared types, palette-independent formatting, rate limiter
src/engine/download.rs  probe, range workers, supervisor
src/engine/manager.rs   jobs, queue and the manager
src/api.rs           loopback JSON API (ping / add / jobs)
src/store.rs         settings and history persistence
src/brand.rs         palette, flat surfaces, backdrop, fonts, logo texture
assets/logo.svg      vector brand mark, traced from the original artwork
assets/mark.png      embedded mark, used by the app and the README
assets/fonts/        Inter 400/500/600, embedded into the binary
extension/           Manifest V3 browser handoff extension
```

## Lineage

The download-manager model here — parallel byte ranges, a queue with a concurrency
limit, browser interception and per-connection telemetry — follows
[QDM · PBhadoo/QDM](https://github.com/PBhadoo/QDM), which is MIT licensed. Pookye DM
is an independent Rust implementation: the engine, UI, local API and extension were
written from scratch for this project, with the logo's palette as the visual identity.

## Limitations (alpha)

- Pausing is honoured between 64 KiB reads, so it is not instantaneous mid-chunk.
- Progress resumes within a session; a restarted app does not continue an interrupted
  partial file.
- Streams share one network route. There is no proxy multiplexing or per-adapter routing.
- No video-site support (no yt-dlp/HLS), no tray icon, no desktop notifications and no
  auto-updater yet.
- The extension is unpacked only; it is not published to the Chrome Web Store.

## License

MIT — see [LICENSE](LICENSE). See [CHANGELOG.md](CHANGELOG.md) for release history.
