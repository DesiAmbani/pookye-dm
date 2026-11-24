//! The bottom status bar: speed, totals, folder and the bridge port.

use super::prelude::*;

impl App {
    pub(crate) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(brand::STATUS_H)
            .frame(egui::Frame::new().fill(brand::PANEL))
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                ui.painter().hline(
                    bar.x_range(),
                    bar.top() + 0.5,
                    Stroke::new(1.0_f32, brand::hairline()),
                );
                ui.horizontal_centered(|ui| {
                    ui.add_space(18.0);
                    let speed = self.manager.total_speed();
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        2.6,
                        if speed > 0.0 {
                            brand::ACCENT_BRIGHT
                        } else {
                            brand::DIM
                        },
                    );
                    ui.label(
                        RichText::new(if speed > 0.0 {
                            format!("Average speed {}", format_speed(speed))
                        } else {
                            "Idle".to_string()
                        })
                        .size(10.6)
                        .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} downloaded · {} streams per network",
                            format_bytes(self.manager.total_downloaded()),
                            self.settings.connections
                        ))
                        .size(10.6)
                        .color(brand::DIM),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(18.0);
                        let awaiting = self.bridge.awaiting_count();
                        let bridge = match self.bridge.port() {
                            Some(port) if awaiting > 0 => {
                                format!("Browser bridge · {port} · {awaiting} to confirm")
                            }
                            Some(port) => format!("Browser bridge · {port}"),
                            None => "Browser bridge offline".to_string(),
                        };
                        ui.label(RichText::new(bridge).size(10.2).color(brand::DIM));
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(self.settings.folder.as_str())
                                .size(10.2)
                                .color(brand::DIM),
                        );
                    });
                });
            });
    }
}
