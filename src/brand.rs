//! Brand palette and logo loading, taken from the Pookye logo.

use eframe::egui::{self, Color32, TextureHandle};

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

const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");

pub fn load_logo(ctx: &egui::Context) -> Option<TextureHandle> {
    let image = image::load_from_memory_with_format(LOGO_PNG, image::ImageFormat::Png).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture("pookye-logo", color_image, egui::TextureOptions::LINEAR))
}

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
