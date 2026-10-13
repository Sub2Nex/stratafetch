use super::{
    icons::{Icon, button},
    ui_state::*,
};

use eframe::egui;

use strata_core::{Job, Status};

pub fn show(root: &mut egui::Ui, state: &mut State, jobs: &[Job]) -> Vec<Action> {
    let mut actions = Vec::new();

    egui::Panel::top("menu_toolbar").show(root, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            for (menu, items) in [
                (
                    "File",
                    vec![
                        ("Add URL…  Ctrl+N", Action::Add),
                        ("Remove selected…  Delete", Action::Remove),
                        ("Remove completed…", Action::RemoveCompleted),
                    ],
                ),
                (
                    "Downloads",
                    vec![
                        ("Resume  Ctrl+R", Action::Resume),
                        ("Pause  Ctrl+P", Action::Pause),
                        ("Stop", Action::Stop),
                        ("Start main queue", Action::StartQueue),
                        ("Stop queued jobs", Action::StopQueue),
                    ],
                ),
                (
                    "Tools",
                    vec![
                        ("Scheduler…", Action::Scheduler),
                        ("Transfer statistics…", Action::Statistics),
                        ("Settings…  Ctrl+,", Action::Settings),
                    ],
                ),
            ] {
                ui.menu_button(text(state.prefs.swedish, menu), |ui| {
                    for (label, action) in items {
                        if ui.button(label).clicked() {
                            actions.push(action);
                            ui.close();
                        }
                    }
                });
            }

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut state.prefs.sidebar, "Navigation tree");
                if ui.button("Choose / reorder columns…").clicked() {
                    actions.push(Action::Columns);
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text(text(state.prefs.swedish, "Search downloads"))
                        .desired_width(210.0),
                );
                state.search_focused = response.has_focus();
                if response.changed() {
                    state.dirty = true;
                }
            });
        });

        ui.separator();

        let selected = state.selected(jobs);
        let can_resume = selected.iter().any(|j| {
            matches!(
                j.status,
                Status::Paused | Status::Failed | Status::Cancelled
            )
        });
        let can_pause = selected
            .iter()
            .any(|j| matches!(j.status, Status::Downloading | Status::Queued));
        let can_remove = selected
            .iter()
            .any(|j| !matches!(j.status, Status::Downloading | Status::Queued));

        ui.horizontal_wrapped(|ui| {

            for (icon,
label,
enabled,
tip,
action) in [
                (Icon::Add,
"Add URL",
true,
"Add a download (Ctrl+N). Ctrl+V pastes a URL.",
Action::Add),

                (Icon::Resume,
"Resume",
can_resume,
"Resume selected paused, stopped or failed downloads (Ctrl+R).",
Action::Resume),

                (Icon::Pause,
"Pause",
can_pause,
"Pause selected active/queued downloads (Ctrl+P).",
Action::Pause),

                (Icon::Stop,
"Stop",
can_pause||can_resume,
"Stop selected downloads; verified partial data is kept.",
Action::Stop),

                (Icon::Pause,
"Stop all",
jobs.iter().any(|j|matches!(j.status,
Status::Downloading|Status::Queued)),
"Pause all active and queued downloads.",
Action::StopAll),

                (Icon::Remove,
"Remove",
can_remove,
"Remove selected inactive history and partial data. Completed files are kept (Delete).",
Action::Remove),

                (Icon::Remove,
"Clear finished",
jobs.iter().any(|j|j.status==Status::Completed),
"Remove completed history; keep downloaded files.",
Action::RemoveCompleted),

                (Icon::Clock,
"Scheduler",
true,
"Schedule a selected inactive download, or view scheduled jobs.",
Action::Scheduler),

                (Icon::Queue,
"Start queue",
jobs.iter().any(|j|j.status==Status::Paused),
"Queue all paused jobs; scheduled times and priorities are respected.",
Action::StartQueue),

                (Icon::Stop,
"Stop queue",
jobs.iter().any(|j|j.status==Status::Queued),
"Pause queued jobs; active transfers continue.",
Action::StopQueue),

                (Icon::Settings,
"Settings",
true,
"Engine and appearance settings (Ctrl+,).",
Action::Settings),

            ] {
if matches!(&action, Action::Resume | Action::Remove | Action::Scheduler | Action::Settings) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0,22.0),egui::Sense::hover());
    ui.painter().line_segment([rect.center_top(),rect.center_bottom()],egui::Stroke::new(0.5,ui.visuals().widgets.noninteractive.bg_stroke.color));
}
if button(ui,
icon,
text(state.prefs.swedish,label),
enabled,
tip){
actions.push(action);
}
}

        }
);
    });

    actions
}
