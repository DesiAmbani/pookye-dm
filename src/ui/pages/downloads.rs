//! Downloads page: the paste bar, filters, batch actions and the rows.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
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
