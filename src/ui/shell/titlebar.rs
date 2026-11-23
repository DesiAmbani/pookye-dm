//! The custom title bar: brand block, drag region and window controls.
//!
//! The three buttons sit flush against the top-right corner the way Windows
//! draws them: 46 × 48 each, no gaps, minimize / maximize / close from left to
//! right, so close is the outermost one. The rest of the bar drags the window.

use super::prelude::*;

impl App {
    pub(crate) fn title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(brand::TITLE_H)
            .frame(egui::Frame::new().fill(brand::BG))
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let controls = WindowGlyph::WIDTH * 3.0;
                let drag = Rect::from_min_max(
                    bar.min,
                    Pos2::new(bar.right() - controls, bar.max.y),
                );
                let response =
                    ui.interact(drag, egui::Id::new("titlebar-drag"), Sense::click_and_drag());
                if response.drag_started() && self.resizing.is_none() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if response.double_clicked() {
                    self.toggle_maximize(ctx);
                }

                let brand_rect = Rect::from_min_max(
                    Pos2::new(bar.left() + 18.0, bar.top()),
                    Pos2::new(bar.right() - controls, bar.max.y),
                );
                let mut left = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(brand_rect)
                        .layout(Layout::left_to_right(Align::Center)),
                );
                let (mark, _) = left.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                if let Some(logo) = &self.logo {
                    left.painter().image(
                        logo.id(),
                        mark,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                left.add_space(9.0);
                left.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(
                        RichText::new("Pookye DM")
                            .font(brand::semi(13.5))
                            .color(brand::INK),
                    );
                    ui.label(
                        RichText::new("Download Manager")
                            .size(10.0)
                            .color(brand::DIM),
                    );
                });

                let controls_rect =
                    Rect::from_min_max(Pos2::new(bar.right() - controls, bar.top()), bar.max);
                let mut right = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(controls_rect)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                right.spacing_mut().item_spacing = Vec2::ZERO;
                if window_button(&mut right, WindowGlyph::Close) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if window_button(&mut right, WindowGlyph::Maximize(self.maximized)) {
                    self.toggle_maximize(ctx);
                }
                if window_button(&mut right, WindowGlyph::Minimize) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                // A hairline closes the bar off from the page below it.
                ui.painter().hline(
                    bar.x_range(),
                    bar.bottom() - 0.5,
                    Stroke::new(1.0_f32, brand::hairline()),
                );
            });
    }
}

enum WindowGlyph {
    Minimize,
    Maximize(bool),
    Close,
}

impl WindowGlyph {
    /// The Windows control width; the bar is 48 tall, so these fill it.
    const WIDTH: f32 = 46.0;
}

fn window_button(ui: &mut egui::Ui, glyph: WindowGlyph) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(WindowGlyph::WIDTH, brand::TITLE_H), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.1);
    let closing = matches!(glyph, WindowGlyph::Close);
    let fill = if closing {
        brand::mix(Color32::TRANSPARENT, brand::ROSE, hover)
    } else {
        brand::mix(Color32::TRANSPARENT, brand::SURFACE_RAISED, hover)
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, fill);

    let color = if closing && hover > 0.4 {
        Color32::WHITE
    } else {
        brand::mix(brand::MUTED, brand::INK, hover)
    };
    let stroke = Stroke::new(1.1_f32, color);
    let c = rect.center();
    match glyph {
        WindowGlyph::Minimize => {
            ui.painter()
                .line_segment([c + Vec2::new(-5.0, 0.0), c + Vec2::new(5.0, 0.0)], stroke);
        }
        WindowGlyph::Maximize(maximized) => {
            if maximized {
                ui.painter().rect_stroke(
                    Rect::from_center_size(c + Vec2::new(2.0, -2.0), Vec2::splat(7.0)),
                    CornerRadius::same(1),
                    Stroke::new(1.0_f32, color),
                    StrokeKind::Inside,
                );
                ui.painter().rect_stroke(
                    Rect::from_center_size(c + Vec2::new(-2.0, 2.0), Vec2::splat(7.0)),
                    CornerRadius::same(1),
                    Stroke::new(1.0_f32, color),
                    StrokeKind::Inside,
                );
            } else {
                ui.painter().rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(9.0)),
                    CornerRadius::same(1),
                    Stroke::new(1.0_f32, color),
                    StrokeKind::Inside,
                );
            }
        }
        WindowGlyph::Close => {
            ui.painter().line_segment(
                [c + Vec2::new(-5.0, -5.0), c + Vec2::new(5.0, 5.0)],
                stroke,
            );
            ui.painter().line_segment(
                [c + Vec2::new(5.0, -5.0), c + Vec2::new(-5.0, 5.0)],
                stroke,
            );
        }
    }
    response.clicked()
}
