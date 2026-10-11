#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use eframe::egui::{self, Color32, RichText};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, path::PathBuf, sync::mpsc, time::Duration};
use strata_app::{Command, Reply, download_dir, execute, ipc, shutdown, state_dir};
use strata_core::{Job, Manager, Options, Status};

#[derive(Serialize, Deserialize)]
struct Preferences {
    folder: String,
    dark: bool,
    swedish: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            folder: download_dir().to_string_lossy().into_owned(),
            dark: true,
            swedish: false,
        }
    }
}
struct App {
    commands: tokio::sync::mpsc::Sender<Command>,
    snapshots: mpsc::Receiver<Reply>,
    jobs: Vec<Job>,
    selected: Option<String>,
    filter: Option<Status>,
    search: String,
    urls: String,
    add_open: bool,
    settings_open: bool,
    error: Option<String>,
    delete: Option<String>,
    prefs: Preferences,
    history: VecDeque<f64>,
    last_sample: f64,
    smoke_started: std::time::Instant,
    smoke_requested: bool,
    worker: Option<std::thread::JoinHandle<()>>,
    messages: mpsc::Receiver<String>,
    message_tx: mpsc::SyncSender<String>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let prefs = cc
            .storage
            .and_then(|s| eframe::get_value(s, "preferences"))
            .unwrap_or_default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let (snap_tx, snap_rx) = mpsc::sync_channel(4);
        let (message_tx, messages) = mpsc::sync_channel(64);
        let worker_messages = message_tx.clone();
        let ctx = cc.egui_ctx.clone();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("Tokio runtime");
            runtime.block_on(async move {
                let directory=state_dir();
                let manager=match Manager::open(&directory,Options::default()) {
                    Ok(manager)=>Some(manager),
                    Err(error)=>match ipc::call(&directory,Command::List).await {
                        Ok(reply)=>{let _=snap_tx.try_send(reply);None},
                        Err(_)=>{let _=worker_messages.try_send(format!("{error:#}"));ctx.request_repaint();return;}
                    }
                };
                let service=manager.clone().map(|manager|{let directory=directory.clone();let errors=worker_messages.clone();tokio::spawn(async move {if let Err(error)=ipc::serve(&directory,manager).await {let _=errors.try_send(format!("Local control service failed: {error:#}"));}})});
                let mut tick=tokio::time::interval(Duration::from_millis(250));
                loop {
                    let command=tokio::select! {command=rx.recv()=>match command {Some(command)=>command,None=>break},_=tick.tick()=>Command::List};
                    let verifying=matches!(command,Command::Verify{..});
                    let reply=if let Some(manager)=&manager {execute(manager,command)} else {match ipc::call(&directory,command).await {Ok(reply)=>reply,Err(error)=>Reply {version:1,jobs:vec![],error:Some(format!("Local service unavailable: {error:#}")),accepted_id:None}}};
                    if verifying && reply.error.is_none(){let _=worker_messages.try_send("SHA-256 verified: the file matches its recorded checksum.".into());}
                    if let Some(error)=&reply.error {let _=worker_messages.try_send(error.clone());}let _=snap_tx.try_send(reply);ctx.request_repaint();
                }
                if let Some(manager)=manager {shutdown(&manager).await;}
                if let Some(service)=service {service.abort();}
            });
        });
        Self {
            commands: tx,
            snapshots: snap_rx,
            jobs: vec![],
            selected: None,
            filter: None,
            search: String::new(),
            urls: String::new(),
            add_open: false,
            settings_open: false,
            error: None,
            delete: None,
            prefs,
            history: VecDeque::new(),
            last_sample: 0.0,
            smoke_started: std::time::Instant::now(),
            smoke_requested: false,
            worker: Some(worker),
            messages,
            message_tx,
        }
    }
    fn send(&mut self, command: Command) {
        if let Err(error) = self.commands.try_send(command) {
            self.error = Some(format!("Engine unavailable or busy: {error}"));
        }
    }
    fn t<'a>(&self, en: &'a str, sv: &'a str) -> &'a str {
        if self.prefs.swedish { sv } else { en }
    }
    fn add(&mut self) {
        let urls: Vec<String> = self
            .urls
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        for url in urls {
            self.send(Command::Add {
                url,
                directory: PathBuf::from(&self.prefs.folder),
                name: None,
            });
        }
        self.urls.clear();
        self.add_open = false;
    }
    fn actions(&mut self, ui: &mut egui::Ui, job: &Job) {
        if job.status == Status::Completed
            && ui
                .button(self.t("Verify checksum", "Verifiera kontrollsumma"))
                .clicked()
        {
            self.send(Command::Verify {
                id: job.id.clone(),
                sha256: None,
            });
        }
        ui.horizontal_wrapped(|ui| {
            if matches!(
                job.status,
                Status::Paused | Status::Failed | Status::Cancelled
            ) && ui.button(self.t("Resume", "Fortsätt")).clicked()
            {
                self.send(Command::Start { id: job.id.clone() });
            }
            if matches!(job.status, Status::Downloading | Status::Queued)
                && ui.button(self.t("Pause", "Pausa")).clicked()
            {
                self.send(Command::Pause { id: job.id.clone() });
            }
            if job.status != Status::Completed && ui.button(self.t("Cancel", "Avbryt")).clicked() {
                self.send(Command::Cancel { id: job.id.clone() });
            }
            if matches!(
                job.status,
                Status::Failed | Status::Cancelled | Status::Paused
            ) && ui.button(self.t("Restart", "Börja om")).clicked()
            {
                self.send(Command::Restart { id: job.id.clone() });
            }
            if job.status == Status::Completed
                && ui.button(self.t("Open file", "Öppna fil")).clicked()
            {
                let path = job.destination.clone();
                let errors = self.message_tx.clone();
                std::thread::spawn(move || {
                    if let Err(error) = open::that(path) {
                        let _ = errors.try_send(format!("Cannot open file: {error}"));
                    }
                });
            }
            if ui.button(self.t("Open folder", "Öppna mapp")).clicked() {
                let path = job
                    .destination
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf();
                let errors = self.message_tx.clone();
                std::thread::spawn(move || {
                    if let Err(error) = open::that(path) {
                        let _ = errors.try_send(format!("Cannot open folder: {error}"));
                    }
                });
            }
            if ui.button(self.t("Remove", "Ta bort")).clicked() {
                self.delete = Some(job.id.clone());
            }
        });
    }
}
fn bytes(value: f64) -> String {
    let value = if value <= 0.0 { 0.0 } else { value };
    if value >= 1_073_741_824.0 {
        format!("{:.2} GiB", value / 1_073_741_824.0)
    } else if value >= 1_048_576.0 {
        format!("{:.1} MiB", value / 1_048_576.0)
    } else if value >= 1024.0 {
        format!("{:.1} KiB", value / 1024.0)
    } else {
        format!("{value:.0} B")
    }
}
fn name(job: &Job) -> String {
    job.destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
impl eframe::App for App {
    fn on_exit(&mut self) {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        self.commands = tx;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "preferences", &self.prefs);
    }
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.messages.try_recv() {
            self.error = Some(message);
        }
        while let Ok(reply) = self.snapshots.try_recv() {
            self.jobs = reply.jobs;
            if let Some(error) = reply.error {
                self.error = Some(error);
            }
        }
        let ctx = root.ctx().clone();
        if let Some(output) = std::env::var_os("STRATAFETCH_SMOKE_OUTPUT") {
            if self.smoke_started.elapsed() > Duration::from_secs(3) && !self.smoke_requested {
                self.smoke_requested = true;
                self.selected = self
                    .jobs
                    .iter()
                    .find(|j| j.status == Status::Downloading)
                    .or_else(|| self.jobs.first())
                    .map(|j| j.id.clone());
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let screenshot = ctx.input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let bytes: Vec<u8> = image
                        .pixels
                        .iter()
                        .flat_map(|pixel| pixel.to_array())
                        .collect();
                    if let Err(error) = image::save_buffer(
                        std::path::PathBuf::from(output),
                        &bytes,
                        image.size[0] as u32,
                        image.size[1] as u32,
                        image::ColorType::Rgba8,
                    ) {
                        eprintln!("Screenshot failed: {error}");
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                });
            }
        }
        ctx.set_visuals(if self.prefs.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ctx.style_mut_of(
            if self.prefs.dark {
                egui::Theme::Dark
            } else {
                egui::Theme::Light
            },
            |style| {
                style.spacing.item_spacing = egui::vec2(10.0, 10.0);
                style.spacing.button_padding = egui::vec2(12.0, 7.0);
                style.visuals.selection.bg_fill = Color32::from_rgb(39, 107, 128);
            },
        );
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N)) {
            self.add_open = true;
        }
        let pasted = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Paste(text) = e {
                    Some(text.clone())
                } else {
                    None
                }
            })
        });
        if !self.add_open
            && !self.settings_open
            && let Some(text) = pasted
            && text.trim().starts_with("http")
        {
            self.urls = text;
            self.add_open = true;
        }
        let speed: f64 = self
            .jobs
            .iter()
            .filter(|j| j.status == Status::Downloading)
            .map(|j| j.speed)
            .sum();
        let time = ctx.input(|i| i.time);
        if time - self.last_sample >= 1.0 {
            self.last_sample = time;
            self.history.push_back(speed);
            if self.history.len() > 90 {
                self.history.pop_front();
            }
        }
        egui::Panel::left("navigation")
            .default_size(195.0)
            .resizable(false)
            .show(root, |ui| {
                ui.add_space(20.0);
                ui.heading(RichText::new("STRATA").color(Color32::from_rgb(73, 204, 194)));
                ui.label(RichText::new("FETCH").size(26.0).strong());
                ui.label(self.t("Your downloads. Your control.", "Dina filer. Din kontroll."));
                ui.add_space(30.0);
                for (status, en, sv) in [
                    (None, "All downloads", "Alla nedladdningar"),
                    (Some(Status::Downloading), "Downloading", "Laddar ned"),
                    (Some(Status::Queued), "Queued", "I kö"),
                    (Some(Status::Paused), "Paused", "Pausade"),
                    (Some(Status::Completed), "Completed", "Slutförda"),
                    (Some(Status::Failed), "Failed", "Misslyckade"),
                    (Some(Status::Cancelled), "Cancelled", "Avbrutna"),
                ] {
                    let count = self
                        .jobs
                        .iter()
                        .filter(|j| status.is_none_or(|s| j.status == s))
                        .count();
                    if ui
                        .selectable_label(
                            self.filter == status,
                            format!("{}  {count}", self.t(en, sv)),
                        )
                        .clicked()
                    {
                        self.filter = status;
                    }
                }
                ui.add_space(25.0);
                if ui.button(self.t("Settings", "Inställningar")).clicked() {
                    self.settings_open = true;
                }
                ui.separator();
                ui.label(self.t("Local-first · Open source", "Lokalt · Öppen källkod"));
            });
        egui::Panel::bottom("status").show(root, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} / s", bytes(speed)));
                ui.separator();
                ui.label(format!(
                    "{} active",
                    self.jobs
                        .iter()
                        .filter(|j| j.status == Status::Downloading)
                        .count()
                ));
                ui.separator();
                ui.label(format!(
                    "{} completed",
                    self.jobs
                        .iter()
                        .filter(|j| j.status == Status::Completed)
                        .count()
                ));
            });
        });
        if let Some(job) = self
            .selected
            .as_ref()
            .and_then(|id| self.jobs.iter().find(|j| &j.id == id))
            .cloned()
        {
            egui::Panel::right("details")
                .default_size(285.0)
                .show(root, |ui| {
                    ui.add_space(15.0);
                    ui.heading(self.t("Transfer details", "Överföringsdetaljer"));
                    ui.label(RichText::new(name(&job)).strong());
                    ui.label(format!("{:?}", job.status));
                    ui.add(
                        egui::ProgressBar::new(
                            job.size
                                .filter(|n| *n > 0)
                                .map(|n| job.downloaded as f32 / n as f32)
                                .unwrap_or(0.0),
                        )
                        .show_percentage(),
                    );
                    ui.label(format!(
                        "{} / {}",
                        bytes(job.downloaded as f64),
                        job.size
                            .map(|n| bytes(n as f64))
                            .unwrap_or_else(|| "Unknown".into())
                    ));
                    ui.heading(format!("{} / s", bytes(job.speed)));
                    ui.label(format!("{} connections", job.connections));
                    if job.speed > 0.0
                        && let Some(total) = job.size
                    {
                        ui.label(format!(
                            "~{:.0} s remaining",
                            total.saturating_sub(job.downloaded) as f64 / job.speed
                        ));
                    }
                    ui.separator();
                    ui.label(self.t("Segments", "Segment"));
                    for segment in &job.segments {
                        let length = segment.end.saturating_sub(segment.start) + 1;
                        ui.add(
                            egui::ProgressBar::new((segment.done as f32 / length as f32).min(1.0))
                                .text(bytes(segment.done as f64)),
                        );
                    }
                    ui.separator();
                    ui.label(job.destination.to_string_lossy());
                    ui.label(format!("ID: {}", job.id));
                    if let Some(hash) = &job.sha256 {
                        ui.label("SHA-256");
                        ui.add(egui::Label::new(hash).wrap());
                    }
                    if let Some(error) = &job.error {
                        ui.colored_label(Color32::from_rgb(255, 135, 120), error);
                    }
                    ui.add_space(15.0);
                    self.actions(ui, &job);
                });
        }
        egui::CentralPanel::default().show(root, |ui| {
            ui.add_space(15.0);
            ui.horizontal(|ui| {
                ui.heading(self.t("Downloads", "Nedladdningar"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(self.t("+ Add download", "+ Lägg till")).clicked() {
                        self.add_open = true;
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search filenames…")
                            .desired_width(200.0),
                    );
                });
            });
            ui.add_space(10.0);
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 48.0), egui::Sense::hover());
            let max = self.history.iter().copied().fold(1.0, f64::max);
            let points: Vec<_> = self
                .history
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    egui::pos2(
                        rect.left() + rect.width() * i as f32 / 89.0,
                        rect.bottom() - rect.height() * (*value / max) as f32,
                    )
                })
                .collect();
            if points.len() > 1 {
                ui.painter().add(egui::Shape::line(
                    points,
                    egui::Stroke::new(2.0, Color32::from_rgb(73, 204, 194)),
                ));
            }
            ui.separator();
            let jobs: Vec<_> = self
                .jobs
                .iter()
                .filter(|j| {
                    self.filter.is_none_or(|s| s == j.status)
                        && name(j).to_lowercase().contains(&self.search.to_lowercase())
                })
                .cloned()
                .collect();
            if jobs.is_empty() {
                ui.add_space(65.0);
                ui.vertical_centered(|ui| {
                    ui.heading(
                        self.t("Ready for your next download", "Redo för nästa nedladdning"),
                    );
                    ui.label(self.t(
                        "Add a URL or press Ctrl+V to get started.",
                        "Lägg till en URL eller tryck Ctrl+V.",
                    ));
                });
            }
            egui::ScrollArea::vertical().show_rows(ui, 80.0, jobs.len(), |ui, range| {
                for index in range {
                    let job = &jobs[index];
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        self.selected.as_ref() == Some(&job.id),
                                        RichText::new(name(job)).strong(),
                                    )
                                    .clicked()
                                {
                                    self.selected = Some(job.id.clone());
                                }
                                ui.label(format!("{:?}", job.status));
                            });
                            ui.add(
                                egui::ProgressBar::new(
                                    job.size
                                        .filter(|n| *n > 0)
                                        .map(|n| job.downloaded as f32 / n as f32)
                                        .unwrap_or(0.0),
                                )
                                .desired_width(ui.available_width())
                                .text(format!(
                                    "{}  ·  {} / s",
                                    bytes(job.downloaded as f64),
                                    bytes(job.speed)
                                )),
                            );
                        })
                        .response
                        .context_menu(|ui| {
                            self.actions(ui, job);
                        });
                }
            });
        });
        if self.add_open {
            let mut open = true;
            egui::Window::new(self.t("Add downloads", "Lägg till nedladdningar"))
                .open(&mut open)
                .collapsible(false)
                .default_width(520.0)
                .show(&ctx, |ui| {
                    ui.label(self.t(
                        "Direct HTTP/HTTPS URLs, one per line",
                        "Direkta HTTP/HTTPS-adresser, en per rad",
                    ));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.urls)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label(self.t("Destination folder", "Målmapp"));
                    ui.text_edit_singleline(&mut self.prefs.folder);
                    if ui
                        .add_enabled(
                            !self.urls.trim().is_empty(),
                            egui::Button::new(self.t("Start downloading", "Starta nedladdning")),
                        )
                        .clicked()
                    {
                        self.add();
                    }
                });
            if !open {
                self.add_open = false;
            }
        }
        if self.settings_open {
            let mut open = true;
            egui::Window::new(self.t("Settings", "Inställningar"))
                .open(&mut open)
                .show(&ctx, |ui| {
                    ui.checkbox(&mut self.prefs.dark, "Dark theme / Mörkt tema");
                    ui.checkbox(&mut self.prefs.swedish, "Svenska");
                    ui.label(self.t("Default destination", "Standardmapp"));
                    ui.text_edit_singleline(&mut self.prefs.folder);
                    ui.label("4 connections per file · 3 concurrent jobs");
                    ui.label("Downloads are paused after an unexpected shutdown.");
                    ui.label("No telemetry, accounts, or background clipboard monitoring.");
                });
            self.settings_open = open;
        }
        if let Some(id) = self.delete.clone() {
            egui::Window::new(self.t("Remove download?", "Ta bort nedladdning?"))
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label(self.t(
                        "Removes history and partial data. Completed files are kept.",
                        "Tar bort historik och deldata. Slutförda filer behålls.",
                    ));
                    ui.horizontal(|ui| {
                        if ui.button(self.t("Remove", "Ta bort")).clicked() {
                            self.send(Command::Remove { id });
                            self.delete = None;
                        }
                        if ui.button(self.t("Keep", "Behåll")).clicked() {
                            self.delete = None;
                        }
                    });
                });
        }
        if let Some(error) = self.error.clone() {
            egui::Window::new(self.t("StrataFetch message", "StrataFetch-meddelande"))
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label(error);
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "StrataFetch",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
