//! Pookye Download Manager - a fast, segmented download manager.

mod actions;
mod api;
mod brand;
mod dialog;
mod engine;
mod pages;
mod store;
mod widgets;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, RichText};
use engine::{format_bytes, format_speed, Category, Manager};
use dialog::ProbeForm;

fn main() -> eframe::Result {
    let settings = store::load();
    let bridge = Arc::new(api::Bridge::new());
    api::spawn(bridge.clone(), engine::VERSION.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Pookye Download Manager"),
        ..Default::default()
    };

    eframe::run_native(
        "Pookye Download Manager",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, settings, bridge)))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Downloads,
    Speed,
    Networks,
    Schedule,
    Settings,
    About,
}

pub(crate) struct App {
    pub(crate) manager: Manager,
    pub(crate) settings: store::Settings,
    pub(crate) bridge: Arc<api::Bridge>,
    pub(crate) logo: Option<egui::TextureHandle>,
    pub(crate) page: Page,
    pub(crate) url_input: String,
    pub(crate) search: String,
    pub(crate) category: Option<Category>,
    pub(crate) selected: HashSet<u64>,
    pub(crate) show_add: bool,
    pub(crate) add: ProbeForm,
    pub(crate) notice: Option<(String, bool, Instant)>,
    pub(crate) dirty: bool,
    saved_at: Instant,
    pub(crate) clipboard: Option<arboard::Clipboard>,
    clipboard_checked: Instant,
    last_clipboard: String,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        settings: store::Settings,
        bridge: Arc<api::Bridge>,
    ) -> Self {
        brand::apply_theme(&cc.egui_ctx);
        let logo = brand::load_logo(&cc.egui_ctx);
        let manager = Manager::new(settings.max_concurrent, settings.speed_limit as u64);
        let clipboard = if settings.clipboard {
            arboard::Clipboard::new().ok()
        } else {
            None
        };
        Self {
            manager,
            add: ProbeForm::new(&settings),
            settings,
            bridge,
            logo,
            page: Page::Downloads,
            url_input: String::new(),
            search: String::new(),
            category: None,
            selected: HashSet::new(),
            show_add: false,
            notice: None,
            dirty: false,
            saved_at: Instant::now(),
            clipboard,
            clipboard_checked: Instant::now(),
            last_clipboard: String::new(),
        }
    }

    pub(crate) fn notify(&mut self, message: impl Into<String>, bad: bool) {
        self.notice = Some((message.into(), bad, Instant::now()));
    }

    pub(crate) fn apply_limits(&mut self) {
        self.manager.set_speed_limit(self.settings.speed_limit as u64);
        self.manager.set_max_concurrent(self.settings.max_concurrent);
        self.dirty = true;
    }

    fn start_download(&mut self, url: String, name: Option<String>) -> bool {
        let folder = PathBuf::from(self.settings.folder.trim());
        match self
            .manager
            .add(url, name, &folder, self.settings.connections)
        {
            Ok(_) => {
                self.notify("Added to the queue.", false);
                true
            }
            Err(message) => {
                self.notify(message, true);
                false
            }
        }
    }

    fn start_pasted(&mut self) {
        let url = self.url_input.trim().to_string();
        if url.is_empty() {
            self.notify("Paste a link to a file first.", true);
            return;
        }
        if self.start_download(url, None) {
            self.url_input.clear();
        }
    }

    pub(crate) fn open_add(&mut self) {
        self.add = ProbeForm::new(&self.settings);
        self.show_add = true;
    }

    pub(crate) fn check_link(&mut self) {
        if self.add.checking {
            return;
        }
        let url = self.add.url.trim().to_string();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            self.add.error =
                Some("That isn't a web link. Links start with http:// or https://.".to_string());
            return;
        }
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(engine::probe(&url));
        });
        self.add.checking = true;
        self.add.rx = Some(rx);
        self.add.error = None;
        self.add.info = None;
    }

    fn poll_probe(&mut self) {
        let Some(rx) = &self.add.rx else { return };
        match rx.try_recv() {
            Ok(result) => {
                self.add.checking = false;
                self.add.rx = None;
                match result {
                    Ok(info) => {
                        if self.add.name.trim().is_empty() {
                            if let Some(name) = &info.suggested {
                                self.add.name = name.clone();
                            }
                        }
                        self.add.info = Some(info);
                        self.add.error = None;
                    }
                    Err(message) => self.add.error = Some(message),
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.add.checking = false;
                self.add.rx = None;
            }
        }
    }

    fn poll_clipboard(&mut self) {
        if !self.settings.clipboard
            || self.clipboard_checked.elapsed() < Duration::from_millis(700)
        {
            return;
        }
        self.clipboard_checked = Instant::now();
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        let text = {
            let Some(clipboard) = self.clipboard.as_mut() else {
                return;
            };
            match clipboard.get_text() {
                Ok(text) => text.trim().to_string(),
                Err(_) => return,
            }
        };
        if !(text.starts_with("http://") || text.starts_with("https://")) {
            return;
        }
        if text == self.last_clipboard {
            return;
        }
        self.last_clipboard = text.clone();
        if text == self.url_input.trim() {
            return;
        }
        if self.manager.jobs.iter().any(|job| job.url == text) {
            return;
        }
        self.url_input = text;
        self.notify("Link copied — press Start download.", false);
    }

    fn pump_api(&mut self) {
        for request in self.bridge.drain() {
            let folder = PathBuf::from(self.settings.folder.trim());
            if let Err(message) = self.manager.add(
                request.url,
                request.file_name,
                &folder,
                self.settings.connections,
            ) {
                self.notify(message, true);
            }
        }
        self.bridge.publish(self.manager.snapshot_json());
    }

    pub(crate) fn filtered_rows(&self) -> Vec<u64> {
        let needle = self.search.trim().to_lowercase();
        let mut rows: Vec<u64> = self
            .manager
            .jobs
            .iter()
            .filter(|job| {
                self.category
                    .map(|category| job.category == category)
                    .unwrap_or(true)
            })
            .filter(|job| {
                needle.is_empty()
                    || job.display_name().to_lowercase().contains(&needle)
                    || job.url.to_lowercase().contains(&needle)
            })
            .map(|job| job.id)
            .collect();
        rows.reverse();
        rows
    }

    pub(crate) fn notice_line(&self, ui: &mut egui::Ui) {
        if let Some((message, bad, at)) = &self.notice {
            if at.elapsed() < Duration::from_secs(8) {
                ui.label(
                    RichText::new(message)
                        .size(11.5)
                        .color(if *bad { brand::ACCENT_BRIGHT } else { brand::MUTED }),
                );
            }
        }
    }

    fn chrome(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("chrome")
            .frame(
                egui::Frame::default()
                    .fill(brand::BG)
                    .inner_margin(egui::Margin::symmetric(18, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            logo.id(),
                            egui::vec2(38.0, 38.0),
                        )));
                        ui.add_space(2.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Pookye DM")
                                .size(19.0)
                                .strong()
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new("Pookye Download Manager")
                                .size(11.0)
                                .color(brand::ACCENT_BRIGHT),
                        );
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(format!("v{}", engine::VERSION))
                                    .size(11.0)
                                    .color(brand::MUTED),
                            );
                            ui.add_space(8.0);
                            match self.bridge.port() {
                                Some(port) => ui.label(
                                    RichText::new(format!("● Browser bridge :{port}"))
                                        .size(11.0)
                                        .color(brand::MUTED),
                                ),
                                None => ui.label(
                                    RichText::new("● Browser bridge offline")
                                        .size(11.0)
                                        .color(brand::ACCENT),
                                ),
                            };
                            ui.add_space(8.0);
                            if self.manager.total_speed() > 0.0 {
                                ui.label(
                                    RichText::new(format_speed(self.manager.total_speed()))
                                        .size(12.5)
                                        .strong()
                                        .color(brand::ACCENT_BRIGHT),
                                );
                            }
                        },
                    );
                });
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(206.0)
            .frame(
                egui::Frame::default()
                    .fill(brand::BG)
                    .inner_margin(egui::Margin::symmetric(12, 14)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("DOWNLOADS").size(9.5).color(brand::MUTED));
                if widgets::nav(
                    ui,
                    "Downloads",
                    self.page == Page::Downloads,
                    Some(self.manager.jobs.len()),
                ) {
                    self.page = Page::Downloads;
                }
                if widgets::nav(ui, "Speed", self.page == Page::Speed, None) {
                    self.page = Page::Speed;
                }
                if widgets::nav(ui, "Networks", self.page == Page::Networks, None) {
                    self.page = Page::Networks;
                }
                if widgets::nav(
                    ui,
                    "Schedule",
                    self.page == Page::Schedule,
                    Some(self.manager.queued_count()),
                ) {
                    self.page = Page::Schedule;
                }

                ui.add_space(14.0);
                ui.label(RichText::new("APP").size(9.5).color(brand::MUTED));
                if widgets::nav(ui, "Settings", self.page == Page::Settings, None) {
                    self.page = Page::Settings;
                }
                if widgets::nav(ui, "About", self.page == Page::About, None) {
                    self.page = Page::About;
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} active · {} queued",
                            self.manager.downloading_count(),
                            self.manager.queued_count()
                        ))
                        .size(10.5)
                        .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new(format!("{} done", self.manager.completed_count()))
                            .size(10.5)
                            .color(brand::MUTED),
                    );
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
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
                            format!("Average speed {}", format_speed(speed))
                        } else {
                            "Idle".to_string()
                        })
                        .size(11.0)
                        .color(brand::INK),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} downloaded · {} streams per download",
                            format_bytes(self.manager.total_downloaded()),
                            self.settings.connections
                        ))
                        .size(11.0)
                        .color(brand::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(self.settings.folder.as_str())
                                .size(10.5)
                                .color(brand::MUTED),
                        );
                    });
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(200));

        let completed = self.manager.tick();
        for job in completed {
            let file_name = job.display_name();
            let bytes = job.progress.bytes();
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
            self.dirty = true;
        }

        self.pump_api();
        self.poll_clipboard();
        self.poll_probe();

        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.show_add {
                self.show_add = false;
            } else if !self.selected.is_empty() {
                self.selected.clear();
            }
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::N)) {
            self.open_add();
        }

        self.chrome(ctx);
        self.status_bar(ctx);
        self.sidebar(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(brand::PANEL)
                    .inner_margin(egui::Margin::symmetric(18, 14)),
            )
            .show(ctx, |ui| pages::draw(self, ui));

        dialog::draw(self, ctx);

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
