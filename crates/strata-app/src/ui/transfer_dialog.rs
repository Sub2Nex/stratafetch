use super::ui_state::*;

use eframe::egui;

use strata_app::Command;

use strata_core::{Job, Status};

pub enum Output {
    Action(Action),

    Command(Command),
}

pub fn show(
    ctx: &egui::Context,

    state: &mut State,

    jobs: &[Job],

    average: impl Fn(&str) -> Option<f64>,
) -> Vec<Output> {
    let mut output = Vec::new();

    let Some(job) = state
        .transfer
        .as_ref()
        .and_then(|id| jobs.iter().find(|j| j.id == *id))
    else {
        state.transfer = None;
        return output;
    };

    let mut open = true;

    egui::Window::new(strata_app::i18n::tr("Transfer details")).id(egui::Id::new(("transfer",
&job.id))).open(&mut open).default_width((ctx.content_rect().width()-40.0).min(600.0)).default_height(480.0).default_pos(ctx.content_rect().center()-egui::vec2(300.0,
220.0)).max_height((ctx.content_rect().height()-40.0).max(160.0)).show(ctx,
|ui| {

        ui.horizontal_wrapped(|ui| {
for (index,
label) in ["Download status",
"Connections",
"Speed limiter",
"On completion"].iter().enumerate(){
ui.selectable_value(&mut state.transfer_tab,
index,strata_app::i18n::tr(*label));
}
}
);
ui.separator();

        egui::ScrollArea::vertical().id_salt("transfer_content").max_height((ctx.content_rect().height()-190.0).clamp(80.0,330.0)).show(ui, |ui| {match state.transfer_tab {

            0=> {

                ui.add(egui::Label::new(egui::RichText::new(name(job)).strong()).wrap());
ui.add(egui::Label::new(&job.url).wrap());

                ui.add(egui::ProgressBar::new(percent(job)).desired_width(ui.available_width()).text(format!("{:.1}% — {}",
percent(job)*100.0,
status(job.status))));

                egui::Grid::new("transfer_summary").num_columns(2).min_row_height(18.0).spacing([24.0,
6.0]).show(ui,
|ui| {

                    for (label,
value) in [("File size",
job.size.map(|n|bytes(n as f64)).unwrap_or_else(||"Unknown".into())),
("Downloaded",
bytes(job.downloaded as f64)),
("Current speed",
format!("{}/s",
bytes(job.speed))),
("Session average",
average(&job.id).map(|speed|format!("{}/s",
bytes(speed))).unwrap_or_else(||"—".into())),
("Time left",
eta(job).map(duration).unwrap_or_else(||"—".into())),
("Connections",
job.connections.to_string()),
("Resume",
if job.resumable {
"Validated byte ranges"}
else if job.downloaded>0 {
"Restart required if interrupted"}
else{
"Determined when download begins"}
.into())] {
ui.label(strata_app::i18n::tr(label));
ui.label(strata_app::i18n::tr(&value));
ui.end_row();
}

                }
);
ui.label(strata_app::localized_format!("Saved to: {}", "Sparad i: {}",
job.destination.to_string_lossy()));

                if let Some(error)=&job.error {
ui.colored_label(egui::Color32::from_rgb(206,
108,
99),
strata_app::i18n::message(error));
}

if let Some(description)=state.prefs.descriptions.get(&job.id).filter(|text|!text.is_empty()){
ui.label(description);
}

            }
,

            1=> {
ui.label(if job.resumable {strata_app::i18n::tr("Verified range segments")} else {strata_app::i18n::tr("Single-stream transfer")});
for (index,
segment) in job.segments.iter().enumerate(){
let length=if job.resumable {segment.end.saturating_sub(segment.start).saturating_add(1)} else {job.size.unwrap_or(job.downloaded)};
ui.label(strata_app::localized_format!("Connection {} · {} – {}", "Anslutning {} · {} – {}",
index+1,
bytes(segment.start as f64),
bytes(segment.end as f64)));
ui.add(egui::ProgressBar::new((segment.done as f32/length.max(1) as f32).clamp(0.0,1.0)).desired_width(ui.available_width()).text(format!("{} / {}",
bytes(segment.done as f64),
bytes(length as f64))));
}

if job.segments.is_empty(){
ui.label(strata_app::i18n::tr("Connections appear when the transfer starts."));
}
}
,

            2=> {
if state.draft.as_ref().is_none_or(|draft|draft.id!=job.id){
state.draft=Some(TransferDraft {
id:job.id.clone(),
rate:job.bytes_per_second,
priority:job.priority,
category:job.category.clone(),
delay_minutes:0}
);
}
let draft=state.draft.as_mut().unwrap();
ui.label(strata_app::i18n::tr("Download bandwidth (bytes/second; 0 = unlimited)"));
ui.add(egui::DragValue::new(&mut draft.rate).range(0..=1_000_000_000u64).speed(1024.0));
ui.label(strata_app::i18n::tr("Queue priority (higher runs first)"));
ui.add(egui::DragValue::new(&mut draft.priority).range(-100..=100));
if ui.add_enabled(!matches!(job.status,
Status::Downloading|Status::Queued|Status::Completed),
egui::Button::new(strata_app::i18n::tr("Apply to this download"))).clicked(){
output.push(Output::Command(Command::Configure{
id:job.id.clone(),
priority:draft.priority,
bytes_per_second:draft.rate}
));
}
ui.small(strata_app::i18n::tr("Pause and wait for the transfer to stop before changing these settings."));
}
,

            _=> {
let action=state.prefs.completion.entry(job.id.clone()).or_default();
ui.radio_value(action,
Completion::None,
strata_app::i18n::tr("No action"));
ui.radio_value(action,
Completion::Message,
strata_app::i18n::tr("Show a message in StrataFetch"));
ui.radio_value(action,
Completion::OpenFolder,
strata_app::i18n::tr("Open the destination folder"));
ui.small(strata_app::i18n::tr("Applies only to a future completion while this desktop is running. Files are never automatically executed."));
}
,

        }

        egui::CollapsingHeader::new("Technical information").show(ui,
|ui| {
ui.label(strata_app::localized_format!("Download ID: {}", "Hämtnings-ID: {}",
job.id));
ui.label(strata_app::localized_format!("Validator: {}", "Validerare: {}",
job.validator.as_deref().unwrap_or(strata_app::i18n::tr("Unavailable"))));
if let Some(hash)=&job.sha256{
ui.label(format!("SHA-256: {hash}"));
if ui.button(strata_app::i18n::tr("Verify file checksum")).clicked(){
output.push(Output::Action(Action::Verify(job.id.clone())));
}
}

if job.status!=Status::Completed&&ui.add_enabled(!matches!(job.status,
Status::Downloading|Status::Queued),
egui::Button::new(strata_app::i18n::tr("Restart from beginning"))).clicked(){
output.push(Output::Action(Action::Restart(job.id.clone())));
}
}
);

        });
        ui.separator();
ui.horizontal_wrapped(|ui| {

            if ui.add_enabled(matches!(job.status,
Status::Paused|Status::Failed|Status::Cancelled),
egui::Button::new(strata_app::i18n::tr("Resume"))).clicked(){
output.push(Output::Command(Command::Start{
id:job.id.clone()}
));
}

if ui.add_enabled(matches!(job.status,
Status::Downloading|Status::Queued),
egui::Button::new(strata_app::i18n::tr("Pause"))).clicked(){
output.push(Output::Command(Command::Pause{
id:job.id.clone()}
));
}

if ui.add_enabled(job.status!=Status::Completed,
egui::Button::new(strata_app::i18n::tr("Stop"))).clicked(){
output.push(Output::Command(Command::Cancel{
id:job.id.clone()}
));
}

if ui.button(strata_app::i18n::tr("Open folder")).clicked(){
output.push(Output::Action(Action::OpenFolder(job.id.clone())));
}

if ui.add_enabled(job.status==Status::Completed,
egui::Button::new(strata_app::i18n::tr("Open file"))).clicked(){
output.push(Output::Action(Action::OpenFile(job.id.clone())));
}

        }
);


    }
);

    if !open {
        state.transfer = None;
    }

    output
}
