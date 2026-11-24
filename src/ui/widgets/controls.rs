//! Flat buttons, chips and the round selection control.

use super::prelude::*;


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
    let radius = CornerRadius::same(brand::R_PILL);

    // Flat fills only: bright pink for the one primary action, deep rose for a
    // secondary one, and a borderless text button for everything else.
    let (fill, text, border) = match kind {
        Pill::Primary => (
            brand::mix(brand::ACCENT, brand::ACCENT_BRIGHT, hover * 0.6),
            Color32::from_rgb(0x12, 0x05, 0x0B),
            None,
        ),
        Pill::Glass => (
            brand::mix(brand::ROSE, brand::ACCENT, hover * 0.45),
            brand::INK,
            None,
        ),
        Pill::Quiet => (
            brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover),
            brand::mix(brand::MUTED, brand::INK, hover),
            Some(brand::hairline()),
        ),
    };

    let painter = ui.painter();
    painter.rect_filled(rect, radius, fill);
    if let Some(colour) = border {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0_f32, colour),
            StrokeKind::Inside,
        );
    }
    if pressed {
        painter.rect_filled(rect, radius, Color32::from_black_alpha(40));
    }

    let text_pos = rect.center() - galley.size() * 0.5;
    painter.galley(text_pos, galley, text);

    response.clicked()
}

/// A bare text action, the way QDM labels `Clear list` beside a list heading.
pub(crate) fn text_link(ui: &mut Ui, label: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        brand::medium(11.5),
        Color32::PLACEHOLDER,
    );
    let size = Vec2::new(galley.size().x + 6.0, galley.size().y + 6.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    ui.painter().galley(
        rect.center() - galley.size() * 0.5,
        galley,
        brand::mix(brand::MUTED, brand::INK, t),
    );
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
    let radius = CornerRadius::same(brand::R_PILL);

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        brand::mix(
            brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover),
            brand::ROSE,
            t,
        ),
    );
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0_f32, brand::mix(brand::hairline(), brand::ACCENT, t)),
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
