# Changelog

All notable changes to Pookye Download Manager are recorded here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/).

## [0.1.0-alpha.1] — 2026-11-22

First alpha release. Source-only: build with `cargo build --release`.

### Added

- **Sidebar-driven interface** with Downloads, Speed, Networks, Schedule, Settings and
  About pages, built from the logo's maroon-and-crimson palette.
- **Download queue** with a configurable concurrency limit (1–10). New downloads wait
  as *Queued* and start automatically when a slot frees up.
- **System-wide speed limit** shared by every stream of every download, with
  Unlimited/1/5/10/50 MB/s presets and a KB/s slider.
- **New download dialog** with link preflight: size, suggested file name, media type
  and `Range` support, plus editable name, folder and stream count.
- **Category filters** (Videos, Music, Documents, Programs, Archives, Other) and
  instant search across file names and URLs.
- **Multi-select batch actions**: pause, resume and remove selected downloads, plus
  pause all, resume all and clear finished.
- **Per-download actions**: pause, resume, stop, try again, open file, show in folder,
  copy link, remove, and delete the partial file.
- **Networks page** showing what each connection carries: its byte range, bytes done
  and a per-stream completion bar.
- **Schedule page** listing waiting downloads with *Start now*.
- **Clipboard watch**: copied links are offered in the paste bar.
- **Settings persistence** for folder, segments per download, concurrency, speed limit,
  clipboard watch and the last 40 completed downloads.
- **Loopback API** (`/ping`, `/add`, `/jobs`) reporting queue and per-job telemetry, with
  CORS for the bundled Manifest V3 extension.

### Changed

- Renamed and rebranded the project as **Pookye Download Manager (PDM)** throughout the
  app, extension, settings folder (`%APPDATA%\PookyeDM`) and documentation.
- Reworked the engine into a manager that owns the queue, the rate limiter and job
  lifecycle instead of starting every download immediately.
- Split the UI into modules (`pages`, `dialog`, `widgets`, `actions`) so each part is
  independently readable.
- `/add` now answers `{ "ok": true, "queued": true }`; job ids come from `/jobs`.
- Version is now `0.1.0-alpha.1`.

### Fixed

- Settings from earlier builds load unchanged: new fields have defaults.
- Retrying a failed download resets its progress and reuses the chosen file name.
- A paused download that never started returns to the queue on resume instead of
  pretending to run.
- Per-stream bars and speed sampling no longer flicker when a job moves to *Queued*.

### Removed

- Historical references to the previous brand from the application and README; the
  repository now documents only Pookye DM and its QDM lineage.

### Known limitations

- Pause takes effect between 64 KiB reads.
- Partial progress is not persisted across app restarts.
- No video-site support (yt-dlp/HLS), tray icon, desktop notifications or auto-updater.
- No prebuilt binaries in this alpha.
