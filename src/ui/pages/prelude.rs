//! Imports shared by the page modules.

pub(super) use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense};

pub(super) use crate::actions::{self, Action};
pub(super) use crate::engine::{self, format_bytes, format_speed, Status};
pub(super) use crate::widgets::{self as w, card, chip, host_of, metric, page_heading, pill, Pill};
pub(super) use crate::{brand, App, Page};

pub(super) const REPO_URL: &str = "https://github.com/DesiAmbani/pookye-dm";
