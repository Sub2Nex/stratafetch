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

    egui::Window::new(strata_app::i18n::tr("Settings")).id(egui::Id::new("settings")).open(&mut open).default_width((ctx.content_rect().width()-40.0).min(690.0)).default_height(460.0).default_pos(ctx.content_rect().center()-egui::vec2(320.0,
220.0)).vscroll(true).max_height((ctx.content_rect().height()-40.0).max(160.0)).resizable(true).show(ctx,
|ui| {

        ui.horizontal_top(|ui| {

            ui.vertical(|ui| {
ui.set_width(155.0); egui::ScrollArea::vertical().id_salt("settings_sections").max_height((ctx.content_rect().height()-150.0).clamp(120.0,350.0)).show(ui, |ui| {
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
index,strata_app::i18n::tr(*label));
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
ui.strong(strata_app::i18n::tr("General"));
ui.checkbox(&mut state.prefs.automatic_updates, strata_app::i18n::tr("Automatically check for updates"));
#[cfg(windows)]
ui.checkbox(&mut state.prefs.close_to_tray, strata_app::i18n::tr("Keep StrataFetch running in the system tray when the window closes"));
ui.small(strata_app::i18n::tr("Tools → Check for updates downloads and installs Windows desktop updates."));
ui.label(strata_app::i18n::tr("Default destination directory"));
ui.add(egui::TextEdit::singleline(&mut state.prefs.folder).desired_width(f32::INFINITY));
if ui.button(strata_app::i18n::tr("Choose folder…")).clicked() { state.choose_preferences_folder = true; }
ui.checkbox(&mut state.prefs.open_completed,
strata_app::i18n::tr("Double-click completed downloads to open the file"));
ui.small(strata_app::i18n::tr("Otherwise double-click opens transfer details. Files are never opened automatically by default."));
}
,

                1=>{
ui.strong(strata_app::i18n::tr("Downloads"));
ui.label(strata_app::i18n::tr("Maximum simultaneous downloads"));
ui.add(egui::Slider::new(&mut state.settings.concurrent_jobs,
1..=32));
ui.small(strata_app::i18n::tr("Reducing this limit allows active jobs to finish before admitting more."));
}
,

                2=>{
ui.strong(strata_app::i18n::tr("Connections"));
ui.label(strata_app::i18n::tr("Connections for new downloads"));
ui.add(egui::Slider::new(&mut state.settings.connections,
1..=16));
ui.small(strata_app::i18n::tr("Existing partial files keep their verified segment layout. Servers without reliable byte ranges use one connection."));
}
,

                3=>{
ui.strong(strata_app::i18n::tr("Bandwidth"));
ui.label(strata_app::i18n::tr("Aggregate payload bytes / second (0 = unlimited)"));
ui.add(egui::DragValue::new(&mut state.settings.global_bytes_per_second).range(0..=1_000_000_000u64).speed(1024.0));
ui.small(strata_app::i18n::tr("Shared across active downloads. Individual limits are in each transfer's Speed limiter tab."));
}
,

                4=>{
ui.strong(strata_app::i18n::tr("File types"));
ui.checkbox(&mut state.settings.organize_by_extension,
strata_app::i18n::tr("Organize new downloads into extension-based folders"));
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
ui.label(format!("{}: {examples}", strata_app::i18n::tr(label)));
}
ui.small(strata_app::i18n::tr("Classification uses the existing engine rules; this is not an editable extension mapping."));
}
,

                5=>{
ui.strong(strata_app::i18n::tr("Categories"));
ui.label(strata_app::i18n::tr("Categories are persisted labels. Choose a category in Add download or edit an inactive download in Scheduler."));
ui.small(strata_app::i18n::tr("Changing a label does not move existing files. Automatic folders use the detected file extension."));
}
,

                6=>{
ui.strong(strata_app::i18n::tr("Scheduler"));
ui.label(strata_app::i18n::tr("Timed downloads require a running desktop or service. After shutdown, unfinished work reopens paused."));
if ui.button(strata_app::i18n::tr("Open scheduler…")).clicked(){
state.scheduler_open=true;
}
}
,

                7=>{
ui.strong(strata_app::i18n::tr("Browser integration"));
if cfg!(windows) && ui.button(strata_app::i18n::tr("Repair browser components…")).clicked() { state.browser_repair = true; }
if let Some(warning) = strata_app::browser_integration::browser_components_warning() { ui.colored_label(egui::Color32::from_rgb(255, 190, 80), strata_app::i18n::message(&warning)); }
ui.label(strata_app::i18n::tr("Send browser downloads to StrataFetch and open the app automatically. Setup registers the native bridge and opens your browser and the local extension folder."));
for browser in [strata_app::browser_integration::Browser::Chrome, strata_app::browser_integration::Browser::Edge, strata_app::browser_integration::Browser::Firefox] {
    if ui.button(strata_app::localized_format!("Set up {}…", "Konfigurera {}…", browser.label())).clicked() { state.browser_setup = Some(browser); }
}
ui.separator();
ui.label(strata_app::i18n::tr("Chrome / Edge: enable Developer mode on the extensions page, choose Load unpacked and select the chromium folder opened by Setup."));
ui.label(strata_app::i18n::tr("Firefox: choose Load Temporary Add-on and select firefox/manifest.json. Unsigned local add-ons must be loaded again after Firefox restarts."));
if let Ok(folder) = strata_app::browser_integration::extension_folder(strata_app::browser_integration::Browser::Chrome) {
    ui.label(strata_app::localized_format!("Local Chromium extension: {}", "Lokalt Chromium-tillägg: {}", folder.display()));
    if ui.button(strata_app::i18n::tr("Copy Chromium folder path")).clicked() { ctx.copy_text(folder.to_string_lossy().into()); }
}
ui.small(strata_app::i18n::tr("The extension popup can enable or disable automatic capture. A confirmation appears before handoff; a failed or declined handoff resumes the browser download. Private downloads, extension packages and webpage exports stay in the browser. Cookies are never copied; login-dependent downloads may need the browser."));
ui.hyperlink_to(strata_app::i18n::tr("Local setup instructions"), "https://github.com/Sub2Nex/stratafetch/blob/main/docs/BROWSER.md");
ui.hyperlink_to(strata_app::i18n::tr("Full setup package and browser component updates"), "https://github.com/Sub2Nex/stratafetch/releases/latest");
}
,

                8=>{
ui.strong(strata_app::i18n::tr("Appearance")); ui.checkbox(&mut state.prefs.swedish,"Svenska");
ui.checkbox(&mut state.prefs.dark,
strata_app::i18n::tr("Dark theme"));
ui.checkbox(&mut state.prefs.sidebar,
strata_app::i18n::tr("Show navigation tree"));
if ui.button(strata_app::i18n::tr("Configure table columns…")).clicked(){
state.columns_open=true;
}
ui.small(strata_app::i18n::tr("Native DPI scaling follows the system. Table widths and visible column order are retained."));
}
,

                9=>{
ui.strong(strata_app::i18n::tr("History recipes"));
ui.label(strata_app::i18n::tr("JSON path"));
ui.text_edit_singleline(&mut state.history_path);
ui.small(strata_app::i18n::tr("Exports may contain private URL query tokens. Imports create paused jobs with new IDs. Existing exports are not overwritten."));
ui.horizontal(|ui| {
if ui.button(strata_app::i18n::tr("Export")).clicked(){
commands.push(Command::ExportHistory{
path:state.history_path.clone().into()}
);
}

if ui.button(strata_app::i18n::tr("Import")).clicked(){
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
ui.label(strata_app::i18n::tr("Native Rust · MIT OR Apache-2.0"));
ui.label(strata_app::i18n::tr("No ads, telemetry or required account."));
ui.hyperlink_to(strata_app::i18n::tr("Public source"),
"https://github.com/Sub2Nex/stratafetch");
ui.label(strata_app::i18n::tr("Experimental desktop preview. See the documentation for platform support and browser setup."));
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
if ui.button(strata_app::i18n::tr("Apply engine settings")).clicked(){
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

    egui::Window::new(strata_app::i18n::tr("Download table columns")).open(&mut open).default_width(390.0).show(ctx,
|ui| {

        ui.label(strata_app::i18n::tr("Visible columns"));
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
ui.label(strata_app::i18n::tr("Column order"));
let current=state.prefs.columns.clone();
for (index,
column) in current.iter().enumerate(){
ui.horizontal(|ui| {
ui.label(strata_app::i18n::tr(column.title()));
if ui.add_enabled(index>0,
egui::Button::new("↑")).on_hover_text(strata_app::i18n::tr("Move left")).clicked(){
state.prefs.columns.swap(index,
index-1);
}

if ui.add_enabled(index+1<current.len(),
egui::Button::new("↓")).on_hover_text(strata_app::i18n::tr("Move right")).clicked(){
state.prefs.columns.swap(index,
index+1);
}
}
);
}

        ui.small(strata_app::i18n::tr("Drag boundaries in the table to resize. Widths are retained for each column layout."));
if ui.button(strata_app::i18n::tr("Restore default columns")).clicked(){
state.prefs.columns=Column::ALL[..8].to_vec();
state.dirty=true;
}

    }
);

    state.columns_open = open;
}
