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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub folder: String,
    pub connections: usize,
    pub history: Vec<HistoryEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            folder: default_folder().to_string_lossy().to_string(),
            connections: 8,
            history: Vec::new(),
        }
    }
}

pub fn default_folder() -> PathBuf {
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
        settings.folder = default_folder().to_string_lossy().to_string();
    }
    settings.connections = settings.connections.clamp(1, 32);
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
