use super::ui_state::FolderTarget;
use eframe::egui;

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

use strata_app::{Command, Reply, execute, ipc, shutdown, state_dir};

use strata_core::{Manager, Options, Settings, metadata::Metadata};

pub enum Request {
    BrowserSetup(strata_app::browser_integration::Browser),
    Command(Command),
    Sequence(Vec<Command>),

    Probe {
        url: String,
        form_id: u64,
        probe_id: u64,
    },
    ChooseFolder {
        target: FolderTarget,
        initial: String,
    },

    Add {
        url: String,

        folder: PathBuf,

        name: Option<String>,

        category: String,

        description: String,

        start: bool,
    },
}

pub enum Event {
    Focus(strata_app::browser_integration::Attention),
    Message(String),

    Metadata {
        form_id: u64,
        probe_id: u64,
        url: String,

        result: Result<Metadata, String>,
    },
    Added {
        id: String,

        description: String,
    },
    FolderSelected {
        target: FolderTarget,
        initial: String,
        path: PathBuf,
    },
}

pub struct Worker {
    pub commands: tokio::sync::mpsc::Sender<Request>,

    pub events: mpsc::Receiver<Event>,

    pub snapshot: Arc<Mutex<Option<Reply>>>,

    thread: Option<std::thread::JoinHandle<()>>,
    stop: tokio::sync::watch::Sender<bool>,
}

async fn call(manager: Option<&Manager>, directory: &std::path::Path, command: Command) -> Reply {
    if let Some(manager) = manager {
        let manager = manager.clone();

        match tokio::task::spawn_blocking(move || execute(&manager, command)).await {
            Ok(reply) => reply,

            Err(e) => Reply {
                version: 1,

                jobs: vec![],

                error: Some(format!("Engine task failed: {e}")),

                accepted_id: None,

                settings: Settings::default(),
            },
        }
    } else {
        match ipc::call(directory, command).await {
            Ok(reply) => reply,

            Err(e) => Reply {
                version: 1,

                jobs: vec![],

                error: Some(format!("Local service unavailable: {e:#}")),

                accepted_id: None,

                settings: Settings::default(),
            },
        }
    }
}

fn emit(events: &mpsc::Sender<Event>, ctx: &egui::Context, event: Event) {
    let _ = events.send(event);

    ctx.request_repaint();
}

impl Worker {
    pub fn new(ctx: egui::Context) -> Self {
        Self::in_directory(ctx, state_dir())
    }

    fn in_directory(ctx: egui::Context, directory: PathBuf) -> Self {
        let (commands, mut requests) = tokio::sync::mpsc::channel(64);
        let (stop, mut stopping) = tokio::sync::watch::channel(false);

        let (events, receiver) = mpsc::channel();

        let latest = Arc::new(Mutex::new(None));

        let snapshot = latest.clone();

        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("Tokio runtime");

            runtime.block_on(async move {
                let manager = match Manager::open(&directory, Options::default()) {
                    Ok(manager) => Some(manager),

                    Err(error) => {
                        let mut connected = false;

                        for _ in 0..10 {
                            if ipc::call(&directory, Command::List).await.is_ok() {
                                connected = true;

                                break;
                            }

                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        if !connected {
                            emit(&events, &ctx, Event::Message(format!("{error:#}")));

                            return;
                        }

                        None
                    }
                };

                let service = manager.clone().map(|manager| {
                    let dir = directory.clone();

                    let events = events.clone();

                    let ctx = ctx.clone();

                    tokio::spawn(async move {
                        if let Err(error) = ipc::serve(&dir, manager).await {
                            emit(
                                &events,
                                &ctx,
                                Event::Message(format!("Local service failed: {error:#}")),
                            );
                        }
                    })
                });

                let snapshots = {
                    let events = events.clone();
                    let latest = latest.clone();

                    let ctx = ctx.clone();

                    let manager = manager.clone();

                    let dir = directory.clone();

                    let generation = Arc::new(AtomicU64::new(0));
                    let polling_generation = generation.clone();

                    let task = tokio::spawn(async move {
                        let mut tick = tokio::time::interval(Duration::from_millis(250));

                        loop {
                            tick.tick().await;
                            if let Ok(Some(attention)) = strata_app::browser_integration::take_attention(&dir) {
                                emit(&events, &ctx, Event::Focus(attention));
                            }

                            let before = polling_generation.load(Ordering::SeqCst);
                            if !before.is_multiple_of(2) {
                                continue;
                            }

                            let reply = call(manager.as_ref(), &dir, Command::List).await;

                            let mut latest = latest.lock().unwrap();
                            if polling_generation.load(Ordering::SeqCst) == before {
                                *latest = Some(reply);
                            }

                            ctx.request_repaint();
                        }
                    });
                    (task, generation)
                };

                loop {
                    let request = tokio::select! {
                        biased;
                        _ = stopping.changed() => break,
                        request = requests.recv() => match request {
                            Some(request) => request,
                            None => break,
                        },
                    };
                    if *stopping.borrow() {
                        break;
                    }
                    if !matches!(
                        request,
                        Request::Probe { .. } | Request::ChooseFolder { .. } | Request::BrowserSetup(_)
                    ) {
                        snapshots.1.fetch_add(1, Ordering::SeqCst);
                    }
                    let reply = match request {
                        Request::BrowserSetup(browser) => {
                            let events = events.clone();
                            let ctx = ctx.clone();
                            tokio::task::spawn_blocking(move || {
                                let message = match strata_app::browser_integration::setup(browser) {
                                    Ok(()) => format!("{} is open and the native bridge is ready. Follow the local extension steps in Browser integration.", browser.label()),
                                    Err(error) => format!("Browser setup: {error:#}"),
                                };
                                emit(&events, &ctx, Event::Message(message));
                            });
                            continue;
                        }
                        Request::ChooseFolder { target, initial } => {
                            let events = events.clone();
                            let ctx = ctx.clone();
                            tokio::spawn(async move {
                                let mut dialog = rfd::AsyncFileDialog::new()
                                    .set_title("Choose download directory");
                                if std::path::Path::new(&initial).is_dir() {
                                    dialog = dialog.set_directory(&initial);
                                }
                                if let Some(folder) = dialog.pick_folder().await {
                                    emit(
                                        &events,
                                        &ctx,
                                        Event::FolderSelected {
                                            target,
                                            initial,
                                            path: folder.path().to_owned(),
                                        },
                                    );
                                }
                            });
                            continue;
                        }
                        Request::Probe {
                            url,
                            form_id,
                            probe_id,
                        } => {
                            let events = events.clone();

                            let ctx = ctx.clone();

                            tokio::spawn(async move {
                                let result = strata_core::metadata::probe(&url)
                                    .await
                                    .map_err(|e| format!("{e:#}"));

                                emit(
                                    &events,
                                    &ctx,
                                    Event::Metadata {
                                        url,
                                        form_id,
                                        probe_id,
                                        result,
                                    },
                                );
                            });

                            continue;
                        }

                        Request::Command(command) => {
                            let verifying = matches!(command, Command::Verify { .. });

                            let reply = call(manager.as_ref(), &directory, command).await;

                            if verifying && reply.error.is_none() {
                                emit(
                                    &events,
                                    &ctx,
                                    Event::Message(
                                        "SHA-256 verified. The file matches its recorded checksum."
                                            .into(),
                                    ),
                                );
                            }

                            reply
                        }
                        Request::Sequence(commands) => {
                            let mut reply = call(manager.as_ref(), &directory, Command::List).await;
                            for command in commands {
                                if reply.error.is_some() || *stopping.borrow() {
                                    break;
                                }
                                reply = call(manager.as_ref(), &directory, command).await;
                            }
                            reply
                        }

                        Request::Add {
                            url,

                            folder,

                            name,

                            category,

                            description,

                            start,
                        } => {
                            let mut reply = call(
                                manager.as_ref(),
                                &directory,
                                Command::AddPaused {
                                    url,

                                    directory: folder,

                                    name,

                                    category,
                                },
                            )
                            .await;

                            if reply.error.is_none()
                                && let Some(id) = reply.accepted_id.clone()
                            {
                                emit(
                                    &events,
                                    &ctx,
                                    Event::Added {
                                        id: id.clone(),

                                        description,
                                    },
                                );

                                if start && !*stopping.borrow() {
                                    reply =
                                        call(manager.as_ref(), &directory, Command::Start { id })
                                            .await;
                                }
                            }

                            reply
                        }
                    };

                    if let Some(error) = &reply.error {
                        emit(&events, &ctx, Event::Message(error.clone()));
                    }

                    *latest.lock().unwrap() = Some(reply);
                    snapshots.1.fetch_add(1, Ordering::SeqCst);

                    ctx.request_repaint();
                }

                snapshots.0.abort();

                if let Some(manager) = &manager {
                    shutdown(manager).await;
                }

                if let Some(service) = service {
                    service.abort();
                }
            });
        });

        Self {
            commands,

            events: receiver,

            snapshot,

            thread: Some(thread),
            stop,
        }
    }

    pub fn close(&mut self) {
        // Closing must discard pending user work, rather than drain the queue
        // and start new transfers while the application is exiting.
        let _ = self.stop.send(true);
        let (replacement, _) = tokio::sync::mpsc::channel(1);

        self.commands = replacement;

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepted_events_are_not_lost_when_a_batch_fills_the_queue() {
        let (events, receiver) = mpsc::channel();
        let ctx = egui::Context::default();
        for index in 0..65 {
            emit(&events, &ctx, Event::Message(format!("{index}")));
        }
        assert_eq!(
            receiver.try_iter().count(),
            65,
            "a metadata or folder result must not disappear after a batch"
        );
    }

    #[test]
    fn closing_discards_pending_adds_and_releases_the_profile() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("state");
        let mut worker = Worker::in_directory(egui::Context::default(), directory.clone());
        for index in 0..64 {
            worker
                .commands
                .try_send(Request::Add {
                    url: format!("https://example.org/file-{index}"),
                    folder: temp.path().join("out"),
                    name: None,
                    category: String::new(),
                    description: String::new(),
                    start: false,
                })
                .unwrap_or_else(|_| panic!("test queue unexpectedly full"));
        }
        worker.close();
        let manager = Manager::open(&directory, Options::default()).unwrap();
        assert!(
            manager.list().len() < 64,
            "closing drained the whole pending queue"
        );
        assert!(!manager.is_running());
    }
}
