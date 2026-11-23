//! Shared drawing helpers: glass cards, pills, navigation, progress and rows.

use std::path::Path;

use eframe::egui::{
    self, Color32, CornerRadius, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

use crate::actions::Action;
use crate::brand;
use crate::engine::{format_bytes, format_eta, format_percent, format_speed, Job, Status};

// ---------------------------------------------------------------- surfaces

pub(crate) fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    brand::card_frame().show(ui, add).inner
}

pub(crate) fn page_heading(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .font(brand::semi(23.0))
            .color(brand::INK),
    );
    ui.label(RichText::new(subtitle).size(11.8).color(brand::DIM));
}

pub(crate) fn metric(ui: &mut Ui, label: &str, value: String) {
    brand::tile_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(label.to_uppercase()).size(9.2).color(brand::DIM));
            ui.label(RichText::new(value).font(brand::semi(16.0)).color(brand::INK));
        });
    });
}

/// A faint rule used to separate blocks inside a card.
pub(crate) fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(255, 226, 238, 22),
    );
}

pub(crate) fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(brand::medium(9.2))
            .color(brand::DIM),
    );
}

// ---------------------------------------------------------------- controls

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pill {
    Primary,
    Glass,
    Quiet,
}

pub(crate) fn pill(ui: &mut Ui, label: &str, kind: Pill) -> bool {
    pill_sized(ui, label, kind, 0.0)
}

pub(crate) fn pill_sized(ui: &mut Ui, label: &str, kind: Pill, min_width: f32) -> bool {
    let font = brand::medium(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, Color32::PLACEHOLDER);
    let pad = Vec2::new(15.0, 7.0);
    let size = Vec2::new(
        (galley.size().x + pad.x * 2.0).max(min_width),
        (galley.size().y + pad.y * 2.0).max(30.0),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.13);
    let pressed = response.is_pointer_button_down_on();
    let radius = CornerRadius::same((rect.height() * 0.5).round() as u8);

    let (fill, text) = match kind {
        Pill::Primary => {
            let base = brand::ACCENT;
            let lit = brand::mix(base, Color32::from_rgb(0xFF, 0x45, 0x8A), 0.35 + hover * 0.35);
            (
                brand::mix(base, lit, hover),
                brand::mix(Color32::WHITE, Color32::WHITE, 0.0),
            )
        }
        Pill::Glass => (
            brand::mix(brand::glass_raised(), brand::sheen(46), hover),
            brand::mix(brand::MUTED, brand::INK, hover.max(0.35)),
        ),
        Pill::Quiet => (
            brand::mix(Color32::TRANSPARENT, brand::sheen(26), hover),
            brand::mix(brand::MUTED, brand::INK, hover),
        ),
    };

    let painter = ui.painter();
    painter.rect_filled(rect, radius, fill);
    if kind == Pill::Primary {
        // Glossy top edge, the way a glass button catches the light.
        let gloss = Rect::from_min_max(
            rect.min + Vec2::new(1.0, 1.0),
            Pos2::new(rect.max.x - 1.0, rect.min.y + rect.height() * 0.52),
        );
        painter.rect_filled(
            gloss,
            CornerRadius::same((gloss.height() * 0.5).round() as u8),
            brand::sheen(38),
        );
    } else {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0_f32, brand::hairline()),
            StrokeKind::Inside,
        );
    }
    if pressed {
        painter.rect_filled(rect, radius, Color32::from_black_alpha(28));
    }

    let text_pos = rect.center() - galley.size() * 0.5;
    painter.galley(text_pos, galley, text);

    response.clicked()
}

pub(crate) fn chip(ui: &mut Ui, label: &str, active: bool) -> bool {
    let font = brand::medium(11.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, Color32::PLACEHOLDER);
    let size = Vec2::new(galley.size().x + 26.0, 27.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, active, 0.16);
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("h"), response.hovered(), 0.12);
    let radius = CornerRadius::same(13);

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        brand::mix(
            brand::mix(Color32::TRANSPARENT, brand::sheen(20), hover),
            Color32::from_rgba_unmultiplied(255, 240, 247, 44),
            t,
        ),
    );
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(
            1.0_f32,
            brand::mix(brand::hairline(), Color32::from_rgba_unmultiplied(255, 236, 244, 76), t),
        ),
        StrokeKind::Inside,
    );
    let color = brand::mix(brand::mix(brand::MUTED, brand::INK, hover), brand::INK, t);
    painter.galley(rect.center() - galley.size() * 0.5, galley, color);
    response.clicked()
}

/// A round selection control, in place of egui's square checkbox.
pub(crate) fn check(ui: &mut Ui, salt: impl std::hash::Hash, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui.ctx().animate_bool_with_time(
        egui::Id::new(("check", salt, response.id)),
        selected,
        0.15,
    );
    let centre = rect.center();
    let radius = 9.0;
    let painter = ui.painter();
    painter.circle_filled(
        centre,
        radius,
        brand::mix(Color32::TRANSPARENT, brand::ACCENT, t),
    );
    painter.circle_stroke(
        centre,
        radius,
        Stroke::new(
            1.2_f32,
            brand::mix(
                Color32::from_rgba_unmultiplied(255, 232, 242, 92),
                brand::ACCENT_BRIGHT,
                t,
            ),
        ),
    );
    if t > 0.05 {
        let arm = 4.0 * t;
        let dot = Stroke::new(1.7_f32, brand::mix(brand::ACCENT, Color32::WHITE, t));
        painter.add(egui::Shape::line(
            vec![
                centre + Vec2::new(-arm, 0.0),
                centre + Vec2::new(-arm * 0.3, arm * 0.75),
                centre + Vec2::new(arm, -arm * 0.85),
            ],
            dot,
        ));
    }
    response
}

// ---------------------------------------------------------------- progress

pub(crate) fn progress(ui: &mut Ui, id: egui::Id, fraction: f32, height: f32, tint: Color32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    paint_progress(ui, rect, id, fraction, tint);
}

fn paint_progress(ui: &Ui, rect: Rect, id: egui::Id, fraction: f32, tint: Color32) {
    let value = ui
        .ctx()
        .animate_value_with_time(id, fraction.clamp(0.0, 1.0), 0.38);
    let radius = CornerRadius::same((rect.height() * 0.5).round() as u8);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        Color32::from_rgba_unmultiplied(8, 0, 5, 96),
    );
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 232, 242, 16)),
        StrokeKind::Inside,
    );
    if value <= 0.001 {
        return;
    }
    let width = (rect.width() * value).max(rect.height());
    let fill = Rect::from_min_size(rect.min, Vec2::new(width.min(rect.width()), rect.height()));
    painter.rect_filled(fill, radius, tint);
    let gloss = Rect::from_min_max(
        fill.min + Vec2::new(1.0, 0.8),
        Pos2::new(fill.max.x - 1.0, fill.center().y),
    );
    painter.rect_filled(
        gloss,
        CornerRadius::same((gloss.height() * 0.5).round() as u8),
        brand::sheen(34),
    );
    if width > rect.height() * 1.4 {
        painter.circle_filled(
            Pos2::new(fill.right() - rect.height() * 0.5, rect.center().y),
            (rect.height() * 0.5 - 1.2).max(0.0),
            brand::sheen(56),
        );
    }
}

/// One range track: a thin, rounded bar that animates to its own share.
pub(crate) fn segment(ui: &mut Ui, id: egui::Id, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(3.0), 5.0), Sense::hover());
    let value = ui
        .ctx()
        .animate_value_with_time(id, fraction.clamp(0.0, 1.0), 0.38);
    let radius = CornerRadius::same(3);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, Color32::from_rgba_unmultiplied(255, 236, 244, 18));
    if value > 0.002 {
        let fill = Rect::from_min_size(
            rect.min,
            Vec2::new((rect.width() * value).max(3.0), rect.height()),
        );
        painter.rect_filled(fill, radius, Color32::from_rgba_unmultiplied(255, 90, 150, 220));
    }
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Download,
    Speed,
    Network,
    Clock,
    Settings,
    Info,
    Check,
    Alert,
}

pub(crate) fn icon(painter: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let r = rect.width().min(rect.height()) * 0.5;
    let c = rect.center();
    let stroke = Stroke::new(1.45_f32, color);
    let arc = |from: f32, to: f32, radius: f32, steps: usize| {
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let a = from + (to - from) * i as f32 / steps as f32;
            points.push(c + Vec2::new(a.cos(), a.sin()) * radius);
        }
        points
    };
    match kind {
        Icon::Download => {
            painter.line_segment(
                [c + Vec2::new(0.0, -r * 0.82), c + Vec2::new(0.0, r * 0.12)],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.42, -r * 0.28),
                    c + Vec2::new(0.0, r * 0.16),
                    c + Vec2::new(r * 0.42, -r * 0.28),
                ],
                stroke,
            ));
            painter.line_segment(
                [c + Vec2::new(-r * 0.72, r * 0.62), c + Vec2::new(r * 0.72, r * 0.62)],
                stroke,
            );
        }
        Icon::Speed => {
            painter.add(egui::Shape::line(
                arc(std::f32::consts::PI * 1.18, std::f32::consts::PI * 1.82, r * 0.86, 18),
                stroke,
            ));
            painter.line_segment([c + Vec2::new(0.0, 0.0), c + Vec2::new(r * 0.52, -r * 0.42)], stroke);
            painter.circle_filled(c, 1.6, color);
        }
        Icon::Network => {
            painter.circle_stroke(c, r * 0.82, stroke);
            painter.add(egui::Shape::line(
                arc(0.0, std::f32::consts::TAU, r * 0.38, 22)
                    .into_iter()
                    .map(|p| Pos2::new(p.x, c.y + (p.y - c.y) * 2.1))
                    .collect(),
                Stroke::new(1.2_f32, color),
            ));
            painter.line_segment(
                [c + Vec2::new(-r * 0.82, 0.0), c + Vec2::new(r * 0.82, 0.0)],
                Stroke::new(1.2_f32, color),
            );
        }
        Icon::Clock => {
            painter.circle_stroke(c, r * 0.82, stroke);
            painter.line_segment([c, c + Vec2::new(0.0, -r * 0.5)], stroke);
            painter.line_segment([c, c + Vec2::new(r * 0.4, r * 0.14)], stroke);
        }
        Icon::Settings => {
            for (offset, knob) in [(-r * 0.42, r * 0.36), (r * 0.38, -r * 0.5)] {
                painter.line_segment(
                    [
                        c + Vec2::new(-r * 0.82, offset),
                        c + Vec2::new(r * 0.82, offset),
                    ],
                    Stroke::new(1.3_f32, color),
                );
                painter.circle_filled(c + Vec2::new(knob, offset), r * 0.24, color);
            }
        }
        Icon::Check => {
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.62, r * 0.02),
                    c + Vec2::new(-r * 0.16, r * 0.48),
                    c + Vec2::new(r * 0.66, -r * 0.46),
                ],
                Stroke::new(1.6_f32, color),
            ));
        }
        Icon::Alert => {
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(0.0, -r * 0.78),
                    c + Vec2::new(-r * 0.76, r * 0.56),
                    c + Vec2::new(r * 0.76, r * 0.56),
                    c + Vec2::new(0.0, -r * 0.78),
                ],
                stroke,
            ));
            painter.circle_filled(c + Vec2::new(0.0, r * 0.12), 1.5, color);
            painter.line_segment(
                [
                    c + Vec2::new(0.0, r * 0.24),
                    c + Vec2::new(0.0, r * 0.44),
                ],
                Stroke::new(1.5_f32, color),
            );
        }
        Icon::Info => {
            painter.circle_stroke(c, r * 0.82, stroke);
            painter.circle_filled(c + Vec2::new(0.0, -r * 0.4), 1.5, color);
            painter.line_segment(
                [c + Vec2::new(0.0, -r * 0.1), c + Vec2::new(0.0, r * 0.45)],
                Stroke::new(1.7_f32, color),
            );
        }
    }
}

// ---------------------------------------------------------------- navigation

pub(crate) fn nav(ui: &mut Ui, kind: Icon, label: &str, active: bool, badge: Option<usize>) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, active, 0.2);
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("h"), response.hovered(), 0.12);
    let radius = CornerRadius::same(10);
    let painter = ui.painter();

    painter.rect_filled(
        rect,
        radius,
        brand::mix(Color32::TRANSPARENT, brand::sheen(20), hover),
    );
    if t > 0.01 {
        painter.rect_filled(
            rect,
            radius,
            Color32::from_rgba_unmultiplied(255, 240, 246, (34.0 * t) as u8),
        );
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(255, 236, 244, (46.0 * t) as u8),
            ),
            StrokeKind::Inside,
        );
    }

    let tint = brand::mix(brand::mix(brand::MUTED, brand::INK, hover), brand::INK, t);
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Vec2::splat(16.0),
    );
    icon(
        painter,
        icon_rect,
        kind,
        brand::mix(tint, brand::ACCENT_BRIGHT, t * 0.5),
    );

    let font = if active {
        brand::medium(12.8)
    } else {
        brand::medium(12.8)
    };
    painter.text(
        Pos2::new(rect.left() + 36.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        tint,
    );

    if let Some(count) = badge.filter(|count| *count > 0) {
        let text = count.to_string();
        let galley = painter.layout_no_wrap(text, brand::medium(10.5), brand::DIM);
        let size = Vec2::new(galley.size().x + 14.0, 18.0);
        let pill = Rect::from_center_size(
            Pos2::new(rect.right() - 14.0 - size.x * 0.5, rect.center().y),
            size,
        );
        painter.rect_filled(
            pill,
            CornerRadius::same(9),
            Color32::from_rgba_unmultiplied(255, 240, 246, 26),
        );
        painter.galley(pill.center() - galley.size() * 0.5, galley, tint);
    }

    response.clicked()
}

/// A small iOS-style switch, animated on change.
pub(crate) fn switch(ui: &mut Ui, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 20.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, on, 0.18);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(10),
        brand::mix(
            Color32::from_rgba_unmultiplied(255, 236, 244, 22),
            brand::ACCENT,
            t,
        ),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0_f32, brand::hairline()),
        StrokeKind::Inside,
    );
    let knob_x = rect.left() + 10.0 + (rect.width() - 20.0) * t;
    painter.circle_filled(
        Pos2::new(knob_x, rect.center().y),
        7.0,
        brand::mix(brand::MUTED, Color32::WHITE, t),
    );
    response
}

// ---------------------------------------------------------------- download row

pub(crate) fn download_row(ui: &mut Ui, job: &Job, selected: bool) -> Option<Action> {
    let mut action = None;

    brand::card_frame()
        .fill(brand::mix(
            brand::glass(),
            Color32::from_rgba_unmultiplied(255, 240, 246, 26),
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
                        RichText::new(format!(
                            "{} · {} · {} streams",
                            host_of(&job.url),
                            job.category.label(),
                            job.connections
                        ))
                        .size(10.8)
                        .color(brand::DIM),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_pill(ui, job.status);
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
                    for (index, (length, done, _start)) in segments.iter().enumerate() {
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

pub(crate) fn host_of(url: &str) -> String {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

pub(crate) fn show_in_folder(path: &Path) {
    if path.exists() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Some(folder) = path.parent() {
                let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
            }
        }
    } else if let Some(folder) = path.parent() {
        let folder = folder.to_path_buf();
        if folder.exists() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer").arg(&folder).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&folder).spawn();
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
            }
        }
    }
}

pub(crate) fn open_file(path: &Path) {
    if !path.exists() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
