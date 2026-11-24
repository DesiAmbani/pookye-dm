//! Job records and the manager that owns the queue and the rate limiter.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use reqwest::blocking::Client;

use super::download::{client, name_from_url, sanitize_name, supervisor};
use super::video::{find_tools, is_video_url, INSTALL_HINT, supervisor as video_supervisor};
use super::{now_secs, Category, Limiter, Progress, RequestContext, Status};

pub struct Job {
    pub id: u64,
    pub url: String,
    pub file_name: String,
    pub folder: PathBuf,
    pub category: Category,
    pub connections: usize,
    pub status: Status,
    pub error: Option<String>,
    pub progress: Arc<Progress>,
    pub speed: f64,
    pub peak: f64,
    pub started_at: Instant,
    pub finished_at: Option<u64>,
    /// The name the user chose, kept so a retry reuses it.
    pub requested_name: Option<String>,
    /// The supervisor fills this in once the server has been asked for the name.
    pub name_slot: Arc<Mutex<Option<String>>>,
    /// Browser cookies, referer and agent to replay with every request.
    pub context: RequestContext,
    /// A video page that yt-dlp has to resolve instead of a plain file.
    pub is_video: bool,
    pub started: bool,
    sampled_bytes: u64,
    sampled_at: Instant,
    worker: Option<thread::JoinHandle<()>>,
}

pub struct Manager {
    pub jobs: Vec<Job>,
    next_id: u64,
    client: Option<Client>,
    proxy: Option<String>,
    pub max_concurrent: usize,
    /// False while a schedule window is shut; queued jobs then stay queued.
    allowed: bool,
    limiter: Arc<Limiter>,
}

impl Manager {
    pub fn new(max_concurrent: usize, speed_limit_kbps: u64) -> Self {
        let limiter = Arc::new(Limiter::new());
        limiter.set_rate(speed_limit_kbps as f64 * 1024.0);
        Self {
            jobs: Vec::new(),
            next_id: 1,
            client: client(None).ok(),
            proxy: None,
            max_concurrent: max_concurrent.clamp(1, 10),
            allowed: true,
            limiter,
        }
    }

    /// Sets the proxy used by downloads started from now on. Running jobs keep
    /// the connection they were built with, like the original's "next downloads".
    pub fn set_proxy(&mut self, proxy: Option<String>) {
        if self.proxy == proxy {
            return;
        }
        self.client = client(proxy.as_deref()).ok();
        self.proxy = proxy;
    }

    /// Opens or shuts the start gate used by the weekly schedule.
    pub fn set_allowed(&mut self, allowed: bool) {
        if self.allowed == allowed {
            return;
        }
        self.allowed = allowed;
        self.pump();
    }

    pub fn allowed(&self) -> bool {
        self.allowed
    }

    pub fn set_max_concurrent(&mut self, value: usize) {
        self.max_concurrent = value.clamp(1, 10);
        self.pump();
    }

    pub fn set_speed_limit(&self, kbps: u64) {
        self.limiter.set_rate(kbps as f64 * 1024.0);
    }

    pub fn add(
        &mut self,
        url: String,
        requested_name: Option<String>,
        folder: &Path,
        connections: usize,
        context: RequestContext,
    ) -> Result<u64, String> {
        let url = url.trim().to_string();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("That isn't a web link. Links start with http:// or https://.".to_string());
        }
        // Video pages need both tools; say the install command instead of failing later.
        let is_video = is_video_url(&url);
        if is_video && find_tools().is_none() {
            return Err(INSTALL_HINT.to_string());
        }
        if self.client.is_none() && !is_video {
            return Err("The HTTP client isn't available.".to_string());
        }

        let id = self.next_id;
        self.next_id += 1;

        let requested = requested_name
            .map(|name| sanitize_name(&name))
            .filter(|name| !name.is_empty());
        let name = requested
            .clone()
            .or_else(|| name_from_url(&url))
            .unwrap_or_else(|| "download".to_string());
        // A page has no file name yet; yt-dlp supplies the title once it runs.
        let name = if is_video && requested.is_none() {
            "video.mp4".to_string()
        } else {
            name
        };

        self.jobs.push(Job {
            id,
            url,
            category: Category::of(&name),
            file_name: name,
            folder: folder.to_path_buf(),
            connections: connections.clamp(1, 32),
            status: Status::Queued,
            error: None,
            progress: Arc::new(Progress::new()),
            speed: 0.0,
            peak: 0.0,
            started_at: Instant::now(),
            finished_at: None,
            requested_name: requested,
            name_slot: Arc::new(Mutex::new(None)),
            context,
            is_video,
            started: false,
            sampled_bytes: 0,
            sampled_at: Instant::now(),
            worker: None,
        });
        self.pump();
        Ok(id)
    }

    fn start(&mut self, index: usize) {
        if self.jobs[index].started || self.jobs[index].status != Status::Queued {
            return;
        }
        let is_video = self.jobs[index].is_video;
        if is_video && find_tools().is_none() {
            let job = &mut self.jobs[index];
            job.status = Status::Failed;
            job.error = Some(INSTALL_HINT.to_string());
            return;
        }
        let client = if is_video { None } else { self.client.clone() };
        if !is_video && client.is_none() {
            let job = &mut self.jobs[index];
            job.status = Status::Failed;
            job.error = Some("The HTTP client isn't available.".to_string());
            return;
        }
        let job = &mut self.jobs[index];
        job.started = true;
        job.status = Status::Downloading;
        job.error = None;
        job.started_at = Instant::now();
        job.sampled_at = Instant::now();
        job.sampled_bytes = 0;
        job.speed = 0.0;

        let progress = job.progress.clone();
        let slot = job.name_slot.clone();
        let url = job.url.clone();
        let folder = job.folder.clone();
        let requested = job.requested_name.clone();
        let handle = if is_video {
            thread::spawn(move || video_supervisor(url, folder, requested, progress, slot))
        } else {
            let connections = job.connections;
            let context = job.context.clone();
            let limiter = self.limiter.clone();
            let client = client.expect("client checked above");
            thread::spawn(move || {
                supervisor(
                    client, url, folder, requested, connections, context, progress, slot, limiter,
                )
            })
        };
        job.worker = Some(handle);
    }

    /// Starts queued downloads while there is room under the concurrency limit.
    pub fn pump(&mut self) {
        if !self.allowed {
            return;
        }
        let mut active = self
            .jobs
            .iter()
            .filter(|job| job.status == Status::Downloading)
            .count();
        for index in 0..self.jobs.len() {
            if active >= self.max_concurrent {
                break;
            }
            if self.jobs[index].status == Status::Queued && !self.jobs[index].started {
                self.start(index);
                active += 1;
            }
        }
    }

    pub fn pause(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if job.status == Status::Downloading || job.status == Status::Queued {
                if job.is_video {
                    // yt-dlp cannot be paused midway, so stop it and start over on resume.
                    job.progress.cancel.store(true, Ordering::Relaxed);
                    job.progress = Arc::new(Progress::new());
                } else {
                    job.progress.pause.store(true, Ordering::Relaxed);
                }
                job.status = Status::Paused;
                job.speed = 0.0;
            }
        }
    }

    pub fn resume(&mut self, id: u64) {
        let Some(index) = self.jobs.iter().position(|job| job.id == id) else {
            return;
        };
        if self.jobs[index].status != Status::Paused {
            return;
        }
        let video = self.jobs[index].is_video;
        if video {
            // The stopped run left no usable state, so queue it again.
            let job = &mut self.jobs[index];
            job.started = false;
            job.error = None;
            job.status = Status::Queued;
            job.sampled_at = Instant::now();
            job.sampled_bytes = 0;
            self.pump();
            return;
        }
        let job = &mut self.jobs[index];
        job.progress.pause.store(false, Ordering::Relaxed);
        if job.started {
            job.status = Status::Downloading;
            job.sampled_at = Instant::now();
            job.sampled_bytes = job.progress.bytes();
        } else {
            job.status = Status::Queued;
        }
        self.pump();
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
        self.pump();
    }

    pub fn pause_all(&mut self) {
        let ids: Vec<u64> = self
            .jobs
            .iter()
            .filter(|job| matches!(job.status, Status::Downloading | Status::Queued))
            .map(|job| job.id)
            .collect();
        for id in ids {
            self.pause(id);
        }
    }

    pub fn resume_all(&mut self) {
        let ids: Vec<u64> = self
            .jobs
            .iter()
            .filter(|job| job.status == Status::Paused)
            .map(|job| job.id)
            .collect();
        for id in ids {
            self.resume(id);
        }
    }

    pub fn retry(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            if !matches!(job.status, Status::Failed | Status::Cancelled) {
                return;
            }
            job.progress = Arc::new(Progress::new());
            job.name_slot = Arc::new(Mutex::new(None));
            job.error = None;
            job.started = false;
            job.speed = 0.0;
            job.finished_at = None;
            job.status = Status::Queued;
            job.file_name = job
                .requested_name
                .clone()
                .or_else(|| name_from_url(&job.url))
                .unwrap_or_else(|| "download".to_string());
            job.is_video = is_video_url(&job.url);
            if job.is_video && job.requested_name.is_none() {
                job.file_name = "video.mp4".to_string();
            }
            job.category = Category::of(&job.file_name);
        }
        self.pump();
    }

    /// Refresh statuses and speed samples. Call once per frame.
    pub fn tick(&mut self) -> Vec<Job> {
        let mut completed = Vec::new();
        for job in self.jobs.iter_mut() {
            let total = job.progress.total();
            let bytes = job.progress.bytes();

            let resolved = job.name_slot.lock().ok().and_then(|slot| slot.clone());
            if let Some(name) = resolved {
                if job.file_name != name {
                    job.category = Category::of(&name);
                    job.file_name = name;
                }
            }

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
        self.pump();
        completed
    }

    pub fn total_speed(&self) -> f64 {
        // Folded from positive zero: `f64`'s `Sum` identity is `-0.0`, which an
        // idle queue would hand to the API and paint as a negative speed.
        self.jobs
            .iter()
            .filter(|j| j.status == Status::Downloading)
            .fold(0.0, |total, job| total + job.speed)
    }

    pub fn total_downloaded(&self) -> u64 {
        self.jobs.iter().map(|job| job.progress.bytes()).sum()
    }

    pub fn active_count(&self) -> usize {
        self.jobs.iter().filter(|j| j.status.is_active()).count()
    }

    pub fn downloading_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == Status::Downloading)
            .count()
    }

    pub fn queued_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == Status::Queued)
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|j| j.status == Status::Completed)
            .count()
    }

    pub fn clear_finished(&mut self) {
        self.jobs.retain(|job| !job.status.is_finished());
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
            if job.status.is_active() {
                return;
            }
        }
        if let Some(position) = self.jobs.iter().position(|j| j.id == id) {
            self.jobs.remove(position);
        }
    }

    /// Removes a finished job and deletes the partial file it wrote.
    pub fn remove_with_file(&mut self, id: u64) {
        let path = self
            .jobs
            .iter()
            .find(|job| job.id == id)
            .filter(|job| !job.status.is_active())
            .map(|job| job.path());
        if let Some(path) = path {
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
            self.remove(id);
        }
    }

    /// Compact JSON view of the queue, served by the local API.
    pub fn snapshot_json(&self) -> String {
        let jobs: Vec<serde_json::Value> = self
            .jobs
            .iter()
            .map(|job| {
                let total = job.progress.total();
                serde_json::json!({
                    "id": job.id,
                    "url": job.url,
                    "fileName": job.display_name(),
                    "category": job.category.label(),
                    "status": job.status.label().to_lowercase(),
                    "totalBytes": total,
                    "bytesDownloaded": job.progress.bytes(),
                    "speedBps": job.speed,
                    "connections": job.connections,
                    "folder": job.folder.to_string_lossy(),
                    "error": job.error,
                })
            })
            .collect();
        serde_json::json!({
            "app": "pookye",
            "api": 1,
            "version": super::VERSION,
            "averageSpeedBps": self.total_speed(),
            "active": self.active_count(),
            "queued": self.queued_count(),
            "jobs": jobs
        })
        .to_string()
    }
}

impl Job {
    fn clone_for_history(&self) -> Job {
        Job {
            id: self.id,
            url: self.url.clone(),
            file_name: self.file_name.clone(),
            folder: self.folder.clone(),
            category: self.category,
            connections: self.connections,
            status: self.status,
            error: self.error.clone(),
            progress: self.progress.clone(),
            speed: 0.0,
            peak: self.peak,
            started_at: self.started_at,
            finished_at: self.finished_at,
            requested_name: self.requested_name.clone(),
            name_slot: self.name_slot.clone(),
            context: self.context.clone(),
            is_video: self.is_video,
            started: self.started,
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

    pub fn path(&self) -> PathBuf {
        self.folder.join(self.display_name())
    }

    pub fn percent(&self) -> Option<f32> {
        let total = self.progress.total();
        if total == 0 {
            return None;
        }
        let bytes = self.progress.bytes();
        Some((bytes.min(total) as f32 / total as f32).clamp(0.0, 1.0))
    }

    pub fn seconds_left(&self) -> Option<f64> {
        let total = self.progress.total();
        let bytes = self.progress.bytes();
        if total == 0 || self.speed <= 1.0 || bytes >= total {
            return None;
        }
        Some((total - bytes) as f64 / self.speed)
    }
}
