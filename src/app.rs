//! Application state: settings, the job manager, selection and the frame loop.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Margin};

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
    /// The notification-area icon, present while the window may close to it.
    pub(crate) tray: Option<crate::tray::Tray>,
    /// True while only the tray icon is visible; downloads keep running.
    pub(crate) hidden: bool,
    /// The OS window, parked off-screen while hidden.
    pub(crate) window: Option<crate::tray::AppWindow>,
    parked_at: Option<(i32, i32)>,
    /// Set when the tray asked to quit, so the close request is not caught.
    quitting: bool,
    /// An edge drag that is resizing the window right now.
    pub(crate) resizing: Option<crate::ui::shell::resize::Resize>,
    /// True while the window fills the monitor's work area.
    pub(crate) maximized: bool,
    /// Where to put the window back when it is restored.
    restore_geometry: Option<(egui::Vec2, egui::Pos2)>,
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
        let tray = if settings.close_to_tray {
            crate::tray::Tray::new()
        } else {
            None
        };
        let window = crate::tray::AppWindow::find();
        Self {
            manager,
            add: ProbeForm::new(&settings),
            settings,
            bridge,
            logo,
            page: Page::Downloads,
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
            tray,
            hidden: false,
            window,
            parked_at: None,
            quitting: false,
            resizing: None,
            maximized: false,
            restore_geometry: None,
        }
    }

    /// Maximizes to the monitor work area by hand: a native maximize of a
    /// frameless window keeps the framed client size and leaves a dead band.
    pub(crate) fn toggle_maximize(&mut self, ctx: &egui::Context) {
        if self.maximized {
            if let Some((size, position)) = self.restore_geometry.take() {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
            }
            self.maximized = false;
            return;
        }
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some((x, y, width, height)) = window.work_area() else {
            return;
        };
        let size = ctx.screen_rect().size();
        let position = ctx
            .input(|input| input.viewport().outer_rect)
            .map_or(egui::Pos2::ZERO, |rect| rect.min);
        self.restore_geometry = Some((size, position));
        self.maximized = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            width as f32,
            height as f32,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            x as f32, y as f32,
        )));
    }

    pub(crate) fn notify(&mut self, message: impl Into<String>, bad: bool) {
        self.notice = Some((message.into(), bad, Instant::now()));
    }

    pub(crate) fn go(&mut self, page: Page) {
        self.page = page;
    }

    /// Creates or drops the tray icon to match the setting.
    pub(crate) fn sync_tray(&mut self) {
        if self.settings.close_to_tray {
            if self.tray.is_none() {
                self.tray = crate::tray::Tray::new();
                if self.tray.is_none() {
                    self.notify("The tray icon could not be created here.", true);
                }
            }
        } else {
            self.tray = None;
        }
    }

    /// A close hides the window when the setting asks for the tray.
    fn handle_close(&mut self, ctx: &egui::Context) {
        if self.quitting || !self.settings.close_to_tray {
            return;
        }
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        self.sync_tray();
        if self.tray.is_none() {
            // Without an icon there would be no way back, so let it close.
            return;
        }
        if self.window.is_none() {
            self.window = crate::tray::AppWindow::find();
        }
        let Some(window) = self.window.as_ref() else {
            self.notify("This window couldn't be tucked away.", true);
            return;
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        // Parked, not hidden: a hidden winit window stops drawing, and the
        // frame that draws is also the frame that pumps the queue.
        self.parked_at = window.park();
        self.hidden = true;
    }

    /// Answers the tray icon: show the window again, or leave for good.
    fn handle_tray(&mut self, ctx: &egui::Context) {
        match crate::tray::poll() {
            Some(crate::tray::TrayEvent::Show) => {
                self.hidden = false;
                if self.window.is_none() {
                    self.window = crate::tray::AppWindow::find();
                }
                match self.window.as_ref() {
                    Some(window) => window.restore(self.parked_at.take()),
                    None => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                }
            }
            Some(crate::tray::TrayEvent::Quit) => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            None => {}
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
        self.resize_edges(ctx);
        // winit keeps the caption and resize styles on a frameless window and only
        // hides the non-client area in WM_NCCALCSIZE. Windows still lays the drawn
        // surface out inside that frame, which shifted and clipped the interface, so
        // the styles are stripped here (a no-op once they are already gone).
        if let Some(window) = self.window.as_ref() {
            window.strip_frame();
        }
        self.handle_tray(ctx);
        self.handle_close(ctx);
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

        if self.hidden {
            // Nothing is drawn, but the queue, the workers and the bridge run.
            if self.dirty && self.saved_at.elapsed() > Duration::from_millis(800) {
                store::save(&self.settings);
                self.dirty = false;
                self.saved_at = Instant::now();
            }
            return;
        }

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
                        left: brand::PAGE_PAD,
                        right: brand::PAGE_PAD,
                        top: 14,
                        bottom: 12,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| pages::draw(self, ui));

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
