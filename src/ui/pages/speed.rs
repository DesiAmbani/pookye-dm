//! Speed page: the system-wide limit and each download's share of it.

use super::prelude::*;

pub(super) fn draw(app: &mut App, ui: &mut egui::Ui) {
    page_heading(
        ui,
        "Speed",
        "All downloads together, over every connection.",
    );
    ui.add_space(14.0);

    let mut limit_changed = false;
    card(ui, |ui| {
        ui.label(
            RichText::new("System-wide limit")
                .font(brand::medium(13.5))
                .color(brand::INK),
        );
        ui.label(
            RichText::new("One budget shared by every stream of every download.")
                .size(10.8)
                .color(brand::DIM),
        );
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if chip(ui, "Unlimited", app.settings.speed_limit == 0) {
                app.settings.speed_limit = 0;
                limit_changed = true;
            }
            for megabytes in [1usize, 5, 10, 50] {
                let kbps = megabytes * 1024;
                if chip(
                    ui,
                    &format!("{megabytes} MB/s"),
                    app.settings.speed_limit == kbps,
                ) {
                    app.settings.speed_limit = kbps;
                    limit_changed = true;
                }
            }
        });
        ui.add_space(6.0);
        let mut slider = app.settings.speed_limit as u64;
        if ui
            .add(egui::Slider::new(&mut slider, 0..=102_400).text("KB/s · 0 is unlimited"))
            .changed()
        {
            app.settings.speed_limit = slider as usize;
            limit_changed = true;
        }
        ui.label(
            RichText::new(if app.settings.speed_limit == 0 {
                "Currently unlimited".to_string()
            } else {
                format!(
                    "Capped at {} of traffic per second",
                    format_bytes(app.settings.speed_limit as u64 * 1024)
                )
            })
            .size(10.8)
            .color(brand::DIM),
        );
    });
    if limit_changed {
        app.apply_limits();
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        metric(ui, "Average speed", format_speed(app.manager.total_speed()));
        metric(ui, "Downloaded", format_bytes(app.manager.total_downloaded()));
        metric(
            ui,
            "Streams",
            format!("{} per download", app.settings.connections),
        );
        metric(
            ui,
            "Concurrent",
            format!(
                "{} of {}",
                app.manager.downloading_count(),
                app.manager.max_concurrent
            ),
        );
    });

    ui.add_space(12.0);
    if app.manager.jobs.is_empty() {
        ui.label(
            RichText::new("No downloads to show yet.")
                .size(11.5)
                .color(brand::DIM),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for job in &app.manager.jobs {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(job.display_name())
                                .font(brand::medium(12.8))
                                .color(brand::INK),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format_speed(job.speed))
                                    .font(brand::medium(12.0))
                                    .color(w::status_colour(job.status)),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    let share = if app.manager.total_speed() > 0.0 {
                        (job.speed / app.manager.total_speed()).clamp(0.0, 1.0) as f32
                    } else {
                        0.0
                    };
                    w::progress(
                        ui,
                        egui::Id::new(("share", job.id)),
                        share,
                        6.0,
                        brand::ACCENT_BRIGHT,
                    );
                });
                ui.add_space(8.0);
            }
        });
}
