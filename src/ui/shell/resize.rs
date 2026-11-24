//! Drag resizing for the frameless window.
//!
//! Windows only runs its own resize loop for a window with `WS_THICKFRAME`, and
//! that style is what puts the drawn surface inside a frame. The window keeps no
//! frame at all, so the edges are tracked here and the size is sent as viewport
//! commands. Everything is measured from the cursor in screen coordinates and the
//! geometry captured when the drag started, because egui's window-relative pointer
//! jumps whenever the window itself moves.

use super::prelude::*;

const EDGE: f32 = 7.0;
const MIN_WIDTH: f32 = 880.0;
const MIN_HEIGHT: f32 = 540.0;

/// Which edges of the window a drag is moving.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Direction {
    North,
    South,
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}

impl Direction {
    fn from_pointer(rect: Rect, pointer: Pos2) -> Option<Self> {
        let left = pointer.x <= rect.left() + EDGE;
        let right = pointer.x >= rect.right() - EDGE;
        let top = pointer.y <= rect.top() + EDGE;
        let bottom = pointer.y >= rect.bottom() - EDGE;
        match (left, right, top, bottom) {
            (true, _, true, _) => Some(Self::NorthWest),
            (_, true, true, _) => Some(Self::NorthEast),
            (true, _, _, true) => Some(Self::SouthWest),
            (_, true, _, true) => Some(Self::SouthEast),
            (true, _, _, _) => Some(Self::West),
            (_, true, _, _) => Some(Self::East),
            (_, _, true, _) => Some(Self::North),
            (_, _, _, true) => Some(Self::South),
            _ => None,
        }
    }

    fn cursor(self) -> egui::CursorIcon {
        match self {
            Self::West | Self::East => egui::CursorIcon::ResizeHorizontal,
            Self::North | Self::South => egui::CursorIcon::ResizeVertical,
            Self::NorthWest | Self::SouthEast => egui::CursorIcon::ResizeNwSe,
            Self::NorthEast | Self::SouthWest => egui::CursorIcon::ResizeNeSw,
        }
    }

    fn moves_west(self) -> bool {
        matches!(self, Self::West | Self::NorthWest | Self::SouthWest)
    }

    fn grows_east(self) -> bool {
        matches!(self, Self::East | Self::NorthEast | Self::SouthEast)
    }

    fn moves_north(self) -> bool {
        matches!(self, Self::North | Self::NorthWest | Self::NorthEast)
    }

    fn grows_south(self) -> bool {
        matches!(self, Self::South | Self::SouthWest | Self::SouthEast)
    }
}

/// An in-flight edge drag, anchored where the drag began.
#[derive(Clone, Copy)]
pub(crate) struct Resize {
    direction: Direction,
    cursor: (i32, i32),
    size: Vec2,
    origin: (i32, i32),
    applied_size: Vec2,
    applied_origin: (i32, i32),
}

impl App {
    pub(crate) fn resize_edges(&mut self, ctx: &egui::Context) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let Some(drag) = self.resizing else {
            let Some(local) = ctx.pointer_latest_pos() else {
                return;
            };
            let Some(direction) = Direction::from_pointer(ctx.screen_rect(), local) else {
                return;
            };
            ctx.set_cursor_icon(direction.cursor());
            if !ctx.input(|input| input.pointer.primary_pressed()) {
                return;
            }
            let (Some(cursor), Some(origin)) = (window.cursor(), window.origin()) else {
                return;
            };
            let size = ctx.screen_rect().size();
            self.maximized = false;
            self.resizing = Some(Resize {
                direction,
                cursor,
                size,
                origin,
                applied_size: size,
                applied_origin: origin,
            });
            return;
        };

        if !ctx.input(|input| input.pointer.primary_down()) {
            self.resizing = None;
            return;
        }

        let Some(cursor) = window.cursor() else {
            return;
        };
        let dx = (cursor.0 - drag.cursor.0) as f32;
        let dy = (cursor.1 - drag.cursor.1) as f32;

        let mut size = drag.size;
        let mut origin = drag.origin;
        if drag.direction.moves_west() {
            size.x -= dx;
            origin.0 += dx as i32;
        }
        if drag.direction.grows_east() {
            size.x += dx;
        }
        if drag.direction.moves_north() {
            size.y -= dy;
            origin.1 += dy as i32;
        }
        if drag.direction.grows_south() {
            size.y += dy;
        }

        // Keep the opposite edge still when a minimum is reached.
        if size.x < MIN_WIDTH {
            if drag.direction.moves_west() {
                origin.0 = drag.origin.0 + (drag.size.x - MIN_WIDTH) as i32;
            }
            size.x = MIN_WIDTH;
        }
        if size.y < MIN_HEIGHT {
            if drag.direction.moves_north() {
                origin.1 = drag.origin.1 + (drag.size.y - MIN_HEIGHT) as i32;
            }
            size.y = MIN_HEIGHT;
        }

        if size != drag.applied_size {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        if origin != drag.applied_origin {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(
                origin.0 as f32,
                origin.1 as f32,
            )));
        }

        self.resizing = Some(Resize {
            direction: drag.direction,
            cursor: drag.cursor,
            size: drag.size,
            origin: drag.origin,
            applied_size: size,
            applied_origin: origin,
        });
    }
}
