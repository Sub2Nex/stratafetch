use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use strata_app::updates::{self, PreparedUpdate, Release};

enum Event {
    Checked(Result<Option<Release>, String>),
    Downloaded(Result<PreparedUpdate, String>),
}

pub struct Updater {
    pub exiting: bool,
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
    last_check: Option<Instant>,
    busy: bool,
    open: bool,
    message: String,
    release: Option<Release>,
    prepared: Option<PreparedUpdate>,
}

impl Updater {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            exiting: false,
            sender,
            receiver,
            last_check: None,
            busy: false,
            open: false,
            message: "Check for a newer version of StrataFetch.".into(),
            release: None,
            prepared: None,
        }
    }

    pub fn repair(&mut self) {
        self.check_mode(true, true);
    }

    pub fn check(&mut self, manual: bool) {
        self.check_mode(manual, false);
    }

    fn check_mode(&mut self, manual: bool, repair: bool) {
        self.open |= manual;
        if self.busy {
            return;
        }
        self.last_check = Some(Instant::now());
        self.busy = true;
        self.release = None;
        self.prepared = None;
        self.message = "Checking GitHub Releases…".into();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(anyhow::Error::from)
                .and_then(|rt| {
                    rt.block_on(async {
                        if repair {
                            updates::check_repair().await
                        } else {
                            updates::check().await
                        }
                    })
                });
            let _ = sender.send(Event::Checked(result.map_err(|e| format!("{e:#}"))));
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, automatic: &mut bool) {
        if *automatic
            && std::env::var_os("STRATAFETCH_SMOKE_OUTPUT").is_none()
            && self
                .last_check
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(24 * 60 * 60))
            && !self.busy
        {
            self.check(false);
        }
        while let Ok(event) = self.receiver.try_recv() {
            self.busy = false;
            match event {
                Event::Checked(Ok(Some(release))) => {
                    self.message = strata_app::localized_format!(
                        "StrataFetch {} is available.",
                        "StrataFetch {} är tillgänglig.",
                        release.tag_name
                    );
                    self.release = Some(release);
                    self.open = true;
                }
                Event::Checked(Ok(None)) => self.message = "You’re up to date.".into(),
                Event::Downloaded(Ok(prepared)) => {
                    self.prepared = Some(prepared);
                    self.message = "Full Windows setup verified and ready. Restart to install all components; active transfers will pause.".into();
                    self.open = true;
                }
                Event::Checked(Err(error)) | Event::Downloaded(Err(error)) => {
                    self.message = strata_app::localized_format!(
                        "Update failed: {error}. You can try again.",
                        "Uppdateringen misslyckades: {error}. Du kan försöka igen."
                    );
                }
            }
        }
        ctx.request_repaint_after(Duration::from_secs(1));
        if !self.open {
            return;
        }
        let mut open = true;
        egui::Window::new(strata_app::i18n::tr("StrataFetch updates")).open(&mut open).default_width(440.0).resizable(false).show(ctx, |ui| {
            ui.label(strata_app::localized_format!("Installed: {}", "Installerad version: {}", env!("CARGO_PKG_VERSION")));
            ui.label(&self.message);
            if self.busy { ui.spinner(); }
            ui.checkbox(automatic, strata_app::i18n::tr("Automatically check for updates"));
            ui.small(strata_app::i18n::tr("Checks GitHub on startup and every 24 hours while open. Downloads and restarts require your click."));
            if ui.add_enabled(!self.busy, egui::Button::new(strata_app::i18n::tr("Check for updates"))).clicked() { self.check(true); }
            if let Some(release) = self.release.clone() {
                if release.can_install() {
                    if self.prepared.is_none() && ui.add_enabled(!self.busy, egui::Button::new(strata_app::i18n::tr("Download update"))).clicked() {
                        self.busy = true;
                        self.message = "Downloading and verifying update…".into();
                        let sender = self.sender.clone();
                        std::thread::spawn(move || {
                            let result = tokio::runtime::Runtime::new().map_err(anyhow::Error::from)
                                .and_then(|rt| rt.block_on(updates::download(&release)));
                            let _ = sender.send(Event::Downloaded(result.map_err(|e| format!("{e:#}"))));
                        });
                    }
                } else {
                    ui.label(strata_app::i18n::tr("This platform or release requires installation from the Releases page."));
                }
            }
            if self.prepared.is_some() && ui.button(strata_app::i18n::tr("Restart and install")).clicked() {
                match self.prepared.take().unwrap().launch() {
                    Ok(()) => { self.exiting = true; ctx.send_viewport_cmd(egui::ViewportCommand::Close); },
                    Err(error) => self.message = strata_app::localized_format!("Cannot install update: {error:#}", "Det gick inte att installera uppdateringen: {error:#}"),
                }
            }
            ui.hyperlink_to(strata_app::i18n::tr("Release notes and downloads"), updates::RELEASES);
            ui.small(strata_app::i18n::tr("Windows updates install the full package, including the desktop, command line, browser host and bundled extensions. History and settings are preserved."));
        });
        self.open = open;
    }
}
