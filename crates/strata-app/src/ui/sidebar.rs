use super::icons::{self, Icon};

use super::ui_state::*;

use eframe::egui;

use std::collections::BTreeSet;

use strata_core::{Job, Status};

fn item(ui: &mut egui::Ui, state: &mut State, jobs: &[Job], label: &str, filter: Filter) {
    let count = jobs.iter().filter(|j| filter.matches(j)).count();

    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
    let selected = state.filter == filter;
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            2.0,
            if selected {
                ui.visuals().selection.bg_fill
            } else {
                ui.visuals().widgets.hovered.bg_fill
            },
        );
    }
    let icon = match &filter {
        Filter::Status(Status::Downloading) => Icon::Resume,
        Filter::Status(Status::Paused) => Icon::Pause,
        Filter::Status(Status::Failed | Status::Cancelled) => Icon::Stop,
        Filter::Status(Status::Completed) => Icon::File,
        Filter::Scheduled => Icon::Clock,
        Filter::Queue => Icon::Queue,
        _ => Icon::Folder,
    };
    icons::paint(
        ui,
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 10.0, rect.center().y),
            egui::vec2(13.0, 13.0),
        ),
        icon,
        ui.visuals().weak_text_color(),
    );
    let count_text = ui.painter().layout_no_wrap(
        count.to_string(),
        egui::FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
    let count_left = rect.right() - count_text.size().x - 5.0;
    ui.painter().galley(
        egui::pos2(count_left, rect.center().y - count_text.size().y / 2.0),
        count_text,
        ui.visuals().weak_text_color(),
    );
    ui.put(
        egui::Rect::from_min_max(
            egui::pos2(rect.left() + 23.0, rect.top()),
            egui::pos2(count_left - 6.0, rect.bottom()),
        ),
        egui::Label::new(text(state.prefs.swedish, label))
            .truncate()
            .selectable(false),
    );
    if response
        .on_hover_text(strata_app::localized_format!(
            "{label}: {count} downloads",
            "{label}: {count} hämtningar"
        ))
        .clicked()
    {
        state.filter = filter;

        state.dirty = true;
    }
}

pub fn show(root: &mut egui::Ui, state: &mut State, jobs: &[Job]) {
    if !state.prefs.sidebar {
        return;
    }

    egui::Panel::left("category_tree")
        .default_size(180.0)
        .size_range(140.0..=280.0)
        .resizable(true)
        .show(root, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            ui.spacing_mut().button_padding.y = 3.0;

            ui.spacing_mut().interact_size.y = 22.0;

            egui::ScrollArea::vertical().show(ui, |ui| {
                item(ui, state, jobs, "All downloads", Filter::All);

                egui::CollapsingHeader::new(text(state.prefs.swedish, "Unfinished"))
                    .default_open(true)
                    .show(ui, |ui| {
                        item(ui, state, jobs, "All unfinished", Filter::Unfinished);

                        for (label, status) in [
                            ("Downloading", Status::Downloading),
                            ("Queued", Status::Queued),
                            ("Paused", Status::Paused),
                            ("Failed", Status::Failed),
                            ("Stopped", Status::Cancelled),
                        ] {
                            item(ui, state, jobs, label, Filter::Status(status));
                        }
                    });

                item(
                    ui,
                    state,
                    jobs,
                    "Finished",
                    Filter::Status(Status::Completed),
                );

                ui.separator();

                egui::CollapsingHeader::new(text(state.prefs.swedish, "Categories"))
                    .default_open(true)
                    .show(ui, |ui| {
                        let standard = [
                            ("General", "Other"),
                            ("Compressed", "Archives"),
                            ("Documents", "Documents"),
                            ("Music", "Audio"),
                            ("Programs", "Applications"),
                            ("Video", "Video"),
                            ("Images", "Images"),
                        ];

                        for (label, key) in standard {
                            item(ui, state, jobs, label, Filter::Category(key.into()));
                        }

                        let extras: BTreeSet<_> = jobs
                            .iter()
                            .map(|j| j.category.as_str())
                            .filter(|c| !standard.iter().any(|(_, key)| c == key) && !c.is_empty())
                            .collect();

                        for category in extras {
                            item(ui, state, jobs, category, Filter::Category(category.into()));
                        }
                    });

                ui.separator();

                egui::CollapsingHeader::new(text(state.prefs.swedish, "Queues"))
                    .default_open(true)
                    .show(ui, |ui| {
                        item(ui, state, jobs, "Main queue", Filter::Queue);

                        item(ui, state, jobs, "Scheduled", Filter::Scheduled);
                    });
            });
        });
}
