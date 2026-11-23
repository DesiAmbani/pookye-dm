//! Palette, typography, flat surfaces and the window backdrop.
//!
//! The interface is mostly black: solid near-black surfaces, no gradients, no
//! frosted sheets and no drop shadows. Pink carries the remaining weight — a
//! deep rose for buttons and selected fills, and one bright pink reserved for
//! the primary action and the live figures. The brand mark is embedded as a PNG
//! so the binary stays self-contained.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Stroke, Vec2,
};

/// The window background: flat black.
pub const BG: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0C);
/// A flat card / panel surface: black with a hint of rose.
pub const SURFACE: Color32 = Color32::from_rgb(0x15, 0x14, 0x16);
/// A raised control or a tile inside a card.
pub const SURFACE_RAISED: Color32 = Color32::from_rgb(0x1E, 0x1B, 0x1D);
/// The secondary colour: a deep rose used for secondary buttons and selections.
pub const ROSE: Color32 = Color32::from_rgb(0x7C, 0x2D, 0x48);
/// A very dark rose, for selected rows and quiet fills.
pub const ROSE_DEEP: Color32 = Color32::from_rgb(0x32, 0x17, 0x22);
/// The bright pink: primary buttons, progress and live figures only.
pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x2E, 0x74);
/// A lighter pink for text that needs to pop on black.
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0xFF, 0x5C, 0x8A);
/// Primary text.
pub const INK: Color32 = Color32::from_rgb(0xF7, 0xF3, 0xF5);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(0xC3, 0xA6, 0xB1);
/// Tertiary text, labels and hints.
pub const DIM: Color32 = Color32::from_rgb(0x8A, 0x6E, 0x79);

pub const R_CARD: u8 = 10;
pub const R_TILE: u8 = 7;
pub const R_PILL: u8 = 6;

/// The standard card surface.
pub fn glass() -> Color32 {
    SURFACE
}

/// A raised control surface, one step above a card.
pub fn glass_raised() -> Color32 {
    SURFACE_RAISED
}

/// The 1 px rule drawn around flat surfaces.
pub fn hairline() -> Color32 {
    Color32::from_rgb(0x2B, 0x24, 0x28)
}

/// A flat pink wash, used for hover states instead of a glass sheen.
pub fn sheen(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 46, 116, (alpha as u16 * 2).min(255) as u8)
}

/// The standard card surface: flat fill, hairline, no shadow.
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(glass())
        .stroke(Stroke::new(1.0_f32, hairline()))
        .corner_radius(CornerRadius::same(R_CARD))
        .inner_margin(Margin::same(16))
}

/// A card used inside another card.
pub fn tile_frame() -> Frame {
    Frame::new()
        .fill(glass_raised())
        .stroke(Stroke::new(1.0_f32, hairline()))
        .corner_radius(CornerRadius::same(R_TILE))
        .inner_margin(Margin::symmetric(14, 11))
}

/// The dialog surface: a flat sheet above the window.
pub fn window_frame() -> Frame {
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, hairline()))
        .corner_radius(CornerRadius::same(R_CARD))
        .inner_margin(Margin::same(20))
}

pub fn semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let faces = [
        (
            "inter",
            include_bytes!("../../assets/fonts/Inter_400Regular.ttf").as_slice(),
        ),
        (
            "inter-medium",
            include_bytes!("../../assets/fonts/Inter_500Medium.ttf").as_slice(),
        ),
        (
            "inter-semibold",
            include_bytes!("../../assets/fonts/Inter_600SemiBold.ttf").as_slice(),
        ),
    ];
    for (name, bytes) in faces {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            list.insert(0, "inter".to_owned());
        }
    }
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec!["inter-medium".to_owned(), "inter".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("semibold".into()),
        vec!["inter-semibold".to_owned(), "inter".to_owned()],
    );
    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(0x08, 0x08, 0x0A);
    visuals.faint_bg_color = SURFACE;
    visuals.override_text_color = Some(INK);
    visuals.hyperlink_color = ACCENT_BRIGHT;
    visuals.selection.bg_fill = ROSE;
    visuals.selection.stroke = Stroke::new(1.0_f32, INK);
    visuals.window_stroke = Stroke::new(1.0_f32, hairline());
    visuals.window_corner_radius = CornerRadius::same(R_CARD);
    visuals.menu_corner_radius = CornerRadius::same(R_TILE);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, hairline());
    widgets.noninteractive.corner_radius = CornerRadius::same(R_PILL);
    widgets.inactive.bg_fill = glass_raised();
    widgets.inactive.weak_bg_fill = glass_raised();
    widgets.inactive.bg_stroke = Stroke::new(1.0_f32, hairline());
    widgets.inactive.corner_radius = CornerRadius::same(R_PILL);
    widgets.hovered.bg_fill = Color32::from_rgb(0x2A, 0x1D, 0x24);
    widgets.hovered.weak_bg_fill = Color32::from_rgb(0x2A, 0x1D, 0x24);
    widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ROSE);
    widgets.hovered.corner_radius = CornerRadius::same(R_PILL);
    widgets.active.bg_fill = ROSE;
    widgets.active.weak_bg_fill = ROSE;
    widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    widgets.active.corner_radius = CornerRadius::same(R_PILL);
    widgets.open.bg_fill = glass_raised();
    widgets.open.weak_bg_fill = glass_raised();
    widgets.open.corner_radius = CornerRadius::same(R_PILL);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 9.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.bar_inner_margin = 4.0;
    style.text_styles = [
        (egui::TextStyle::Heading, semi(21.0)),
        (egui::TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
        (egui::TextStyle::Button, medium(13.0)),
        (egui::TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        (egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

/// Paints the window backdrop: one flat black fill.
pub fn paint_background(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(ctx.screen_rect(), CornerRadius::ZERO, BG);
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

/// Rasterises the traced brand mark from the embedded PNG.
pub fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let bytes = include_bytes!("../../assets/mark.png");
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(ctx.load_texture(
        "pookye-mark",
        egui::ColorImage { size, pixels },
        egui::TextureOptions::LINEAR,
    ))
}
