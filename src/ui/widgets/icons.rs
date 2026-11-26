//! The small line icons drawn for the sidebar.

use super::prelude::*;


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
    Play,
    Pause,
    Folder,
    External,
    Retry,
    Trash,
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
        Icon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(-r * 0.44, -r * 0.62),
                    c + Vec2::new(r * 0.66, 0.0),
                    c + Vec2::new(-r * 0.44, r * 0.62),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            for offset in [-r * 0.36, r * 0.36] {
                painter.rect_filled(
                    Rect::from_center_size(
                        c + Vec2::new(offset, 0.0),
                        Vec2::new(r * 0.34, r * 1.24),
                    ),
                    CornerRadius::same(1),
                    color,
                );
            }
        }
        Icon::Folder => {
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.82, r * 0.62),
                    c + Vec2::new(-r * 0.82, -r * 0.5),
                    c + Vec2::new(-r * 0.24, -r * 0.5),
                    c + Vec2::new(-r * 0.04, -r * 0.26),
                    c + Vec2::new(r * 0.82, -r * 0.26),
                    c + Vec2::new(r * 0.82, r * 0.62),
                    c + Vec2::new(-r * 0.82, r * 0.62),
                ],
                stroke,
            ));
        }
        Icon::External => {
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.74, r * 0.02),
                    c + Vec2::new(-r * 0.74, r * 0.72),
                    c + Vec2::new(r * 0.72, r * 0.72),
                    c + Vec2::new(r * 0.72, -r * 0.02),
                ],
                stroke,
            ));
            painter.line_segment(
                [
                    c + Vec2::new(-r * 0.12, r * 0.12),
                    c + Vec2::new(r * 0.74, -r * 0.74),
                ],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(r * 0.24, -r * 0.74),
                    c + Vec2::new(r * 0.74, -r * 0.74),
                    c + Vec2::new(r * 0.74, -r * 0.24),
                ],
                stroke,
            ));
        }
        Icon::Retry => {
            let start = std::f32::consts::PI * 1.15;
            painter.add(egui::Shape::line(
                arc(start, std::f32::consts::PI * 2.35, r * 0.7, 20),
                stroke,
            ));
            let tip = c + Vec2::new(start.cos(), start.sin()) * r * 0.7;
            painter.add(egui::Shape::line(
                vec![
                    tip + Vec2::new(-r * 0.3, -r * 0.02),
                    tip,
                    tip + Vec2::new(r * 0.02, r * 0.3),
                ],
                stroke,
            ));
        }
        Icon::Trash => {
            painter.line_segment(
                [
                    c + Vec2::new(-r * 0.72, -r * 0.5),
                    c + Vec2::new(r * 0.72, -r * 0.5),
                ],
                stroke,
            );
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.22, -r * 0.5),
                    c + Vec2::new(-r * 0.22, -r * 0.72),
                    c + Vec2::new(r * 0.22, -r * 0.72),
                    c + Vec2::new(r * 0.22, -r * 0.5),
                ],
                stroke,
            ));
            painter.add(egui::Shape::line(
                vec![
                    c + Vec2::new(-r * 0.54, -r * 0.3),
                    c + Vec2::new(-r * 0.44, r * 0.74),
                    c + Vec2::new(r * 0.44, r * 0.74),
                    c + Vec2::new(r * 0.54, -r * 0.3),
                ],
                stroke,
            ));
        }
    }
}
