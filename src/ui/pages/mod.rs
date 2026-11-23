//! The six pages reachable from the sidebar, one module each.

mod about;
mod downloads;
mod networks;
mod prelude;
mod schedule;
mod settings;
mod speed;

use eframe::egui;

use crate::{App, Page};

pub(crate) fn draw(app: &mut App, ui: &mut egui::Ui) {
    match app.page {
        Page::Downloads => downloads::draw(app, ui),
        Page::Speed => speed::draw(app, ui),
        Page::Networks => networks::draw(app, ui),
        Page::Schedule => schedule::draw(app, ui),
        Page::Settings => settings::draw(app, ui),
        Page::About => about::draw(app, ui),
    }
}
