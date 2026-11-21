//! Brand palette, theme, and the runtime-drawn brand mark.
//!
//! The mark is rasterised into a texture at start-up from the same shapes and
//! colours as `assets/logo.svg`, so the binary stays self-contained.

use eframe::egui::{self, Color32, TextureHandle, TextureOptions};

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

const MARK_PIXELS: usize = 128;

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

/// Rasterises the brand mark and hands it to egui as a texture.
pub fn load_logo(ctx: &egui::Context) -> Option<TextureHandle> {
    let image = mark_image(MARK_PIXELS);
    Some(ctx.load_texture("pookye-mark", image, TextureOptions::LINEAR))
}

fn mark_image(size: usize) -> egui::ColorImage {
    let n = size as f32;
    let mut pixels = vec![Color32::TRANSPARENT; size * size];

    // The tile the mark sits on: a rounded square with the logo's keyline.
    let inset = 0.02 * n;
    let radius = 0.22 * n;
    let half = n * 0.5 - inset;
    let centre = n * 0.5;
    let keyline = 0.045 * n;

    // The logo's four pale facets, in tile-relative coordinates.
    let facets = [
        ([0.30, 0.34], [0.48, 0.28], [0.40, 0.50]),
        ([0.58, 0.30], [0.76, 0.46], [0.56, 0.48]),
        ([0.28, 0.62], [0.48, 0.56], [0.30, 0.80]),
        ([0.58, 0.62], [0.78, 0.76], [0.58, 0.84]),
    ];
    let facet_width = 0.035 * n;

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let tile_d = rounded_box_distance(px - centre, py - centre, half, radius);
            let tile = coverage(tile_d);
            if tile <= 0.0 {
                continue;
            }

            // Body colour: bright pink at the crown fading into crimson.
            let t = ((py / n) - 0.10).clamp(0.0, 1.0);
            let mut colour = mix(ACCENT_BRIGHT, ACCENT, t);

            // Facets, drawn with their own dark keyline.
            for (a, b, c) in facets {
                let tri = [
                    [a[0] * n, a[1] * n],
                    [b[0] * n, b[1] * n],
                    [c[0] * n, c[1] * n],
                ];
                let d = triangle_distance(px, py, tri);
                let stroke = coverage(facet_width - d.abs());
                let fill = coverage(-d);
                if stroke > 0.0 {
                    colour = blend(colour, OUTLINE, stroke);
                }
                if fill > 0.0 {
                    colour = blend(colour, INK, fill);
                }
            }

            // Keyline around the tile itself.
            let ring = coverage(keyline - (tile_d + keyline).abs()) * coverage(-keyline - tile_d);
            if ring > 0.0 {
                colour = blend(colour, OUTLINE, ring);
            }

            pixels[y * size + x] = Color32::from_rgba_unmultiplied(
                colour.r(),
                colour.g(),
                colour.b(),
                (tile * 255.0).round() as u8,
            );
        }
    }

    egui::ColorImage {
        size: [size, size],
        pixels,
    }
}

fn rounded_box_distance(dx: f32, dy: f32, half: f32, radius: f32) -> f32 {
    let qx = dx.abs() - (half - radius);
    let qy = dy.abs() - (half - radius);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

fn triangle_distance(px: f32, py: f32, points: [[f32; 2]; 3]) -> f32 {
    let mut distance = f32::NEG_INFINITY;
    for i in 0..3 {
        let (ax, ay) = (points[i][0], points[i][1]);
        let (bx, by) = (points[(i + 1) % 3][0], points[(i + 1) % 3][1]);
        let (ex, ey) = (bx - ax, by - ay);
        let length = (ex * ex + ey * ey).sqrt().max(f32::EPSILON);
        // Positive inside, for a counter-clockwise triangle.
        let side = (ex * (py - ay) - ey * (px - ax)) / length;
        distance = distance.max(-side);
    }
    distance
}

fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

fn blend(base: Color32, top: Color32, alpha: f32) -> Color32 {
    mix(base, top, alpha)
}
