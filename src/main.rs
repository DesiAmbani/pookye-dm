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

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use dialog::ProbeForm;
use engine::{format_bytes, format_speed, Manager, Status};

fn main() -> eframe::Result {
    let settings = store::load();
    let bridge = Arc::new(api::Bridge::new());
    api::spawn(bridge.clone(), engine::VERSION.to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1220.0, 800.0])
            .with_min_inner_size([960.0, 640.0])
            .with_decorations(false)
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
    fn new(
        cc: &eframe::CreationContext<'_>,
        settings: store::Settings,
        bridge: Arc<api::Bridge>,
    ) -> Self {
        brand::apply_theme(&cc.egui_ctx);
        let logo = brand::load_logo(&cc.egui_ctx);
        let manager = Manager::new(settings.max_concurrent, settings.effective_rate() / 1024);
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

    // ------------------------------------------------------------ chrome

    fn title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(54.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(16, 10))
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let left = Rect::from_min_max(bar.min, Pos2::new(bar.left() + 300.0, bar.max.y));
                let right = Rect::from_min_max(
                    Pos2::new(bar.right() - 118.0, bar.top()),
                    bar.max,
                );
                let drag = Rect::from_min_max(
                    Pos2::new(left.right(), bar.top()),
                    Pos2::new(right.left(), bar.max.y),
                );

                let drag_response = ui.interact(drag, egui::Id::new("titlebar-drag"), Sense::click_and_drag());
                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_response.double_clicked() {
                    let maximized = ctx.input(|input| input.viewport().maximized).unwrap_or(false);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left));
                left_ui.horizontal_centered(|ui| {
                    let (mark, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    brand::radial_glow(
                        ui.painter(),
                        mark.center(),
                        27.0,
                        Color32::from_rgba_unmultiplied(255, 54, 124, 62),
                    );
                    if let Some(logo) = &self.logo {
                        ui.painter().image(
                            logo.id(),
                            mark,
                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("Pookye DM")
                                .font(brand::semi(14.5))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new("Download Manager")
                                .size(10.2)
                                .color(brand::DIM),
                        );
                    });
                });

                let mut right_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(right)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                right_ui.spacing_mut().item_spacing.x = 6.0;
                if window_button(&mut right_ui, WindowGlyph::Close) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                let maximized = ctx.input(|input| input.viewport().maximized).unwrap_or(false);
                if window_button(&mut right_ui, WindowGlyph::Maximize(maximized)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if window_button(&mut right_ui, WindowGlyph::Minimize) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(232.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 14,
                        right: 2,
                        top: 2,
                        bottom: 12,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                let height = ui.available_height();
                egui::Frame::new()
                    .fill(brand::glass())
                    .stroke(Stroke::new(1.0_f32, brand::hairline()))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(height - 24.0);

                        if widgets::pill_sized(
                            ui,
                            "+  New download",
                            widgets::Pill::Primary,
                            ui.available_width(),
                        ) {
                            self.open_add();
                        }

                        ui.add_space(12.0);
                        widgets::section_label(ui, "Downloads");
                        ui.add_space(2.0);
                        for (filter, icon) in [
                            (Filter::All, widgets::Icon::Download),
                            (Filter::Active, widgets::Icon::Speed),
                            (Filter::Finished, widgets::Icon::Check),
                            (Filter::Failed, widgets::Icon::Alert),
                        ] {
                            let count = self.count_of(filter);
                            let active = self.page == Page::Downloads && self.filter == filter;
                            if widgets::nav(ui, icon, filter.label(), active, Some(count)) {
                                self.filter = filter;
                                self.go(Page::Downloads);
                            }
                        }

                        ui.add_space(14.0);
                        widgets::section_label(ui, "App");
                        ui.add_space(2.0);
                        if widgets::nav(
                            ui,
                            widgets::Icon::Settings,
                            "Settings",
                            self.page == Page::Settings,
                            None,
                        ) {
                            self.go(Page::Settings);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Clock,
                            "Schedule",
                            self.page == Page::Schedule,
                            Some(self.manager.queued_count()),
                        ) {
                            self.go(Page::Schedule);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Speed,
                            "Speed",
                            self.page == Page::Speed,
                            None,
                        ) {
                            self.go(Page::Speed);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Network,
                            "Networks",
                            self.page == Page::Networks,
                            None,
                        ) {
                            self.go(Page::Networks);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Info,
                            "About",
                            self.page == Page::About,
                            None,
                        ) {
                            self.go(Page::About);
                        }

                        // Push the footer to the bottom of the sheet.
                        let remaining = ui.available_height();
                        if remaining > 92.0 {
                            ui.add_space(remaining - 92.0);
                        }
                        widgets::divider(ui);
                        ui.add_space(6.0);
                        {
                            ui.horizontal(|ui| {
                                let (dot, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                ui.painter().circle_filled(
                                    dot.center(),
                                    3.0,
                                    if self.manager.downloading_count() > 0 {
                                        brand::ACCENT_BRIGHT
                                    } else {
                                        brand::DIM
                                    },
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} active · {} queued",
                                        self.manager.downloading_count(),
                                        self.manager.queued_count()
                                    ))
                                    .size(10.4)
                                    .color(brand::DIM),
                                );
                                ui.with_layout(
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} done",
                                                self.manager.completed_count()
                                            ))
                                            .size(10.4)
                                            .color(brand::DIM),
                                        );
                                    },
                                );
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.label(
                                        RichText::new("Slow mode")
                                            .font(brand::medium(11.2))
                                            .color(brand::MUTED),
                                    );
                                    ui.label(
                                        RichText::new(format_speed(
                                            self.settings.slow_mode_speed as f64,
                                        ))
                                        .size(9.8)
                                        .color(brand::DIM),
                                    );
                                });
                                ui.with_layout(
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        if widgets::switch(ui, self.settings.slow_mode).clicked()
                                        {
                                            self.settings.slow_mode =
                                                !self.settings.slow_mode;
                                            self.dirty = true;
                                            self.apply_limits();
                                        }
                                    },
                                );
                            });
                        }
                    });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 22,
                        right: 22,
                        top: 4,
                        bottom: 10,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let speed = self.manager.total_speed();
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        3.0,
                        if speed > 0.0 {
                            brand::ACCENT_BRIGHT
                        } else {
                            brand::DIM
                        },
                    );
                    ui.label(
                        RichText::new(if speed > 0.0 {
                            format!("Average speed {}", format_speed(speed))
                        } else {
                            "Idle".to_string()
                        })
                        .size(10.8)
                        .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} downloaded · {} streams per download",
                            format_bytes(self.manager.total_downloaded()),
                            self.settings.connections
                        ))
                        .size(10.8)
                        .color(brand::DIM),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let awaiting = self.bridge.awaiting_count();
                        match self.bridge.port() {
                            Some(port) if awaiting > 0 => ui.label(
                                RichText::new(format!(
                                    "Browser bridge · {port} · {awaiting} to confirm"
                                ))
                                .size(10.2)
                                .color(brand::DIM),
                            ),
                            Some(port) => ui.label(
                                RichText::new(format!("Browser bridge · {port}"))
                                    .size(10.2)
                                    .color(brand::DIM),
                            ),
                            None => ui.label(
                                RichText::new("Browser bridge offline")
                                    .size(10.2)
                                    .color(brand::DIM),
                            ),
                        };
                        ui.label(
                            RichText::new(self.settings.folder.as_str())
                                .size(10.2)
                                .color(brand::DIM),
                        );
                    });
                });
            });
    }

    /// Toast: a small glass pill that fades in above the status bar.
    fn toast(&mut self, ctx: &egui::Context) {
        let Some((message, bad, at)) = self.notice.clone() else {
            return;
        };
        let age = at.elapsed().as_secs_f32();
        let life = 4.2;
        if age > life {
            self.notice = None;
            return;
        }
        let alpha = if age < 0.2 {
            age / 0.2
        } else if age > life - 0.6 {
            ((life - age) / 0.6).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let lift = (1.0 - alpha) * 10.0;
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -52.0 + lift))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let tint = if bad {
                    Color32::from_rgb(0xFF, 0x8F, 0x9E)
                } else {
                    brand::INK
                };
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(
                        28,
                        2,
                        17,
                        (218.0 * alpha) as u8,
                    ))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 230, 240, (44.0 * alpha) as u8),
                    ))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(Margin::symmetric(18, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 28,
                        spread: 0,
                        color: Color32::from_black_alpha((120.0 * alpha) as u8),
                    })
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(message)
                                .font(brand::medium(12.0))
                                .color(tint.gamma_multiply(alpha.max(0.1))),
                        );
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    /// Frames without a native border need their own resize edges.
    fn resize_edges(ctx: &egui::Context) {
        let Some(pointer) = ctx.pointer_latest_pos() else {
            return;
        };
        let rect = ctx.screen_rect();
        let edge = 6.0;
        let left = pointer.x <= rect.left() + edge;
        let right = pointer.x >= rect.right() - edge;
        let top = pointer.y <= rect.top() + edge;
        let bottom = pointer.y >= rect.bottom() - edge;
        let direction = match (left, right, top, bottom) {
            (true, _, true, _) => Some((egui::ResizeDirection::NorthWest, egui::CursorIcon::ResizeNwSe)),
            (_, true, true, _) => Some((egui::ResizeDirection::NorthEast, egui::CursorIcon::ResizeNeSw)),
            (true, _, _, true) => Some((egui::ResizeDirection::SouthWest, egui::CursorIcon::ResizeNeSw)),
            (_, true, _, true) => Some((egui::ResizeDirection::SouthEast, egui::CursorIcon::ResizeNwSe)),
            (true, _, _, _) => Some((egui::ResizeDirection::West, egui::CursorIcon::ResizeHorizontal)),
            (_, true, _, _) => Some((egui::ResizeDirection::East, egui::CursorIcon::ResizeHorizontal)),
            (_, _, true, _) => Some((egui::ResizeDirection::North, egui::CursorIcon::ResizeVertical)),
            (_, _, _, true) => Some((egui::ResizeDirection::South, egui::CursorIcon::ResizeVertical)),
            _ => None,
        };
        if let Some((direction, cursor)) = direction {
            ctx.set_cursor_icon(cursor);
            if ctx.input(|input| input.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
            }
        }
    }
}

#[derive(Clone, Copy)]
enum WindowGlyph {
    Minimize,
    Maximize(bool),
    Close,
}

fn window_button(ui: &mut egui::Ui, glyph: WindowGlyph) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(32.0, 26.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    let closing = matches!(glyph, WindowGlyph::Close);
    let fill = if closing {
        brand::mix(Color32::TRANSPARENT, Color32::from_rgb(0xE8, 0x11, 0x23), hover)
    } else {
        brand::mix(Color32::TRANSPARENT, brand::sheen(30), hover)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(8), fill);
    let color = if closing && hover > 0.5 {
        Color32::WHITE
    } else {
        brand::mix(brand::MUTED, brand::INK, hover)
    };
    let stroke = Stroke::new(1.15_f32, color);
    let c = rect.center() - Vec2::new(0.5, 0.5);
    match glyph {
        WindowGlyph::Minimize => {
            painter.line_segment([c + Vec2::new(-5.0, 3.0), c + Vec2::new(5.0, 3.0)], stroke);
        }
        WindowGlyph::Maximize(maximized) => {
            if maximized {
                painter.rect_stroke(
                    Rect::from_center_size(c + Vec2::new(0.5, -0.5), Vec2::splat(8.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
                painter.rect_stroke(
                    Rect::from_center_size(c + Vec2::new(-0.5, 0.5), Vec2::splat(8.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
            } else {
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(9.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
            }
        }
        WindowGlyph::Close => {
            painter.line_segment(
                [c + Vec2::new(-4.5, -4.5), c + Vec2::new(4.5, 4.5)],
                stroke,
            );
            painter.line_segment(
                [c + Vec2::new(4.5, -4.5), c + Vec2::new(-4.5, 4.5)],
                stroke,
            );
        }
    }
    response.clicked()
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(120));
        Self::resize_edges(ctx);
        brand::paint_background(ctx);

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
