//! The download row: file tile, status pill and row actions.
//!
//! A row is a flat line in a list, the way QDM's are: no card around it, a
//! hairline underneath, a header line, a progress bar and a facts line.
//! Per-stream tracks, the statistics tiles and the speed chart live behind the
//! chevron; actions appear as a strip on hover, again from `···`, or from a
//! right-click menu, so a list of twenty downloads is twenty short lines rather
//! than twenty framed button decks.

use super::prelude::*;
use super::{
    bars::{progress, segment, sparkline},
    controls::check,
    files::host_of,
    icons::{icon, Icon},
};

pub(crate) fn download_row(
    ui: &mut Ui,
    job: &Job,
    selected: bool,
    expanded: bool,
) -> Option<Action> {
    let mut action = None;

    // A right-click anywhere on the row opens the same menu as `···`. The area
    // comes from the row's last measured height and is registered before the
    // row's own controls draw, so those keep first claim on their clicks.
    let height_id = egui::Id::new(("row-height", job.id));
    let height = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(height_id))
        .unwrap_or(84.0);
    let surface = ui.interact(
        Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), height)),
        egui::Id::new(("row-surface", job.id)),
        Sense::click(),
    );
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .is_some_and(|pointer| surface.rect.contains(pointer));
    let reveal = ui.ctx().animate_bool_with_time(
        egui::Id::new(("row-actions", job.id)),
        hovered,
        0.10,
    );

    // QDM's list is a list, not a stack of cards: the row sits flat on the page
    // and a hairline closes it, so twenty downloads read as twenty lines.
    let row = egui::Frame::new()
        .fill(if selected {
            brand::mix(brand::glass(), brand::ROSE_DEEP, 1.0)
        } else {
            Color32::TRANSPARENT
        })
        .inner_margin(egui::Margin::symmetric(11, 9))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if check(ui, job.id, selected).clicked() {
                    action = Some(Action::Toggle(job.id));
                }
                ui.add_space(2.0);
                file_tile(ui, &job.display_name(), 30.0);
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
                    if let Some(chosen) = row_actions(ui, job, reveal) {
                        action = Some(chosen);
                    }
                    if let Some(chosen) = overflow(ui, job) {
                        action = Some(chosen);
                    }
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

            ui.add_space(7.0);
            let fraction = job.percent().unwrap_or(0.0);
            progress(
                ui,
                egui::Id::new(("row-progress", job.id)),
                fraction,
                if job.status == Status::Completed { 5.0 } else { 6.0 },
                match job.status {
                    Status::Completed => brand::mix(brand::ACCENT, Color32::WHITE, 0.25),
                    Status::Paused | Status::Cancelled => brand::mix(brand::ACCENT, brand::BG, 0.45),
                    Status::Failed => Color32::from_rgb(0xFF, 0x6B, 0x6B),
                    _ => brand::ACCENT,
                },
            );

            ui.add_space(6.0);
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
                if job.missing {
                    ui.label(
                        RichText::new("moved or deleted")
                            .font(brand::medium(10.8))
                            .color(brand::WARN),
                    );
                }
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
                    if let Some(finished) = job.finished_at {
                        let when = crate::store::when_label(finished);
                        if !when.is_empty() {
                            ui.label(
                                RichText::new(format!("{when} ·"))
                                    .size(10.6)
                                    .color(brand::DIM),
                            );
                        }
                    }
                });
            });

            if expanded {
                ui.add_space(10.0);
                if job.missing {
                    missing_banner(ui, &job.path());
                    ui.add_space(10.0);
                }
                let segments = job.progress.snapshot_segments();
                if segments.len() > 1 {
                    ui.label(
                        RichText::new("PER-STREAM PROGRESS")
                            .font(brand::medium(9.2))
                            .color(brand::DIM),
                    );
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let width = (ui.available_width() - (segments.len() as f32 - 1.0) * 4.0)
                            / segments.len() as f32;
                        for (index, (length, done, _start, _network)) in
                            segments.iter().enumerate()
                        {
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
                    ui.add_space(10.0);
                }
                // QDM's strip: what the file weighs, what carried it, and the
                // speeds and time it took. Tiles share the width so a long
                // adapter name truncates instead of pushing the strip apart.
                let tiles = [
                    ("Size", size_label(job.progress.total())),
                    ("Networks", networks_label(&job.networks)),
                    ("Peak", format_speed(job.peak)),
                    ("Average", format_speed(job.average_speed())),
                    ("Time", format_duration(job.elapsed_secs())),
                ];
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let width = (ui.available_width()
                        - 6.0 * (tiles.len() as f32 - 1.0))
                        / tiles.len() as f32;
                    for (label, value) in tiles {
                        stat(ui, width, label, value);
                    }
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
        });

    ui.ctx()
        .data_mut(|data| data.insert_temp(height_id, row.response.rect.height()));

    let mut menu_action = None;
    surface.context_menu(|ui| {
        menu_action = row_menu(ui, job);
    });
    if let Some(chosen) = menu_action {
        action = Some(chosen);
    }

    // A status stripe hugs the row's left edge, the way QDM marks one, and a
    // hairline closes the row off from the next.
    let edge = row.response.rect;
    ui.painter().rect_filled(
        Rect::from_min_max(
            Pos2::new(edge.left(), edge.top() + 8.0),
            Pos2::new(edge.left() + 3.0, edge.bottom() - 8.0),
        ),
        CornerRadius::same(2),
        brand::mix(status_colour(job.status), brand::glass(), 0.12),
    );
    ui.painter().line_segment(
        [
            Pos2::new(edge.left(), edge.bottom() - 0.5),
            Pos2::new(edge.right(), edge.bottom() - 0.5),
        ],
        Stroke::new(1.0_f32, brand::hairline()),
    );
    action
}

/// QDM's hover strip: the one action the row's state calls for, then reveal and
/// remove. It keeps its space while hidden, so nothing shifts as you move down
/// the list, and it fades in as the pointer enters the row.
fn row_actions(ui: &mut Ui, job: &Job, reveal: f32) -> Option<Action> {
    let mut action = None;
    match job.status {
        Status::Downloading | Status::Queued => {
            if icon_action(ui, Icon::Pause, "Pause", brand::ACCENT_BRIGHT, reveal) {
                action = Some(Action::Pause(job.id));
            }
        }
        Status::Paused => {
            if icon_action(ui, Icon::Play, "Resume", brand::ACCENT_BRIGHT, reveal) {
                action = Some(Action::Resume(job.id));
            }
        }
        Status::Failed | Status::Cancelled => {
            if icon_action(ui, Icon::Retry, "Try again", brand::ACCENT_BRIGHT, reveal) {
                action = Some(Action::Retry(job.id));
            }
        }
        Status::Completed => {
            if !job.missing
                && icon_action(ui, Icon::External, "Open file", brand::ACCENT_BRIGHT, reveal)
            {
                action = Some(Action::OpenFile(job.id));
            }
        }
    }
    if job.status.is_finished()
        && icon_action(ui, Icon::Folder, "Show in folder", brand::MUTED, reveal)
    {
        action = Some(Action::OpenFolder(job.id));
    }
    if icon_action(ui, Icon::Trash, "Remove", brand::MUTED, reveal) {
        action = Some(Action::Remove(job.id));
    }
    action
}

/// One hover-strip button: a rounded square that lights up under the pointer.
fn icon_action(ui: &mut Ui, kind: Icon, tip: &str, tint: Color32, reveal: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.10);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(7),
        brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover).gamma_multiply(reveal),
    );
    let colour = brand::mix(brand::MUTED, tint, hover.max(0.4)).gamma_multiply(reveal);
    icon(painter, rect.shrink(6.5), kind, colour);
    response.on_hover_text(tip).clicked()
}

/// The `···` control that holds every per-row action, so the compact row needs
/// no button deck of its own.
fn overflow(ui: &mut Ui, job: &Job) -> Option<Action> {
    let mut action = None;
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(4.0, 1.0);
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        widgets.inactive.bg_stroke = Stroke::NONE;
        widgets.inactive.corner_radius = CornerRadius::same(6);
        widgets.hovered.weak_bg_fill = brand::SURFACE_RAISED;
        widgets.hovered.bg_stroke = Stroke::NONE;
        widgets.hovered.corner_radius = CornerRadius::same(6);
        widgets.active.weak_bg_fill = brand::ROSE;
        widgets.active.bg_stroke = Stroke::NONE;
        widgets.active.corner_radius = CornerRadius::same(6);
        let label = RichText::new("···").font(brand::medium(14.0)).color(brand::MUTED);
        ui.menu_button(label, |ui| {
            action = row_menu(ui, job);
        });
    });
    action
}

/// The menu behind `···`: the same actions the row used to spell out as pills.
fn row_menu(ui: &mut Ui, job: &Job) -> Option<Action> {
    let mut action = None;
    ui.set_min_width(168.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    ui.spacing_mut().button_padding = Vec2::new(9.0, 5.0);
    let item = |ui: &mut Ui, label: &str| -> bool {
        ui.horizontal(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(9.0, 5.0);
            ui.button(RichText::new(label).font(brand::medium(12.0)))
                .clicked()
        })
        .inner
    };

    match job.status {
        Status::Downloading | Status::Queued => {
            if item(ui, "Pause") {
                action = Some(Action::Pause(job.id));
            }
        }
        Status::Paused => {
            if item(ui, "Resume") {
                action = Some(Action::Resume(job.id));
            }
        }
        _ => {}
    }
    if job.status.is_active() && item(ui, "Stop") {
        action = Some(Action::Cancel(job.id));
    }
    if job.status.is_finished() && item(ui, "Show in folder") {
        action = Some(Action::OpenFolder(job.id));
    }
    if job.status == Status::Completed && !job.missing && item(ui, "Open file") {
        action = Some(Action::OpenFile(job.id));
    }
    if matches!(job.status, Status::Failed | Status::Cancelled) && item(ui, "Try again") {
        action = Some(Action::Retry(job.id));
    }
    if item(ui, "Copy link") {
        action = Some(Action::CopyLink(job.url.clone()));
    }
    if job.status.is_finished() {
        ui.separator();
        if item(ui, "Delete file") {
            action = Some(Action::Delete(job.id));
        }
    }
    if !job.status.is_active() && item(ui, "Remove") {
        action = Some(Action::Remove(job.id));
    }
    if action.is_some() {
        ui.close_menu();
    }
    action
}

/// QDM's amber note on an expanded row: the saved file is gone from its folder.
fn missing_banner(ui: &mut Ui, path: &Path) {
    let frame = egui::Frame::new()
        .fill(brand::mix(brand::glass(), brand::WARN, 0.10))
        .stroke(Stroke::new(
            1.0_f32,
            brand::mix(brand::glass(), brand::WARN, 0.40),
        ))
        .corner_radius(CornerRadius::same(brand::R_TILE))
        .inner_margin(egui::Margin::symmetric(12, 9));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.4, brand::WARN);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new("This file is no longer where it was saved")
                        .font(brand::medium(11.5))
                        .color(brand::WARN),
                );
                ui.label(
                    RichText::new("It may have been moved or deleted.")
                        .size(10.4)
                        .color(brand::mix(brand::WARN, brand::DIM, 0.55)),
                );
                ui.label(
                    RichText::new(path.display().to_string())
                        .size(10.2)
                        .color(brand::DIM),
                );
            });
        });
    });
}

/// One tile in the expanded row's statistics strip, held to a shared width.
fn stat(ui: &mut Ui, width: f32, label: &str, value: String) {
    brand::tile_frame().show(ui, |ui| {
        ui.set_width((width - 30.0).max(40.0));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new(label.to_uppercase())
                    .size(9.2)
                    .color(brand::DIM),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(value.clone())
                        .font(brand::semi(15.5))
                        .color(brand::INK),
                )
                .truncate(),
            )
            .on_hover_text(value);
        });
    });
}

/// The file's full size, or a dash while the server has not said it yet.
fn size_label(total: u64) -> String {
    if total == 0 {
        "Unknown".to_string()
    } else {
        format_bytes(total)
    }
}

/// The adapters that carried the job: one name, or how many shared it.
fn networks_label(networks: &[String]) -> String {
    match networks {
        [] => "Default route".to_string(),
        [one] => one.split(" · ").next().unwrap_or(one).to_string(),
        many => format!("{} networks", many.len()),
    }
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
fn file_tile(ui: &mut Ui, name: &str, size: f32) {
    let ext = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.chars().take(4).collect::<String>().to_uppercase())
        .unwrap_or_else(|| "FILE".to_string());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    let corner = CornerRadius::same((size * 0.29) as u8);
    painter.rect_filled(
        rect,
        corner,
        Color32::from_rgba_unmultiplied(255, 240, 246, 20),
    );
    painter.rect_stroke(
        rect,
        corner,
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
