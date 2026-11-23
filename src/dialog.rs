//! The New download dialog: link, probe result, name, folder and streams.
//!
//! Drawn as a frameless glass sheet over a dimmed backdrop so it reads like a
//! native sheet sliding over the window rather than a separate OS window.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2,
};

use crate::store;
use crate::widgets::{self, Pill};
use crate::{brand, engine, App, Page};

pub(crate) struct ProbeForm {
    pub(crate) url: String,
    pub(crate) name: String,
    pub(crate) folder: String,
    pub(crate) connections: usize,
    pub(crate) checking: bool,
    pub(crate) rx: Option<Receiver<Result<engine::ProbeInfo, String>>>,
    pub(crate) info: Option<engine::ProbeInfo>,
    pub(crate) error: Option<String>,
}

impl ProbeForm {
    pub(crate) fn new(settings: &store::Settings) -> Self {
        Self {
            url: String::new(),
            name: String::new(),
            folder: settings.folder.clone(),
            connections: settings.connections,
            checking: false,
            rx: None,
            info: None,
            error: None,
        }
    }
}

/// A sunken input well: dark glass with a hairline, so text sits *in* the sheet.
fn field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> egui::Response {
    let frame = Frame::new()
        .fill(Color32::from_rgba_unmultiplied(6, 0, 4, 110))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(255, 226, 238, 26),
        ))
        .corner_radius(CornerRadius::same(11))
        .inner_margin(Margin::symmetric(12, 9));
    frame
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .frame(false)
                    .hint_text(RichText::new(hint).color(brand::DIM))
                    .desired_width(width),
            )
        })
        .inner
}

fn close_button(ui: &mut Ui) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = ui.ctx().animate_bool_with_time(
        response.id.with("hover"),
        response.hovered(),
        0.13,
    );
    let painter = ui.painter();
    painter.circle_filled(
        rect.center(),
        13.0,
        brand::mix(Color32::TRANSPARENT, brand::sheen(38), hover),
    );
    let c = rect.center();
    let ink = brand::mix(brand::MUTED, brand::INK, hover);
    let d = 3.4;
    painter.line_segment(
        [c + Vec2::new(-d, -d), c + Vec2::new(d, d)],
        Stroke::new(1.4_f32, ink),
    );
    painter.line_segment(
        [c + Vec2::new(d, -d), c + Vec2::new(-d, d)],
        Stroke::new(1.4_f32, ink),
    );
    response.clicked()
}

pub(crate) fn draw(app: &mut App, ctx: &egui::Context) {
    if !app.show_add {
        return;
    }

    let fade = ctx.animate_bool_with_time(egui::Id::new("add-fade"), true, 0.22);
    if fade <= 0.001 {
        ctx.request_repaint();
    }

    // Dim the window behind the sheet.
    let screen = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("add-dim"),
    ))
    .rect_filled(
        screen,
        CornerRadius::ZERO,
        Color32::from_black_alpha((110.0 * fade) as u8),
    );

    let mut close = false;
    let mut check = false;
    let mut start: Option<(String, Option<String>)> = None;

    egui::Window::new("new-download")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size(Vec2::new(568.0, 0.0))
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -30.0 + (1.0 - fade) * 14.0))
        .frame(brand::window_frame())
        .show(ctx, |ui| {
            ui.set_width(528.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.label(
                        RichText::new("New download")
                            .font(brand::semi(19.0))
                            .color(brand::INK),
                    );
                    ui.label(
                        RichText::new("Paste a link to a file and choose where it lands.")
                            .size(11.5)
                            .color(brand::DIM),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if close_button(ui) {
                        close = true;
                    }
                });
            });

            ui.add_space(14.0);
            widgets::section_label(ui, "Link");
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let button = 96.0;
                let width = (ui.available_width() - button - 10.0).max(120.0);
                let response = field(ui, &mut app.add.url, "https://example.com/file.iso", width);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    check = true;
                }
                let label = if app.add.checking {
                    "Checking…"
                } else {
                    "Check link"
                };
                if widgets::pill_sized(ui, label, Pill::Glass, button) {
                    check = true;
                }
            });

            ui.add_space(6.0);
            let (line, colour) = if app.add.checking {
                ("Reaching the server…".to_string(), brand::MUTED)
            } else if let Some(error) = &app.add.error {
                (error.clone(), brand::ACCENT_BRIGHT)
            } else if let Some(info) = &app.add.info {
                let size = info
                    .total
                    .map(engine::format_bytes)
                    .unwrap_or_else(|| "unknown size".to_string());
                let ranges = if info.ranges {
                    "parallel streams available"
                } else {
                    "single stream only"
                };
                let media = if info.is_video { " · media stream" } else { "" };
                (format!("{size} · {ranges}{media}"), brand::MUTED)
            } else {
                (
                    "Size and streams show up once the link is checked.".to_string(),
                    brand::DIM,
                )
            };
            ui.label(RichText::new(line).size(11.2).color(colour));

            ui.add_space(12.0);
            widgets::divider(ui);
            ui.add_space(12.0);

            widgets::section_label(ui, "Destination");
            ui.add_space(2.0);
            ui.label(RichText::new("File name").size(11.2).color(brand::MUTED));
            field(ui, &mut app.add.name, "Taken from the link", 480.0);
            ui.add_space(6.0);
            ui.label(RichText::new("Save in").size(11.2).color(brand::MUTED));
            field(ui, &mut app.add.folder, "Folder path", 480.0);

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Streams per download")
                        .size(11.2)
                        .color(brand::MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Slider::new(&mut app.add.connections, 1..=32)
                            .show_value(true)
                            .trailing_fill(true),
                    );
                });
            });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if widgets::pill(ui, "Start download", Pill::Primary) {
                    start = Some((app.add.url.clone(), Some(app.add.name.clone())));
                }
                if widgets::pill(ui, "Cancel", Pill::Glass) {
                    close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Enter to check · Esc to close")
                            .size(10.2)
                            .color(brand::DIM),
                    );
                });
            });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if check {
        app.check_link();
    }
    if let Some((url, name)) = start {
        let folder = PathBuf::from(app.add.folder.trim());
        let name = name.filter(|name| !name.trim().is_empty());
        match app
            .manager
            .add(
                url,
                name,
                &folder,
                app.add.connections,
                crate::engine::RequestContext::default(),
            )
        {
            Ok(_) => {
                app.settings.folder = app.add.folder.clone();
                app.settings.connections = app.add.connections;
                app.dirty = true;
                app.go(Page::Downloads);
                app.notify("Added to the queue.", false);
                close = true;
            }
            Err(message) => app.add.error = Some(message),
        }
    }
    if close {
        app.show_add = false;
    }
}
