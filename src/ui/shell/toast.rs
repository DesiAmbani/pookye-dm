//! The transient notice shown above the status bar.

use super::prelude::*;

impl App {
    pub(crate) fn toast(&mut self, ctx: &egui::Context) {
        let Some((message, bad, at)) = self.notice.clone() else {
            return;
        };
        let age = at.elapsed().as_secs_f32();
        let life = 4.2;
        if age > life {
            self.notice = None;
            return;
        }
        let alpha = if age < 0.2 {
            age / 0.2
        } else if age > life - 0.6 {
            ((life - age) / 0.6).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let lift = (1.0 - alpha) * 10.0;
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -52.0 + lift))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                let tint = if bad {
                    Color32::from_rgb(0xFF, 0x8F, 0x9E)
                } else {
                    brand::INK
                };
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(
                        22,
                        21,
                        23,
                        (228.0 * alpha) as u8,
                    ))
                    .stroke(Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 46, 116, (110.0 * alpha) as u8),
                    ))
                    .corner_radius(CornerRadius::same(brand::R_CARD))
                    .inner_margin(Margin::symmetric(18, 10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(message)
                                .font(brand::medium(12.0))
                                .color(tint.gamma_multiply(alpha.max(0.1))),
                        );
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
