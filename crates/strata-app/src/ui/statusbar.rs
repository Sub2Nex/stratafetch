use super::ui_state::*;

use eframe::egui;

use strata_core::{Job, Status};

pub fn show(root: &mut egui::Ui, state: &State, jobs: &[Job]) {
    egui::Panel::bottom("statusbar").show(root, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!("{} downloads", jobs.len()));

            ui.separator();

            ui.label(format!("{} selected", state.selection.ids.len()));

            ui.separator();

            ui.label(format!(
                "{} active",
                jobs.iter()
                    .filter(|j| j.status == Status::Downloading)
                    .count()
            ));

            ui.separator();

            ui.label(format!(
                "{} queued",
                jobs.iter().filter(|j| j.status == Status::Queued).count()
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!(
                    "{}/s",
                    bytes(
                        jobs.iter()
                            .filter(|j| j.status == Status::Downloading)
                            .map(|j| j.speed)
                            .sum()
                    )
                ));
            });
        });
    });
}
