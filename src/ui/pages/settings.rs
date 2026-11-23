//! Settings page: folder, segments, concurrency and slow mode.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
    page_heading(ui, "Settings", "Where downloads go and how hard they push.");
    ui.add_space(14.0);
    let mut changed = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            card(ui, |ui| {
                ui.label(
                    RichText::new("Storage")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(6.0);
                ui.label(RichText::new("Download folder").size(10.8).color(brand::DIM));
                egui::Frame::new()
                    .fill(Color32::from_rgb(0x0E, 0x0D, 0x0F))
                    .stroke(egui::Stroke::new(1.0_f32, brand::hairline()))
                    .corner_radius(CornerRadius::same(brand::R_PILL))
                    .inner_margin(Margin::symmetric(12, 4))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.settings.folder)
                                    .frame(false)
                                    .desired_width(f32::INFINITY),
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
                ui.label(
                    RichText::new("Downloads run from this folder. Missing folders are created.")
                        .size(10.2)
                        .color(brand::DIM),
                );
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Performance")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Segments per download")
                        .size(10.8)
                        .color(brand::DIM),
                );
                if ui
                    .add(egui::Slider::new(&mut app.settings.connections, 1..=32))
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    RichText::new(format!(
                        "{} parallel connections. Use one for servers that block parallel requests.",
                        app.settings.connections
                    ))
                    .size(10.2)
                    .color(brand::DIM),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Downloads at once")
                        .size(10.8)
                        .color(brand::DIM),
                );
                if ui
                    .add(egui::Slider::new(&mut app.settings.max_concurrent, 1..=10))
                    .changed()
                {
                    changed = true;
                }
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Slow mode")
                                .font(brand::medium(13.5))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new(
                                "Holds every download to one gentle speed, for a call or a game.",
                            )
                            .size(10.4)
                            .color(brand::DIM),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if w::switch(ui, app.settings.slow_mode).clicked() {
                            app.settings.slow_mode = !app.settings.slow_mode;
                            changed = true;
                        }
                    });
                });
                if app.settings.slow_mode {
                    ui.add_space(6.0);
                    let mut mb = app.settings.slow_mode_speed as f64 / (1024.0 * 1024.0);
                    if ui
                        .add(
                            egui::Slider::new(&mut mb, 0.25..=20.0)
                                .suffix(" MB/s")
                                .fixed_decimals(2),
                        )
                        .changed()
                    {
                        app.settings.slow_mode_speed = (mb * 1024.0 * 1024.0) as usize;
                        changed = true;
                    }
                }
            });

            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new("Behaviour")
                        .font(brand::medium(13.5))
                        .color(brand::INK),
                );
                ui.add_space(4.0);
                if ui
                    .checkbox(
                        &mut app.settings.clipboard,
                        "Watch the clipboard for copied links",
                    )
                    .changed()
                {
                    if app.settings.clipboard && app.clipboard.is_none() {
                        app.clipboard = arboard::Clipboard::new().ok();
                    }
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} entries in history",
                            app.settings.history.len()
                        ))
                        .size(10.8)
                        .color(brand::DIM),
                    );
                    if !app.settings.history.is_empty() && pill(ui, "Clear history", Pill::Quiet) {
                        app.settings.history.clear();
                        changed = true;
                    }
                });
            });

            if changed {
                app.apply_limits();
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new("Settings are saved automatically.")
                    .size(10.2)
                    .color(brand::DIM),
            );
        });
}
