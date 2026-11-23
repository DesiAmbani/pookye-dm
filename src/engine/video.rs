//! Video pages: yt-dlp fetches the stream, ffmpeg finishes the mp4.
//!
//! The original treats a list of video sites as "pages" rather than files: it
//! asks yt-dlp for the stream's details, downloads the best audio and video it
//! offers over one network, then remuxes the result into a fast-start mp4.
//! Without the two tools it says which install command to run instead of
//! failing a download. This module does the same.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::download::{sanitize_name, set_error, unique_path};
use super::{now_secs, ProbeInfo, Progress};

/// The original's answer when the tools are missing.
pub const INSTALL_HINT: &str =
    "Downloading videos needs yt-dlp and ffmpeg. Install them with: winget install yt-dlp ffmpeg";

/// Best video with sound, falling back to whatever single file has both.
const FORMAT: &str = "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/b";

/// Sites the original hands to yt-dlp.
const HOSTS: &[&str] = &[
    "youtube.com",
    "youtu.be",
    "music.youtube.com",
    "vimeo.com",
    "dailymotion.com",
    "facebook.com",
    "instagram.com",
    "tiktok.com",
    "x.com",
    "twitter.com",
    "reddit.com",
    "clips.twitch.tv",
    "twitch.tv",
    "ted.com",
    "bilibili.com",
];

fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, rest)| rest)?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = authority.split(':').next()?.trim_end_matches('.');
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Whether a link points at a page yt-dlp has to resolve first.
pub fn is_video_url(url: &str) -> bool {
    let Some(host) = host_of(url.trim()) else {
        return false;
    };
    if HOSTS
        .iter()
        .any(|site| host == *site || host.ends_with(&format!(".{site}")))
    {
        return true;
    }
    if host == "archive.org" || host.ends_with(".archive.org") {
        let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or("");
        let path = rest.split_once('/').map(|(_, path)| path).unwrap_or("");
        return path.starts_with("details/") || path.starts_with("video/");
    }
    false
}

/// yt-dlp and ffmpeg, from the app's `tools` folder or from `PATH`.
#[derive(Clone, Debug)]
pub struct Tools {
    pub yt_dlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffmpeg_dir: PathBuf,
}

fn executable(dir: &Path, stem: &str) -> Option<PathBuf> {
    let names: [String; 2] = if cfg!(windows) {
        [format!("{stem}.exe"), stem.to_string()]
    } else {
        [stem.to_string(), format!("{stem}.exe")]
    };
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// Both tools have to be there: the original says it needs both.
pub fn find_tools() -> Option<Tools> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("tools"));
            dirs.push(dir.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("tools"));
        dirs.push(cwd);
    }
    for dir in &dirs {
        if let (Some(yt_dlp), Some(ffmpeg)) = (executable(dir, "yt-dlp"), executable(dir, "ffmpeg")) {
            return Some(Tools {
                yt_dlp,
                ffmpeg,
                ffmpeg_dir: dir.clone(),
            });
        }
    }
    let path = std::env::var_os("PATH")?;
    let yt_dlp = std::env::split_paths(&path).find_map(|dir| executable(&dir, "yt-dlp"))?;
    let ffmpeg = std::env::split_paths(&path).find_map(|dir| executable(&dir, "ffmpeg"))?;
    let ffmpeg_dir = ffmpeg.parent().map(Path::to_path_buf).unwrap_or_default();
    Some(Tools {
        yt_dlp,
        ffmpeg,
        ffmpeg_dir,
    })
}

/// A child process that never flashes a console window.
fn command(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn kill(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// Stops the child when the job is cancelled.
fn watch_cancel(progress: Arc<Progress>, pid: u32, done: Arc<AtomicBool>) {
    thread::spawn(move || {
        while !done.load(Ordering::Relaxed) {
            if progress.cancel.load(Ordering::Relaxed) {
                kill(pid);
                return;
            }
            thread::sleep(Duration::from_millis(150));
        }
    });
}

fn failure(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let line = text
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("WARNING"))
        .unwrap_or("");
    if line.is_empty() {
        "yt-dlp stopped early.".to_string()
    } else {
        line.trim_start_matches("ERROR: ").to_string()
    }
}

fn parse_size(token: &str) -> Option<u64> {
    let split = token
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(token.len());
    let value: f64 = token[..split].parse().ok()?;
    let unit = token[split..].trim().to_ascii_lowercase();
    let factor = match unit.as_str() {
        "" | "b" => 1.0,
        "kib" | "kb" => 1024.0,
        "mib" | "mb" => 1024.0 * 1024.0,
        "gib" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * factor) as u64)
}

/// Reads `[download]  12.3% of  4.56MiB …` into a fraction and a total.
fn parse_progress(line: &str) -> Option<(f64, Option<u64>)> {
    let line = line.trim_start();
    if !line.starts_with("[download]") {
        return None;
    }
    let percent_end = line.find('%')?;
    let percent = line[..percent_end]
        .rsplit(|c: char| c.is_whitespace())
        .next()?
        .parse::<f64>()
        .ok()?;
    let total = line
        .find(" of ")
        .and_then(|index| line[index + 4..].split_whitespace().next())
        .and_then(parse_size);
    Some((percent / 100.0, total))
}

/// What yt-dlp reported about a page.
struct Video {
    title: String,
    total: Option<u64>,
    is_live: bool,
}

fn dump(tools: &Tools, url: &str) -> Result<Video, String> {
    let mut cmd = command(&tools.yt_dlp);
    cmd.arg("--dump-single-json")
        .arg("--no-playlist")
        .arg("--no-warnings")
        .arg("--skip-download")
        .arg("--socket-timeout")
        .arg("10")
        .arg("-f")
        .arg(FORMAT)
        .arg("--ffmpeg-location")
        .arg(&tools.ffmpeg_dir)
        .arg(url);
    let output = cmd
        .output()
        .map_err(|e| format!("couldn't run yt-dlp: {e}"))?;
    if !output.status.success() {
        return Err(failure(&output.stderr));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(start) = text.find('{') else {
        return Err("yt-dlp gave no output".to_string());
    };
    let value: serde_json::Value =
        serde_json::from_str(text[start..].trim()).map_err(|_| "yt-dlp gave no output".to_string())?;

    let number = |key: &str| value.get(key).and_then(serde_json::Value::as_u64);
    let total = number("filesize").or_else(|| number("filesize_approx")).or_else(|| {
        value
            .get("requested_formats")
            .and_then(serde_json::Value::as_array)
            .map(|formats| {
                formats
                    .iter()
                    .filter_map(|format| format.get("filesize").and_then(serde_json::Value::as_u64))
                    .sum::<u64>()
            })
            .filter(|total| *total > 0)
    });

    Ok(Video {
        title: value
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("video")
            .to_string(),
        total,
        is_live: value
            .get("is_live")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

/// Size and name of a video page, for the New download dialog.
pub fn probe(url: &str) -> Result<ProbeInfo, String> {
    let tools = find_tools().ok_or_else(|| INSTALL_HINT.to_string())?;
    let video = dump(&tools, url)?;
    if video.is_live {
        return Err("A live stream has no end to download.".to_string());
    }
    Ok(ProbeInfo {
        total: video.total,
        ranges: false,
        suggested: Some(format!("{}.mp4", sanitize_name(&video.title))),
        is_video: true,
    })
}

/// Strips a media extension so the result can be renamed to `.mp4`.
fn stem_of(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for ext in [".mp4", ".mkv", ".webm", ".m4v", ".mov", ".avi", ".flv"] {
        if lower.ends_with(ext) {
            return name[..name.len() - ext.len()].to_string();
        }
    }
    name.to_string()
}

fn media_files(dir: &Path, stem: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        if !name.starts_with(&stem.to_ascii_lowercase()) {
            continue;
        }
        if [".mp4", ".mkv", ".webm", ".m4v", ".mov", ".flv"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            return Some(path);
        }
    }
    None
}

fn tidy(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn finish(source: &Path, target: &Path, ffmpeg: &Path) -> Result<(), String> {
    // A fast-start mp4 is what the original ends up with. If the remux cannot
    // run, the stream yt-dlp already produced is still a working file.
    if target.exists() {
        tidy(target);
    }
    let remuxed = command(ffmpeg)
        .args(["-y", "-loglevel", "error", "-i"])
        .arg(source)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(target)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if remuxed && target.is_file() {
        tidy(source);
        return Ok(());
    }
    if std::fs::rename(source, target).is_ok() {
        return Ok(());
    }
    std::fs::copy(source, target)
        .map(|_| tidy(source))
        .map_err(|e| format!("couldn't save the video: {e}"))
}

fn run(
    url: &str,
    folder: &Path,
    requested: Option<&str>,
    progress: &Arc<Progress>,
    name_slot: &Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    let tools = find_tools().ok_or_else(|| INSTALL_HINT.to_string())?;
    let video = dump(&tools, url)?;
    if video.is_live {
        return Err("A live stream has no end to download.".to_string());
    }
    if let Some(total) = video.total {
        progress.total.store(total, Ordering::Relaxed);
    }

    let title = requested
        .map(sanitize_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| sanitize_name(&video.title));
    let stem = stem_of(&title);
    let final_name = format!("{stem}.mp4");
    if let Ok(mut slot) = name_slot.lock() {
        *slot = Some(final_name.clone());
    }

    std::fs::create_dir_all(folder)
        .map_err(|e| format!("could not create {}: {e}", folder.display()))?;
    let scratch = folder.join(format!(".pookye-video-{}-{}", std::process::id(), now_secs()));
    std::fs::create_dir_all(&scratch)
        .map_err(|e| format!("could not create {}: {e}", scratch.display()))?;

    let template = scratch.join(format!("{stem}.%(ext)s"));
    let mut cmd = command(&tools.yt_dlp);
    cmd.arg("-f")
        .arg(FORMAT)
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--no-playlist")
        .arg("--no-warnings")
        .arg("--newline")
        .arg("--socket-timeout")
        .arg("20")
        .arg("-o")
        .arg(&template)
        .arg("--ffmpeg-location")
        .arg(&tools.ffmpeg_dir)
        .arg(url);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("couldn't run yt-dlp: {e}"))?;
    let done = Arc::new(AtomicBool::new(false));
    if let Some(stdout) = child.stdout.take() {
        watch_cancel(progress.clone(), child.id(), done.clone());
        let total = AtomicU64::new(progress.total());
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if progress.cancel.load(Ordering::Relaxed) {
                break;
            }
            if let Some((fraction, reported)) = parse_progress(&line) {
                if let Some(bytes) = reported.filter(|bytes| *bytes > 0) {
                    total.store(bytes, Ordering::Relaxed);
                    progress.total.store(bytes, Ordering::Relaxed);
                }
                let total = total.load(Ordering::Relaxed);
                if total > 0 {
                    let bytes = (total as f64 * fraction.clamp(0.0, 1.0)) as u64;
                    let seen = progress.bytes.load(Ordering::Relaxed);
                    if bytes > seen {
                        progress.bytes.store(bytes, Ordering::Relaxed);
                    }
                }
            }
        }
    }
    done.store(true, Ordering::Relaxed);
    let status = child.wait().map_err(|e| format!("yt-dlp stopped: {e}"))?;
    let stderr = child
        .stderr
        .take()
        .and_then(|mut pipe| {
            let mut text = Vec::new();
            std::io::Read::read_to_end(&mut pipe, &mut text).ok()?;
            Some(text)
        })
        .unwrap_or_default();

    if progress.cancel.load(Ordering::Relaxed) {
        let _ = std::fs::remove_dir_all(&scratch);
        return Ok(());
    }
    if !status.success() {
        let _ = std::fs::remove_dir_all(&scratch);
        return Err(failure(&stderr));
    }

    let produced = media_files(&scratch, &stem).ok_or_else(|| "yt-dlp saved nothing.".to_string())?;
    let target = unique_path(folder, &final_name);
    let result = finish(&produced, &target, &tools.ffmpeg);
    let _ = std::fs::remove_dir_all(&scratch);
    result?;

    if let Ok(mut slot) = name_slot.lock() {
        *slot = Some(
            target
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or(final_name),
        );
    }
    if let Ok(metadata) = std::fs::metadata(&target) {
        progress.total.store(metadata.len(), Ordering::Relaxed);
        progress.bytes.store(metadata.len(), Ordering::Relaxed);
    }
    Ok(())
}

/// Downloads a video page as one job, reporting yt-dlp's own progress.
pub fn supervisor(
    url: String,
    folder: PathBuf,
    requested: Option<String>,
    progress: Arc<Progress>,
    file_name_slot: Arc<Mutex<Option<String>>>,
) {
    let result = run(&url, &folder, requested.as_deref(), &progress, &file_name_slot);
    if !progress.cancel.load(Ordering::Relaxed) {
        if let Err(message) = result {
            set_error(&progress, message);
        }
    }
    progress.finished.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_pages_are_recognised_by_host() {
        assert!(is_video_url("https://www.youtube.com/watch?v=abc"));
        assert!(is_video_url("https://youtu.be/abc"));
        assert!(is_video_url("https://x.com/user/status/1"));
        assert!(is_video_url("https://archive.org/details/some-film"));
        assert!(!is_video_url("https://example.com/video.mp4"));
        assert!(!is_video_url("https://notyoutube.com/watch?v=abc"));
    }

    #[test]
    fn yt_dlp_progress_lines_become_bytes() {
        let (fraction, total) =
            parse_progress("[download]  42.5% of  10.00MiB at  1.20MiB/s ETA 00:05").unwrap();
        assert!((fraction - 0.425).abs() < 0.001);
        assert_eq!(total, Some(10 * 1024 * 1024));
    }
}
