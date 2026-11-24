//! Networks page: what each network carries, and the ranges under it.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Networks",
        "Every stream is its own range request, and each network carries its share.",
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

                    let total: u64 = segments.iter().map(|(length, _, _, _)| *length).sum();
                    let mut done_by_net = vec![0u64; job.networks.len().max(1)];
                    for (_length, done, _start, network) in &segments {
                        let lane = (*network).min(done_by_net.len() - 1);
                        done_by_net[lane] += *done;
                    }

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("CONTRIBUTION BY NETWORK")
                            .font(brand::medium(9.2))
                            .color(brand::DIM),
                    );
                    ui.add_space(5.0);
                    for (lane, done) in done_by_net.iter().enumerate() {
                        let label = job
                            .networks
                            .get(lane)
                            .cloned()
                            .unwrap_or_else(|| "Default route".to_string());
                        let share = if total == 0 {
                            0.0
                        } else {
                            *done as f32 / total as f32
                        };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(label).size(10.9).color(brand::MUTED));
                            w::progress(
                                ui,
                                egui::Id::new(("net-share", job.id, lane)),
                                share,
                                6.0,
                                brand::ACCENT,
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let speed = job.net_speed.get(lane).copied().unwrap_or(0.0);
                                    if speed > 1.0 {
                                        ui.label(
                                            RichText::new(format_speed(speed))
                                                .size(10.0)
                                                .color(brand::ACCENT_BRIGHT),
                                        );
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {:.0}%",
                                            format_bytes(*done),
                                            share * 100.0
                                        ))
                                        .size(10.0)
                                        .color(brand::MUTED),
                                    );
                                },
                            );
                        });
                    }

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("STREAMS")
                            .font(brand::medium(9.2))
                            .color(brand::DIM),
                    );
                    ui.add_space(5.0);
                    for (index, (length, done, start, network)) in segments.iter().enumerate() {
                        let part = if *length == 0 {
                            0.0
                        } else {
                            (*done as f32 / *length as f32).clamp(0.0, 1.0)
                        };
                        let lane = job
                            .networks
                            .get(*network)
                            .cloned()
                            .unwrap_or_else(|| "Default route".to_string());
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Connection {}", index + 1))
                                    .size(10.5)
                                    .color(brand::DIM),
                            );
                            let bar_width = (ui.available_width() - 300.0).max(60.0);
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
                                RichText::new(format!("byte {} · {}", format_bytes(*start), lane))
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
                        RichText::new("Start a download and its networks appear here.")
                            .size(11.5)
                            .color(brand::DIM),
                    );
                });
            }
        });
}
