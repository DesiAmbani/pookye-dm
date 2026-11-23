//! Pookye Download Manager - a fast, segmented download manager.
//!
//! Everything lives in a small module: the engine in `engine`, settings and the
//! local API in `store`/`api`, and the interface under `ui` (pages, widgets and
//! window shell). This file only declares them and starts the window.

mod actions;
mod api;
mod app;
mod engine;
mod store;
mod tray;
mod ui;

pub(crate) use app::{App, Filter, Page};
pub(crate) use ui::{brand, dialog, pages, widgets};

use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result {
    let settings = store::load();
    let bridge = Arc::new(api::Bridge::new());
    api::spawn(bridge.clone(), engine::VERSION.to_string());

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1160.0, 700.0])
        .with_min_inner_size([880.0, 540.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_title("Pookye Download Manager");
    if let Some(icon) = brand::window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        // A window parked off-screen must not be restored there next launch.
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "Pookye Download Manager",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, settings, bridge)))),
    )
}
