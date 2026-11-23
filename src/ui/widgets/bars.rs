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
