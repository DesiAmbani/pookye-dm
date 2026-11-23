//! Palette, typography, glass surfaces and the window backdrop.
//!
//! Everything is translucent: the window paints one deep-maroon gradient with
//! two soft accent lights, and each surface on top is a thin frosted sheet so
//! that gradient glows through. The brand mark is the traced logo, embedded as
//! a PNG so the binary stays self-contained.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Shadow, Stroke, Vec2,
};

/// Top of the window gradient.
pub const BG: Color32 = Color32::from_rgb(0x1A, 0x01, 0x0F);
/// Bottom of the window gradient.
pub const BG_DEEP: Color32 = Color32::from_rgb(0x08, 0x00, 0x05);
/// The crimson the logo body is built from.
pub const ACCENT: Color32 = Color32::from_rgb(0xE8, 0x01, 0x54);
/// The hot pink highlight of the logo.
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0xFF, 0x3D, 0x7F);
/// Primary text.
pub const INK: Color32 = Color32::from_rgb(0xFF, 0xF4, 0xF8);
/// Secondary text.
pub const MUTED: Color32 = Color32::from_rgb(0xC7, 0x9C, 0xAF);
/// Tertiary text, labels and hints.
pub const DIM: Color32 = Color32::from_rgb(0x8C, 0x60, 0x73);

pub const R_CARD: u8 = 18;
pub const R_TILE: u8 = 14;
pub const R_PILL: u8 = 9;

/// A frosted surface: barely-there white over the backdrop. Sheets are kept very
/// thin because panels, cards and tiles stack; a strong fill on each would
/// bleach the maroon backdrop into flat mauve.
pub fn glass() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 238, 245, 7)
}

/// A slightly stronger sheet, for controls resting on a card.
pub fn glass_raised() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 238, 245, 13)
}

/// Hairline around a glass surface.
pub fn hairline() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 226, 238, 26)
}

/// Soft white used for the light that catches the top edge of a surface.
pub fn sheen(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 245, 249, alpha)
}

pub fn shadow() -> Shadow {
    Shadow {
        offset: [0, 14],
        blur: 36,
        spread: 0,
        color: Color32::from_black_alpha(70),
    }
}

/// The standard card surface.
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(glass())
        .stroke(Stroke::new(1.0_f32, hairline()))
        .corner_radius(CornerRadius::same(R_CARD))
        .inner_margin(Margin::same(16))
        .shadow(shadow())
}

/// A card used inside another card, without its own shadow.
pub fn tile_frame() -> Frame {
    Frame::new()
        .fill(glass_raised())
        .corner_radius(CornerRadius::same(R_TILE))
        .inner_margin(Margin::symmetric(14, 11))
}

/// The dialog surface: a taller sheet with a deeper shadow.
pub fn window_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgb(0x1C, 0x01, 0x11))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 226, 238, 34)))
        .corner_radius(CornerRadius::same(22))
        .inner_margin(Margin::same(20))
        .shadow(Shadow {
            offset: [0, 24],
            blur: 60,
            spread: 0,
            color: Color32::from_black_alpha(150),
        })
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
            include_bytes!("../assets/fonts/Inter_400Regular.ttf").as_slice(),
        ),
        (
            "inter-medium",
            include_bytes!("../assets/fonts/Inter_500Medium.ttf").as_slice(),
        ),
        (
            "inter-semibold",
            include_bytes!("../assets/fonts/Inter_600SemiBold.ttf").as_slice(),
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
    visuals.window_fill = Color32::from_rgb(0x1C, 0x01, 0x11);
    visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(10, 0, 6, 120);
    visuals.faint_bg_color = glass();
    visuals.override_text_color = Some(INK);
    visuals.hyperlink_color = ACCENT_BRIGHT;
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(232, 1, 84, 90);
    visuals.selection.stroke = Stroke::new(1.0_f32, INK);
    visuals.window_stroke = Stroke::new(1.0_f32, hairline());
    visuals.window_corner_radius = CornerRadius::same(20);
    visuals.menu_corner_radius = CornerRadius::same(14);
    visuals.window_shadow = Shadow {
        offset: [0, 18],
        blur: 44,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    visuals.popup_shadow = Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, hairline());
    widgets.noninteractive.corner_radius = CornerRadius::same(R_PILL);
    widgets.inactive.bg_fill = glass_raised();
    widgets.inactive.weak_bg_fill = glass_raised();
    widgets.inactive.bg_stroke = Stroke::new(1.0_f32, hairline());
    widgets.inactive.corner_radius = CornerRadius::same(R_PILL);
    widgets.hovered.bg_fill = Color32::from_rgba_unmultiplied(255, 240, 247, 38);
    widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 240, 247, 38);
    widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 226, 238, 60));
    widgets.hovered.corner_radius = CornerRadius::same(R_PILL);
    widgets.active.bg_fill = Color32::from_rgba_unmultiplied(255, 240, 247, 52);
    widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 240, 247, 52);
    widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_BRIGHT);
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

/// Paints the window backdrop: one vertical gradient plus two accent lights.
pub fn paint_background(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), BG);
    mesh.colored_vertex(rect.right_top(), BG);
    mesh.colored_vertex(rect.left_bottom(), BG_DEEP);
    mesh.colored_vertex(rect.right_bottom(), BG_DEEP);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(egui::Shape::mesh(mesh));

    let wide = rect.width().max(1000.0);
    radial_glow(
        &painter,
        egui::pos2(rect.left() + wide * 0.72, rect.top() + rect.height() * 0.02),
        wide * 0.42,
        Color32::from_rgba_unmultiplied(255, 42, 120, 24),
    );
    radial_glow(
        &painter,
        egui::pos2(rect.left() + wide * 0.02, rect.bottom() + 60.0),
        wide * 0.36,
        Color32::from_rgba_unmultiplied(232, 1, 84, 18),
    );
}

/// A soft light: one fan mesh, interpolated by the GPU, so the falloff is smooth.
pub fn radial_glow(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    let clear = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    let steps = 48;
    for i in 0..=steps {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            center + egui::vec2(angle.cos(), angle.sin()) * radius,
            clear,
        );
    }
    for i in 1..=steps {
        mesh.add_triangle(0, i as u32, (i % steps + 1) as u32);
    }
    painter.add(egui::Shape::mesh(mesh));
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
    let bytes = include_bytes!("../assets/mark.png");
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
