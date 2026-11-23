//! Imports shared by the widget modules.

pub(super) use std::path::Path;

pub(super) use eframe::egui::{
    self, Color32, CornerRadius, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

pub(super) use crate::actions::Action;
pub(super) use crate::brand;
pub(super) use crate::engine::{
    format_bytes, format_eta, format_percent, format_speed, Job, Status,
};
