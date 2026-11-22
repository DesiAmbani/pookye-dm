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
