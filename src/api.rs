//! Loopback HTTP API so browsers and scripts can hand downloads to Pookye DM.
//!
//! Endpoints (all JSON, CORS enabled for browser extensions):
//!   GET  /ping     -> { app, api, version, name }
//!   GET  /jobs     -> { app, api, version, averageSpeedBps, active, queued, jobs: [...] }
//!   POST /add      <- { url, fileName?, browser? } -> { ok, token, fileName, queued }
//!   POST /confirm  <- { token }                    -> { ok, queued }
//!
//! A hand-off that carries `browser` (cookies, referer, userAgent, name) is held
//! until the extension confirms its token, so the page can finish any last
//! checks first. Plain `url` hand-offs are queued immediately and still receive
//! a token, which keeps simple scripts working.

use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

use crate::engine::RequestContext;

pub const PORTS: [u16; 3] = [19330, 19331, 19332];

/// The browser's side of a hand-off.
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BrowserContext {
    #[serde(default)]
    pub cookies: Option<String>,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default, alias = "user_agent", alias = "useragent")]
    pub user_agent: Option<String>,
    /// The name the browser would have given the file.
    #[serde(default)]
    pub name: Option<String>,
}

impl BrowserContext {
    fn is_empty(&self) -> bool {
        self.cookies.is_none() && self.referer.is_none() && self.user_agent.is_none()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct AddRequest {
    pub url: String,
    #[serde(default, alias = "filename", alias = "file_name", alias = "fileName")]
    pub file_name: Option<String>,
    /// Cookies and headers the browser holds for this URL.
    #[serde(default)]
    pub browser: Option<BrowserContext>,
}

impl AddRequest {
    /// The name to write to disk: an explicit one wins, then the browser's.
    pub fn file_name(&self) -> Option<String> {
        self.file_name
            .clone()
            .or_else(|| self.browser.as_ref().and_then(|b| b.name.clone()))
    }

    pub fn context(&self) -> RequestContext {
        let Some(browser) = &self.browser else {
            return RequestContext::default();
        };
        RequestContext {
            cookies: browser.cookies.clone().filter(|c| !c.trim().is_empty()),
            referer: browser.referer.clone().filter(|r| !r.trim().is_empty()),
            user_agent: browser
                .user_agent
                .clone()
                .filter(|a| !a.trim().is_empty()),
        }
    }
}

struct Pending {
    token: String,
    request: AddRequest,
}

pub struct Bridge {
    pub add_queue: Mutex<Vec<AddRequest>>,
    awaiting: Mutex<Vec<Pending>>,
    tokens: AtomicU64,
    pub snapshot: Mutex<String>,
    pub port: Mutex<Option<u16>>,
    pub requests: AtomicU64,
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Bridge {
    pub fn new() -> Self {
        Self {
            add_queue: Mutex::new(Vec::new()),
            awaiting: Mutex::new(Vec::new()),
            tokens: AtomicU64::new(1),
            snapshot: Mutex::new(String::from("{\"app\":\"pookye\",\"api\":1,\"jobs\":[]}")),
            port: Mutex::new(None),
            requests: AtomicU64::new(0),
        }
    }

    pub fn drain(&self) -> Vec<AddRequest> {
        self.add_queue
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default()
    }

    /// Register a hand-off waiting for its token to be confirmed.
    fn hold(&self, request: AddRequest) -> String {
        let token = self.new_token();
        if let Ok(mut pending) = self.awaiting.lock() {
            pending.push(Pending {
                token: token.clone(),
                request,
            });
            // Nobody is going to confirm ancient tokens; keep the list small.
            if pending.len() > 32 {
                pending.remove(0);
            }
        }
        token
    }

    /// Moves a held hand-off into the queue. `false` when the token is unknown.
    fn confirm(&self, token: &str) -> bool {
        let found = {
            let Ok(mut pending) = self.awaiting.lock() else {
                return false;
            };
            pending
                .iter()
                .position(|entry| entry.token == token)
                .map(|index| pending.remove(index))
        };
        let Some(entry) = found else { return false };
        if let Ok(mut queue) = self.add_queue.lock() {
            queue.push(entry.request);
        }
        true
    }

    fn new_token(&self) -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let sequence = self.tokens.fetch_add(1, Ordering::Relaxed);
        format!("{millis:x}{sequence:x}")
    }

    pub fn publish(&self, json: String) {
        if let Ok(mut slot) = self.snapshot.lock() {
            *slot = json;
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.port.lock().ok().and_then(|p| *p)
    }

    /// Hand-offs that arrived but have not been confirmed yet.
    pub fn awaiting_count(&self) -> usize {
        self.awaiting.lock().map(|p| p.len()).unwrap_or(0)
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static header")
}

fn respond(request: Request, code: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(code));
    for h in [
        header("Content-Type", "application/json; charset=utf-8"),
        header("Access-Control-Allow-Origin", "*"),
        header(
            "Access-Control-Allow-Headers",
            "content-type, x-pookye-client, x-quantumdm-client",
        ),
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        header("Cache-Control", "no-store"),
    ] {
        response.add_header(h);
    }
    let _ = request.respond(response);
}

fn suggested_name(url: &str) -> Option<String> {
    url.split(['?', '#'])
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .filter(|last| !last.is_empty())
        .map(|last| last.to_string())
}

/// Reads `token` from a JSON body or a `?token=` query.
fn token_from(body: &str, query: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(token) = value.get("token").and_then(|t| t.as_str()) {
            if !token.trim().is_empty() {
                return Some(token.trim().to_string());
            }
        }
    }
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "token" && !value.is_empty() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

/// Start the loopback server on the first free port of `PORTS`.
pub fn spawn(bridge: Arc<Bridge>, version: String) {
    thread::spawn(move || {
        let mut server = None;
        for port in PORTS {
            if let Ok(candidate) = Server::http(("127.0.0.1", port)) {
                if let Ok(mut slot) = bridge.port.lock() {
                    *slot = Some(port);
                }
                server = Some(candidate);
                break;
            }
        }
        let Some(server) = server else { return };

        for mut request in server.incoming_requests() {
            bridge.requests.fetch_add(1, Ordering::Relaxed);
            let method = request.method().clone();
            let url = request.url().to_string();
            let query = url.split_once('?').map(|(_, q)| q).unwrap_or("").to_string();
            let path = url
                .split('?')
                .next()
                .unwrap_or("/")
                .trim_end_matches('/')
                .to_string();

            if method == Method::Options {
                let _ = request.respond(Response::empty(StatusCode(204)).with_header(header(
                    "Access-Control-Allow-Origin",
                    "*",
                )));
                continue;
            }

            match (&method, path.as_str()) {
                (Method::Get, "/ping") | (Method::Get, "") => {
                    let body = serde_json::json!({
                        "app": "pookye",
                        "api": 1,
                        "version": version,
                        "name": "Pookye Download Manager",
                        "awaiting": bridge.awaiting_count(),
                    })
                    .to_string();
                    respond(request, 200, body);
                }
                (Method::Get, "/jobs") => {
                    let body = bridge
                        .snapshot
                        .lock()
                        .map(|s| s.clone())
                        .unwrap_or_else(|_| "{}".to_string());
                    respond(request, 200, body);
                }
                (Method::Post, "/add") => {
                    let mut body = String::new();
                    let _ = request.as_reader().take(256 * 1024).read_to_string(&mut body);
                    match serde_json::from_str::<AddRequest>(&body) {
                        Ok(mut add) => {
                            add.url = add.url.trim().to_string();
                            if !(add.url.starts_with("http://")
                                || add.url.starts_with("https://"))
                            {
                                respond(
                                    request,
                                    400,
                                    serde_json::json!({
                                        "ok": false,
                                        "reason": "That isn't a web link. Links start with http:// or https://."
                                    })
                                    .to_string(),
                                );
                                continue;
                            }
                            let file_name = add
                                .file_name()
                                .or_else(|| suggested_name(&add.url))
                                .unwrap_or_else(|| "download".to_string());
                            let browser = add.browser.clone().filter(|b| !b.is_empty());
                            // A browser hand-off waits for /confirm; a plain link starts now.
                            let token = if browser.is_some() {
                                bridge.hold(add)
                            } else {
                                let token = bridge.new_token();
                                if let Ok(mut queue) = bridge.add_queue.lock() {
                                    queue.push(add);
                                }
                                token
                            };
                            respond(
                                request,
                                200,
                                serde_json::json!({
                                    "ok": true,
                                    "token": token,
                                    "fileName": file_name,
                                    "queued": browser.is_none(),
                                })
                                .to_string(),
                            );
                        }
                        Err(error) => respond(
                            request,
                            400,
                            serde_json::json!({
                                "ok": false,
                                "reason": format!("That request wasn't understood: {error}")
                            })
                            .to_string(),
                        ),
                    }
                }
                (Method::Post, "/confirm") => {
                    let mut body = String::new();
                    let _ = request.as_reader().take(16 * 1024).read_to_string(&mut body);
                    match token_from(&body, &query) {
                        Some(token) if bridge.confirm(&token) => respond(
                            request,
                            200,
                            serde_json::json!({ "ok": true, "queued": true }).to_string(),
                        ),
                        Some(_) => respond(
                            request,
                            404,
                            serde_json::json!({
                                "ok": false,
                                "reason": "That download token is unknown or already confirmed."
                            })
                            .to_string(),
                        ),
                        None => respond(
                            request,
                            400,
                            serde_json::json!({
                                "ok": false,
                                "reason": "Send the token from /add as { \"token\": \"…\" }."
                            })
                            .to_string(),
                        ),
                    }
                }
                _ => respond(
                    request,
                    404,
                    serde_json::json!({ "ok": false, "reason": "No such endpoint." }).to_string(),
                ),
            }
        }
    });
}
