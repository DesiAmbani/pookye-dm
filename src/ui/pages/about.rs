//! About page: version, licence and lineage.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "About",
        "Version, licence and where this project comes from.",
    );
    ui.add_space(14.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), Sense::hover());
                    if let Some(logo) = &app.logo {
                        ui.painter().image(
                            logo.id(),
                            rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Pookye Download Manager")
                                .font(brand::semi(18.0))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(format!("PDM · version {}", engine::VERSION))
                                .size(11.2)
                                .color(brand::MUTED),
                        );
                        ui.label(
                            RichText::new("Multi-segment downloads in Rust, MIT licensed.")
                                .size(11.2)
                                .color(brand::DIM),
                        );
                    });
                });
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                metric(ui, "Version", engine::VERSION.to_string());
                metric(ui, "Licence", "MIT".to_string());
                metric(ui, "Streams per network", "up to 32".to_string());
                metric(
                    ui,
                    "Networks",
                    format!("{} found", app.adapters.len()),
                );
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Lineage")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "The download-manager model here — parallel byte ranges, spreading one \
                         file over several network adapters, a queue with a concurrency limit, \
                         browser interception and per-connection telemetry — follows QDM \
                         (PBhadoo/QDM), which is MIT licensed. Pookye DM is an independent Rust \
                         implementation: the engine, interface, local API and browser bridge here \
                         are original code.",
                    )
                    .size(11.0)
                    .color(brand::MUTED),
                );
                ui.add_space(6.0);
                ui.hyperlink_to(
                    RichText::new(REPO_URL).size(11.0).color(brand::ACCENT_BRIGHT),
                    REPO_URL,
                );
            });
        });
}
