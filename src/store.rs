//! Persistent settings and download history.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    pub url: String,
    pub file_name: String,
    pub bytes: u64,
    pub finished_at: u64,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
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
    /// Watch the clipboard for copied links.
    #[serde(default = "default_true")]
    pub clipboard: bool,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            folder: default_folder(),
            connections: default_connections(),
            max_concurrent: default_concurrent(),
            speed_limit: 0,
            clipboard: true,
            history: Vec::new(),
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
