//! Shared drawing helpers: cards, chips, navigation and the download row.

use std::path::Path;

use eframe::egui::{self, Color32, RichText};
use crate::engine::{format_bytes, format_eta, format_percent, format_speed, Job, Status};

use crate::actions::Action;
use crate::brand;

pub(crate) fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::default()
        .fill(brand::CARD)
        .inner_margin(egui::Margin::same(13))
        .show(ui, add)
        .inner
}

pub(crate) fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(21.0).strong().color(brand::INK));
    ui.label(RichText::new(subtitle).size(11.5).color(brand::MUTED));
    ui.add_space(8.0);
}

pub(crate) fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    egui::Frame::default()
        .fill(brand::CARD)
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(9.5).color(brand::MUTED));
                ui.label(RichText::new(value).size(14.0).strong().color(brand::INK));
            });
        });
}

pub(crate) fn nav(ui: &mut egui::Ui, label: &str, active: bool, badge: Option<usize>) -> bool {
    let width = ui.available_width();
    let text = match badge {
        Some(count) if count > 0 => format!("{label}    {count}"),
        _ => label.to_string(),
    };
    let button = egui::Button::new(
        RichText::new(text)
            .size(13.0)
            .color(if active { Color32::WHITE } else { brand::INK }),
    )
    .fill(if active { brand::ACCENT } else { brand::BG })
    .stroke(egui::Stroke::NONE);
    ui.add_sized([width, 30.0], button).clicked()
}

pub(crate) fn chip(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let button = egui::Button::new(
        RichText::new(label)
            .size(10.5)
            .color(if active { Color32::WHITE } else { brand::MUTED }),
    )
    .fill(if active { brand::ACCENT } else { brand::CARD })
    .stroke(egui::Stroke::NONE);
    ui.add(button).clicked()
}

pub(crate) fn download_row(
    ui: &mut egui::Ui,
    job: &Job,
    selected: bool,
) -> Option<Action> {
    let mut action = None;
    let mut checked = selected;

    egui::Frame::default()
        .fill(if selected { brand::CARD } else { brand::PANEL })
        .inner_margin(egui::Margin::same(13))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut checked, "").changed() {
                    action = Some(Action::Toggle(job.id));
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(job.display_name())
                            .size(13.5)
                            .strong()
                            .color(brand::INK),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}  ·  {} streams",
                            host_of(&job.url),
                            job.category.label(),
                            job.connections
                        ))
                        .size(10.5)
                        .color(brand::MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(job.status.label())
                            .size(10.5)
                            .strong()
                            .color(status_colour(job.status)),
                    );
                    if job.status == Status::Downloading {
                        ui.label(
                            RichText::new(format_speed(job.speed))
                                .size(11.5)
                                .color(brand::ACCENT_BRIGHT),
                        );
                    }
                });
            });

            ui.add_space(2.0);
            let fraction = job.percent().unwrap_or(0.0);
            let label = match job.status {
                Status::Queued => "Waiting for a free slot".to_string(),
                Status::Paused => format!("Paused · {}", format_percent(job.percent())),
                _ => format_percent(job.percent()),
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_height(9.0)
                    .fill(if job.status == Status::Completed {
                        brand::INK
                    } else {
                        brand::ACCENT
                    })
                    .text(RichText::new(label).size(9.5).color(brand::INK)),
            );

            let segments = job.progress.snapshot_segments();
            if segments.len() > 1 {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let width = (ui.available_width()
                        - (segments.len() as f32 - 1.0) * 3.0)
                        / segments.len() as f32;
                    for (length, done, _start) in &segments {
                        let part = if *length == 0 {
                            0.0
                        } else {
                            (*done as f32 / *length as f32).clamp(0.0, 1.0)
                        };
                        ui.add_sized(
                            [width.max(3.0), 4.0],
                            egui::ProgressBar::new(part)
                                .desired_height(4.0)
                                .fill(brand::ACCENT_BRIGHT),
                        );
                    }
                });
            }

            let total = job.progress.total();
            let bytes = job.progress.bytes();
            let mut facts = Vec::new();
            if total > 0 {
                facts.push(format!(
                    "{} of {}",
                    format_bytes(bytes),
                    format_bytes(total)
                ));
            } else if bytes > 0 {
                facts.push(format!("{} downloaded", format_bytes(bytes)));
            }
            if job.status == Status::Downloading {
                if let Some(left) = job.seconds_left() {
                    let eta = format_eta(left);
                    if !eta.is_empty() {
                        facts.push(eta);
                    }
                }
            }
            if let Some(error) = &job.error {
                facts.push(error.clone());
            }
            if !facts.is_empty() {
                ui.label(
                    RichText::new(facts.join("  ·  "))
                        .size(10.5)
                        .color(if job.error.is_some() {
                            brand::ACCENT_BRIGHT
                        } else {
                            brand::MUTED
                        }),
                );
            }

            ui.horizontal(|ui| {
                match job.status {
                    Status::Downloading | Status::Queued => {
                        if ui.small_button("Pause").clicked() {
                            action = Some(Action::Pause(job.id));
                        }
                    }
                    Status::Paused => {
                        if ui.small_button("Resume").clicked() {
                            action = Some(Action::Resume(job.id));
                        }
                    }
                    _ => {}
                }
                if job.status.is_active() && ui.small_button("Stop").clicked() {
                    action = Some(Action::Cancel(job.id));
                }
                if job.status.is_finished() && ui.small_button("Show in folder").clicked() {
                    action = Some(Action::OpenFolder(job.id));
                }
                if job.status == Status::Completed && ui.small_button("Open file").clicked() {
                    action = Some(Action::OpenFile(job.id));
                }
                if matches!(job.status, Status::Failed | Status::Cancelled)
                    && ui.small_button("Try again").clicked()
                {
                    action = Some(Action::Retry(job.id));
                }
                if ui.small_button("Copy link").clicked() {
                    action = Some(Action::CopyLink(job.url.clone()));
                }
                if job.status.is_finished() && ui.small_button("Delete file").clicked() {
                    action = Some(Action::Delete(job.id));
                }
                if !job.status.is_active() && ui.small_button("Remove").clicked() {
                    action = Some(Action::Remove(job.id));
                }
            });
        });
    ui.add_space(6.0);
    action
}

pub(crate) fn status_colour(status: Status) -> Color32 {
    match status {
        Status::Downloading => brand::ACCENT_BRIGHT,
        Status::Queued | Status::Paused | Status::Cancelled => brand::MUTED,
        Status::Completed => brand::INK,
        Status::Failed => brand::ACCENT,
    }
}

pub(crate) fn host_of(url: &str) -> String {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .to_string()
}

pub(crate) fn show_in_folder(path: &Path) {
    if path.exists() {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Some(folder) = path.parent() {
                let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
            }
        }
    } else if let Some(folder) = path.parent() {
        let folder = folder.to_path_buf();
        if folder.exists() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer").arg(&folder).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&folder).spawn();
            }
            #[cfg(all(unix, not(target_os = "macos")))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
            }
        }
    }
}

pub(crate) fn open_file(path: &Path) {
    if !path.exists() {
        return;
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}
