//! Segmented, multi-connection HTTP download engine.
//!
//! A job is split into byte ranges that are fetched in parallel, each range
//! written straight into its offset in one preallocated file. Progress is kept
//! in atomics so the UI can read it without locking the workers.
//!
//! A manager also owns the queue: only `max_concurrent` jobs download at once,
//! and every worker passes its bytes through one shared rate limiter so a
//! system-wide speed cap applies to all connections.

mod download;
mod manager;
mod video;

pub use download::{proxy_is_valid, ProbeInfo};
pub use manager::{Job, Manager};
pub use video::{find_tools, is_video_url, INSTALL_HINT};

/// Asks about a link: video pages go through yt-dlp, everything else is a HEAD.
pub fn probe(url: &str, proxy: Option<&str>) -> Result<ProbeInfo, String> {
    let url = url.trim();
    if video::is_video_url(url) {
        return video::probe(url);
    }
    download::probe(url, proxy)
}

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USER_AGENT_VALUE: &str = concat!(
    "PookyeDM/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/DesiAmbani/pookye-dm)"
);

pub(crate) const CHUNK: usize = 64 * 1024;
pub(crate) const MAX_RETRIES: u32 = 6;
pub(crate) const MIN_SEGMENT: u64 = 256 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Queued => "Queued",
            Status::Downloading => "Downloading",
            Status::Paused => "Paused",
            Status::Completed => "Complete",
            Status::Failed => "Stopped",
            Status::Cancelled => "Cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Status::Queued | Status::Downloading | Status::Paused)
    }

    pub fn is_finished(self) -> bool {
        matches!(self, Status::Completed | Status::Failed | Status::Cancelled)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Videos,
    Music,
    Documents,
    Programs,
    Archives,
    Other,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Videos => "Videos",
            Category::Music => "Music",
            Category::Documents => "Documents",
            Category::Programs => "Programs",
            Category::Archives => "Archives",
            Category::Other => "Other",
        }
    }

    pub fn of(name: &str) -> Self {
        let ext = name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v" | "flv" | "wmv" | "mpg" | "mpeg"
            | "ts" | "m2ts" => Category::Videos,
            "mp3" | "m4a" | "flac" | "wav" | "ogg" | "opus" | "aac" | "wma" => Category::Music,
            "pdf" | "doc" | "docx" | "txt" | "md" | "epub" | "rtf" | "odt" | "xls" | "xlsx"
            | "ppt" | "pptx" | "csv" => Category::Documents,
            "exe" | "msi" | "apk" | "dmg" | "pkg" | "deb" | "rpm" | "appimage" | "bat" | "sh" => {
                Category::Programs
            }
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "zst" | "iso" => {
                Category::Archives
            }
            _ => Category::Other,
        }
    }
}

/// Browser context handed over with a download: its cookies, referer and agent.
///
/// Sites that only serve a file to a signed-in session need the cookies the
/// browser already holds; passing the referer and agent too keeps hotlink
/// protection and server-side checks happy.
#[derive(Clone, Debug, Default)]
pub struct RequestContext {
    pub cookies: Option<String>,
    pub referer: Option<String>,
    pub user_agent: Option<String>,
}

/// One byte range of a job. `done` counts bytes written inside the range.
pub struct Segment {
    pub start: u64,
    pub end: u64,
    pub done: AtomicU64,
    pub unbounded: bool,
}

impl Segment {
    pub fn length(&self) -> u64 {
        self.end.saturating_sub(self.start).saturating_add(1)
    }
}

/// Everything the workers share with the UI.
pub struct Progress {
    pub total: AtomicU64,
    pub bytes: AtomicU64,
    pub cancel: AtomicBool,
    pub pause: AtomicBool,
    pub finished: AtomicBool,
    pub outcome: Mutex<Option<Result<(), String>>>,
    pub segments: Mutex<Vec<std::sync::Arc<Segment>>>,
    pub ranges: AtomicBool,
}

impl Progress {
    pub(crate) fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            cancel: AtomicBool::new(false),
            pause: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            outcome: Mutex::new(None),
            segments: Mutex::new(Vec::new()),
            ranges: AtomicBool::new(false),
        }
    }

    pub fn snapshot_segments(&self) -> Vec<(u64, u64, u64)> {
        match self.segments.lock() {
            Ok(segments) => segments
                .iter()
                .map(|s| {
                    let done = s.done.load(Ordering::Relaxed);
                    let len = if s.unbounded { done.max(1) } else { s.length() };
                    (len, done.min(len), s.start)
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }
}

/// One shared bandwidth budget for every worker in every job.
pub struct Limiter {
    state: Mutex<LimitState>,
}

struct LimitState {
    next: Instant,
    rate: f64,
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Limiter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(LimitState {
                next: Instant::now(),
                rate: 0.0,
            }),
        }
    }

    /// Bytes per second; zero or less means unlimited.
    pub fn set_rate(&self, bytes_per_second: f64) {
        if let Ok(mut state) = self.state.lock() {
            state.rate = bytes_per_second.max(0.0);
            state.next = Instant::now();
        }
    }

    pub(crate) fn consume(&self, bytes: u64, cancel: &AtomicBool) {
        let wait = {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.rate <= 0.0 {
                state.next = Instant::now();
                return;
            }
            let now = Instant::now();
            if state.next < now {
                state.next = now;
            }
            let rate = state.rate;
            state.next += Duration::from_secs_f64(bytes as f64 / rate);
            state.next.saturating_duration_since(now)
        };
        let mut left = wait;
        while left > Duration::ZERO {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let slice = left.min(Duration::from_millis(80));
            thread::sleep(slice);
            left -= slice;
        }
    }
}

pub(crate) fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 || value >= 100.0 {
        format!("{:.0} {}", value, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

pub fn format_speed(bytes_per_second: f64) -> String {
    if bytes_per_second <= 0.0 {
        return "0 B/s".to_string();
    }
    format!("{}/s", format_bytes(bytes_per_second.round() as u64))
}

pub fn format_eta(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return String::new();
    }
    if seconds < 60.0 {
        format!("{}s left", seconds.ceil() as u64)
    } else if seconds < 3600.0 {
        format!("{}m left", (seconds / 60.0).ceil() as u64)
    } else {
        format!("{:.1}h left", seconds / 3600.0)
    }
}

pub fn format_percent(value: Option<f32>) -> String {
    match value {
        Some(fraction) => format!("{}%", (fraction * 100.0).round() as u64),
        None => "—".to_string(),
    }
}
