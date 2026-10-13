use eframe::egui;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use strata_app::{Command, Reply, execute, ipc, shutdown, state_dir};

use strata_core::{Manager, Options, Settings, metadata::Metadata};

pub enum Request {
    Command(Command),

    Probe(String),

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
    Message(String),

    Metadata {
        url: String,

        result: Result<Metadata, String>,
    },
    Added {
        id: String,

        description: String,
    },
}

pub struct Worker {
    pub commands: tokio::sync::mpsc::Sender<Request>,

    pub events: mpsc::Receiver<Event>,

    pub snapshot: Arc<Mutex<Option<Reply>>>,

    thread: Option<std::thread::JoinHandle<()>>,
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

fn emit(events: &mpsc::SyncSender<Event>, ctx: &egui::Context, event: Event) {
    let _ = events.try_send(event);

    ctx.request_repaint();
}

impl Worker {
    pub fn new(ctx: egui::Context) -> Self {
        let (commands, mut requests) = tokio::sync::mpsc::channel(64);

        let (events, receiver) = mpsc::sync_channel(64);

        let latest = Arc::new(Mutex::new(None));

        let snapshot = latest.clone();

        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect("Tokio runtime");

            runtime.block_on(async move {
                let directory = state_dir();

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
                    let latest = latest.clone();

                    let ctx = ctx.clone();

                    let manager = manager.clone();

                    let dir = directory.clone();

                    tokio::spawn(async move {
                        let mut tick = tokio::time::interval(Duration::from_millis(250));

                        loop {
                            tick.tick().await;

                            let reply = call(manager.as_ref(), &dir, Command::List).await;

                            *latest.lock().unwrap() = Some(reply);

                            ctx.request_repaint();
                        }
                    })
                };

                while let Some(request) = requests.recv().await {
                    let reply = match request {
                        Request::Probe(url) => {
                            let events = events.clone();

                            let ctx = ctx.clone();

                            tokio::spawn(async move {
                                let result = strata_core::metadata::probe(&url)
                                    .await
                                    .map_err(|e| format!("{e:#}"));

                                emit(&events, &ctx, Event::Metadata { url, result });
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

                                if start {
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

                    ctx.request_repaint();
                }

                snapshots.abort();

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
        }
    }

    pub fn close(&mut self) {
        let (replacement, _) = tokio::sync::mpsc::channel(1);

        self.commands = replacement;

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
