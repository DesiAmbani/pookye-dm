//! Pookye Download Manager - a fast, segmented download manager.

mod api;
mod brand;
mod engine;
mod store;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};
use engine::{format_bytes, format_eta, format_speed, Manager, Status};

fn main() -> eframe::Result {
    let settings = store::load();
    let bridge = Arc::new(api::Bridge::new());
    api::spawn(bridge.clone(), engine::VERSION.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([720.0, 520.0])
            .with_title("Pookye Download Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "Pookye Download Manager",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, settings, bridge)))),
    )
}

#[derive(Clone, Copy)]
enum Action {
    Pause(u64),
    Resume(u64),
    Cancel(u64),
    Open,
    Remove(u64),
}

struct App {
    manager: Manager,
    settings: store::Settings,
    bridge: Arc<api::Bridge>,
    logo: Option<egui::TextureHandle>,
    url_input: String,
    folder_edit: String,
    notice: Option<(String, bool)>,
    dirty: bool,
    saved_at: Instant,
    history_added: usize,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, settings: store::Settings, bridge: Arc<api::Bridge>) -> Self {
        brand::apply_theme(&cc.egui_ctx);
        let logo = brand::load_logo(&cc.egui_ctx);
        Self {
            manager: Manager::new(),
            folder_edit: settings.folder.clone(),
            settings,
            bridge,
            logo,
            url_input: String::new(),
            notice: None,
            dirty: false,
            saved_at: Instant::now(),
            history_added: 0,
        }
    }

    fn start_download(&mut self) {
        let folder = PathBuf::from(self.settings.folder.trim());
        match self.manager.add(
            self.url_input.clone(),
            None,
            &folder,
            self.settings.connections,
        ) {
            Ok(_) => {
                self.url_input.clear();
                self.notice = None;
            }
            Err(message) => self.notice = Some((message, true)),
        }
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Pause(id) => self.manager.pause(id),
            Action::Resume(id) => self.manager.resume(id),
            Action::Cancel(id) => self.manager.cancel(id),
            Action::Open => open_folder(&self.settings.folder),
            Action::Remove(id) => self.manager.remove(id),
        }
    }

    fn pump_api(&mut self) {
        for request in self.bridge.drain() {
            let folder = PathBuf::from(self.settings.folder.trim());
            let outcome = self.manager.add(
                request.url,
                request.file_name,
                &folder,
                self.settings.connections,
            );
            if let Err(message) = outcome {
                self.notice = Some((message, true));
            }
        }
        self.bridge.publish(self.manager.snapshot_json());
    }

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::default()
                    .fill(brand::BG)
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            logo.id(),
                            egui::vec2(42.0, 42.0),
                        )));
                        ui.add_space(4.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Pookye DM")
                                .size(22.0)
                                .strong()
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new("Pookye Download Manager")
                                .size(11.5)
                                .color(brand::ACCENT_BRIGHT),
                        );
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let (text, colour) = match self.bridge.port() {
                                Some(port) => (
                                    format!("Browser handoff 127.0.0.1:{port}"),
                                    brand::MUTED,
                                ),
                                None => ("Browser handoff offline".to_string(), brand::ACCENT),
                            };
                            ui.label(RichText::new(text).size(11.0).color(colour));
                            ui.label(
                                RichText::new(format!("v{}", engine::VERSION))
                                    .size(11.0)
                                    .color(brand::MUTED),
                            );
                        },
                    );
                });
            });
    }

    fn footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::default()
                    .fill(brand::BG)
                    .inner_margin(egui::Margin::symmetric(18, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let speed = self.manager.total_speed();
                    ui.label(
                        RichText::new(if speed > 0.0 {
                            format!("{} total", format_speed(speed))
                        } else {
                            "Idle".to_string()
                        })
                        .size(11.5)
                        .color(brand::INK),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} active · {} in the queue",
                            self.manager.active_count(),
                            self.manager.jobs.len()
                        ))
                        .size(11.5)
                        .color(brand::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.settings.folder.as_str())
                                .size(11.0)
                                .color(brand::MUTED),
                        );
                    });
                });
            });
    }

    fn new_download_card(&mut self, ui: &mut egui::Ui) {
        egui::Frame::default()
            .fill(brand::CARD)
            .inner_margin(egui::Margin::same(14))
            .show(ui, |ui| {
                ui.label(RichText::new("New download").strong().color(brand::INK));
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 110.0).max(140.0);
                    let response = ui.add_sized(
                        [width, 28.0],
                        egui::TextEdit::singleline(&mut self.url_input)
                            .hint_text("https://example.com/file.iso"),
                    );
                    let submitted = response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    let clicked = ui
                        .add_sized(
                            [96.0, 28.0],
                            egui::Button::new(
                                RichText::new("Download").strong().color(Color32::WHITE),
                            )
                            .fill(brand::ACCENT),
                        )
                        .clicked();
                    if submitted || clicked {
                        self.start_download();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Save to").size(11.5).color(brand::MUTED));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.folder_edit)
                            .desired_width(f32::INFINITY)
                            .text_color(brand::INK),
                    );
                    if response.changed() {
                        self.settings.folder = self.folder_edit.clone();
                        self.dirty = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Connections")
                            .size(11.5)
                            .color(brand::MUTED),
                    );
                    if ui
                        .add(egui::Slider::new(&mut self.settings.connections, 1..=32))
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.label(
                        RichText::new(format!(
                            "{} parallel ranges per file",
                            self.settings.connections
                        ))
                        .size(11.0)
                        .color(brand::MUTED),
                    );
                });

                if let Some((message, bad)) = &self.notice {
                    ui.label(
                        RichText::new(message)
                            .size(11.5)
                            .color(if *bad { brand::ACCENT_BRIGHT } else { brand::MUTED }),
                    );
                }
            });
    }

    fn job_card(ui: &mut egui::Ui, job: &engine::Job, actions: &mut Vec<Action>) {
        egui::Frame::default()
            .fill(brand::CARD)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .strong()
                                .size(14.5)
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · {} connections",
                                host_of(&job.url),
                                job.connections
                            ))
                            .size(11.0)
                            .color(brand::MUTED),
                        );
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(job.status.label())
                                    .size(11.0)
                                    .color(status_colour(job.status)),
                            );
                        },
                    );
                });

                ui.add_space(2.0);
                let fraction = job.percent().unwrap_or(0.0);
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_height(10.0)
                        .fill(brand::ACCENT),
                );

                let segments = job.progress.snapshot_segments();
                if segments.len() > 1 {
                    let width = (ui.available_width() - (segments.len() as f32 - 1.0) * 3.0)
                        / segments.len() as f32;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for (length, done, _start) in &segments {
                            let part = if *length == 0 {
                                0.0
                            } else {
                                (*done as f32 / *length as f32).clamp(0.0, 1.0)
                            };
                            ui.add_sized(
                                [width.max(4.0), 5.0],
                                egui::ProgressBar::new(part)
                                    .desired_height(5.0)
                                    .fill(brand::ACCENT_BRIGHT),
                            );
                        }
                    });
                }

                let total = job.progress.total.load(std::sync::atomic::Ordering::Relaxed);
                let bytes = job
                    .progress
                    .bytes
                    .load(std::sync::atomic::Ordering::Relaxed);
                let mut facts = Vec::new();
                if total > 0 {
                    facts.push(format!(
                        "{} of {}",
                        format_bytes(bytes),
                        format_bytes(total)
                    ));
                    facts.push(format!(
                        "{}%",
                        ((bytes.min(total) as f64 / total as f64) * 100.0).round() as u64
                    ));
                } else {
                    facts.push(format!("{} downloaded", format_bytes(bytes)));
                }
                if job.status == Status::Downloading {
                    facts.push(format_speed(job.speed));
                    if let Some(left) = job.seconds_left() {
                        let eta = format_eta(left);
                        if !eta.is_empty() {
                            facts.push(eta);
                        }
                    }
                }
                if let Some(error) = &job.error {
                    facts.push(error.clone());
                }
                ui.label(
                    RichText::new(facts.join("  ·  "))
                        .size(11.5)
                        .color(if job.error.is_some() {
                            brand::ACCENT_BRIGHT
                        } else {
                            brand::MUTED
                        }),
                );

                ui.horizontal(|ui| {
                    match job.status {
                        Status::Downloading | Status::Queued => {
                            if ui.button("Pause").clicked() {
                                actions.push(Action::Pause(job.id));
                            }
                        }
                        Status::Paused => {
                            if ui.button("Resume").clicked() {
                                actions.push(Action::Resume(job.id));
                            }
                        }
                        _ => {}
                    }
                    if job.status.is_active() && ui.button("Stop").clicked() {
                        actions.push(Action::Cancel(job.id));
                    }
                    if matches!(
                        job.status,
                        Status::Completed | Status::Failed | Status::Cancelled
                    ) {
                        if ui.button("Open folder").clicked() {
                            actions.push(Action::Open);
                        }
                        if ui.button("Remove").clicked() {
                            actions.push(Action::Remove(job.id));
                        }
                    }
                });
            });
        ui.add_space(6.0);
    }

    fn body(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(brand::PANEL)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ctx, |ui| {
                self.new_download_card(ui);
                ui.add_space(10.0);

                let mut actions: Vec<Action> = Vec::new();
                if self.manager.jobs.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            RichText::new("Nothing downloading yet")
                                .size(16.0)
                                .color(brand::MUTED),
                        );
                        ui.label(
                            RichText::new("Paste a link above, or let the browser extension hand one over.")
                                .size(12.0)
                                .color(brand::MUTED),
                        );
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for job in &self.manager.jobs {
                                Self::job_card(ui, job, &mut actions);
                            }
                            if !self.settings.history.is_empty() {
                                ui.collapsing(
                                    RichText::new(format!(
                                        "Recent downloads ({})",
                                        self.settings.history.len()
                                    ))
                                    .size(12.0)
                                    .color(brand::MUTED),
                                    |ui| {
                                        for entry in self.settings.history.iter().take(12) {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}  ·  {}",
                                                    entry.file_name,
                                                    format_bytes(entry.bytes)
                                                ))
                                                .size(11.0)
                                                .color(brand::MUTED),
                                            );
                                        }
                                    },
                                );
                            }
                        });
                }

                for action in actions {
                    self.apply_action(action);
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(250));

        let completed = self.manager.tick();
        for job in completed {
            let bytes = job
                .progress
                .bytes
                .load(std::sync::atomic::Ordering::Relaxed);
            let file_name = job.display_name();
            self.settings.history.insert(
                0,
                store::HistoryEntry {
                    url: job.url,
                    file_name,
                    bytes,
                    finished_at: job.finished_at.unwrap_or_else(engine::now_secs),
                },
            );
            self.settings.history.truncate(40);
            self.history_added += 1;
            self.dirty = true;
        }

        self.pump_api();
        self.header(ctx);
        self.footer(ctx);
        self.body(ctx);

        if self.dirty && self.saved_at.elapsed() > Duration::from_millis(800) {
            store::save(&self.settings);
            self.dirty = false;
            self.saved_at = Instant::now();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        store::save(&self.settings);
    }
}

fn host_of(url: &str) -> String {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

fn status_colour(status: Status) -> Color32 {
    match status {
        Status::Downloading => brand::ACCENT_BRIGHT,
        Status::Queued | Status::Paused => brand::MUTED,
        Status::Completed => brand::INK,
        Status::Failed => brand::ACCENT,
        Status::Cancelled => brand::MUTED,
    }
}

fn open_folder(folder: &str) {
    let path = PathBuf::from(folder);
    if !path.exists() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(&path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    }
}
