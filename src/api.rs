//! Loopback HTTP API so browsers and scripts can hand downloads to Pookye DM.
//!
//! Endpoints (all JSON, CORS enabled for browser extensions):
//!   GET  /ping   -> { app, api, version }
//!   POST /add    <- { url, fileName? }  -> { ok: true, id }
//!   GET  /jobs   -> { app, api, jobs: [...] }

use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Deserialize;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub const PORTS: [u16; 3] = [19330, 19331, 19332];

#[derive(Deserialize, Clone, Debug)]
pub struct AddRequest {
    pub url: String,
    #[serde(default, alias = "filename", alias = "file_name", alias = "fileName")]
    pub file_name: Option<String>,
}

pub struct Bridge {
    pub add_queue: Mutex<Vec<AddRequest>>,
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

    pub fn publish(&self, json: String) {
        if let Ok(mut slot) = self.snapshot.lock() {
            *slot = json;
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.port.lock().ok().and_then(|p| *p)
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
        header("Access-Control-Allow-Headers", "content-type, x-pookye-client"),
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        header("Cache-Control", "no-store"),
    ] {
        response.add_header(h);
    }
    let _ = request.respond(response);
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
            let path = request
                .url()
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
                    let _ = request.as_reader().take(64 * 1024).read_to_string(&mut body);
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
                            let id = if let Ok(mut queue) = bridge.add_queue.lock() {
                                queue.push(add);
                                queue.len()
                            } else {
                                0
                            };
                            respond(
                                request,
                                200,
                                serde_json::json!({ "ok": true, "id": id }).to_string(),
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
                _ => respond(
                    request,
                    404,
                    serde_json::json!({ "ok": false, "reason": "No such endpoint." }).to_string(),
                ),
            }
        }
    });
}
