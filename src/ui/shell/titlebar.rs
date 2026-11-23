//! The custom title bar, its drag region, window controls and glyphs.

use super::prelude::*;

impl App {
    pub(crate) fn title_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(54.0)
            .frame(
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(16, 10))
                    .fill(Color32::TRANSPARENT),
            )
            .show(ctx, |ui| {
                let bar = ui.max_rect();
                let left = Rect::from_min_max(bar.min, Pos2::new(bar.left() + 300.0, bar.max.y));
                let right = Rect::from_min_max(
                    Pos2::new(bar.right() - 118.0, bar.top()),
                    bar.max,
                );
                let drag = Rect::from_min_max(
                    Pos2::new(left.right(), bar.top()),
                    Pos2::new(right.left(), bar.max.y),
                );

                let drag_response = ui.interact(drag, egui::Id::new("titlebar-drag"), Sense::click_and_drag());
                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_response.double_clicked() {
                    let maximized = ctx.input(|input| input.viewport().maximized).unwrap_or(false);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                let mut left_ui = ui.new_child(egui::UiBuilder::new().max_rect(left));
                left_ui.horizontal_centered(|ui| {
                    let (mark, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                    if let Some(logo) = &self.logo {
                        ui.painter().image(
                            logo.id(),
                            mark,
                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("Pookye DM")
                                .font(brand::semi(14.5))
                                .color(brand::INK),
                        );
                        ui.label(
                            RichText::new("Download Manager")
                                .size(10.2)
                                .color(brand::DIM),
                        );
                    });
                });

                let mut right_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(right)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                right_ui.spacing_mut().item_spacing.x = 6.0;
                if window_button(&mut right_ui, WindowGlyph::Close) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                let maximized = ctx.input(|input| input.viewport().maximized).unwrap_or(false);
                if window_button(&mut right_ui, WindowGlyph::Maximize(maximized)) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                if window_button(&mut right_ui, WindowGlyph::Minimize) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
    }
}

enum WindowGlyph {
    Minimize,
    Maximize(bool),
    Close,
}

fn window_button(ui: &mut egui::Ui, glyph: WindowGlyph) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(32.0, 26.0), Sense::click());
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.12);
    let closing = matches!(glyph, WindowGlyph::Close);
    let fill = if closing {
        brand::mix(Color32::TRANSPARENT, brand::ROSE, hover)
    } else {
        brand::mix(Color32::TRANSPARENT, brand::sheen(30), hover)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(8), fill);
    let color = if closing && hover > 0.5 {
        Color32::WHITE
    } else {
        brand::mix(brand::MUTED, brand::INK, hover)
    };
    let stroke = Stroke::new(1.15_f32, color);
    let c = rect.center() - Vec2::new(0.5, 0.5);
    match glyph {
        WindowGlyph::Minimize => {
            painter.line_segment([c + Vec2::new(-5.0, 3.0), c + Vec2::new(5.0, 3.0)], stroke);
        }
        WindowGlyph::Maximize(maximized) => {
            if maximized {
                painter.rect_stroke(
                    Rect::from_center_size(c + Vec2::new(0.5, -0.5), Vec2::splat(8.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
                painter.rect_stroke(
                    Rect::from_center_size(c + Vec2::new(-0.5, 0.5), Vec2::splat(8.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
            } else {
                painter.rect_stroke(
                    Rect::from_center_size(c, Vec2::splat(9.0)),
                    CornerRadius::same(2),
                    Stroke::new(1.1_f32, color),
                    StrokeKind::Inside,
                );
            }
        }
        WindowGlyph::Close => {
            painter.line_segment(
                [c + Vec2::new(-4.5, -4.5), c + Vec2::new(4.5, 4.5)],
                stroke,
            );
            painter.line_segment(
                [c + Vec2::new(4.5, -4.5), c + Vec2::new(-4.5, 4.5)],
                stroke,
            );
        }
    }
    response.clicked()
}
