//! The left sidebar: new download, filters, pages, networks and the slow-mode switch.
//!
//! Painted edge to edge on its own surface, with no card around it, so the page
//! beside it is the first and only framed block in the window. The column scrolls
//! once networks, the countdown and the footer no longer fit a short window,
//! instead of stacking into each other.

use super::prelude::*;

/// One clickable network row: state dot, name, and live speed or address.
/// Clicking it leaves the adapter out of future downloads, or puts it back.
fn net_row(
    ui: &mut egui::Ui,
    adapter: &crate::engine::Adapter,
    enabled: bool,
    speed: f64,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(brand::R_TILE),
        brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover),
    );
    let dot = if !enabled {
        brand::ROSE
    } else if speed > 1.0 {
        brand::ACCENT_BRIGHT
    } else {
        brand::MUTED
    };
    painter.circle_filled(Pos2::new(rect.left() + 7.0, rect.center().y), 2.8, dot);

    // The column is narrow: elide long adapter names, the full one is on hover.
    let name: String = if adapter.name.chars().count() > 14 {
        format!("{}…", adapter.name.chars().take(13).collect::<String>())
    } else {
        adapter.name.clone()
    };
    painter.text(
        Pos2::new(rect.left() + 15.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        brand::medium(11.2),
        if enabled { brand::MUTED } else { brand::DIM },
    );
    let right = if !enabled {
        "off".to_string()
    } else if speed > 1.0 {
        format_speed(speed)
    } else {
        adapter.ip.to_string()
    };
    painter.text(
        Pos2::new(rect.right() - 5.0, rect.center().y),
        Align2::RIGHT_CENTER,
        right,
        brand::medium(9.8),
        if !enabled {
            brand::ROSE
        } else if speed > 1.0 {
            brand::ACCENT_BRIGHT
        } else {
            brand::DIM
        },
    );

    let hover_text = match adapter.route_hint() {
        Some(why) => format!("{} — {why}", adapter.label()),
        None => adapter.label(),
    };
    response.on_hover_text(hover_text)
}

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
                egui::ScrollArea::vertical()
                    .id_salt("sidebar-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
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

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            widgets::section_label(ui, "Networks");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(self.adapters.len().to_string())
                                        .size(10.0)
                                        .color(brand::DIM),
                                );
                            });
                        });
                        ui.label(
                            RichText::new("Click a network to leave it out")
                                .size(9.4)
                                .color(brand::DIM),
                        );
                        ui.add_space(2.0);
                        for adapter in self.adapters.clone() {
                            let enabled = self.adapter_enabled(&adapter);
                            let speed = self.network_speed(&adapter);
                            if net_row(ui, &adapter, enabled, speed).clicked() {
                                self.toggle_adapter(&adapter);
                            }
                            if !enabled {
                                // Indented and wrapped: the reason is a sentence, not a
                                // label that can run off the side of the column.
                                egui::Frame::new()
                                    .inner_margin(Margin {
                                        left: 15,
                                        right: 4,
                                        top: 0,
                                        bottom: 2,
                                    })
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(format!(
                                                    "Left out · {}",
                                                    adapter.route_hint().unwrap_or(
                                                        "its share fails when this adapter has no \
                                                         route to the server"
                                                    )
                                                ))
                                                .size(9.4)
                                                .color(brand::ROSE),
                                            )
                                            .wrap(),
                                        );
                                    });
                            }
                        }

                        ui.add_space(10.0);
                        if let Some(left) = self.manager.time_left() {
                            egui::Frame::new()
                                .fill(brand::ROSE_DEEP)
                                .stroke(egui::Stroke::new(1.0_f32, brand::ROSE))
                                .corner_radius(egui::CornerRadius::same(brand::R_CARD))
                                .inner_margin(Margin::symmetric(12, 9))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(
                                        RichText::new("Time left").size(9.8).color(brand::DIM),
                                    );
                                    ui.label(
                                        RichText::new(crate::engine::format_clock(left))
                                            .font(brand::semi(19.0))
                                            .color(brand::ACCENT_BRIGHT),
                                    );
                                });
                            ui.add_space(8.0);
                        }

                        widgets::divider(ui);
                        ui.add_space(6.0);
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
                        ui.add_space(6.0);
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
                    });
            });
    }
}
