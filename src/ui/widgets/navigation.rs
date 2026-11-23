//! Sidebar navigation rows and the toggle switch.

use super::prelude::*;
use super::icons::{icon, Icon};


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
    let radius = CornerRadius::same(brand::R_PILL);
    let painter = ui.painter();

    painter.rect_filled(
        rect,
        radius,
        brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover),
    );
    if t > 0.01 {
        painter.rect_filled(rect, radius, brand::ROSE_DEEP);
        let bar = Rect::from_min_size(
            Pos2::new(rect.left(), rect.center().y - 7.0),
            Vec2::new(3.0, 14.0),
        );
        painter.rect_filled(bar, CornerRadius::same(2), brand::ACCENT);
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
        painter.rect_filled(pill, CornerRadius::same(brand::R_PILL), brand::SURFACE_RAISED);
        painter.galley(pill.center() - galley.size() * 0.5, galley, tint);
    }

    response.clicked()
}

/// A flat toggle switch, animated on change.
pub(crate) fn switch(ui: &mut Ui, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 20.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, on, 0.18);
    let radius = CornerRadius::same(brand::R_PILL);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        brand::mix(brand::SURFACE_RAISED, brand::ACCENT, t),
    );
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, brand::mix(brand::hairline(), brand::ACCENT, t)),
        StrokeKind::Inside,
    );
    let knob = Rect::from_center_size(
        Pos2::new(
            rect.left() + 11.0 + (rect.width() - 22.0) * t,
            rect.center().y,
        ),
        Vec2::splat(12.0),
    );
    painter.rect_filled(
        knob,
        CornerRadius::same(3),
        brand::mix(brand::MUTED, Color32::WHITE, t),
    );
    response
}
