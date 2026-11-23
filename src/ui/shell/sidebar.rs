//! The left sidebar: new download, filters, pages and the slow-mode switch.
//!
//! Painted edge to edge on its own surface, with no card around it, so the page
//! beside it is the first and only framed block in the window.

use super::prelude::*;

impl App {
    pub(crate) fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(brand::SIDEBAR_W)
            .frame(
                egui::Frame::new()
                    .fill(brand::PANEL)
                    .inner_margin(Margin {
                        left: 12,
                        right: 12,
                        top: 12,
                        bottom: 12,
                    }),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);

                if widgets::pill_sized(
                    ui,
                    "+  New download",
                    widgets::Pill::Primary,
                    ui.available_width(),
                ) {
                    self.open_add();
                }

                ui.add_space(10.0);
                widgets::section_label(ui, "Downloads");
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

                ui.add_space(10.0);
                widgets::section_label(ui, "App");
                for (page, icon, label) in [
                    (Page::Settings, widgets::Icon::Settings, "Settings"),
                    (Page::Schedule, widgets::Icon::Clock, "Schedule"),
                    (Page::Speed, widgets::Icon::Speed, "Speed"),
                    (Page::Networks, widgets::Icon::Network, "Networks"),
                    (Page::About, widgets::Icon::Info, "About"),
                ] {
                    if widgets::nav(ui, icon, label, self.page == page, None) {
                        self.go(page);
                    }
                }

                // The footer belongs to the bottom of the column, whatever the
                // window height is, instead of being pushed by a magic number.
                let rest = ui.available_height();
                if rest > 0.0 {
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), rest),
                        Layout::bottom_up(Align::Min),
                        |ui| {
                            // Bottom-up: the first block added is the bottom one.
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} active · {} queued",
                                        self.manager.downloading_count(),
                                        self.manager.queued_count()
                                    ))
                                    .size(10.2)
                                    .color(brand::DIM),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} done",
                                            self.manager.completed_count()
                                        ))
                                        .size(10.2)
                                        .color(brand::DIM),
                                    );
                                });
                            });
                            ui.add_space(6.0);
                            widgets::divider(ui);
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
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if widgets::switch(ui, self.settings.slow_mode).clicked() {
                                        self.settings.slow_mode = !self.settings.slow_mode;
                                        self.dirty = true;
                                        self.apply_limits();
                                    }
                                });
                            });
                        },
                    );
                }
            });
    }
}
