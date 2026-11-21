<div align="center">

<img src="assets/logo.png" alt="Pookye Download Manager" width="180" />

# Pookye Download Manager

**PDM · Pookye DM** — a fast, segmented download manager written in Rust.

Downloads are split into byte ranges and fetched over several connections at once,
so a single file finishes faster than the browser can manage on one stream.

<sub>Rust · egui · reqwest</sub>

</div>

---

## What it does

- **Segmented downloads** — every file is split into up to 32 byte ranges, each fetched
  in parallel and written straight to its offset in the destination file.
- **Resume and pause** — ranges keep their own progress; pausing stops the readers,
  resuming continues from each range's last byte instead of starting over.
- **Live telemetry** — total speed, per-connection mini-bars, bytes done, percentage and ETA.
- **Browser handoff** — a small MV3 extension forwards Chrome/Edge downloads and
  right-clicked links to the app over a loopback API.
- **Server-aware** — if a server does not support `Range` requests, the job falls back to a
  single stream instead of failing.
- **Remembers itself** — download folder, connection count and recent history persist in
  `%APPDATA%\PookyeDM\settings.json`.
- **Brand-native UI** — the whole interface is built from the logo's maroon-and-crimson palette.

## Build from source

Requirements: Rust 1.75 or newer (`rustup`), plus a C toolchain for the `windows-gnu` or
`windows-msvc` target.

```bash
git clone https://github.com/DesiAmbani/pookye-dm.git
cd pookye-dm
cargo build --release
./target/release/pookye-dm
```

The resulting binary is self-contained: the logo is embedded at compile time.

## Using it

1. Paste a link into **New download** and press **Download** (or Enter).
2. Pick a folder and the number of connections (1–32).
3. Watch the block bar fill and the per-connection tracks race each other.
4. Pause, resume, stop, or open the folder from the job card.

## Browser extension

`extension/` is an unpacked Manifest V3 extension:

1. Open `chrome://extensions`, enable **Developer mode**.
2. Choose **Load unpacked** and select the `extension` folder.
3. Start Pookye DM. Downloads are now taken over by it; a right-click on any link also
   offers *Download with Pookye DM*.

The extension talks to the app on `127.0.0.1:19330` and falls back to `19331`/`19332` if
that port is taken.

## Local API

The app serves a tiny JSON API on loopback only:

| Method | Path | Body | Answer |
| --- | --- | --- | --- |
| `GET` | `/ping` | — | `{ "app": "pookye", "api": 1, "version": "0.1.0" }` |
| `POST` | `/add` | `{ "url": "...", "fileName": "optional.bin" }` | `{ "ok": true, "id": 1 }` |
| `GET` | `/jobs` | — | `{ "app": "pookye", "api": 1, "jobs": [ ... ] }` |

CORS is open for `http://127.0.0.1` clients so an extension service worker can post to it.

```bash
curl -s http://127.0.0.1:19330/ping
curl -s -X POST http://127.0.0.1:19330/add \
  -H "Content-Type: application/json" \
  -d '{"url":"https://download.blender.org/release/Blender4.2/blender-4.2.3-windows-x64.msi"}'
```

## Brand palette

Sampled from `Logo.png` and applied throughout the UI:

| Role | Colour |
| --- | --- |
| Window background (logo's outer field) | `#330114` |
| Panels | `#4A021E` |
| Cards | `#5B0325` |
| Progress track | `#2A0110` |
| Crimson (logo body) | `#EA0159` |
| Hot pink highlight | `#FF2D7A` |
| Pale facet / primary text | `#FFE9F0` |
| Secondary text | `#C292A6` |
| Keyline | `#140006` |

## Project layout

```
src/main.rs      egui application: header, add bar, job cards, footer
src/engine.rs    segmented HTTP engine, probe, workers, speed sampling, formatting
src/api.rs       loopback JSON API (ping / add / jobs)
src/store.rs     settings + history persistence
src/brand.rs     palette and embedded logo
assets/logo.png  brand mark, also embedded into the binary
extension/       Manifest V3 browser handoff extension
```

## Where this comes from

This project is a clean-room Rust rewrite built after reverse engineering the Windows
installer `QuantumDM-Setup-x64.exe` (Quantum Download Manager 0.2.14, Inno Setup 6.7.0).
The installer was unpacked with innounp (Inno 6.7 is too new for innoextract 1.9), which
revealed:

- `quantumdm.exe` — a Rust binary (Slint 1.18 GUI, tokio, tray-icon) that splits a download
  across several network connections, with a `quantumdm-system.exe` helper and `wintun.dll`
  for additional interfaces.
- `extension/` — an MV3 extension that intercepts browser downloads and posts them to a
  loopback API (`/ping`, `/add`, `/hold`, `/confirm`, `/inspect`, `/begin`, `/job`,
  `/job/action`, `/pick-folder`, ports 19320–19322, API version 2, client header
  `X-QuantumDM-Client`).
- Bundled `deno.exe`, `ffmpeg.exe`, `yt-dlp.exe` for video grabbing, plus a licensing and
  auto-update client pointing at a private release feed.

Pookye DM keeps the parts that make the idea useful — range splitting across parallel
connections, pause/resume, browser handoff, live per-connection telemetry — and leaves out
the licensing, updater, VPN helper and video tooling. The implementation here is original:
the engine, the UI, the local API and the extension were written from scratch in Rust and
vanilla JavaScript.

## Limitations

- Pausing is honoured between 64 KiB reads, so it is not instantaneous mid-chunk.
- Progress is resumed within a session; a restarted app starts new jobs rather than
  continuing an interrupted partial file.
- Servers without `Range` support get one connection and no resume.
- No authentication, no proxy multiplexing, no video site support (yet).

## License

MIT — see [LICENSE](LICENSE).
