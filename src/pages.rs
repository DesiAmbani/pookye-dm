//! The six pages reachable from the sidebar.

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense};
use engine::{format_bytes, format_speed, Status};

use crate::actions::{self, Action};
use crate::widgets::{self as w, card, chip, host_of, metric, page_heading, pill, Pill};
use crate::{brand, engine, App, Page};

const REPO_URL: &str = "https://github.com/DesiAmbani/pookye-dm";

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

    let speed = app.manager.total_speed();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.label(
                RichText::new(format_speed(speed))
                    .font(brand::semi(30.0))
                    .color(brand::INK),
            );
            ui.label(
                RichText::new(if speed > 0.0 {
                    format!(
                        "{} active · {} streams each",
                        app.manager.downloading_count(),
                        app.settings.connections
                    )
                } else if app.settings.slow_mode {
                    "Slow mode is holding downloads back".to_string()
                } else {
                    "Nothing downloading".to_string()
                })
                .size(11.2)
                .color(brand::DIM),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pill(ui, "New download", Pill::Primary) {
                app.open_add();
            }
            if pill(ui, "Clear list", Pill::Quiet) {
                actions.push(Action::ClearList);
            }
            if pill(ui, "Resume all", Pill::Quiet) {
                actions.push(Action::ResumeAll);
            }
            if pill(ui, "Pause all", Pill::Quiet) {
                actions.push(Action::PauseAll);
            }
        });
    });

    // One thin line for the whole queue, like the original's header rule.
    let rows_now = app.filtered_rows().len();
    let share = if rows_now == 0 {
        0.0
    } else {
        app.manager
            .jobs
            .iter()
            .filter(|job| job.status.is_active())
            .map(|job| {
                let total = job.progress.total();
                if total == 0 {
                    0.0
                } else {
                    job.progress.bytes() as f32 / total as f32
                }
            })
            .sum::<f32>()
            / rows_now as f32
    };
    w::progress(
        ui,
        egui::Id::new("queue-line"),
        share,
        3.0,
        brand::ACCENT_BRIGHT,
    );

    ui.add_space(14.0);

    let mut paste = false;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let field_width = (ui.available_width() - 160.0).max(140.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(0x0E, 0x0D, 0x0F))
                .stroke(egui::Stroke::new(1.0_f32, brand::hairline()))
                .corner_radius(CornerRadius::same(brand::R_PILL))
                .inner_margin(Margin::symmetric(13, 5))
                .show(ui, |ui| {
                    let edit = ui.add_sized(
                        [field_width, 24.0],
                        egui::TextEdit::singleline(&mut app.url_input)
                            .frame(false)
                            .hint_text("Paste a link to a file"),
                    );
                    if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        paste = true;
                    }
                });
            ui.add_space(4.0);
            if w::pill_sized(ui, "Start download", Pill::Primary, 132.0) {
                paste = true;
            }
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Save to").size(10.8).color(brand::DIM));
            ui.label(
                RichText::new(app.settings.folder.as_str())
                    .size(10.8)
                    .color(brand::MUTED),
            );
            ui.label(
                RichText::new(format!("· {} streams", app.settings.connections))
                    .size(10.8)
                    .color(brand::DIM),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill(ui, "Change", Pill::Quiet) {
                    app.go(Page::Settings);
                }
            });
        });
    });
    if paste {
        app.start_pasted();
    }

    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        egui::Frame::new()
            .fill(Color32::from_rgb(0x0E, 0x0D, 0x0F))
            .stroke(egui::Stroke::new(1.0_f32, brand::hairline()))
            .corner_radius(CornerRadius::same(brand::R_PILL))
            .inner_margin(Margin::symmetric(12, 4))
            .show(ui, |ui| {
                ui.add_sized(
                    [180.0, 24.0],
                    egui::TextEdit::singleline(&mut app.search)
                        .frame(false)
                        .hint_text("Search downloads"),
                );
            });
        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("{} · {}", app.filter.label(), app.count_of(app.filter)))
                .size(10.8)
                .color(brand::DIM),
        );
    });

    if !app.selected.is_empty() {
        ui.add_space(10.0);
        let count = app.selected.len();
        brand::tile_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{count} selected"))
                        .font(brand::medium(11.5))
                        .color(brand::INK),
                );
                ui.add_space(4.0);
                if pill(ui, "Pause", Pill::Quiet) {
                    actions.push(Action::PauseSelected);
                }
                if pill(ui, "Resume", Pill::Quiet) {
                    actions.push(Action::ResumeSelected);
                }
                if pill(ui, "Remove", Pill::Quiet) {
                    actions.push(Action::RemoveSelected);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill(ui, "Clear selection (Esc)", Pill::Quiet) {
                        app.selected.clear();
                    }
                });
            });
        });
    }

    ui.add_space(12.0);
    let rows = app.filtered_rows();
    let empty = app.manager.jobs.is_empty();
    if rows.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 72.0), Sense::hover());
            if let Some(logo) = &app.logo {
                ui.painter().image(
                    logo.id(),
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    Color32::from_white_alpha(38),
                );
            }
            ui.add_space(12.0);
            ui.label(
                RichText::new(if empty {
                    "Nothing downloading yet"
                } else {
                    "Nothing here"
                })
                .font(brand::semi(17.0))
                .color(brand::MUTED),
            );
            ui.label(
                RichText::new(if empty {
                    "Paste a link to a file, or let the browser extension hand one over."
                } else {
                    "No download here matches this search."
                })
                .size(11.5)
                .color(brand::DIM),
            );
            if !empty {
                ui.add_space(8.0);
                if pill(ui, "Show all downloads", Pill::Glass) {
                    app.filter = crate::Filter::All;
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
                    ui.add_space(4.0);
                    let count = app.settings.history.len();
                    egui::CollapsingHeader::new(
                        RichText::new(format!("Recent downloads · {count}"))
                            .size(11.2)
                            .color(brand::DIM),
                    )
                    .id_salt("history")
                    .show(ui, |ui| {
                        for entry in app.settings.history.iter().take(12) {
                            ui.label(
                                RichText::new(format!(
                                    "{}  ·  {}",
                                    entry.file_name,
                                    format_bytes(entry.bytes)
                                ))
                                .size(10.8)
                                .color(brand::DIM),
                            );
                        }
                    });
                }
            });
    }

    for action in actions {
        actions::apply(app, action);
    }
}

fn speed(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Speed",
        "All downloads together, over every connection.",
    );
    ui.add_space(14.0);

    let mut limit_changed = false;
    card(ui, |ui| {
        ui.label(
            RichText::new("System-wide limit")
                .font(brand::medium(13.5))
                .color(brand::INK),
        );
        ui.label(
            RichText::new("One budget shared by every stream of every download.")
                .size(10.8)
                .color(brand::DIM),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if chip(ui, "Unlimited", app.settings.speed_limit == 0) {
                app.settings.speed_limit = 0;
                limit_changed = true;
            }
            for megabytes in [1usize, 5, 10, 50] {
                let kbps = megabytes * 1024;
                if chip(
                    ui,
                    &format!("{megabytes} MB/s"),
                    app.settings.speed_limit == kbps,
                ) {
                    app.settings.speed_limit = kbps;
                    limit_changed = true;
                }
            }
        });
        ui.add_space(6.0);
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
                    "Capped at {} of traffic per second",
                    format_bytes(app.settings.speed_limit as u64 * 1024)
                )
            })
            .size(10.8)
            .color(brand::DIM),
        );
    });
    if limit_changed {
        app.apply_limits();
    }

    ui.add_space(12.0);
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

    ui.add_space(12.0);
    if app.manager.jobs.is_empty() {
        ui.label(
            RichText::new("No downloads to show yet.")
                .size(11.5)
                .color(brand::DIM),
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
                                .font(brand::medium(12.8))
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format_speed(job.speed))
                                    .font(brand::medium(12.0))
                                    .color(w::status_colour(job.status)),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    let share = if app.manager.total_speed() > 0.0 {
                        (job.speed / app.manager.total_speed()).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    w::progress(
                        ui,
                        egui::Id::new(("share", job.id)),
                        share,
                        6.0,
                        brand::ACCENT_BRIGHT,
                    );
                });
                ui.add_space(8.0);
            }
        });
}

fn networks(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Networks",
        "What each connection carries: every stream is an independent range request.",
    );
    ui.add_space(14.0);

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
                                .font(brand::medium(12.8))
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            w::status_pill(ui, job.status);
                            ui.label(
                                RichText::new(format!("{} connections", segments.len()))
                                    .size(10.5)
                                    .color(brand::DIM),
                            );
                        });
                    });
                    ui.add_space(8.0);
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
                                    .color(brand::DIM),
                            );
                            let bar_width = (ui.available_width() - 190.0).max(70.0);
                            w::segment(
                                ui,
                                egui::Id::new(("net", job.id, index)),
                                part,
                                bar_width,
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
                                    .size(9.6)
                                    .color(brand::DIM),
                            );
                        });
                    }
                });
                ui.add_space(8.0);
            }
            if shown == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(70.0);
                    ui.label(
                        RichText::new("No connections carrying traffic")
                            .font(brand::semi(16.0))
                            .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new("Start a download and its streams appear here.")
                            .size(11.5)
                            .color(brand::DIM),
                    );
                });
            }
        });
}

fn schedule(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Schedule",
        "Downloads only start inside the window; the rest wait in the queue.",
    );
    ui.add_space(14.0);

    let (weekday, minute) = crate::store::local_now();
    let open = app.settings.schedule.allows(weekday, minute);
    let mut schedule_changed = false;

    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Weekly schedule")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(if app.settings.schedule.enabled {
                        format!(
                            "Starts {}–{} · {}",
                            crate::store::clock(app.settings.schedule.start_minute),
                            crate::store::clock(app.settings.schedule.end_minute),
                            if open { "open now" } else { "shut right now" }
                        )
                    } else {
                        "Downloads start as soon as they are added.".to_string()
                    })
                    .size(10.8)
                    .color(brand::DIM),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if w::switch(ui, app.settings.schedule.enabled).clicked() {
                    app.settings.schedule.enabled = !app.settings.schedule.enabled;
                    schedule_changed = true;
                }
            });
        });

        if app.settings.schedule.enabled {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Start").size(10.8).color(brand::DIM));
                if ui
                    .add(
                        egui::Slider::new(&mut app.settings.schedule.start_minute, 0..=1439)
                            .step_by(15.0)
                            .custom_formatter(|v, _| crate::store::clock(v as u32)),
                    )
                    .changed()
                {
                    schedule_changed = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new("End").size(10.8).color(brand::DIM));
                if ui
                    .add(
                        egui::Slider::new(&mut app.settings.schedule.end_minute, 0..=1439)
                            .step_by(15.0)
                            .custom_formatter(|v, _| crate::store::clock(v as u32)),
                    )
                    .changed()
                {
                    schedule_changed = true;
                }
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for day in 0..7u8 {
                    let label = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                        [day as usize];
                    let active = app.settings.schedule.days.contains(&day);
                    if w::chip(ui, label, active) {
                        if active {
                            app.settings.schedule.days.retain(|d| *d != day);
                        } else {
                            app.settings.schedule.days.push(day);
                            app.settings.schedule.days.sort_unstable();
                        }
                        schedule_changed = true;
                    }
                }
            });
            if app.settings.schedule.days.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("No days picked, so the window stays open every day.")
                        .size(10.5)
                        .color(brand::DIM),
                );
            }
        }
    });
    if schedule_changed {
        app.dirty = true;
    }

    ui.add_space(14.0);
    let mut pending: Vec<Action> = Vec::new();
    let mut concurrency_changed = false;
    card(ui, |ui| {
        ui.label(
            RichText::new("Downloads at once")
                .font(brand::medium(13.5))
                .color(brand::INK),
        );
        ui.add_space(4.0);
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
            .size(10.8)
            .color(brand::DIM),
        );
    });
    if concurrency_changed {
        app.apply_limits();
    }

    if app.settings.schedule.enabled && !app.manager.allowed() {
        ui.add_space(10.0);
        ui.label(
            RichText::new("New downloads are held until the window opens.")
                .size(11.2)
                .color(brand::MUTED),
        );
    }

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Waiting · {}", app.manager.queued_count()))
                .font(brand::medium(12.5))
                .color(brand::INK),
        );
    });
    ui.add_space(4.0);
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
                .size(11.2)
                .color(brand::DIM),
        );
    } else {
        for id in queued {
            if let Some(job) = app.manager.jobs.iter().find(|job| job.id == id) {
                brand::tile_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .size(12.0)
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · {} streams",
                                host_of(&job.url),
                                job.connections
                            ))
                            .size(10.5)
                            .color(brand::DIM),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if pill(ui, "Remove", Pill::Quiet) {
                                pending.push(Action::Remove(id));
                            }
                            if pill(ui, "Start now", Pill::Glass) {
                                pending.push(Action::Resume(id));
                            }
                        });
                    });
                });
                ui.add_space(6.0);
            }
        }
    }

    for action in pending {
        actions::apply(app, action);
    }
}

fn settings(app: &mut App, ui: &mut egui::Ui) {
    page_heading(ui, "Settings", "Where downloads go and how hard they push.");
    ui.add_space(14.0);
    let mut changed = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            card(ui, |ui| {
                ui.label(
                    RichText::new("Storage")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(6.0);
                ui.label(RichText::new("Download folder").size(10.8).color(brand::DIM));
                egui::Frame::new()
                    .fill(Color32::from_rgb(0x0E, 0x0D, 0x0F))
                    .stroke(egui::Stroke::new(1.0_f32, brand::hairline()))
                    .corner_radius(CornerRadius::same(brand::R_PILL))
                    .inner_margin(Margin::symmetric(12, 4))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.settings.folder)
                                    .frame(false)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                ui.label(
                    RichText::new("Downloads run from this folder. Missing folders are created.")
                        .size(10.2)
                        .color(brand::DIM),
                );
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Performance")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Segments per download")
                        .size(10.8)
                        .color(brand::DIM),
                );
                if ui
                    .add(egui::Slider::new(&mut app.settings.connections, 1..=32))
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    RichText::new(format!(
                        "{} parallel connections. Use one for servers that block parallel requests.",
                        app.settings.connections
                    ))
                    .size(10.2)
                    .color(brand::DIM),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Downloads at once")
                        .size(10.8)
                        .color(brand::DIM),
                );
                if ui
                    .add(egui::Slider::new(&mut app.settings.max_concurrent, 1..=10))
                    .changed()
                {
                    changed = true;
                }
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Slow mode")
                                .font(brand::medium(13.5))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(
                                "Holds every download to one gentle speed, for a call or a game.",
                            )
                            .size(10.4)
                            .color(brand::DIM),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if w::switch(ui, app.settings.slow_mode).clicked() {
                            app.settings.slow_mode = !app.settings.slow_mode;
                            changed = true;
                        }
                    });
                });
                if app.settings.slow_mode {
                    ui.add_space(6.0);
                    let mut mb = app.settings.slow_mode_speed as f64 / (1024.0 * 1024.0);
                    if ui
                        .add(
                            egui::Slider::new(&mut mb, 0.25..=20.0)
                                .suffix(" MB/s")
                                .fixed_decimals(2),
                        )
                        .changed()
                    {
                        app.settings.slow_mode_speed = (mb * 1024.0 * 1024.0) as usize;
                        changed = true;
                    }
                }
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Behaviour")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(4.0);
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
                        RichText::new(format!(
                            "{} entries in history",
                            app.settings.history.len()
                        ))
                        .size(10.8)
                        .color(brand::DIM),
                    );
                    if !app.settings.history.is_empty() && pill(ui, "Clear history", Pill::Quiet) {
                        app.settings.history.clear();
                        changed = true;
                    }
                });
            });

            if changed {
                app.apply_limits();
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new("Settings are saved automatically.")
                    .size(10.2)
                    .color(brand::DIM),
            );
        });
}

fn about(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "About",
        "Version, licence and where this project comes from.",
    );
    ui.add_space(14.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                    if let Some(logo) = &app.logo {
                        ui.painter().image(
                            logo.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Pookye Download Manager")
                                .font(brand::semi(18.0))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(format!("PDM · version {}", engine::VERSION))
                                .size(11.2)
                                .color(brand::MUTED),
                        );
                        ui.label(
                            RichText::new("Multi-segment downloads in Rust, MIT licensed.")
                                .size(11.2)
                                .color(brand::DIM),
                        );
                    });
                });
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                metric(ui, "Version", engine::VERSION.to_string());
                metric(ui, "Licence", "MIT".to_string());
                metric(ui, "Streams per file", "up to 32".to_string());
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Lineage")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "The download-manager model here — parallel byte ranges, a queue with a \
                         concurrency limit, browser interception and per-connection telemetry — \
                         follows QDM (PBhadoo/QDM), which is MIT licensed. Pookye DM is an \
                         independent Rust implementation: the engine, interface, local API and \
                         browser bridge here are original code.",
                    )
                    .size(11.0)
                    .color(brand::MUTED),
                );
                ui.add_space(6.0);
                ui.hyperlink_to(
                    RichText::new(REPO_URL).size(11.0).color(brand::ACCENT_BRIGHT),
                    REPO_URL,
                );
            });
        });
}
