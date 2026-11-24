//! Progress bars and per-stream tracks.

use super::prelude::*;


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
    painter.rect_filled(rect, radius, brand::SURFACE_RAISED);
    if value <= 0.001 {
        return;
    }
    let width = (rect.width() * value).max(rect.height());
    let fill = Rect::from_min_size(rect.min, Vec2::new(width.min(rect.width()), rect.height()));
    painter.rect_filled(fill, radius, tint);
}

/// A filled area chart of sampled speeds, one sample per second.
pub(crate) fn sparkline(ui: &mut Ui, samples: &[f32], height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(brand::R_TILE),
        Color32::from_rgb(0x0E, 0x0D, 0x0F),
    );
    if samples.len() < 2 {
        return;
    }
    let peak = samples.iter().copied().fold(0.0_f32, f32::max).max(1.0);
    let step = rect.width() / (samples.len() - 1) as f32;
    let point = |index: usize, value: f32| {
        Pos2::new(
            rect.left() + index as f32 * step,
            rect.bottom() - (value / peak).clamp(0.0, 1.0) * (rect.height() - 8.0) - 4.0,
        )
    };
    let wash = Color32::from_rgba_unmultiplied(0xFF, 0x2E, 0x74, 42);
    for (index, value) in samples.iter().enumerate() {
        let top = point(index, *value);
        painter.line_segment(
            [Pos2::new(top.x, rect.bottom()), top],
            Stroke::new(step.max(1.0), wash),
        );
    }
    let line: Vec<Pos2> = samples
        .iter()
        .enumerate()
        .map(|(index, value)| point(index, *value))
        .collect();
    painter.add(egui::Shape::line(
        line,
        Stroke::new(1.4_f32, brand::ACCENT),
    ));
}

/// One range track: a thin, rounded bar that animates to its own share.
pub(crate) fn segment(ui: &mut Ui, id: egui::Id, fraction: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(3.0), 5.0), Sense::hover());
    let value = ui
        .ctx()
        .animate_value_with_time(id, fraction.clamp(0.0, 1.0), 0.38);
    let radius = CornerRadius::same(2);
    let painter = ui.painter();
    painter.rect_filled(rect, radius, brand::SURFACE_RAISED);
    if value > 0.002 {
        let fill = Rect::from_min_size(
            rect.min,
            Vec2::new((rect.width() * value).max(3.0), rect.height()),
        );
        painter.rect_filled(fill, radius, brand::ACCENT);
    }
}
