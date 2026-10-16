use super::ui_state::*;

use eframe::egui;

use strata_app::Command;

pub fn show(ctx: &egui::Context, state: &mut State) -> Vec<Command> {
    let mut commands = Vec::new();

    if !state.settings_open {
        return commands;
    }

    let mut open = true;

    let mut close = false;

    egui::Window::new("Settings").id(egui::Id::new("settings")).open(&mut open).default_width((ctx.content_rect().width()-40.0).min(690.0)).default_height(460.0).default_pos(ctx.content_rect().center()-egui::vec2(320.0,
220.0)).vscroll(true).max_height((ctx.content_rect().height()-40.0).max(160.0)).resizable(true).show(ctx,
|ui| {

        ui.horizontal_top(|ui| {

            ui.vertical(|ui| {
ui.set_width(135.0); egui::ScrollArea::vertical().id_salt("settings_sections").max_height((ctx.content_rect().height()-150.0).clamp(120.0,350.0)).show(ui, |ui| {
for (index,
label) in ["General",
"Downloads",
"Connections",
"Bandwidth",
"File types",
"Categories",
"Scheduler",
"Browser integration",
"Appearance",
"Advanced",
"About"].iter().enumerate(){
ui.selectable_value(&mut state.settings_page,
index,
*label);
}
});
}
);

            let (divider, _) = ui.allocate_exact_size(egui::vec2(1.0,(ctx.content_rect().height()-150.0).clamp(120.0,350.0)),egui::Sense::hover()); ui.painter().line_segment([divider.center_top(),divider.center_bottom()],egui::Stroke::new(0.5,ui.visuals().widgets.noninteractive.bg_stroke.color));

            ui.vertical(|ui| { egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-150.0).clamp(120.0,340.0)).show(ui,
|ui| {
ui.set_min_width(250.0);
match state.settings_page {

                0=>{
ui.strong("General");
ui.checkbox(&mut state.prefs.automatic_updates, "Automatically check for updates");
ui.small("Tools → Check for updates downloads and installs Windows desktop updates.");
ui.label("Default destination directory");
ui.add(egui::TextEdit::singleline(&mut state.prefs.folder).desired_width(f32::INFINITY));
if ui.button("Choose folder…").clicked() { state.choose_preferences_folder = true; }
ui.checkbox(&mut state.prefs.open_completed,
"Double-click completed downloads to open the file");
ui.small("Otherwise double-click opens transfer details. Files are never opened automatically by default.");
}
,

                1=>{
ui.strong("Downloads");
ui.label("Maximum simultaneous downloads");
ui.add(egui::Slider::new(&mut state.settings.concurrent_jobs,
1..=32));
ui.small("Reducing this limit allows active jobs to finish before admitting more.");
}
,

                2=>{
ui.strong("Connections");
ui.label("Connections for new downloads");
ui.add(egui::Slider::new(&mut state.settings.connections,
1..=16));
ui.small("Existing partial files keep their verified segment layout. Servers without reliable byte ranges use one connection.");
}
,

                3=>{
ui.strong("Bandwidth");
ui.label("Aggregate payload bytes / second (0 = unlimited)");
ui.add(egui::DragValue::new(&mut state.settings.global_bytes_per_second).range(0..=1_000_000_000u64).speed(1024.0));
ui.small("Shared across active downloads. Individual limits are in each transfer's Speed limiter tab.");
}
,

                4=>{
ui.strong("File types");
ui.checkbox(&mut state.settings.organize_by_extension,
"Organize new downloads into extension-based folders");
for (label,
examples) in [("Archives",
"ZIP, 7Z, RAR, TAR, GZ"),
("Documents",
"PDF, TXT, DOCX, XLSX, CSV, EPUB"),
("Music",
"MP3, FLAC, WAV, OGG, M4A"),
("Programs",
"EXE, MSI, MSIX"),
("Video",
"MP4, MKV, WEBM, MOV, AVI"),
("Images",
"PNG, JPG, GIF, WEBP, SVG")] {
ui.label(format!("{label}: {examples}"));
}
ui.small("Classification uses the existing engine rules; this is not an editable extension mapping.");
}
,

                5=>{
ui.strong("Categories");
ui.label("Categories are persisted labels. Choose a category in Add download or edit an inactive download in Scheduler.");
ui.small("Changing a label does not move existing files. Automatic folders use the detected file extension.");
}
,

                6=>{
ui.strong("Scheduler");
ui.label("Timed downloads require a running desktop or service. After shutdown, unfinished work reopens paused.");
if ui.button("Open scheduler…").clicked(){
state.scheduler_open=true;
}
}
,

                7=>{
ui.strong("Local browser integration");
ui.label("Send browser downloads to StrataFetch and open the app automatically. Setup registers the native bridge and opens your browser and the local extension folder.");
for browser in [strata_app::browser_integration::Browser::Chrome, strata_app::browser_integration::Browser::Edge, strata_app::browser_integration::Browser::Firefox] {
    if ui.button(format!("Set up {}…", browser.label())).clicked() { state.browser_setup = Some(browser); }
}
ui.separator();
ui.label("Chrome / Edge: enable Developer mode on the extensions page, choose Load unpacked and select the chromium folder opened by Setup.");
ui.label("Firefox: choose Load Temporary Add-on and select firefox/manifest.json. Unsigned local add-ons must be loaded again after Firefox restarts.");
if let Ok(folder) = strata_app::browser_integration::extension_folder(strata_app::browser_integration::Browser::Chrome) {
    ui.label(format!("Local Chromium extension: {}", folder.display()));
    if ui.button("Copy Chromium folder path").clicked() { ctx.copy_text(folder.to_string_lossy().into()); }
}
ui.small("The extension popup can enable or disable automatic capture. A confirmation appears before handoff; a failed or declined handoff resumes the browser download. Private downloads, extension packages and webpage exports stay in the browser. Cookies are never copied; login-dependent downloads may need the browser.");
ui.hyperlink_to("Local setup instructions", "https://github.com/Sub2Nex/stratafetch/blob/main/docs/BROWSER.md");
ui.hyperlink_to("Full setup package and browser component updates", "https://github.com/Sub2Nex/stratafetch/releases/latest");
}
,

                8=>{
ui.strong("Appearance"); ui.checkbox(&mut state.prefs.swedish,"Svenska — core labels (advanced text remains English)");
ui.checkbox(&mut state.prefs.dark,
"Dark theme");
ui.checkbox(&mut state.prefs.sidebar,
"Show navigation tree");
if ui.button("Configure table columns…").clicked(){
state.columns_open=true;
}
ui.small("Native DPI scaling follows the system. Table widths and visible column order are retained.");
}
,

                9=>{
ui.strong("History recipes");
ui.label("JSON path");
ui.text_edit_singleline(&mut state.history_path);
ui.small("Exports may contain private URL query tokens. Imports create paused jobs with new IDs. Existing exports are not overwritten.");
ui.horizontal(|ui| {
if ui.button("Export").clicked(){
commands.push(Command::ExportHistory{
path:state.history_path.clone().into()}
);
}

if ui.button("Import").clicked(){
commands.push(Command::ImportHistory{
path:state.history_path.clone().into(),
directory:state.prefs.folder.clone().into()}
);
}
}
);
}
,

                _=>{
ui.strong(format!("StrataFetch {}",
env!("CARGO_PKG_VERSION")));
ui.label("Native Rust · MIT OR Apache-2.0");
ui.label("No ads, telemetry or required account.");
ui.hyperlink_to("Public source",
"https://github.com/Sub2Nex/stratafetch");
ui.label("Experimental desktop preview. See the documentation for platform support and browser setup.");
}
,

            }
}
);

            });
        }
);
ui.separator();
ui.horizontal(|ui| {
if ui.button("Apply engine settings").clicked(){
commands.push(Command::Settings{
settings:state.settings.clone()}
);
}

if ui.button("OK").clicked(){
commands.push(Command::Settings{
settings:state.settings.clone()}
);
close=true;
}
}
);

    }
);

    state.settings_open = open && !close;

    commands
}

pub fn columns(ctx: &egui::Context, state: &mut State) {
    if !state.columns_open {
        return;
    }

    let mut open = true;

    egui::Window::new("Download table columns").open(&mut open).default_width(390.0).show(ctx,
|ui| {

        ui.label("Visible columns");
for column in Column::ALL {
let mut visible=state.prefs.columns.contains(&column);
if ui.add_enabled(column!=Column::Name,
egui::Checkbox::new(&mut visible,
column.title())).changed(){
if visible {
state.prefs.columns.push(column);
}
else{
state.prefs.columns.retain(|c|*c!=column);
}
state.dirty=true;
}
}

        ui.separator();
ui.label("Column order");
let current=state.prefs.columns.clone();
for (index,
column) in current.iter().enumerate(){
ui.horizontal(|ui| {
ui.label(column.title());
if ui.add_enabled(index>0,
egui::Button::new("↑")).on_hover_text("Move left").clicked(){
state.prefs.columns.swap(index,
index-1);
}

if ui.add_enabled(index+1<current.len(),
egui::Button::new("↓")).on_hover_text("Move right").clicked(){
state.prefs.columns.swap(index,
index+1);
}
}
);
}

        ui.small("Drag boundaries in the table to resize. Widths are retained for each column layout.");
if ui.button("Restore default columns").clicked(){
state.prefs.columns=Column::ALL[..8].to_vec();
state.dirty=true;
}

    }
);

    state.columns_open = open;
}
