//! The six pages reachable from the sidebar.

use eframe::egui::{self, Color32, RichText};
use engine::{format_bytes, format_speed, Category, Status};

use crate::actions::{self, Action};
use crate::widgets::{self as w, card, chip, host_of, metric, page_heading};
use crate::{brand, engine, App, Page};

const REPO_URL: &str = "https://github.com/DesiAmbani/pookye-dm";
const UPSTREAM_URL: &str = "https://github.com/PBhadoo/QDM";

pub(crate) fn draw(app: &mut App, ui: &mut egui::Ui) {
    match app.page {
        Page::Downloads => downloads(app, ui),
        Page::Speed => speed(app, ui),
        Page::Networks => networks(app, ui),
        Page::Schedule => schedule(app, ui),
        Page::Settings => settings(app, ui),
        Page::About => about(app, ui),
    }
}

fn downloads(app: &mut App, ui: &mut egui::Ui) {
    let mut actions: Vec<Action> = Vec::new();

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            page_heading(
                ui,
                "Downloads",
                "Paste a link to a file: Pookye DM splits it across parallel streams.",
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let primary = egui::Button::new(
                RichText::new("+ New download")
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(brand::ACCENT);
            if ui.add(primary).clicked() {
                app.open_add();
            }
            if ui.small_button("Clear list").clicked() {
                actions.push(Action::ClearList);
            }
            if ui.small_button("Resume all").clicked() {
                actions.push(Action::ResumeAll);
            }
            if ui.small_button("Pause all").clicked() {
                actions.push(Action::PauseAll);
            }
        });
    });

    let mut paste = false;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let width = (ui.available_width() - 140.0).max(120.0);
            let edit = ui.add_sized(
                [width, 30.0],
                egui::TextEdit::singleline(&mut app.url_input)
                    .hint_text("Paste a link to a file"),
            );
            let start = ui
                .add_sized(
                    [130.0, 30.0],
                    egui::Button::new(
                        RichText::new("Start download")
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(brand::ACCENT),
                )
                .clicked();
            if start || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                paste = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Save to").size(11.0).color(brand::MUTED));
            ui.label(
                RichText::new(app.settings.folder.as_str())
                    .size(11.0)
                    .color(brand::INK),
            );
            ui.label(
                RichText::new(format!("· {} streams", app.settings.connections))
                    .size(11.0)
                    .color(brand::MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Change").clicked() {
                    app.page = Page::Settings;
                }
            });
        });
    });
    if paste {
        app.start_pasted();
    }

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.search)
                .hint_text("Search downloads")
                .desired_width(180.0),
        );
        if chip(ui, "All", app.category.is_none()) {
            app.category = None;
        }
        for category in Category::ALL {
            if chip(ui, category.label(), app.category == Some(category)) {
                app.category = Some(category);
            }
        }
    });

    app.notice_line(ui);
    ui.add_space(4.0);

    if !app.selected.is_empty() {
        let count = app.selected.len();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{count} selected"))
                    .size(11.0)
                    .color(brand::INK),
            );
            if ui.small_button("Pause").clicked() {
                actions.push(Action::PauseSelected);
            }
            if ui.small_button("Resume").clicked() {
                actions.push(Action::ResumeSelected);
            }
            if ui.small_button("Remove").clicked() {
                actions.push(Action::RemoveSelected);
            }
            if ui.small_button("Clear selection (Esc)").clicked() {
                app.selected.clear();
            }
        });
    }

    let rows = app.filtered_rows();
    let empty = app.manager.jobs.is_empty();
    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.label(
                RichText::new(if empty { "Nothing downloading yet" } else { "Nothing here" })
                    .size(17.0)
                    .color(brand::MUTED),
            );
            ui.label(
                RichText::new(if empty {
                    "Paste a link to a file, or let the browser extension hand one over."
                } else {
                    "No download matches this search or category."
                })
                .size(12.0)
                .color(brand::MUTED),
            );
            if !empty {
                ui.add_space(6.0);
                if ui.button("Show all downloads").clicked() {
                    app.category = None;
                    app.search.clear();
                }
            }
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for id in rows {
                    if let Some(job) = app.manager.jobs.iter().find(|job| job.id == id) {
                        let selected = app.selected.contains(&id);
                        if let Some(action) = w::download_row(ui, job, selected) {
                            actions.push(action);
                        }
                    }
                }
                if !app.settings.history.is_empty() {
                    ui.add_space(6.0);
                    ui.collapsing(
                        RichText::new(format!(
                            "Recent downloads ({})",
                            app.settings.history.len()
                        ))
                        .size(11.5)
                        .color(brand::MUTED),
                        |ui| {
                            for entry in app.settings.history.iter().take(12) {
                                ui.label(
                                    RichText::new(format!(
                                        "{}  ·  {}",
                                        entry.file_name,
                                        format_bytes(entry.bytes)
                                    ))
                                    .size(10.5)
                                    .color(brand::MUTED),
                                );
                            }
                        },
                    );
                }
            });
    }

    for action in actions {
        actions::apply(app, action);
    }
}

fn speed(app: &mut App, ui: &mut egui::Ui) {
    page_heading(ui, "Speed", "All downloads together, over every connection.");

    let mut limit_changed = false;
    card(ui, |ui| {
        ui.label(RichText::new("System-wide limit").strong().color(brand::INK));
        ui.label(
            RichText::new("A single budget shared by every stream of every download.")
                .size(11.0)
                .color(brand::MUTED),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Unlimited").fill(if app.settings.speed_limit == 0 {
                    brand::ACCENT
                } else {
                    brand::PANEL
                }))
                .clicked()
            {
                app.settings.speed_limit = 0;
                limit_changed = true;
            }
            for megabytes in [1usize, 5, 10, 50] {
                let kbps = megabytes * 1024;
                let active = app.settings.speed_limit == kbps;
                if ui
                    .add(egui::Button::new(format!("{megabytes} MB/s")).fill(if active {
                        brand::ACCENT
                    } else {
                        brand::PANEL
                    }))
                    .clicked()
                {
                    app.settings.speed_limit = kbps;
                    limit_changed = true;
                }
            }
        });
        let mut slider = app.settings.speed_limit as u64;
        if ui
            .add(egui::Slider::new(&mut slider, 0..=102_400).text("KB/s · 0 is unlimited"))
            .changed()
        {
            app.settings.speed_limit = slider as usize;
            limit_changed = true;
        }
        ui.label(
            RichText::new(if app.settings.speed_limit == 0 {
                "Currently unlimited".to_string()
            } else {
                format!(
                    "Currently limited to {} of traffic per second",
                    format_bytes(app.settings.speed_limit as u64 * 1024)
                )
            })
            .size(11.0)
            .color(brand::MUTED),
        );
    });
    if limit_changed {
        app.apply_limits();
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        metric(ui, "Average speed", format_speed(app.manager.total_speed()));
        metric(ui, "Downloaded", format_bytes(app.manager.total_downloaded()));
        metric(
            ui,
            "Streams",
            format!("{} per download", app.settings.connections),
        );
        metric(
            ui,
            "Concurrent",
            format!(
                "{} of {}",
                app.manager.downloading_count(),
                app.manager.max_concurrent
            ),
        );
    });

    ui.add_space(8.0);
    if app.manager.jobs.is_empty() {
        ui.label(
            RichText::new("No downloads to show yet.")
                .size(12.0)
                .color(brand::MUTED),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for job in &app.manager.jobs {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .size(12.5)
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format_speed(job.speed))
                                    .size(12.0)
                                    .color(w::status_colour(job.status)),
                            );
                        });
                    });
                    let share = if app.manager.total_speed() > 0.0 {
                        (job.speed / app.manager.total_speed()).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    ui.add(
                        egui::ProgressBar::new(share)
                            .desired_height(5.0)
                            .fill(brand::ACCENT_BRIGHT),
                    );
                });
            }
        });
}

fn networks(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Networks",
        "What each connection carries: every stream is an independent range request.",
    );

    let mut shown = 0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for job in &app.manager.jobs {
                let segments = job.progress.snapshot_segments();
                if segments.is_empty() {
                    continue;
                }
                shown += 1;
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .strong()
                                .size(12.5)
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} connections · {}",
                                    segments.len(),
                                    job.status.label()
                                ))
                                .size(10.5)
                                .color(brand::MUTED),
                            );
                        });
                    });
                    ui.add_space(2.0);
                    for (index, (length, done, start)) in segments.iter().enumerate() {
                        let part = if *length == 0 {
                            0.0
                        } else {
                            (*done as f32 / *length as f32).clamp(0.0, 1.0)
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Connection {}", index + 1))
                                    .size(10.5)
                                    .color(brand::MUTED),
                            );
                            let bar_width = (ui.available_width() - 160.0).max(80.0);
                            ui.add_sized(
                                [bar_width, 6.0],
                                egui::ProgressBar::new(part)
                                    .desired_height(6.0)
                                    .fill(brand::ACCENT_BRIGHT),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} / {}",
                                    format_bytes(*done),
                                    format_bytes(*length)
                                ))
                                .size(10.0)
                                .color(brand::MUTED),
                            );
                            ui.label(
                                RichText::new(format!("byte {}", format_bytes(*start)))
                                    .size(9.5)
                                    .color(brand::MUTED),
                            );
                        });
                    }
                });
                ui.add_space(6.0);
            }
            if shown == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(70.0);
                    ui.label(
                        RichText::new("No connections carrying traffic")
                            .size(16.0)
                            .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new("Start a download and its streams appear here.")
                            .size(11.5)
                            .color(brand::MUTED),
                    );
                });
            }
        });
}

fn schedule(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Schedule",
        "Downloads started outside the window wait in the queue.",
    );

    let mut pending: Vec<Action> = Vec::new();
    let mut concurrency_changed = false;
    card(ui, |ui| {
        ui.label(RichText::new("Downloads at once").strong().color(brand::INK));
        if ui
            .add(egui::Slider::new(&mut app.settings.max_concurrent, 1..=10))
            .changed()
        {
            concurrency_changed = true;
        }
        ui.label(
            RichText::new(format!(
                "Up to {} downloads run together; the rest wait their turn.",
                app.settings.max_concurrent
            ))
            .size(11.0)
            .color(brand::MUTED),
        );
    });
    if concurrency_changed {
        app.apply_limits();
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(format!("Waiting ({})", app.manager.queued_count()))
            .size(12.0)
            .strong()
            .color(brand::INK),
    );
    let queued: Vec<u64> = app
        .manager
        .jobs
        .iter()
        .filter(|job| job.status == Status::Queued)
        .map(|job| job.id)
        .collect();
    if queued.is_empty() {
        ui.label(
            RichText::new("Nothing waiting. Downloads run whenever they're started.")
                .size(11.5)
                .color(brand::MUTED),
        );
    } else {
        for id in queued {
            if let Some(job) = app.manager.jobs.iter().find(|job| job.id == id) {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .size(12.0)
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Remove").clicked() {
                                pending.push(Action::Remove(id));
                            }
                            if ui.small_button("Start now").clicked() {
                                pending.push(Action::Resume(id));
                            }
                            ui.label(
                                RichText::new(format!(
                                    "{} · {} streams",
                                    host_of(&job.url),
                                    job.connections
                                ))
                                .size(10.5)
                                .color(brand::MUTED),
                            );
                        });
                    });
                });
            }
        }
    }

    for action in pending {
        actions::apply(app, action);
    }
}

fn settings(app: &mut App, ui: &mut egui::Ui) {
    page_heading(ui, "Settings", "Where downloads go and how hard they push.");
    let mut changed = false;

    card(ui, |ui| {
        ui.label(RichText::new("Storage").strong().color(brand::INK));
        ui.horizontal(|ui| {
            ui.label(RichText::new("Download folder").size(11.5).color(brand::MUTED));
            if ui
                .add(
                    egui::TextEdit::singleline(&mut app.settings.folder)
                        .desired_width(f32::INFINITY),
                )
                .changed()
            {
                changed = true;
            }
        });
        ui.label(
            RichText::new("Downloads run from this folder. Missing folders are created.")
                .size(10.5)
                .color(brand::MUTED),
        );
    });

    ui.add_space(8.0);
    card(ui, |ui| {
        ui.label(RichText::new("Performance").strong().color(brand::INK));
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Segments per download")
                    .size(11.5)
                    .color(brand::MUTED),
            );
            if ui
                .add(egui::Slider::new(&mut app.settings.connections, 1..=32))
                .changed()
            {
                changed = true;
            }
            ui.label(
                RichText::new(format!("{} parallel connections", app.settings.connections))
                    .size(10.5)
                    .color(brand::MUTED),
            );
        });
        ui.label(
            RichText::new("Use one stream for servers that block parallel requests.")
                .size(10.5)
                .color(brand::MUTED),
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Downloads at once")
                    .size(11.5)
                    .color(brand::MUTED),
            );
            if ui
                .add(egui::Slider::new(&mut app.settings.max_concurrent, 1..=10))
                .changed()
            {
                changed = true;
            }
        });
    });

    ui.add_space(8.0);
    card(ui, |ui| {
        ui.label(RichText::new("Behaviour").strong().color(brand::INK));
        if ui
            .checkbox(
                &mut app.settings.clipboard,
                "Watch the clipboard for copied links",
            )
            .changed()
        {
            if app.settings.clipboard && app.clipboard.is_none() {
                app.clipboard = arboard::Clipboard::new().ok();
            }
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} entries in history", app.settings.history.len()))
                    .size(11.0)
                    .color(brand::MUTED),
            );
            if !app.settings.history.is_empty() && ui.small_button("Clear history").clicked() {
                app.settings.history.clear();
                changed = true;
            }
        });
    });

    if changed {
        app.apply_limits();
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new("Settings are saved automatically.")
            .size(10.5)
            .color(brand::MUTED),
    );
}

fn about(app: &mut App, ui: &mut egui::Ui) {
    page_heading(ui, "About", "Pookye Download Manager");
    card(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(logo) = &app.logo {
                ui.add(egui::Image::new(egui::load::SizedTexture::new(
                    logo.id(),
                    egui::vec2(64.0, 64.0),
                )));
            }
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Pookye Download Manager")
                        .size(17.0)
                        .strong()
                        .color(brand::INK),
                );
                ui.label(
                    RichText::new(format!("Version {} — alpha", engine::VERSION))
                        .size(11.5)
                        .color(brand::ACCENT_BRIGHT),
                );
                ui.label(
                    RichText::new("Multi-segment downloads, queues and browser handoff in Rust.")
                        .size(11.0)
                        .color(brand::MUTED),
                );
            });
        });
    });

    ui.add_space(8.0);
    card(ui, |ui| {
        egui::Grid::new("about-grid")
            .num_columns(2)
            .spacing([18.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("License").size(11.0).color(brand::MUTED));
                ui.label(RichText::new("MIT").size(11.0).color(brand::INK));
                ui.end_row();
                ui.label(RichText::new("Built with").size(11.0).color(brand::MUTED));
                ui.label(RichText::new("Rust · egui · reqwest").size(11.0).color(brand::INK));
                ui.end_row();
                ui.label(RichText::new("Source").size(11.0).color(brand::MUTED));
                ui.hyperlink_to("github.com/DesiAmbani/pookye-dm", REPO_URL);
                ui.end_row();
                ui.label(RichText::new("Model").size(11.0).color(brand::MUTED));
                ui.hyperlink_to("QDM · PBhadoo/QDM (MIT)", UPSTREAM_URL);
                ui.end_row();
                ui.label(RichText::new("Settings file").size(11.0).color(brand::MUTED));
                ui.label(
                    RichText::new(crate::store::config_dir().join("settings.json").to_string_lossy())
                        .size(10.5)
                        .color(brand::MUTED),
                );
                ui.end_row();
            });
    });

    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "Pookye DM is an independent Rust implementation of the multi-segment, \
             queue-driven download-manager model popularised by QDM. The engine, UI and \
             browser bridge here are original code.",
        )
        .size(11.0)
        .color(brand::MUTED),
    );
}
