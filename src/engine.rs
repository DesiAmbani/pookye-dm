//! Segmented, multi-connection HTTP download engine.
//!
//! A job is split into byte ranges that are fetched in parallel, each range
//! written straight into its offset in one preallocated file. Progress is kept
//! in atomics so the UI can read it without locking the workers.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::blocking::{Client, Response};
use reqwest::header::RANGE;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const USER_AGENT_VALUE: &str = "PookyeDM/0.1 (+https://github.com/DesiAmbani/pookye-dm)";

const CHUNK: usize = 64 * 1024;
const MAX_RETRIES: u32 = 6;
const MIN_SEGMENT: u64 = 256 * 1024;

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
    pub segments: Mutex<Vec<Arc<Segment>>>,
    pub ranges: AtomicBool,
}

impl Progress {
    fn new() -> Self {
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

}

pub struct Job {
    pub id: u64,
    pub url: String,
    pub file_name: String,
    pub folder: PathBuf,
    pub is_video: bool,
    pub connections: usize,
    pub status: Status,
    pub error: Option<String>,
    pub progress: Arc<Progress>,
    pub speed: f64,
    pub peak: f64,
    pub started_at: Instant,
    pub finished_at: Option<u64>,
    /// The supervisor fills this in once the server has been asked for the name.
    pub name_slot: Arc<Mutex<Option<String>>>,
    sampled_bytes: u64,
    sampled_at: Instant,
    worker: Option<thread::JoinHandle<()>>,
}

pub fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(USER_AGENT_VALUE)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("could not start the HTTP client: {e}"))
}

fn header_text(response: &Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn name_from_disposition(value: &str) -> Option<String> {
    // filename*=UTF-8''name  |  filename="name"
    let lower = value.to_ascii_lowercase();
    let raw = if let Some(pos) = lower.find("filename*=") {
        let rest = &value[pos + 10..];
        let rest = rest.trim_matches(|c| c == '"' || c == '\'');
        rest.rsplit("''").next().unwrap_or(rest).to_string()
    } else if let Some(pos) = lower.find("filename=") {
        value[pos + 9..]
            .trim()
            .trim_matches('"')
            .split(';')
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        return None;
    };
    Some(percent_decode(&raw))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        "download".to_string()
    } else if cleaned.len() > 180 {
        cleaned.chars().take(180).collect()
    } else {
        cleaned
    }
}

fn name_from_url(url: &str) -> Option<String> {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let last = without_query.rsplit('/').next().unwrap_or("");
    if last.is_empty() {
        None
    } else {
        Some(sanitize_name(&percent_decode(last)))
    }
}

fn unique_path(folder: &Path, name: &str) -> PathBuf {
    let candidate = folder.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let ext = path
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    for n in 1..10_000 {
        let next = folder.join(format!("{stem} ({n}){ext}"));
        if !next.exists() {
            return next;
        }
    }
    folder.join(format!("{stem}-{}{ext}", now_secs()))
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

struct Probe {
    total: Option<u64>,
    ranges: bool,
    suggested: Option<String>,
    is_video: bool,
}

fn probe(client: &Client, url: &str) -> Result<Probe, String> {
    let response = client
        .get(url)
        .header(RANGE, "bytes=0-0")
        .send()
        .map_err(|e| format!("could not reach the server: {e}"))?;

    let status = response.status();
    let content_type = header_text(&response, "content-type")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let disposition = header_text(&response, "content-disposition");
    let suggested = disposition.and_then(|d| name_from_disposition(&d));
    let is_video = content_type.starts_with("video/") || content_type.starts_with("audio/");

    if status.as_u16() == 206 {
        let ranges_header = header_text(&response, "content-range").unwrap_or_default();
        let total = ranges_header
            .rsplit('/')
            .next()
            .and_then(|s| s.trim().parse::<u64>().ok());
        return Ok(Probe {
            total,
            ranges: true,
            suggested,
            is_video,
        });
    }

    if status.is_success() {
        let length = header_text(&response, "content-length").and_then(|s| s.parse::<u64>().ok());
        let accept_ranges = header_text(&response, "accept-ranges")
            .map(|v| v.to_ascii_lowercase().contains("bytes"))
            .unwrap_or(false);
        return Ok(Probe {
            total: length,
            ranges: accept_ranges,
            suggested,
            is_video,
        });
    }

    if status.as_u16() == 416 {
        return Ok(Probe {
            total: Some(0),
            ranges: false,
            suggested,
            is_video,
        });
    }

    Err(format!("the server answered HTTP {}", status.as_u16()))
}

fn set_error(progress: &Progress, message: String) {
    if let Ok(mut slot) = progress.outcome.lock() {
        if slot.is_none() {
            *slot = Some(Err(message));
        }
    }
}

fn worker(
    client: Client,
    url: String,
    path: PathBuf,
    segment: Arc<Segment>,
    progress: Arc<Progress>,
) {
    let mut failures = 0u32;
    loop {
        if progress.cancel.load(Ordering::Relaxed) {
            return;
        }
        while progress.pause.load(Ordering::Relaxed) {
            if progress.cancel.load(Ordering::Relaxed) {
                return;
            }
            thread::sleep(Duration::from_millis(120));
        }

        let mut pos = segment.start + segment.done.load(Ordering::Relaxed);
        if !segment.unbounded && pos > segment.end {
            return;
        }

        let last = if segment.unbounded {
            String::new()
        } else {
            segment.end.to_string()
        };
        let range = format!("bytes={pos}-{last}");
        let response = match client.get(&url).header(RANGE, range).send() {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                failures += 1;
                if failures > MAX_RETRIES {
                    set_error(&progress, format!("the server kept answering HTTP {:?}", r.status().as_u16()));
                    return;
                }
                thread::sleep(Duration::from_millis(400 * failures.min(10) as u64));
                continue;
            }
            Err(e) => {
                failures += 1;
                if failures > MAX_RETRIES {
                    set_error(&progress, format!("connection failed: {e}"));
                    return;
                }
                thread::sleep(Duration::from_millis(400 * failures.min(10) as u64));
                continue;
            }
        };

        let mut file = match OpenOptions::new().write(true).open(&path) {
            Ok(f) => f,
            Err(e) => {
                set_error(&progress, format!("could not write the file: {e}"));
                return;
            }
        };
        if let Err(e) = file.seek(SeekFrom::Start(pos)) {
            set_error(&progress, format!("could not seek in the file: {e}"));
            return;
        }

        let mut response = response;
        let mut buffer = vec![0u8; CHUNK];
        let mut progressed = false;
        let mut finished_stream = false;

        loop {
            if progress.cancel.load(Ordering::Relaxed) || progress.pause.load(Ordering::Relaxed) {
                break;
            }
            let remaining = segment.end.saturating_sub(pos).saturating_add(1);
            let want = if segment.unbounded {
                CHUNK
            } else {
                remaining.min(CHUNK as u64) as usize
            };
            if want == 0 {
                finished_stream = true;
                break;
            }
            match response.read(&mut buffer[..want]) {
                Ok(0) => {
                    finished_stream = true;
                    break;
                }
                Ok(n) => {
                    if let Err(e) = file.write_all(&buffer[..n]) {
                        set_error(&progress, format!("could not write the file: {e}"));
                        return;
                    }
                    pos += n as u64;
                    segment.done.fetch_add(n as u64, Ordering::Relaxed);
                    progress.bytes.fetch_add(n as u64, Ordering::Relaxed);
                    progressed = true;
                }
                Err(e) => {
                    failures += 1;
                    if failures > MAX_RETRIES {
                        set_error(&progress, format!("transfer stalled: {e}"));
                        return;
                    }
                    break;
                }
            }
        }

        if segment.unbounded {
            if finished_stream {
                return;
            }
        } else if pos > segment.end {
            return;
        }

        if progressed {
            failures = 0;
        } else {
            failures += 1;
            if failures > MAX_RETRIES {
                set_error(&progress, "the download kept failing without progress".to_string());
                return;
            }
            thread::sleep(Duration::from_millis(400 * failures.min(10) as u64));
        }
    }
}

fn supervisor(
    client: Client,
    url: String,
    folder: PathBuf,
    requested: Option<String>,
    connections: usize,
    progress: Arc<Progress>,
    file_name_slot: Arc<Mutex<Option<String>>>,
    fallback_name: String,
) {
    let result = (|| -> Result<(), String> {
        let info = probe(&client, &url)?;
        progress.ranges.store(info.ranges, Ordering::Relaxed);

        let mut name = requested
            .as_deref()
            .map(sanitize_name)
            .filter(|n| !n.is_empty())
            .or_else(|| info.suggested.map(|n| sanitize_name(&n)))
            .or_else(|| name_from_url(&url))
            .unwrap_or_else(|| fallback_name.clone());

        if !name.contains('.') && info.is_video {
            name.push_str(".mp4");
        }
        if let Ok(mut slot) = file_name_slot.lock() {
            *slot = Some(name.clone());
        }

        let total = info.total.unwrap_or(0);
        progress.total.store(total, Ordering::Relaxed);

        std::fs::create_dir_all(&folder)
            .map_err(|e| format!("could not create {}: {e}", folder.display()))?;
        let path = unique_path(&folder, &name);
        if let Some(final_name) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
            if let Ok(mut slot) = file_name_slot.lock() {
                *slot = Some(final_name);
            }
        }

        let file = File::create(&path).map_err(|e| format!("could not create the file: {e}"))?;
        if total > 0 {
            let _ = file.set_len(total);
        }
        drop(file);

        // Decide how to split. Never create segments smaller than MIN_SEGMENT.
        let mut segments = Vec::new();
        if total > 0 && info.ranges && connections > 1 {
            let wanted = (total / MIN_SEGMENT).max(1) as usize;
            let count = connections.min(wanted).max(1);
            let size = total / count as u64;
            for index in 0..count {
                let start = index as u64 * size;
                let end = if index + 1 == count { total - 1 } else { start + size - 1 };
                segments.push(Arc::new(Segment {
                    start,
                    end,
                    done: AtomicU64::new(0),
                    unbounded: false,
                }));
            }
        } else if total > 0 {
            segments.push(Arc::new(Segment {
                start: 0,
                end: total - 1,
                done: AtomicU64::new(0),
                unbounded: false,
            }));
        } else {
            segments.push(Arc::new(Segment {
                start: 0,
                end: u64::MAX - 1,
                done: AtomicU64::new(0),
                unbounded: true,
            }));
        }

        if let Ok(mut slot) = progress.segments.lock() {
            *slot = segments.clone();
        }

        let mut handles = Vec::new();
        for segment in segments {
            let client = client.clone();
            let url = url.clone();
            let path = path.clone();
            let progress = progress.clone();
            handles.push(thread::spawn(move || {
                worker(client, url, path, segment, progress)
            }));
        }
        for handle in handles {
            let _ = handle.join();
        }

        if progress.cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Ok(slot) = progress.outcome.lock() {
            if let Some(Err(message)) = &*slot {
                return Err(message.clone());
            }
        }
        Ok(())
    })();

    if !progress.cancel.load(Ordering::Relaxed) && result.is_err() {
        set_error(&progress, result.err().unwrap_or_default());
    }
    progress.finished.store(true, Ordering::Relaxed);
}

pub struct Manager {
    pub jobs: Vec<Job>,
    next_id: u64,
    client: Option<Client>,
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

impl Manager {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            next_id: 1,
            client: client().ok(),
        }
    }

    pub fn add(
        &mut self,
        url: String,
        requested_name: Option<String>,
        folder: &Path,
        connections: usize,
    ) -> Result<u64, String> {
        let url = url.trim().to_string();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("That isn't a web link. Links start with http:// or https://.".to_string());
        }
        let client = self
            .client
            .clone()
            .ok_or_else(|| "The HTTP client isn't available.".to_string())?;

        let id = self.next_id;
        self.next_id += 1;

        let progress = Arc::new(Progress::new());
        let name_slot = Arc::new(Mutex::new(None::<String>));
        let folder = folder.to_path_buf();
        let connections = connections.clamp(1, 32);
        let worker_progress = progress.clone();
        let slot = name_slot.clone();
        let worker_url = url.clone();
        let worker_folder = folder.clone();

        let handle = thread::spawn(move || {
            supervisor(
                client,
                worker_url,
                worker_folder,
                requested_name,
                connections,
                worker_progress,
                slot,
                "download".to_string(),
            )
        });

        let name = name_slot
            .lock()
            .ok()
            .and_then(|n| n.clone())
            .unwrap_or_else(|| {
                name_from_url(&url).unwrap_or_else(|| "download".to_string())
            });

        self.jobs.push(Job {
            id,
            url,
            file_name: name,
            folder,
            is_video: false,
            connections,
            status: Status::Downloading,
            error: None,
            progress,
            speed: 0.0,
            peak: 0.0,
            started_at: Instant::now(),
            finished_at: None,
            name_slot,
            sampled_bytes: 0,
            sampled_at: Instant::now(),
            worker: Some(handle),
        });
        Ok(id)
    }

    pub fn pause(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if job.status == Status::Downloading || job.status == Status::Queued {
                job.progress.pause.store(true, Ordering::Relaxed);
                job.status = Status::Paused;
                job.speed = 0.0;
            }
        }
    }

    pub fn resume(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if job.status == Status::Paused {
                job.progress.pause.store(false, Ordering::Relaxed);
                job.status = Status::Downloading;
                job.sampled_at = Instant::now();
                job.sampled_bytes = job.progress.bytes.load(Ordering::Relaxed);
            }
        }
    }

    pub fn cancel(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if job.status.is_active() {
                job.progress.cancel.store(true, Ordering::Relaxed);
                job.progress.pause.store(false, Ordering::Relaxed);
                job.status = Status::Cancelled;
                job.speed = 0.0;
            }
        }
    }

    /// Refresh statuses and speed samples. Call once per frame.
    pub fn tick(&mut self) -> Vec<Job> {
        let mut completed = Vec::new();
        for job in self.jobs.iter_mut() {
            let total = job.progress.total.load(Ordering::Relaxed);
            let bytes = job.progress.bytes.load(Ordering::Relaxed);

            if job.status == Status::Downloading {
                let elapsed = job.sampled_at.elapsed().as_secs_f64();
                if elapsed >= 0.25 {
                    let delta = bytes.saturating_sub(job.sampled_bytes) as f64;
                    job.speed = delta / elapsed;
                    job.peak = job.peak.max(job.speed);
                    job.sampled_bytes = bytes;
                    job.sampled_at = Instant::now();
                }
            } else {
                job.speed = 0.0;
            }

            if job.progress.finished.load(Ordering::Relaxed)
                && matches!(job.status, Status::Downloading | Status::Paused)
            {
                job.status = match job.progress.outcome.lock().ok().and_then(|o| o.clone()) {
                    None => {
                        if total == 0 || bytes >= total {
                            Status::Completed
                        } else {
                            Status::Failed
                        }
                    }
                    Some(Ok(())) => Status::Completed,
                    Some(Err(message)) => {
                        job.error = Some(message);
                        Status::Failed
                    }
                };
                if job.status == Status::Completed {
                    job.finished_at = Some(now_secs());
                    completed.push(job.clone_for_history());
                }
                job.speed = 0.0;
                if let Some(handle) = job.worker.take() {
                    let _ = handle.join();
                }
            }
        }
        completed
    }

    pub fn total_speed(&self) -> f64 {
        self.jobs
            .iter()
            .filter(|j| j.status == Status::Downloading)
            .map(|j| j.speed)
            .sum()
    }

    pub fn active_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.is_active()).count()
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if job.status.is_active() {
                return;
            }
        }
        if let Some(position) = self.jobs.iter().position(|j| j.id == id) {
            self.jobs.remove(position);
        }
    }

    /// Compact JSON view of the queue, served by the local API.
    pub fn snapshot_json(&self) -> String {
        let jobs: Vec<serde_json::Value> = self
            .jobs
            .iter()
            .map(|job| {
                let total = job.progress.total.load(Ordering::Relaxed);
                serde_json::json!({
                    "id": job.id,
                    "url": job.url,
                    "fileName": job.display_name(),
                    "status": job.status.label().to_lowercase(),
                    "totalBytes": total,
                    "bytesDownloaded": job.progress.bytes.load(Ordering::Relaxed),
                    "speedBps": job.speed,
                    "connections": job.connections,
                    "folder": job.folder.to_string_lossy(),
                    "error": job.error,
                })
            })
            .collect();
        serde_json::json!({ "app": "pookye", "api": 1, "jobs": jobs }).to_string()
    }
}

impl Job {
    fn clone_for_history(&self) -> Job {
        Job {
            id: self.id,
            url: self.url.clone(),
            file_name: self.file_name.clone(),
            folder: self.folder.clone(),
            is_video: self.is_video,
            connections: self.connections,
            status: self.status,
            error: self.error.clone(),
            progress: self.progress.clone(),
            speed: 0.0,
            peak: self.peak,
            started_at: self.started_at,
            finished_at: self.finished_at,
            name_slot: self.name_slot.clone(),
            sampled_bytes: 0,
            sampled_at: Instant::now(),
            worker: None,
        }
    }

    /// The name the file actually got on disk, once the supervisor knows it.
    pub fn display_name(&self) -> String {
        self.name_slot
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_else(|| self.file_name.clone())
    }

    pub fn percent(&self) -> Option<f32> {
        let total = self.progress.total.load(Ordering::Relaxed);
        if total == 0 {
            return None;
        }
        let bytes = self.progress.bytes.load(Ordering::Relaxed);
        Some((bytes.min(total) as f32 / total as f32).clamp(0.0, 1.0))
    }

    pub fn seconds_left(&self) -> Option<f64> {
        let total = self.progress.total.load(Ordering::Relaxed);
        let bytes = self.progress.bytes.load(Ordering::Relaxed);
        if total == 0 || self.speed <= 1.0 || bytes >= total {
            return None;
        }
        Some((total - bytes) as f64 / self.speed)
    }
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
