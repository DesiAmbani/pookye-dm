//! The download row: file tile, status pill and row actions.

use super::prelude::*;
use super::{
    bars::{progress, segment, sparkline},
    controls::{check, pill, Pill},
    files::host_of,
    surfaces::metric,
};


pub(crate) fn download_row(
    ui: &mut Ui,
    job: &Job,
    selected: bool,
    expanded: bool,
) -> Option<Action> {
    let mut action = None;

    brand::card_frame()
        .fill(brand::mix(
            brand::glass(),
            brand::ROSE_DEEP,
            if selected { 1.0 } else { 0.0 },
        ))
        .inner_margin(egui::Margin::same(15))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if check(ui, job.id, selected).clicked() {
                    action = Some(Action::Toggle(job.id));
                }
                ui.add_space(2.0);
                file_tile(ui, &job.display_name());
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new(job.display_name())
                            .font(brand::medium(13.5))
                            .color(brand::INK),
                    );
                    ui.label(
                        RichText::new(if job.networks.len() > 1 {
                            format!(
                                "{} · {} · {} streams on {} networks",
                                host_of(&job.url),
                                job.category.label(),
                                job.connections,
                                job.networks.len()
                            )
                        } else {
                            format!(
                                "{} · {} · {} streams",
                                host_of(&job.url),
                                job.category.label(),
                                job.connections
                            )
                        })
                        .size(10.8)
                        .color(brand::DIM),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_pill(ui, job.status);
                    if chevron(ui, expanded).clicked() {
                        action = Some(Action::ToggleDetails(job.id));
                    }
                    if job.status == Status::Downloading {
                        ui.label(
                            RichText::new(format_speed(job.speed))
                                .font(brand::medium(12.5))
                                .color(brand::ACCENT_BRIGHT),
                        );
                    }
                });
            });

            ui.add_space(11.0);
            let fraction = job.percent().unwrap_or(0.0);
            progress(
                ui,
                egui::Id::new(("row-progress", job.id)),
                fraction,
                if job.status == Status::Completed { 7.0 } else { 8.0 },
                match job.status {
                    Status::Completed => brand::mix(brand::ACCENT, Color32::WHITE, 0.25),
                    Status::Paused | Status::Cancelled => brand::mix(brand::ACCENT, brand::BG, 0.45),
                    Status::Failed => Color32::from_rgb(0xFF, 0x6B, 0x6B),
                    _ => brand::ACCENT,
                },
            );

            let segments = job.progress.snapshot_segments();
            if segments.len() > 1 {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let width = (ui.available_width() - (segments.len() as f32 - 1.0) * 4.0)
                        / segments.len() as f32;
                    for (index, (length, done, _start, _network)) in segments.iter().enumerate() {
                        let part = if *length == 0 {
                            0.0
                        } else {
                            (*done as f32 / *length as f32).clamp(0.0, 1.0)
                        };
                        segment(
                            ui,
                            egui::Id::new(("seg", job.id, index)),
                            part,
                            width.max(3.0),
                        );
                    }
                });
            }

            ui.add_space(9.0);
            ui.horizontal(|ui| {
                let total = job.progress.total();
                let bytes = job.progress.bytes();
                let mut facts = Vec::new();
                if total > 0 {
                    facts.push(format!("{} of {}", format_bytes(bytes), format_bytes(total)));
                } else if bytes > 0 {
                    facts.push(format!("{} downloaded", format_bytes(bytes)));
                }
                if job.status == Status::Downloading {
                    if let Some(left) = job.seconds_left() {
                        let eta = format_eta(left);
                        if !eta.is_empty() {
                            facts.push(eta);
                        }
                    }
                }
                if let Some(error) = &job.error {
                    facts.push(error.clone());
                }
                ui.label(
                    RichText::new(facts.join("  ·  "))
                        .size(10.8)
                        .color(if job.error.is_some() {
                            Color32::from_rgb(0xFF, 0x8F, 0x9E)
                        } else {
                            brand::DIM
                        }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = match job.status {
                        Status::Queued => "Waiting for a free slot".to_string(),
                        Status::Paused => format!("Paused · {}", format_percent(job.percent())),
                        _ => format_percent(job.percent()),
                    };
                    ui.label(
                        RichText::new(label)
                            .font(brand::medium(11.5))
                            .color(brand::MUTED),
                    );
                });
            });

            if expanded {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    metric(ui, "Average", format_speed(job.average_speed()));
                    metric(ui, "Peak", format_speed(job.peak));
                    metric(ui, "Time", format_duration(job.elapsed_secs()));
                    metric(ui, "Streams", job.connections.to_string());
                });
                if job.history.len() > 1 {
                    ui.add_space(9.0);
                    ui.label(
                        RichText::new("SPEED OVER THE TRANSFER")
                            .size(9.2)
                            .color(brand::DIM),
                    );
                    ui.add_space(4.0);
                    sparkline(ui, &job.history, 58.0);
                }
            }

            ui.add_space(7.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                match job.status {
                    Status::Downloading | Status::Queued => {
                        if pill(ui, "Pause", Pill::Quiet) {
                            action = Some(Action::Pause(job.id));
                        }
                    }
                    Status::Paused => {
                        if pill(ui, "Resume", Pill::Quiet) {
                            action = Some(Action::Resume(job.id));
                        }
                    }
                    _ => {}
                }
                if job.status.is_active() && pill(ui, "Stop", Pill::Quiet) {
                    action = Some(Action::Cancel(job.id));
                }
                if job.status.is_finished() && pill(ui, "Show in folder", Pill::Quiet) {
                    action = Some(Action::OpenFolder(job.id));
                }
                if job.status == Status::Completed && pill(ui, "Open file", Pill::Quiet) {
                    action = Some(Action::OpenFile(job.id));
                }
                if matches!(job.status, Status::Failed | Status::Cancelled)
                    && pill(ui, "Try again", Pill::Quiet)
                {
                    action = Some(Action::Retry(job.id));
                }
                if pill(ui, "Copy link", Pill::Quiet) {
                    action = Some(Action::CopyLink(job.url.clone()));
                }
                if job.status.is_finished() && pill(ui, "Delete file", Pill::Quiet) {
                    action = Some(Action::Delete(job.id));
                }
                if !job.status.is_active() && pill(ui, "Remove", Pill::Quiet) {
                    action = Some(Action::Remove(job.id));
                }
            });
        });
    ui.add_space(8.0);
    action
}

/// The disclosure arrow that opens a row's statistics, as in QDM.
fn chevron(ui: &mut Ui, expanded: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(6),
        brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover),
    );
    let c = rect.center();
    let stroke = Stroke::new(1.5_f32, brand::mix(brand::MUTED, brand::INK, hover));
    // Symmetric arms: a collapsed arrow whose right arm stops short of its left
    // one reads as a stray tick rather than a chevron.
    let (tip_y, end_y) = if expanded { (-2.6, 1.6) } else { (2.6, -1.6) };
    painter.line_segment([c + Vec2::new(-5.0, end_y), c + Vec2::new(0.0, tip_y)], stroke);
    painter.line_segment([c + Vec2::new(0.0, tip_y), c + Vec2::new(5.0, end_y)], stroke);
    response
}

/// A rounded tile holding the file's extension, like an app icon.
fn file_tile(ui: &mut Ui, name: &str) {
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.chars().take(4).collect::<String>().to_uppercase())
        .unwrap_or_else(|| "FILE".to_string());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(10),
        Color32::from_rgba_unmultiplied(255, 240, 246, 20),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, brand::hairline()),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        ext,
        brand::medium(9.5),
        brand::MUTED,
    );
}

pub(crate) fn status_pill(ui: &mut Ui, status: Status) {
    let color = status_colour(status);
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(status.label().len() as f32 * 6.6 + 26.0, 22.0),
        Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(11),
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26),
    );
    painter.circle_filled(
        Pos2::new(rect.left() + 12.0, rect.center().y),
        2.6,
        color,
    );
    painter.text(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        status.label(),
        brand::medium(10.5),
        color,
    );
}

pub(crate) fn status_colour(status: Status) -> Color32 {
    match status {
        Status::Downloading => brand::ACCENT_BRIGHT,
        Status::Queued | Status::Paused | Status::Cancelled => brand::MUTED,
        Status::Completed => Color32::from_rgb(0x7A, 0xE0, 0xB0),
        Status::Failed => Color32::from_rgb(0xFF, 0x8F, 0x9E),
    }
}
