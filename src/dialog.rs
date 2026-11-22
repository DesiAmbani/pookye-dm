//! The New download dialog: link, probe result, name, folder and streams.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32, RichText};

use crate::store;
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

pub(crate) fn draw(app: &mut App, ctx: &egui::Context) {
    if !app.show_add {
        return;
    }
    let mut open = true;
    let mut close = false;
    let mut check = false;
    let mut start: Option<(String, Option<String>)> = None;

    egui::Window::new("New download")
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Paste a link to a file")
                    .size(11.5)
                    .color(brand::MUTED),
            );
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 120.0).max(120.0);
                ui.add_sized(
                    [width, 30.0],
                    egui::TextEdit::singleline(&mut app.add.url)
                        .hint_text("https://example.com/file.iso"),
                );
                if ui
                    .add_sized(
                        [110.0, 30.0],
                        egui::Button::new(if app.add.checking {
                            "Checking…"
                        } else {
                            "Check link"
                        }),
                    )
                    .clicked()
                {
                    check = true;
                }
            });

            if app.add.checking {
                ui.label(
                    RichText::new("Checking the link…")
                        .size(11.0)
                        .color(brand::MUTED),
                );
            } else if let Some(error) = &app.add.error {
                ui.label(
                    RichText::new(error)
                        .size(11.0)
                        .color(brand::ACCENT_BRIGHT),
                );
            } else if let Some(info) = &app.add.info {
                let size = info
                    .total
                    .map(engine::format_bytes)
                    .unwrap_or_else(|| "unknown size".to_string());
                let ranges = if info.ranges {
                    "parallel streams available"
                } else {
                    "the server sends it in one piece"
                };
                let media = if info.is_video { " · media stream" } else { "" };
                ui.label(
                    RichText::new(format!("{size} · {ranges}{media}"))
                        .size(11.0)
                        .color(brand::MUTED),
                );
            } else {
                ui.label(
                    RichText::new("Known once the link is checked")
                        .size(11.0)
                        .color(brand::MUTED),
                );
            }

            ui.add_space(6.0);
            egui::Grid::new("add-fields")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("File name").size(11.5).color(brand::MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut app.add.name)
                            .desired_width(400.0)
                            .hint_text("Taken from the link"),
                    );
                    ui.end_row();
                    ui.label(
                        RichText::new("Save this download to")
                            .size(11.5)
                            .color(brand::MUTED),
                    );
                    ui.add(egui::TextEdit::singleline(&mut app.add.folder).desired_width(400.0));
                    ui.end_row();
                    ui.label(
                        RichText::new("Streams per download")
                            .size(11.5)
                            .color(brand::MUTED),
                    );
                    ui.add(egui::Slider::new(&mut app.add.connections, 1..=32));
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let primary = egui::Button::new(
                    RichText::new("Start download")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(brand::ACCENT);
                if ui.add(primary).clicked() {
                    start = Some((app.add.url.clone(), Some(app.add.name.clone())));
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });

    if check {
        app.check_link();
    }
    if let Some((url, name)) = start {
        let folder = PathBuf::from(app.add.folder.trim());
        let name = name.filter(|name| !name.trim().is_empty());
        match app.manager.add(url, name, &folder, app.add.connections) {
            Ok(_) => {
                app.settings.folder = app.add.folder.clone();
                app.settings.connections = app.add.connections;
                app.dirty = true;
                close = true;
                app.page = Page::Downloads;
                app.notify("Added to the queue.", false);
            }
            Err(message) => app.add.error = Some(message),
        }
    }
    if close || !open {
        app.show_add = false;
    }
}
