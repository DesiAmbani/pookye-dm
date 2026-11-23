//! Cards, tiles, headings, metrics and rules.

use super::prelude::*;


pub(crate) fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    brand::card_frame().show(ui, add).inner
}

pub(crate) fn page_heading(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .font(brand::semi(23.0))
            .color(brand::INK),
    );
    ui.label(RichText::new(subtitle).size(11.8).color(brand::DIM));
}

pub(crate) fn metric(ui: &mut Ui, label: &str, value: String) {
    brand::tile_frame().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(label.to_uppercase()).size(9.2).color(brand::DIM));
            ui.label(RichText::new(value).font(brand::semi(16.0)).color(brand::INK));
        });
    });
}

/// A faint rule used to separate blocks inside a card.
pub(crate) fn divider(ui: &mut Ui) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::ZERO,
        Color32::from_rgba_unmultiplied(255, 226, 238, 22),
    );
}

pub(crate) fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(brand::medium(9.2))
            .color(brand::DIM),
    );
}
