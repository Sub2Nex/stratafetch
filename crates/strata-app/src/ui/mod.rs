mod download_dialog;

mod download_table;

mod icons;

mod scheduler;

mod settings;

mod sidebar;

mod statusbar;

mod theme;

mod toolbar;

mod transfer_dialog;

mod ui_state;

#[cfg(test)]
mod tests;
mod worker;

use eframe::egui;

use std::{
    collections::{HashMap, VecDeque},
    sync::mpsc,
    time::{Duration, Instant},
};

use strata_app::{BatchAction, Command};

use strata_core::{Job, Status};

use ui_state::*;

use worker::{Event, Request, Worker};

struct Session {
    initial: u64,

    elapsed: f64,

    last: Instant,

    last_status: Status,

    downloaded: u64,
}

pub struct App {
    state: State,

    jobs: Vec<Job>,

    worker: Worker,

    sessions: HashMap<String, Session>,

    history: VecDeque<f64>,

    last_sample: Instant,

    started: Instant,

    screenshot_requested: bool,
    smoke_prepared: bool,

    open_messages: mpsc::Receiver<String>,

    open_sender: mpsc::Sender<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let prefs = cc
            .storage
            .and_then(|s| eframe::get_value(s, "preferences"))
            .unwrap_or_default();

        let mut state = State::new(prefs);

        if let Ok(font) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "Segoe UI".into(),
                std::sync::Arc::new(egui::FontData::from_owned(font)),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Segoe UI".into());

            cc.egui_ctx.set_fonts(fonts);
        }

        if std::env::var_os("STRATAFETCH_SMOKE_OUTPUT").is_some() {
            state.prefs.dark =
                std::env::var("STRATAFETCH_SMOKE_THEME").unwrap_or_default() != "light";

            if let Ok(scale) = std::env::var("STRATAFETCH_SMOKE_SCALE")
                .unwrap_or_else(|_| "1".into())
                .parse::<f32>()
            {
                cc.egui_ctx.set_pixels_per_point(scale.clamp(1.0, 2.0));
            }
        }

        let (open_sender, open_messages) = mpsc::channel();

        Self {
            state,

            jobs: Vec::new(),

            worker: Worker::new(cc.egui_ctx.clone()),

            sessions: HashMap::new(),

            history: VecDeque::new(),

            last_sample: Instant::now(),

            started: Instant::now(),

            screenshot_requested: false,
            smoke_prepared: false,

            open_messages,

            open_sender,
        }
    }

    fn request(&mut self, request: Request) {
        if let Err(error) = self.worker.commands.try_send(request) {
            let message = format!("Engine unavailable or busy: {error}");
            if let Request::Probe {
                url,
                form_id,
                probe_id,
            } = error.into_inner()
            {
                self.state
                    .apply_metadata(form_id, probe_id, &url, Err(message.clone()));
            }
            self.state.message = Some(message);
        }
    }

    fn command(&mut self, command: Command) {
        self.request(Request::Command(command));
    }

    fn batch(&mut self, ids: Vec<String>, action: BatchAction) {
        for chunk in ids.chunks(256) {
            self.command(Command::Batch {
                ids: chunk.to_vec(),

                action,
            });
        }
    }

    fn open_path(&mut self, path: std::path::PathBuf) {
        let errors = self.open_sender.clone();

        std::thread::spawn(move || {
            if let Err(error) = open::that(path) {
                let _ = errors.send(format!("Cannot open location: {error}"));
            }
        });
    }

    fn action(&mut self, action: Action) {
        match action {
            Action::Add => self.state.show_add(),

            Action::Resume | Action::Pause | Action::Stop => {
                let (batch, eligible): (BatchAction, fn(Status) -> bool) = match action {
                    Action::Resume => (BatchAction::Resume, |s| {
                        matches!(s, Status::Paused | Status::Failed | Status::Cancelled)
                    }),

                    Action::Pause => (BatchAction::Pause, |s| {
                        matches!(s, Status::Downloading | Status::Queued)
                    }),

                    _ => (BatchAction::Cancel, |s| s != Status::Completed),
                };

                let ids = self
                    .state
                    .selected(&self.jobs)
                    .into_iter()
                    .filter(|j| eligible(j.status))
                    .map(|j| j.id.clone())
                    .collect();

                self.batch(ids, batch);
            }

            Action::StopAll => {
                let ids = self
                    .jobs
                    .iter()
                    .filter(|j| matches!(j.status, Status::Downloading | Status::Queued))
                    .map(|j| j.id.clone())
                    .collect();

                self.batch(ids, BatchAction::Pause);
            }

            Action::StopQueue => {
                let ids = self
                    .jobs
                    .iter()
                    .filter(|j| j.status == Status::Queued)
                    .map(|j| j.id.clone())
                    .collect();

                self.batch(ids, BatchAction::Pause);
            }

            Action::StartQueue => {
                let ids = self
                    .jobs
                    .iter()
                    .filter(|j| j.status == Status::Paused)
                    .map(|j| j.id.clone())
                    .collect();

                self.batch(ids, BatchAction::Resume);
            }

            Action::Remove => {
                self.state.remove = Some(
                    self.state
                        .selected(&self.jobs)
                        .into_iter()
                        .filter(|j| !matches!(j.status, Status::Downloading | Status::Queued))
                        .map(|j| j.id.clone())
                        .collect(),
                )
            }

            Action::RemoveCompleted => {
                self.state.remove = Some(
                    self.jobs
                        .iter()
                        .filter(|j| j.status == Status::Completed)
                        .map(|j| j.id.clone())
                        .collect(),
                )
            }

            Action::Details(id) => {
                self.state.transfer = Some(id);

                self.state.transfer_tab = 0;
            }

            Action::OpenFile(id) => {
                if let Some(job) = self
                    .jobs
                    .iter()
                    .find(|j| j.id == id && j.status == Status::Completed)
                {
                    self.open_path(job.destination.clone());
                }
            }

            Action::OpenFolder(id) => {
                if let Some(path) = self
                    .jobs
                    .iter()
                    .find(|j| j.id == id)
                    .and_then(|j| j.destination.parent())
                    .map(|p| p.to_path_buf())
                {
                    self.open_path(path);
                }
            }

            Action::Verify(id) => self.command(Command::Verify { id, sha256: None }),

            Action::Restart(id) => self.command(Command::Restart { id }),

            Action::Settings => self.state.show_settings(),

            Action::Scheduler => self.state.scheduler_open = true,

            Action::Statistics => self.state.statistics_open = true,

            Action::Columns => self.state.columns_open = true,
        }
    }

    fn update(&mut self) {
        while let Ok(message) = self.open_messages.try_recv() {
            self.state.message = Some(message);
        }

        while let Ok(event) = self.worker.events.try_recv() {
            match event {
                Event::FolderSelected {
                    target,
                    initial,
                    path,
                } => {
                    self.state.apply_folder(target, &initial, path);
                }
                Event::Message(message) => self.state.message = Some(message),

                Event::Added { id, description } => {
                    if !description.is_empty() {
                        self.state.prefs.descriptions.insert(id, description);
                    }
                }

                Event::Metadata {
                    url,
                    form_id,
                    probe_id,
                    result,
                } => {
                    self.state.apply_metadata(form_id, probe_id, &url, result);
                }
            }
        }

        let snapshot = self.worker.snapshot.lock().unwrap().take();

        if let Some(reply) = snapshot {
            if let Some(error) = reply.error {
                if self.state.message.is_none() {
                    self.state.message = Some(error);
                }

                return;
            }

            if !self.state.settings_open {
                self.state.settings = reply.settings;
            }

            let mut completions = Vec::new();

            for job in &reply.jobs {
                if let Some(session) = self.sessions.get_mut(&job.id) {
                    if session.last_status == Status::Downloading {
                        session.elapsed += session.last.elapsed().as_secs_f64();
                    }

                    if session.last_status != Status::Completed && job.status == Status::Completed {
                        completions.push((job.id.clone(), name(job)));
                    }

                    session.last = Instant::now();

                    session.last_status = job.status;

                    session.downloaded = job.downloaded;
                } else {
                    self.sessions.insert(
                        job.id.clone(),
                        Session {
                            initial: job.downloaded,

                            downloaded: job.downloaded,

                            elapsed: 0.0,

                            last: Instant::now(),

                            last_status: job.status,
                        },
                    );
                }
            }

            self.jobs = reply.jobs;

            self.state.selection.retain(&self.jobs);

            self.state.dirty = true;

            for (id, name) in completions {
                match self
                    .state
                    .prefs
                    .completion
                    .get(&id)
                    .copied()
                    .unwrap_or_default()
                {
                    Completion::None => (),

                    Completion::Message => {
                        self.state.message = Some(format!("Download complete: {name}"))
                    }

                    Completion::OpenFolder => self.action(Action::OpenFolder(id)),
                }
            }
        }

        if self.last_sample.elapsed() >= Duration::from_secs(1) {
            self.last_sample = Instant::now();

            self.history.push_back(
                self.jobs
                    .iter()
                    .filter(|j| j.status == Status::Downloading)
                    .map(|j| j.speed)
                    .sum(),
            );

            if self.history.len() > 90 {
                self.history.pop_front();
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.state.show_add();
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.state.show_settings();
        }

        if !self.state.add.open
            && self.state.table_keyboard(ctx)
            && let Some(text) = ctx.input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Paste(text) = event {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            })
            && text.trim().starts_with("http")
        {
            self.state.show_add();

            self.state.add.batch = text.lines().filter(|line| !line.trim().is_empty()).count() > 1;

            self.state.add.url = text;
        }

        if self.state.table_keyboard(ctx) {
            for (key, action) in [
                (egui::Key::R, Action::Resume),
                (egui::Key::P, Action::Pause),
            ] {
                if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                    self.action(action);
                }
            }

            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
                self.action(Action::Remove);
            }

            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    || i.consume_key(egui::Modifiers::NONE, egui::Key::F2)
            }) && let Some(id) = self.state.selection.cursor.clone()
            {
                self.action(Action::Details(id));
            }
        }
    }

    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = std::env::var_os("STRATAFETCH_SMOKE_OUTPUT") else {
            return;
        };

        if self.started.elapsed() > Duration::from_secs(1) && !self.smoke_prepared {
            self.smoke_prepared = true;

            if let Some(job) = self
                .jobs
                .iter()
                .find(|j| j.status == Status::Downloading)
                .or_else(|| self.state.visible.first().map(|index| &self.jobs[*index]))
            {
                self.state
                    .selection
                    .click(&job.id, &[job.id.as_str()], false, false);
                self.state.scroll_to = self
                    .state
                    .visible
                    .iter()
                    .position(|index| self.jobs[*index].id == job.id);

                if std::env::var("STRATAFETCH_SMOKE_VIEW").unwrap_or_default() == "transfer" {
                    self.state.transfer = Some(job.id.clone());
                }
            }

            match std::env::var("STRATAFETCH_SMOKE_VIEW")
                .unwrap_or_default()
                .as_str()
            {
                "settings" => self.state.show_settings(),

                "add" => self.state.show_add(),

                "scheduler" => self.state.scheduler_open = true,

                "statistics" => self.state.statistics_open = true,

                _ => (),
            }
        }
        if self.started.elapsed() > Duration::from_secs(3) && !self.screenshot_requested {
            self.screenshot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }

        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) {
            let ctx = ctx.clone();

            std::thread::spawn(move || {
                let pixels: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();

                if let Err(error) = image::save_buffer(
                    std::path::PathBuf::from(path),
                    &pixels,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Native screenshot failed: {error}");
                }

                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            });
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        self.worker.close();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "preferences", &self.state.prefs);
    }

    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.update();

        let ctx = root.ctx().clone();

        theme::apply(&ctx, self.state.prefs.dark);

        self.shortcuts(&ctx);

        self.screenshot(&ctx);

        for action in toolbar::show(root, &mut self.state, &self.jobs) {
            self.action(action);
        }

        statusbar::show(root, &self.state, &self.jobs);

        sidebar::show(root, &mut self.state, &self.jobs);

        let mut actions = Vec::new();

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style_of(if self.state.prefs.dark {
                    egui::Theme::Dark
                } else {
                    egui::Theme::Light
                }))
                .inner_margin(egui::Margin::same(4)),
            )
            .show(root, |ui| {
                actions = download_table::show(ui, &mut self.state, &self.jobs);
            });

        for action in actions {
            self.action(action);
        }

        for request in download_dialog::show(&ctx, &mut self.state) {
            self.request(request);
        }

        let sessions = &self.sessions;

        for output in transfer_dialog::show(&ctx, &mut self.state, &self.jobs, |id| {
            sessions
                .get(id)
                .filter(|s| s.elapsed > 0.2 && s.downloaded > s.initial)
                .map(|s| s.downloaded.saturating_sub(s.initial) as f64 / s.elapsed)
        }) {
            match output {
                transfer_dialog::Output::Action(action) => self.action(action),

                transfer_dialog::Output::Command(command) => self.command(command),
            }
        }

        for command in settings::show(&ctx, &mut self.state) {
            self.command(command);
        }
        if std::mem::take(&mut self.state.choose_preferences_folder) {
            self.request(Request::ChooseFolder {
                target: ui_state::FolderTarget::Preferences(self.state.settings_id),
                initial: self.state.prefs.folder.clone(),
            });
        }

        settings::columns(&ctx, &mut self.state);

        let scheduled = scheduler::show(&ctx, &mut self.state, &self.jobs);
        if !scheduled.is_empty()
            && self
                .worker
                .commands
                .try_send(worker::Request::Sequence(scheduled))
                .is_err()
        {
            self.state.message = Some("The engine is busy. Try saving the schedule again.".into());
        }

        if self.state.statistics_open {
            let mut open = true;

            egui::Window::new("Transfer statistics")
                .open(&mut open)
                .default_width(500.0)
                .show(&ctx, |ui| {
                    let speed: f64 = self
                        .jobs
                        .iter()
                        .filter(|j| j.status == Status::Downloading)
                        .map(|j| j.speed)
                        .sum();

                    ui.label(format!("Current aggregate: {}/s", bytes(speed)));

                    ui.label(format!(
                        "Completed files: {}",
                        self.jobs
                            .iter()
                            .filter(|j| j.status == Status::Completed)
                            .count()
                    ));

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 120.0),
                        egui::Sense::hover(),
                    );

                    let maximum = self.history.iter().copied().fold(1.0, f64::max);

                    let points: Vec<_> = self
                        .history
                        .iter()
                        .enumerate()
                        .map(|(i, speed)| {
                            egui::pos2(
                                rect.left() + rect.width() * i as f32 / 89.0,
                                rect.bottom() - rect.height() * (*speed / maximum) as f32,
                            )
                        })
                        .collect();

                    if points.len() > 1 {
                        ui.painter().add(egui::Shape::line(
                            points,
                            egui::Stroke::new(1.5, theme::ACCENT),
                        ));
                    }

                    ui.small(
                        "Last 90 measured one-second samples; no lifetime throughput estimate.",
                    );
                });

            self.state.statistics_open = open;
        }

        if let Some(ids) = self.state.remove.clone() {
            egui::Window::new("Remove download history?")
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label(format!(
                        "Remove {} inactive records and their partial data?",
                        ids.len()
                    ));

                    ui.label("Completed files are kept.");

                    ui.horizontal(|ui| {
                        if ui.button("Remove").clicked() {
                            self.batch(ids, BatchAction::Remove);

                            self.state.remove = None;
                        }

                        if ui.button("Cancel").clicked() {
                            self.state.remove = None;
                        }
                    });
                });
        }

        if let Some(message) = self.state.message.clone() {
            egui::Window::new("StrataFetch")
                .collapsible(false)
                .default_width(420.0)
                .show(&ctx, |ui| {
                    ui.add(egui::Label::new(message).wrap());

                    if ui.button("OK").clicked() {
                        self.state.message = None;
                    }
                });
        }

        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
