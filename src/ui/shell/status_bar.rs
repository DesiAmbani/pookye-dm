//! The bottom status bar: speed, totals, folder and the bridge port.

use super::prelude::*;

impl App {
    pub(crate) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 22,
                        right: 22,
                        top: 4,
                        bottom: 10,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let speed = self.manager.total_speed();
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        3.0,
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
                        .size(10.8)
                        .color(brand::MUTED),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} downloaded · {} streams per download",
                            format_bytes(self.manager.total_downloaded()),
                            self.settings.connections
                        ))
                        .size(10.8)
                        .color(brand::DIM),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let awaiting = self.bridge.awaiting_count();
                        match self.bridge.port() {
                            Some(port) if awaiting > 0 => ui.label(
                                RichText::new(format!(
                                    "Browser bridge · {port} · {awaiting} to confirm"
                                ))
                                .size(10.2)
                                .color(brand::DIM),
                            ),
                            Some(port) => ui.label(
                                RichText::new(format!("Browser bridge · {port}"))
                                    .size(10.2)
                                    .color(brand::DIM),
                            ),
                            None => ui.label(
                                RichText::new("Browser bridge offline")
                                    .size(10.2)
                                    .color(brand::DIM),
                            ),
                        };
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
