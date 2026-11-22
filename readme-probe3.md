## Local API

Loopback only, JSON, CORS open for extension clients:

| Method | Path | Body | Answer |
| --- | --- | --- | --- |
| `GET` | `/ping` | — | `{ "app": "pookye", "api": 1, "version": "0.1.0-alpha.1", "name": "Pookye Download Manager" }` |
| `POST` | `/add` | `{ "url": "...", "fileName": "optional.bin" }` | `{ "ok": true, "queued": true }` |
| `GET` | `/jobs` | — | `{ "app": "pookye", "api": 1, "averageSpeedBps": 0, "active": 0, "queued": 0, "jobs": [ ... ] }` |

Each job carries `id`, `url`, `fileName`, `category`, `status`, `totalBytes`,
`bytesDownloaded`, `speedBps`, `connections`, `folder` and `error`.

## Brand palette

Sampled from the logo and applied throughout the UI:

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
