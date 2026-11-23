//! The left sidebar: new download, filters, pages and the slow-mode switch.

use super::prelude::*;

impl App {
    pub(crate) fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(232.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin {
                        left: 14,
                        right: 2,
                        top: 2,
                        bottom: 12,
                    })
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                let height = ui.available_height();
                egui::Frame::new()
                    .fill(brand::glass())
                    .stroke(Stroke::new(1.0_f32, brand::hairline()))
                    .corner_radius(CornerRadius::same(brand::R_CARD))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(height - 24.0);

                        if widgets::pill_sized(
                            ui,
                            "+  New download",
                            widgets::Pill::Primary,
                            ui.available_width(),
                        ) {
                            self.open_add();
                        }

                        ui.add_space(12.0);
                        widgets::section_label(ui, "Downloads");
                        ui.add_space(2.0);
                        for (filter, icon) in [
                            (Filter::All, widgets::Icon::Download),
                            (Filter::Active, widgets::Icon::Speed),
                            (Filter::Finished, widgets::Icon::Check),
                            (Filter::Failed, widgets::Icon::Alert),
                        ] {
                            let count = self.count_of(filter);
                            let active = self.page == Page::Downloads && self.filter == filter;
                            if widgets::nav(ui, icon, filter.label(), active, Some(count)) {
                                self.filter = filter;
                                self.go(Page::Downloads);
                            }
                        }

                        ui.add_space(14.0);
                        widgets::section_label(ui, "App");
                        ui.add_space(2.0);
                        if widgets::nav(
                            ui,
                            widgets::Icon::Settings,
                            "Settings",
                            self.page == Page::Settings,
                            None,
                        ) {
                            self.go(Page::Settings);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Clock,
                            "Schedule",
                            self.page == Page::Schedule,
                            Some(self.manager.queued_count()),
                        ) {
                            self.go(Page::Schedule);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Speed,
                            "Speed",
                            self.page == Page::Speed,
                            None,
                        ) {
                            self.go(Page::Speed);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Network,
                            "Networks",
                            self.page == Page::Networks,
                            None,
                        ) {
                            self.go(Page::Networks);
                        }
                        if widgets::nav(
                            ui,
                            widgets::Icon::Info,
                            "About",
                            self.page == Page::About,
                            None,
                        ) {
                            self.go(Page::About);
                        }

                        // Push the footer to the bottom of the sheet.
                        let remaining = ui.available_height();
                        if remaining > 92.0 {
                            ui.add_space(remaining - 92.0);
                        }
                        widgets::divider(ui);
                        ui.add_space(6.0);
                        {
                            ui.horizontal(|ui| {
                                let (dot, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                ui.painter().circle_filled(
                                    dot.center(),
                                    3.0,
                                    if self.manager.downloading_count() > 0 {
                                        brand::ACCENT_BRIGHT
                                    } else {
                                        brand::DIM
                                    },
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} active · {} queued",
                                        self.manager.downloading_count(),
                                        self.manager.queued_count()
                                    ))
                                    .size(10.4)
                                    .color(brand::DIM),
                                );
                                ui.with_layout(
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} done",
                                                self.manager.completed_count()
                                            ))
                                            .size(10.4)
                                            .color(brand::DIM),
                                        );
                                    },
                                );
                            });
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.label(
                                        RichText::new("Slow mode")
                                            .font(brand::medium(11.2))
                                            .color(brand::MUTED),
                                    );
                                    ui.label(
                                        RichText::new(format_speed(
                                            self.settings.slow_mode_speed as f64,
                                        ))
                                        .size(9.8)
                                        .color(brand::DIM),
                                    );
                                });
                                ui.with_layout(
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        if widgets::switch(ui, self.settings.slow_mode).clicked()
                                        {
                                            self.settings.slow_mode =
                                                !self.settings.slow_mode;
                                            self.dirty = true;
                                            self.apply_limits();
                                        }
                                    },
                                );
                            });
                        }
                    });
            });
    }
}
