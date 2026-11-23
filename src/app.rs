//! Application state: settings, the job manager, selection and the frame loop.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke};

use crate::dialog::ProbeForm;
use crate::engine::{self, Manager, Status};
use crate::{api, brand, dialog, pages, store};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Downloads,
    Speed,
    Networks,
    Schedule,
    Settings,
    About,
}

/// The sidebar's list filters, matching the original's four tabs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    All,
    Active,
    Finished,
    Failed,
}

impl Filter {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Finished => "Finished",
            Filter::Failed => "Failed",
        }
    }

    pub(crate) fn holds(self, status: Status) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => matches!(
                status,
                Status::Queued | Status::Downloading | Status::Paused
            ),
            Filter::Finished => status == Status::Completed,
            Filter::Failed => matches!(status, Status::Failed | Status::Cancelled),
        }
    }
}

pub(crate) struct App {
    pub(crate) manager: Manager,
    pub(crate) settings: store::Settings,
    pub(crate) bridge: Arc<api::Bridge>,
    pub(crate) logo: Option<egui::TextureHandle>,
    pub(crate) page: Page,
    page_started: Instant,
    pub(crate) url_input: String,
    pub(crate) search: String,
    pub(crate) filter: Filter,
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
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        settings: store::Settings,
        bridge: Arc<api::Bridge>,
    ) -> Self {
        brand::apply_theme(&cc.egui_ctx);
        let logo = brand::load_logo(&cc.egui_ctx);
        let mut manager = Manager::new(settings.max_concurrent, settings.effective_rate() / 1024);
        manager.set_proxy(settings.active_proxy().filter(|proxy| {
            crate::engine::proxy_is_valid(proxy).is_ok()
        }));
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
            page_started: Instant::now(),
            url_input: String::new(),
            search: String::new(),
            filter: Filter::All,
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

    pub(crate) fn go(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.page_started = Instant::now();
        }
    }

    pub(crate) fn apply_limits(&mut self) {
        self.manager
            .set_speed_limit(self.settings.effective_rate() / 1024);
        self.manager.set_max_concurrent(self.settings.max_concurrent);
        let proxy = self.settings.active_proxy().filter(|proxy| {
            crate::engine::proxy_is_valid(proxy).is_ok()
        });
        self.manager.set_proxy(proxy);
        self.dirty = true;
    }

    pub(crate) fn count_of(&self, filter: Filter) -> usize {
        self.manager
            .jobs
            .iter()
            .filter(|job| filter.holds(job.status))
            .count()
    }

    fn start_download(&mut self, url: String, name: Option<String>) -> bool {
        let folder = PathBuf::from(self.settings.folder.trim());
        match self.manager.add(
            url,
            name,
            &folder,
            self.settings.connections,
            engine::RequestContext::default(),
        ) {
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

    pub(crate) fn start_pasted(&mut self) {
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
        let proxy = self.settings.active_proxy();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(engine::probe(&url, proxy.as_deref()));
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
        if !self.settings.clipboard || self.clipboard_checked.elapsed() < Duration::from_millis(700)
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
            let name = request.file_name();
            let context = request.context();
            if let Err(message) = self.manager.add(
                request.url,
                name,
                &folder,
                self.settings.connections,
                context,
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
            .filter(|job| self.filter.holds(job.status))
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(120));
        Self::resize_edges(ctx);
        brand::paint_background(ctx);

        let completed = self.manager.tick();
        let mut finished: Vec<String> = Vec::new();
        for job in completed {
            let file_name = job.display_name();
            let bytes = job.progress.bytes();
            finished.push(file_name.clone());
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
        match finished.len() {
            0 => {}
            1 => self.notify(format!("{} downloaded.", finished[0]), false),
            count => self.notify(format!("{count} downloads finished."), false),
        }

        let (weekday, minute) = store::local_now();
        self.manager
            .set_allowed(self.settings.schedule.allows(weekday, minute));

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

        self.title_bar(ctx);
        self.status_bar(ctx);
        self.sidebar(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 4,
                        right: 18,
                        top: 2,
                        bottom: 6,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                let height = ui.available_height();
                egui::Frame::new()
                    .fill(brand::glass())
                    .stroke(Stroke::new(1.0_f32, brand::hairline()))
                    .corner_radius(CornerRadius::same(22))
                    .inner_margin(Margin::same(22))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(height - 8.0);
                        pages::draw(self, ui);
                    });
            });

        // A short fade when the page changes.
        let age = self.page_started.elapsed().as_secs_f32();
        if age < 0.18 {
            let alpha = (1.0 - age / 0.18).clamp(0.0, 1.0);
            let rect = ctx.available_rect();
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("page-fade"),
            ))
            .rect_filled(
                rect,
                CornerRadius::same(22),
                Color32::from_rgba_unmultiplied(0x12, 0x00, 0x0A, (alpha * 210.0) as u8),
            );
            ctx.request_repaint();
        }

        dialog::draw(self, ctx);
        self.toast(ctx);

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
