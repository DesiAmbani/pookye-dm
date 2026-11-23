//! Persistent settings and download history.
//!
//! The shape mirrors the original application's `settings.json` so the same
//! concepts exist here: one destination, downloads at once, a hard speed limit,
//! a slow mode, a weekly schedule, proxy and system-wide toggles. Fields the
//! engine cannot honour yet are still stored, so toggling them later needs no
//! migration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub url: String,
    pub file_name: String,
    pub bytes: u64,
    pub finished_at: u64,
}

/// When downloads are allowed to run, in minutes since midnight.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schedule {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_start")]
    pub start_minute: u32,
    #[serde(default = "default_end")]
    pub end_minute: u32,
    /// 0 = Sunday … 6 = Saturday.
    #[serde(default = "default_days")]
    pub days: Vec<u8>,
    #[serde(default)]
    pub end_date: Option<String>,
}

fn default_start() -> u32 {
    60
}

fn default_end() -> u32 {
    420
}

fn default_days() -> Vec<u8> {
    (0..7).collect()
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start_minute: default_start(),
            end_minute: default_end(),
            days: default_days(),
            end_date: None,
        }
    }
}

impl Schedule {
    /// Whether a download may start at `weekday` (0 = Sunday) and `minute`.
    /// A window with the same start and end is open all day.
    pub fn allows(&self, weekday: u8, minute: u32) -> bool {
        if !self.enabled || self.days.is_empty() {
            return true;
        }
        if !self.days.contains(&weekday) {
            return false;
        }
        if self.start_minute == self.end_minute {
            return true;
        }
        if self.start_minute < self.end_minute {
            minute >= self.start_minute && minute < self.end_minute
        } else {
            minute >= self.start_minute || minute < self.end_minute
        }
    }
}

/// Formats a minute of the day as `HH:MM`.
pub fn clock(minute: u32) -> String {
    let minute = minute % (24 * 60);
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// The computer's local weekday (0 = Sunday) and minute of the day.
#[cfg(windows)]
pub fn local_now() -> (u8, u32) {
    #[repr(C)]
    struct LocalTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    extern "system" {
        fn GetLocalTime(time: *mut LocalTime);
    }
    let mut time = LocalTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    unsafe { GetLocalTime(&mut time) };
    (
        (time.day_of_week % 7) as u8,
        time.hour as u32 * 60 + time.minute as u32,
    )
}

/// UTC fallback for platforms without a local-time call.
#[cfg(not(windows))]
pub fn local_now() -> (u8, u32) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = seconds / 86_400;
    let minute = ((seconds % 86_400) / 60) as u32;
    // 1970-01-01 was a Thursday.
    (((days + 4) % 7) as u8, minute)
}

fn default_folder() -> String {
    default_download_folder().to_string_lossy().to_string()
}

fn default_connections() -> usize {
    8
}

fn default_concurrent() -> usize {
    3
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_slow_mode_speed() -> usize {
    2 * 1024 * 1024
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// `system`, `light` or `dark`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Where downloads are written.
    #[serde(default = "default_folder")]
    pub folder: String,
    /// Streams (parallel range requests) per download.
    #[serde(default = "default_connections")]
    pub connections: usize,
    /// How many downloads may run at the same time.
    #[serde(default = "default_concurrent")]
    pub max_concurrent: usize,
    /// System-wide speed cap in KB/s; 0 means unlimited.
    #[serde(default)]
    pub speed_limit: usize,
    /// Hold every download to `slow_mode_speed` while on.
    #[serde(default)]
    pub slow_mode: bool,
    /// Slow-mode cap in bytes per second.
    #[serde(default = "default_slow_mode_speed")]
    pub slow_mode_speed: usize,
    /// When downloads may run.
    #[serde(default)]
    pub schedule: Schedule,
    /// Closing the window keeps the app in the tray.
    #[serde(default)]
    pub close_to_tray: bool,
    /// Route every app's traffic, not just ours.
    #[serde(default)]
    pub system_mode: bool,
    #[serde(default)]
    pub proxy_on: bool,
    #[serde(default)]
    pub proxy: String,
    #[serde(default)]
    pub proxy_auto: bool,
    #[serde(default)]
    pub usb_tether: bool,
    #[serde(default)]
    pub guide_done: bool,
    /// Watch the clipboard for copied links.
    #[serde(default = "default_true")]
    pub clipboard: bool,
    /// Legacy field from before the interface was rebuilt; kept so old settings load.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            folder: default_folder(),
            connections: default_connections(),
            max_concurrent: default_concurrent(),
            speed_limit: 0,
            slow_mode: false,
            slow_mode_speed: default_slow_mode_speed(),
            schedule: Schedule::default(),
            close_to_tray: false,
            system_mode: false,
            proxy_on: false,
            proxy: String::new(),
            proxy_auto: false,
            usb_tether: false,
            guide_done: false,
            clipboard: true,
            category: None,
            history: Vec::new(),
        }
    }
}

impl Settings {
    /// The rate the limiter should hold, in bytes per second. Zero is unlimited.
    pub fn effective_rate(&self) -> u64 {
        let hard = self.speed_limit as u64 * 1024;
        let slow = if self.slow_mode {
            self.slow_mode_speed as u64
        } else {
            0
        };
        match (hard, slow) {
            (0, 0) => 0,
            (0, slow) => slow,
            (hard, 0) => hard,
            (hard, slow) => hard.min(slow),
        }
    }
}

pub fn default_download_folder() -> PathBuf {
    if let Some(downloads) = dirs::download_dir() {
        return downloads;
    }
    if let Some(home) = dirs::home_dir() {
        let downloads = home.join("Downloads");
        if downloads.exists() {
            return downloads;
        }
        return home;
    }
    PathBuf::from(".")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PookyeDM")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    let mut settings = std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default();
    if settings.folder.trim().is_empty() {
        settings.folder = default_folder();
    }
    settings.connections = settings.connections.clamp(1, 32);
    settings.max_concurrent = settings.max_concurrent.clamp(1, 10);
    settings.speed_limit = settings.speed_limit.min(102_400);
    settings.slow_mode_speed = settings.slow_mode_speed.clamp(64 * 1024, 100 * 1024 * 1024);
    settings.schedule.days.retain(|day| *day < 7);
    settings.history.truncate(40);
    settings
}

pub fn save(settings: &Settings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(path, text);
    }
}
