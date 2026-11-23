//! Networks page: what every range worker carries.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
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
