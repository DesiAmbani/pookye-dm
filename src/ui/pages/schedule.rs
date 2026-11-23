//! Schedule page: the weekly window and the waiting queue.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
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
