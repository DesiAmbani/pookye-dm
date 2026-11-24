//! The New download dialog: link, probe result, name, folder and streams.
//!
//! Drawn as a frameless sheet over a dimmed backdrop so it reads like a panel
//! over the window rather than a separate OS window.

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
    /// Streams per selected network; 0 means auto (eight).
    pub(crate) per_network: usize,
    /// One flag per adapter in `App::adapters`.
    pub(crate) networks: Vec<bool>,
    pub(crate) checking: bool,
    pub(crate) rx: Option<Receiver<Result<engine::ProbeInfo, String>>>,
    pub(crate) info: Option<engine::ProbeInfo>,
    pub(crate) error: Option<String>,
}

impl ProbeForm {
    pub(crate) fn new(settings: &store::Settings, adapters: &[engine::Adapter]) -> Self {
        let networks = adapters
            .iter()
            .map(|adapter| {
                settings.networks.is_empty()
                    || settings.networks.iter().any(|ip| ip == &adapter.ip.to_string())
            })
            .collect();
        Self {
            url: String::new(),
            name: String::new(),
            folder: settings.folder.clone(),
            per_network: settings.connections,
            networks,
            checking: false,
            rx: None,
            info: None,
            error: None,
        }
    }
}

/// A sunken input well: near-black with a hairline, so text sits *in* the sheet.
fn field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> egui::Response {
    let frame = Frame::new()
        .fill(Color32::from_rgb(0x0E, 0x0D, 0x0F))
        .stroke(Stroke::new(1.0_f32, brand::hairline()))
        .corner_radius(CornerRadius::same(brand::R_PILL))
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
        brand::mix(Color32::TRANSPARENT, brand::ROSE, hover),
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
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -16.0 + (1.0 - fade) * 14.0))
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

            ui.add_space(12.0);
            widgets::divider(ui);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                widgets::section_label(ui, "Networks");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let chosen = app.add.networks.iter().filter(|on| **on).count();
                    ui.label(
                        RichText::new(format!("{chosen} of {} selected", app.adapters.len()))
                            .size(10.2)
                            .color(brand::DIM),
                    );
                });
            });
            ui.add_space(4.0);
            if app.adapters.is_empty() {
                ui.label(
                    RichText::new("No adapters found; the default route carries the download.")
                        .size(10.8)
                        .color(brand::DIM),
                );
            }
            // Bounded so a machine with many adapters scrolls instead of pushing
            // the sheet past the bottom of a small window.
            egui::ScrollArea::vertical()
                .id_salt("add-networks")
                .max_height(132.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for index in 0..app.adapters.len() {
                        let adapter = app.adapters[index].clone();
                        let selected = app.add.networks.get(index).copied().unwrap_or(false);
                        Frame::new()
                            .fill(if selected {
                                brand::ROSE_DEEP
                            } else {
                                brand::SURFACE_RAISED
                            })
                            .stroke(Stroke::new(
                                1.0_f32,
                                if selected { brand::ROSE } else { brand::hairline() },
                            ))
                            .corner_radius(CornerRadius::same(brand::R_TILE))
                            .inner_margin(Margin::symmetric(10, 5))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let mut on = selected;
                                    if ui.checkbox(&mut on, "").changed() {
                                        app.add.networks[index] = on;
                                    }
                                    ui.label(
                                        RichText::new(&adapter.name)
                                            .font(brand::medium(11.6))
                                            .color(brand::INK),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(adapter.ip.to_string())
                                                    .size(9.8)
                                                    .color(brand::DIM),
                                            );
                                        },
                                    );
                                });
                            })
                            .response
                            .on_hover_text(adapter.label());
                        ui.add_space(3.0);
                    }
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new("Streams per network")
                    .size(11.2)
                    .color(brand::MUTED),
            );
            ui.add_space(3.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for option in [0usize, 4, 8, 16, 32] {
                    let label = if option == 0 {
                        "Auto".to_string()
                    } else {
                        option.to_string()
                    };
                    if widgets::chip(ui, &label, app.add.per_network == option) {
                        app.add.per_network = option;
                    }
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Eight per network when left on Auto.")
                        .size(9.6)
                        .color(brand::DIM),
                );
            });

            ui.add_space(14.0);
            widgets::divider(ui);
            ui.add_space(10.0);
            let chosen = app.add.networks.iter().filter(|on| **on).count().max(1);
            let per = if app.add.per_network == 0 {
                8
            } else {
                app.add.per_network
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} network{} · {} streams",
                        chosen,
                        if chosen == 1 { "" } else { "s" },
                        chosen * per
                    ))
                    .size(10.4)
                    .color(brand::DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::pill(ui, "Start download", Pill::Primary) {
                        start = Some((app.add.url.clone(), Some(app.add.name.clone())));
                    }
                    if widgets::pill(ui, "Cancel", Pill::Glass) {
                        close = true;
                    }
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
        let mut binds = Vec::new();
        let mut labels = Vec::new();
        let mut chosen = Vec::new();
        for (index, adapter) in app.adapters.iter().enumerate() {
            if app.add.networks.get(index).copied().unwrap_or(false) {
                binds.push(Some(std::net::IpAddr::V4(adapter.ip)));
                labels.push(adapter.label());
                chosen.push(adapter.ip.to_string());
            }
        }
        let per = if app.add.per_network == 0 {
            8
        } else {
            app.add.per_network
        };
        match app.manager.add_bound(
            url,
            name,
            &folder,
            per,
            binds,
            labels,
            crate::engine::RequestContext::default(),
        ) {
            Ok(_) => {
                app.settings.folder = app.add.folder.clone();
                app.settings.connections = per;
                app.settings.networks = chosen;
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
