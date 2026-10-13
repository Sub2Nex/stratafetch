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
"Add multiple URLs"}
else{
"Add download"}
).id(egui::Id::new("add_download")).open(&mut open).collapsible(false).resizable(false).default_width(560.0).default_pos(ctx.content_rect().center()-egui::vec2(280.0,
210.0)).vscroll(true).max_height((ctx.content_rect().height()-40.0).max(160.0)).show(ctx,
|ui| {

        ui.horizontal(|ui| {
if ui.selectable_label(!state.add.batch,
"Single URL").clicked(){
state.add.batch=false;
}

if ui.selectable_label(state.add.batch,
"Batch URLs").clicked(){
state.add.batch=true;
}
}
);
ui.separator();

        ui.label(if state.add.batch {
"HTTP / HTTPS URLs — one per line"}
else{
"Download URL"}
);

        let response=if state.add.batch {
ui.add(egui::TextEdit::multiline(&mut state.add.url).desired_rows(5).desired_width(f32::INFINITY))}
else{
ui.add(egui::TextEdit::singleline(&mut state.add.url).desired_width(f32::INFINITY).hint_text("https://…"))}
;

        if state.add.focus_url { response.request_focus(); state.add.focus_url=false; }
        if response.changed(){
state.add.probing=false;
state.add.probe_url.clear();
state.add.metadata=None;
state.add.filename.clear();
state.add.error=None;
}

if !state.add.batch {

            let identity=strata_core::metadata::identity(&state.add.url).ok();

            ui.horizontal(|ui| {
ui.label(format!("Source: {}",
identity.as_ref().map(|m|m.host.as_str()).unwrap_or("—")));
ui.separator();
ui.label(format!("Size: {}",
state.add.metadata.as_ref().and_then(|m|m.size).map(|n|bytes(n as f64)).unwrap_or_else(||"Unknown".into())));
if ui.add_enabled(identity.is_some()&&!state.add.probing,
egui::Button::new(if state.add.probing {
"Checking…"}
else{
"Check metadata"}
)).clicked(){
state.add.probing=true;
state.add.probe_url=state.add.url.trim().to_owned();
requests.push(Request::Probe(state.add.probe_url.clone()));
}
}
);

            ui.label("Filename");
ui.add(egui::TextEdit::singleline(&mut state.add.filename).desired_width(f32::INFINITY).hint_text(identity.as_ref().map(|m|m.filename.as_str()).unwrap_or("Detected from URL or response headers")));

        }

        ui.label("Destination directory");
ui.add(egui::TextEdit::singleline(&mut state.add.folder).desired_width(f32::INFINITY));

        ui.horizontal(|ui| {
ui.label("Category");
egui::ComboBox::from_id_salt("add_category").selected_text(if state.add.category.is_empty(){
"Detect from extension"}
else{
&state.add.category}
).show_ui(ui,
|ui| {
ui.selectable_value(&mut state.add.category,
String::new(),
"Detect from extension");
for category in ["Other",
"Archives",
"Documents",
"Audio",
"Applications",
"Video",
"Images"]{
ui.selectable_value(&mut state.add.category,
category.into(),
category);
}
}
);
}
);

        ui.label("Description (optional)");
ui.text_edit_singleline(&mut state.add.description);

        if let Some(error)=&state.add.error{
ui.colored_label(egui::Color32::from_rgb(206,
108,
99),
error);
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
egui::Button::new(label)).clicked(){
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

if ui.button("Cancel").clicked(){
close=true;
}
}
);

        ui.small("Metadata checks read headers only; they do not create a download. Unknown sizes are supported.");

    }
);

    state.add.open = open && !close;

    requests
}
