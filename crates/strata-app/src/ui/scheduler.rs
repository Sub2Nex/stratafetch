use super::ui_state::*;

use eframe::egui;

use strata_app::Command;

use strata_core::{Job, Status};

pub fn show(ctx: &egui::Context, state: &mut State, jobs: &[Job]) -> Vec<Command> {
    let mut commands = Vec::new();

    if !state.scheduler_open {
        return commands;
    }

    let mut open = true;

    egui::Window::new(strata_app::i18n::tr("Queue scheduler")).open(&mut open).default_width(560.0).vscroll(true).show(ctx,
|ui| {

        let target=state.selection.cursor.as_ref().and_then(|id|jobs.iter().find(|j|j.id==*id));

        if let Some(job)=target {

            ui.strong(name(job));
let inactive=matches!(job.status,
Status::Paused|Status::Failed|Status::Cancelled);

            if state.draft.as_ref().is_none_or(|draft|draft.id!=job.id){
state.draft=Some(TransferDraft{
id:job.id.clone(),
rate:job.bytes_per_second,
priority:job.priority,
category:job.category.clone(),
delay_minutes:0}
);
}

            let draft=state.draft.as_mut().unwrap();
ui.label(strata_app::i18n::tr("Category label"));
ui.text_edit_singleline(&mut draft.category);
ui.label(strata_app::i18n::tr("Start after (minutes; 0 = now)"));
ui.add(egui::DragValue::new(&mut draft.delay_minutes).range(0..=525600));
ui.label(strata_app::i18n::tr("Priority (higher runs first)"));
ui.add(egui::DragValue::new(&mut draft.priority).range(-100..=100));

            let now=time::OffsetDateTime::now_utc().unix_timestamp().max(0) as u64;
ui.label(strata_app::localized_format!("Planned start: {} UTC", "Planerad start: {} UTC",
added(now+draft.delay_minutes*60)));

            if ui.add_enabled(inactive,
egui::Button::new(strata_app::i18n::tr("Save and queue this download"))).clicked(){
commands.push(Command::Configure{
id:job.id.clone(),
priority:draft.priority,
bytes_per_second:draft.rate}
);
commands.push(Command::Schedule{
id:job.id.clone(),
category:draft.category.clone(),
scheduled_at:(draft.delay_minutes>0).then_some(now+draft.delay_minutes*60)}
);
}

if !inactive{
ui.small(strata_app::i18n::tr("Select an inactive download, or pause and wait before editing its schedule."));
}

        }
else{
ui.label(strata_app::i18n::tr("Select an inactive download to schedule it."));
}

        ui.separator();
ui.strong(strata_app::i18n::tr("Scheduled downloads"));
for job in jobs.iter().filter(|j|j.scheduled_at.is_some()&&j.status!=Status::Completed){
ui.label(format!("{} · {} UTC · {}",
name(job),
added(job.scheduled_at.unwrap()),
status(job.status)));
}

        ui.small(strata_app::i18n::tr("The desktop/service must stay running. Interrupted downloads reopen paused; no OS autostart or hosted workflow is used."));

    }
);

    state.scheduler_open = open;

    commands
}
