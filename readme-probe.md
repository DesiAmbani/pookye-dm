<div align="center">

<img src="assets/logo.svg" alt="Pookye Download Manager" width="180" />

# Pookye Download Manager

**PDM · Pookye DM** — a fast, multi-segment download manager written in Rust.

Files are split into byte ranges and fetched over several connections at once,
with a queue, per-connection telemetry and a browser handoff.

<sub>Rust · egui · reqwest · alpha</sub>

</div>

---

> **Alpha:** `0.1.0-alpha.1` is a source release. Build it with the tools below;
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
  right-clicked links to the app over a loopback API.
- **Brand-native UI** — the whole interface uses the logo's maroon-and-crimson palette,
  with the mark rasterised at run time.

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
