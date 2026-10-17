use super::{ui_state::*, worker::Request};

use eframe::egui;

pub fn show(ctx: &egui::Context, state: &mut State) -> Vec<Request> {
    let mut requests = Vec::new();

    if !state.add.open {
        return requests;
    }

    let mut open = true;

    let mut close = false;

    egui::Window::new(if state.add.batch {
strata_app::i18n::tr("Add multiple URLs")}
else{
strata_app::i18n::tr("Add download")}
).id(egui::Id::new("add_download")).open(&mut open).collapsible(false).resizable(false).default_width(560.0).default_pos(ctx.content_rect().center()-egui::vec2(280.0,
210.0)).vscroll(true).max_height((ctx.content_rect().height()-40.0).max(160.0)).show(ctx,
|ui| {

        ui.horizontal(|ui| {
if ui.selectable_label(!state.add.batch,
strata_app::i18n::tr("Single URL")).clicked(){
state.add.batch=false;
}

if ui.selectable_label(state.add.batch,
strata_app::i18n::tr("Batch URLs")).clicked(){
state.add.batch=true;
}
}
);
ui.separator();

        ui.label(if state.add.batch {
strata_app::i18n::tr("HTTP / HTTPS URLs — one per line")}
else{
strata_app::i18n::tr("Download URL")}
);

        let response=if state.add.batch {
ui.add(egui::TextEdit::multiline(&mut state.add.url).desired_rows(5).desired_width(f32::INFINITY))}
else{
ui.add(egui::TextEdit::singleline(&mut state.add.url).desired_width(f32::INFINITY).hint_text("https://…"))}
;

        if state.add.focus_url { response.request_focus(); state.add.focus_url=false; }
        if response.changed(){
state.add.probing=false;
state.add.probe_id = state.add.probe_id.wrapping_add(1);
state.add.probe_url.clear();
state.add.metadata=None;
state.add.filename.clear();
state.add.error=None;
}

if !state.add.batch {

            let identity=strata_core::metadata::identity(&state.add.url).ok();

            ui.horizontal(|ui| {
ui.label(strata_app::localized_format!("Source: {}", "Källa: {}",
identity.as_ref().map(|m|m.host.as_str()).unwrap_or("—")));
ui.separator();
ui.label(strata_app::localized_format!("Size: {}", "Storlek: {}",
state.add.metadata.as_ref().and_then(|m|m.size).map(|n|bytes(n as f64)).unwrap_or_else(||strata_app::i18n::tr("Unknown").into())));
if ui.add_enabled(identity.is_some()&&!state.add.probing,
egui::Button::new(if state.add.probing {
strata_app::i18n::tr("Checking…")}
else{
strata_app::i18n::tr("Check metadata")}
)).clicked(){
state.add.probing=true;
state.add.probe_url=state.add.url.trim().to_owned();
state.add.probe_id = state.add.probe_id.wrapping_add(1);
requests.push(Request::Probe { url: state.add.probe_url.clone(), form_id: state.add.form_id, probe_id: state.add.probe_id });
}
}
);

            ui.label(strata_app::i18n::tr("Filename"));
ui.add(egui::TextEdit::singleline(&mut state.add.filename).desired_width(f32::INFINITY).hint_text(identity.as_ref().map(|m|m.filename.as_str()).unwrap_or(strata_app::i18n::tr("Detected from URL or response headers"))));

        }

        ui.label(strata_app::i18n::tr("Destination directory"));
ui.add(egui::TextEdit::singleline(&mut state.add.folder).desired_width(f32::INFINITY));
if ui.button(strata_app::i18n::tr("Choose folder…")).clicked() {
    requests.push(Request::ChooseFolder { target: FolderTarget::Add(state.add.form_id), initial: state.add.folder.clone() });
}

        ui.horizontal(|ui| {
ui.label(strata_app::i18n::tr("Category"));
egui::ComboBox::from_id_salt("add_category").selected_text(if state.add.category.is_empty(){
strata_app::i18n::tr("Detect from extension")}
else{
&state.add.category}
).show_ui(ui,
|ui| {
ui.selectable_value(&mut state.add.category,
String::new(),
strata_app::i18n::tr("Detect from extension"));
for category in ["Other",
"Archives",
"Documents",
"Audio",
"Applications",
"Video",
"Images"]{
ui.selectable_value(&mut state.add.category,
category.into(),strata_app::i18n::tr(category));
}
}
);
}
);

        ui.label(strata_app::i18n::tr("Description (optional)"));
ui.text_edit_singleline(&mut state.add.description);

        if let Some(error)=&state.add.error{
ui.colored_label(egui::Color32::from_rgb(206,
108,
99),
strata_app::i18n::message(error));
}

        ui.separator();
let valid=state.add.url.lines().any(|line|!line.trim().is_empty())&&!state.add.folder.trim().is_empty();

        ui.horizontal(|ui| {
for (label,
start) in [("Start download",
true),
("Download later",
false)]{
if ui.add_enabled(valid,
egui::Button::new(strata_app::i18n::tr(label))).clicked(){
let urls:Vec<_>=state.add.url.lines().map(str::trim).filter(|url|!url.is_empty()).collect();
if urls.len()>64 {
state.add.error=Some("Add at most 64 URLs in one batch.".into());
}
else if urls.iter().any(|url|strata_core::metadata::identity(url).is_err()){
state.add.error=Some("Every URL must be HTTP/HTTPS with no embedded credentials.".into());
}
else {
for url in urls {
requests.push(Request::Add {
url:url.into(),
folder:state.add.folder.clone().into(),
name:Some(if !state.add.batch && !state.add.filename.trim().is_empty() { state.add.filename.trim().to_owned() } else {strata_core::metadata::identity(url).expect("validated URL").filename}),
category:state.add.category.clone(),
description:state.add.description.clone(),
start}
);
}
close=true;
}
}
}

if ui.button(strata_app::i18n::tr("Cancel")).clicked(){
close=true;
}
}
);

        ui.small(strata_app::i18n::tr("Repeated links reuse an unfinished download in the same folder. Completed files are kept."));
        ui.small(strata_app::i18n::tr("Metadata checks read headers only; they do not create a download. Unknown sizes are supported."));

    }
);

    state.add.open = open && !close;

    requests
}
