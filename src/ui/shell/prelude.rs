//! Imports shared by the shell modules.

pub(super) use std::time::Duration;

pub(super) use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};

pub(super) use crate::engine::{format_bytes, format_speed};
pub(super) use crate::{brand, widgets, App, Filter, Page};
