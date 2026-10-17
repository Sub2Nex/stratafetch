use super::ui_state::*;

use eframe::egui;

use strata_core::{Job, Status};

pub fn show(root: &mut egui::Ui, state: &State, jobs: &[Job]) {
    egui::Panel::bottom("statusbar").show(root, |ui| {
        ui.horizontal(|ui| {
            ui.label(strata_app::localized_format!(
                "{} downloads",
                "{} hämtningar",
                jobs.len()
            ));

            ui.separator();

            ui.label(strata_app::localized_format!(
                "{} selected",
                "{} markerade",
                state.selection.ids.len()
            ));

            ui.separator();

            ui.label(strata_app::localized_format!(
                "{} active",
                "{} pågående",
                jobs.iter()
                    .filter(|j| j.status == Status::Downloading)
                    .count()
            ));

            ui.separator();

            ui.label(strata_app::localized_format!(
                "{} queued",
                "{} i kö",
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
