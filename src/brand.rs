//! Brand palette and the painted brand mark, taken from the Pookye logo.

use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};

/// Deepest maroon, used for the window background (the logo's outer field).
pub const BG: Color32 = Color32::from_rgb(0x33, 0x01, 0x14);
/// Sidebar / header panels.
pub const PANEL: Color32 = Color32::from_rgb(0x4A, 0x02, 0x1E);
/// Cards holding downloads.
pub const CARD: Color32 = Color32::from_rgb(0x5B, 0x03, 0x25);
/// Hairlines and empty progress tracks.
pub const TRACK: Color32 = Color32::from_rgb(0x2A, 0x01, 0x10);
/// The crimson the logo's body is built from.
pub const ACCENT: Color32 = Color32::from_rgb(0xEA, 0x01, 0x59);
/// The hot pink highlight at the top of the logo.
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0xFF, 0x2D, 0x7A);
/// Near-white used for the logo's pale facets; also the primary text colour.
pub const INK: Color32 = Color32::from_rgb(0xFF, 0xE9, 0xF0);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(0xC2, 0x92, 0xA6);
/// Outlines, matching the black keyline inside the logo.
pub const OUTLINE: Color32 = Color32::from_rgb(0x14, 0x00, 0x06);

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = PANEL;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = TRACK;
    visuals.faint_bg_color = CARD;
    visuals.override_text_color = Some(INK);
    visuals.hyperlink_color = ACCENT_BRIGHT;
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke.color = INK;
    visuals.widgets.noninteractive.bg_fill = CARD;
    visuals.widgets.noninteractive.bg_stroke.color = TRACK;
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke.color = OUTLINE;
    visuals.widgets.hovered.bg_fill = ACCENT;
    visuals.widgets.hovered.bg_stroke.color = ACCENT_BRIGHT;
    visuals.widgets.active.bg_fill = ACCENT_BRIGHT;
    visuals.window_stroke.color = TRACK;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    ctx.set_style(style);
}

/// Draws the brand mark: a crimson tile with the logo's four pale facets.
/// The vector original lives in `assets/logo.svg`.
pub fn mark(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let painter = ui.painter();
    let radius = size * 0.22;

    painter.rect_filled(rect, radius, ACCENT);
    // A brighter band across the top, mirroring the logo's highlight.
    let band = Rect::from_min_max(
        rect.left_top() + Vec2::new(0.0, size * 0.06),
        rect.right_top() + Vec2::new(0.0, size * 0.42),
    );
    painter.rect_filled(band, radius * 0.5, ACCENT_BRIGHT);
    painter.rect_stroke(
        rect.shrink(size * 0.02),
        radius,
        Stroke::new(size * 0.05, OUTLINE),
        egui::StrokeKind::Inside,
    );

    let facet = |cx: f32, cy: f32, w: f32, flip: bool| {
        let centre = Pos2::new(rect.left() + size * cx, rect.top() + size * cy);
        let half = size * w * 0.5;
        let points = if flip {
            vec![
                centre + Vec2::new(-half, -half * 0.8),
                centre + Vec2::new(half, half * 0.6),
                centre + Vec2::new(-half, half * 0.9),
            ]
        } else {
            vec![
                centre + Vec2::new(-half, half * 0.9),
                centre + Vec2::new(half, -half * 0.8),
                centre + Vec2::new(half, half * 0.7),
            ]
        };
        painter.add(Shape::convex_polygon(
            points,
            INK,
            Stroke::new(size * 0.035, OUTLINE),
        ));
    };

    facet(0.36, 0.32, 0.22, true);
    facet(0.64, 0.36, 0.24, false);
    facet(0.36, 0.62, 0.24, false);
    facet(0.66, 0.66, 0.22, true);
}
