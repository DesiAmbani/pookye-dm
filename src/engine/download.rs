//! Probing a link and moving its bytes: the range workers and the supervisor.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::header::{COOKIE, RANGE, REFERER, USER_AGENT};

use super::{
    now_secs, Limiter, Progress, RequestContext, Segment, CHUNK, MAX_RETRIES, MIN_SEGMENT,
    USER_AGENT_VALUE,
};

pub(crate) fn client(proxy: Option<&str>) -> Result<Client, String> {
    client_bound(proxy, None)
}

/// A client whose sockets leave from one adapter's address, so a download can
/// be spread across the networks the user picked.
pub(crate) fn client_bound(
    proxy: Option<&str>,
    local: Option<std::net::IpAddr>,
) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent(USER_AGENT_VALUE)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120));
    if let Some(local) = local {
        builder = builder.local_address(local);
    }
    if let Some(proxy) = proxy.map(str::trim).filter(|proxy| !proxy.is_empty()) {
        let proxy = reqwest::Proxy::all(proxy)
            .map_err(|e| format!("that proxy couldn't be used: {e}"))?;
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .map_err(|e| format!("could not start the HTTP client: {e}"))
}

/// Checks a proxy like the original does: a scheme, a host and a port.
pub fn proxy_is_valid(raw: &str) -> Result<(), String> {
    const HINT: &str = "That isn't a proxy like socks5://10.0.0.1:1080.";
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Enter a proxy first, like socks5://10.0.0.1:1080.".to_string());
    }
    let Some((scheme, rest)) = raw.split_once("://") else {
        return Err(HINT.to_string());
    };
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https" | "socks5" | "socks5h") {
        return Err(HINT.to_string());
    }
    let authority = rest.split('/').next().unwrap_or("");
    let Some((host, port)) = authority.rsplit_once(':') else {
        return Err(HINT.to_string());
    };
    if host.trim().is_empty() || port.parse::<u16>().is_err() {
        return Err(HINT.to_string());
    }
    Ok(())
}

/// A GET that carries whatever the browser told us about the request.
fn get(client: &Client, url: &str, context: &RequestContext) -> reqwest::blocking::RequestBuilder {
    let mut builder = client.get(url);
    if let Some(agent) = &context.user_agent {
        builder = builder.header(USER_AGENT, agent);
    }
    if let Some(referer) = &context.referer {
        builder = builder.header(REFERER, referer);
    }
    if let Some(cookies) = &context.cookies {
        builder = builder.header(COOKIE, cookies);
    }
    builder
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

pub(crate) fn sanitize_name(name: &str) -> String {
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

pub(crate) fn name_from_url(url: &str) -> Option<String> {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let last = without_query.rsplit('/').next().unwrap_or("");
    if last.is_empty() {
        None
    } else {
        Some(sanitize_name(&percent_decode(last)))
    }
}

pub(crate) fn unique_path(folder: &Path, name: &str) -> PathBuf {
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

/// What a probe learned about a link, before any bytes are downloaded.
#[derive(Clone, Debug)]
pub struct ProbeInfo {
    pub total: Option<u64>,
    pub ranges: bool,
    pub suggested: Option<String>,
    pub is_video: bool,
}

struct Probe {
    total: Option<u64>,
    ranges: bool,
    suggested: Option<String>,
    is_video: bool,
}

/// Asks the server for the size, name and range support of a link.
pub fn probe(url: &str, proxy: Option<&str>) -> Result<ProbeInfo, String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("That isn't a web link. Links start with http:// or https://.".to_string());
    }
    let client = client(proxy)?;
    let info = probe_with(&client, url, &RequestContext::default())?;
    Ok(ProbeInfo {
        total: info.total,
        ranges: info.ranges,
        suggested: info.suggested,
        is_video: info.is_video,
    })
}

fn probe_with(client: &Client, url: &str, context: &RequestContext) -> Result<Probe, String> {
    let response = get(client, url, context)
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

pub(crate) fn set_error(progress: &Progress, message: String) {
    if let Ok(mut slot) = progress.outcome.lock() {
        if slot.is_none() {
            *slot = Some(Err(message));
        }
    }
}

fn worker(
    client: Client,
    url: String,
    context: RequestContext,
    path: PathBuf,
    segment: Arc<Segment>,
    progress: Arc<Progress>,
    limiter: Arc<Limiter>,
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
        let response = match get(&client, &url, &context).header(RANGE, range).send() {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                failures += 1;
                if failures > MAX_RETRIES {
                    set_error(
                        &progress,
                        format!("the server kept answering HTTP {}", r.status().as_u16()),
                    );
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
                    limiter.consume(n as u64, &progress.cancel);
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

pub(crate) fn supervisor(
    client: Client,
    url: String,
    folder: PathBuf,
    requested: Option<String>,
    connections: usize,
    // One entry per chosen network: the local address its streams bind to.
    binds: Vec<Option<std::net::IpAddr>>,
    proxy: Option<String>,
    context: RequestContext,
    progress: Arc<Progress>,
    file_name_slot: Arc<Mutex<Option<String>>>,
    limiter: Arc<Limiter>,
) {
    let result = (|| -> Result<(), String> {
        let info = probe_with(&client, &url, &context)?;
        progress.ranges.store(info.ranges, Ordering::Relaxed);

        let mut name = requested
            .as_deref()
            .map(sanitize_name)
            .filter(|n| !n.is_empty())
            .or_else(|| info.suggested.map(|n| sanitize_name(&n)))
            .or_else(|| name_from_url(&url))
            .unwrap_or_else(|| "download".to_string());

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
        // Each network gets its own share of streams, so two networks carry twice
        // the ranges one network would.
        let lanes = binds.len().max(1);
        let streams = connections.max(1) * lanes;
        let mut segments = Vec::new();
        if total > 0 && info.ranges && streams > 1 {
            let wanted = (total / MIN_SEGMENT).max(1) as usize;
            let count = streams.min(wanted).max(1);
            let size = total / count as u64;
            for index in 0..count {
                let start = index as u64 * size;
                let end = if index + 1 == count {
                    total - 1
                } else {
                    start + size - 1
                };
                segments.push(Arc::new(Segment {
                    start,
                    end,
                    done: std::sync::atomic::AtomicU64::new(0),
                    unbounded: false,
                    network: index % lanes,
                }));
            }
        } else if total > 0 {
            segments.push(Arc::new(Segment {
                start: 0,
                end: total - 1,
                done: std::sync::atomic::AtomicU64::new(0),
                unbounded: false,
                network: 0,
            }));
        } else {
            segments.push(Arc::new(Segment {
                start: 0,
                end: u64::MAX - 1,
                done: std::sync::atomic::AtomicU64::new(0),
                unbounded: true,
                network: 0,
            }));
        }

        if let Ok(mut slot) = progress.segments.lock() {
            *slot = segments.clone();
        }

        let mut handles = Vec::new();
        for segment in segments {
            let lane = segment.network.min(lanes.saturating_sub(1));
            let bound = binds.get(lane).copied().flatten();
            let client = client_bound(proxy.as_deref(), bound).unwrap_or_else(|_| client.clone());
            let url = url.clone();
            let context = context.clone();
            let path = path.clone();
            let progress = progress.clone();
            let limiter = limiter.clone();
            handles.push(thread::spawn(move || {
                worker(client, url, context, path, segment, progress, limiter)
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
