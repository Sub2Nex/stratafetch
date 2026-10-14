//! Local-first download engine. See [`Manager`] and [`Options`].
pub mod history;
pub mod metadata;
pub mod network;
pub mod storage;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    Queued,
    Downloading,
    Paused,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start: u64,
    pub end: u64,
    pub done: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub url: String,
    pub destination: PathBuf,
    pub status: Status,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub speed: f64,
    pub connections: usize,
    pub validator: Option<String>,
    #[serde(default)]
    pub resumable: bool,
    pub segments: Vec<Segment>,
    pub error: Option<String>,
    pub sha256: Option<String>,
    pub created: u64,
    #[serde(default)]
    pub sequence: u64,
    pub completed: Option<u64>,
    pub priority: i32,
    pub bytes_per_second: u64,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub scheduled_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub connections: usize,
    pub concurrent_jobs: usize,
    pub retries: usize,
    pub redirect_limit: usize,
    pub proxy: Option<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            connections: 4,
            concurrent_jobs: 3,
            retries: 3,
            redirect_limit: 10,
            proxy: None,
        }
    }
}

/// Persisted engine-wide controls, shared by all local clients.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub connections: usize,
    pub concurrent_jobs: usize,
    /// Aggregate payload bytes per second; zero means unlimited.
    pub global_bytes_per_second: u64,
    #[serde(default)]
    pub organize_by_extension: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            connections: 4,
            concurrent_jobs: 3,
            global_bytes_per_second: 0,
            organize_by_extension: false,
        }
    }
}

pub(crate) struct TransferControl {
    cancel: watch::Sender<bool>,
    resume_requested: bool,
}

pub const MAX_URL_BYTES: usize = 16 * 1024;

pub(crate) struct Inner {
    pub jobs: Mutex<HashMap<String, Job>>,
    pub store: Mutex<storage::Store>,
    pub controls: Mutex<HashMap<String, TransferControl>>,
    pub settings: Mutex<Settings>,
    pub bandwidth: tokio::sync::Mutex<()>,
    pub dispatcher_running: AtomicBool,
    pub queue_changed: tokio::sync::Notify,
    pub options: Options,
    pub client: reqwest::Client,
    pub meter: Mutex<HashMap<String, (std::time::Instant, u64)>>,
}

#[derive(Clone)]
pub struct Manager(pub(crate) Arc<Inner>);

impl Manager {
    /// Opens persistent state, with an exclusive process lock. Interrupted work is paused.
    pub fn open(directory: &Path, options: Options) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(directory)?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(directory)?;
        let store = storage::Store::open(directory)?;
        let settings = store.settings()?.unwrap_or(Settings {
            connections: options.connections.clamp(1, 16),
            concurrent_jobs: options.concurrent_jobs.clamp(1, 32),
            global_bytes_per_second: 0,
            organize_by_extension: false,
        });
        anyhow::ensure!(
            (1..=16).contains(&settings.connections)
                && (1..=32).contains(&settings.concurrent_jobs),
            "Invalid persisted engine limits"
        );
        let mut jobs = store.load()?;
        for job in jobs.values_mut() {
            if job.category.is_empty() {
                job.category = category_for(
                    job.destination
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                )
                .to_owned();
                store.save(job)?;
            }
            if storage::recover_published(job)? {
                store.save(job)?;
                if job.status == Status::Completed {
                    let _ = std::fs::remove_file(staging_path(job));
                }
            }
            if matches!(job.status, Status::Downloading | Status::Queued) {
                job.status = Status::Paused;
                job.speed = 0.0;
                job.connections = 0;
                store.save(job)?;
            }
        }
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(options.redirect_limit))
            .connect_timeout(std::time::Duration::from_secs(20))
            .read_timeout(std::time::Duration::from_secs(60))
            .user_agent(concat!("StrataFetch/", env!("CARGO_PKG_VERSION")));
        if let Some(proxy) = &options.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        Ok(Self(Arc::new(Inner {
            jobs: Mutex::new(jobs),
            store: Mutex::new(store),
            controls: Mutex::new(HashMap::new()),
            settings: Mutex::new(settings),
            bandwidth: tokio::sync::Mutex::new(()),
            dispatcher_running: AtomicBool::new(false),
            queue_changed: tokio::sync::Notify::new(),
            client: builder.build()?,
            options,
            meter: Mutex::new(HashMap::new()),
        })))
    }

    pub fn list(&self) -> Vec<Job> {
        let mut jobs: Vec<_> = self.0.jobs.lock().unwrap().values().cloned().collect();
        jobs.sort_by_key(|j| (std::cmp::Reverse(j.priority), j.sequence, j.created));
        jobs
    }
    pub fn settings(&self) -> Settings {
        self.0.settings.lock().unwrap().clone()
    }
    /// A reduced concurrency limit lets active jobs finish; it prevents new admissions.
    pub fn set_settings(&self, settings: Settings) -> Result<()> {
        anyhow::ensure!(
            (1..=16).contains(&settings.connections),
            "Connections must be between 1 and 16"
        );
        anyhow::ensure!(
            (1..=32).contains(&settings.concurrent_jobs),
            "Simultaneous downloads must be between 1 and 32"
        );
        let mut current = self.0.settings.lock().unwrap();
        self.0.store.lock().unwrap().save_settings(&settings)?;
        *current = settings;
        self.0.queue_changed.notify_one();
        Ok(())
    }
    pub fn get(&self, id: &str) -> Result<Job> {
        self.0
            .jobs
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .context("Unknown download ID")
    }
    pub(crate) fn update(&self, job: Job) -> Result<()> {
        self.0.store.lock().unwrap().save(&job)?;
        self.0.jobs.lock().unwrap().insert(job.id.clone(), job);
        Ok(())
    }
    /// Adds a URL. Credentials embedded in URLs are rejected; names never escape the folder.
    pub fn add(&self, raw: &str, directory: &Path, name: Option<&str>) -> Result<Job> {
        // Serialize destination reservations and sequence assignment across IPC clients.
        let _controls = self.0.controls.lock().unwrap();
        anyhow::ensure!(
            raw.trim().len() <= MAX_URL_BYTES,
            "URL exceeds 16 KiB limit"
        );
        let url = url::Url::parse(raw.trim()).context("Enter a valid HTTP or HTTPS URL")?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            bail!("Only HTTP and HTTPS URLs are supported");
        }
        if !url.username().is_empty() || url.password().is_some() {
            bail!("Credentials in URLs are not supported");
        }
        std::fs::create_dir_all(directory)?;
        let mut directory = directory.canonicalize()?;
        let filename = match name {
            Some(name) => sanitize_filename(name),
            None => metadata::identity(raw)?.filename,
        };
        let category = category_for(&filename).to_owned();
        if self.settings().organize_by_extension {
            directory = directory.join(&category);
            std::fs::create_dir_all(&directory)?;
            let organized = directory.canonicalize()?;
            anyhow::ensure!(
                organized.parent() == directory.parent(),
                "Category folder must not redirect outside the destination"
            );
            directory = organized;
        }
        let jobs = self.0.jobs.lock().unwrap();
        let mut destination = directory.join(&filename);
        for index in 1..10000 {
            if !destination.exists() && !jobs.values().any(|j| j.destination == destination) {
                break;
            }
            destination = directory.join(format!("{index}-{filename}"));
        }
        if destination.exists() || jobs.values().any(|j| j.destination == destination) {
            bail!("Too many filename collisions");
        }
        let job = Job {
            id: Uuid::new_v4().to_string(),
            url: url.to_string(),
            destination,
            status: Status::Paused,
            size: None,
            downloaded: 0,
            speed: 0.0,
            connections: 0,
            validator: None,
            resumable: false,
            segments: vec![],
            error: None,
            sha256: None,
            created: now(),
            sequence: jobs
                .values()
                .map(|job| job.sequence)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .context("Download sequence exhausted")?,
            completed: None,
            priority: 0,
            bytes_per_second: 0,
            category,
            scheduled_at: None,
        };
        drop(jobs);
        self.update(job.clone())?;
        Ok(job)
    }
    /// Starts a cooperative transfer on the current Tokio runtime.
    pub fn start(&self, id: &str) -> Result<()> {
        let mut controls = self.0.controls.lock().unwrap();
        if let Some(control) = controls.get_mut(id) {
            if *control.cancel.borrow() {
                control.resume_requested = true;
            }
            return Ok(());
        }
        self.enqueue(id, &mut controls)
    }
    fn enqueue(&self, id: &str, controls: &mut HashMap<String, TransferControl>) -> Result<()> {
        let mut job = self.get(id)?;
        if job.status == Status::Completed {
            bail!("Download already completed");
        }
        job.status = Status::Queued;
        job.error = None;
        self.update(job)?;
        let (tx, _) = watch::channel(false);
        controls.insert(
            id.to_owned(),
            TransferControl {
                cancel: tx,
                resume_requested: false,
            },
        );
        // Serialize dispatcher lifetime with queue mutations to avoid lost admissions.
        if !self.0.dispatcher_running.swap(true, Ordering::SeqCst) {
            let manager = self.clone();
            tokio::spawn(async move { manager.dispatch().await });
        }
        self.0.queue_changed.notify_one();
        Ok(())
    }
    async fn dispatch(self) {
        loop {
            {
                let mut controls = self.0.controls.lock().unwrap();
                if controls.is_empty() {
                    self.0.dispatcher_running.store(false, Ordering::SeqCst);
                    return;
                }
                let limit = self.settings().concurrent_jobs;
                loop {
                    let candidate = {
                        let jobs = self.0.jobs.lock().unwrap();
                        let active = jobs
                            .values()
                            .filter(|j| j.status == Status::Downloading)
                            .count();
                        if active >= limit {
                            None
                        } else {
                            jobs.values()
                                .filter(|j| {
                                    j.status == Status::Queued
                                        && controls.contains_key(&j.id)
                                        && j.scheduled_at.is_none_or(|time| time <= now())
                                })
                                .min_by_key(|j| {
                                    (std::cmp::Reverse(j.priority), j.sequence, j.created)
                                })
                                .cloned()
                        }
                    };
                    let Some(mut job) = candidate else { break };
                    job.status = Status::Downloading;
                    let id = job.id.clone();
                    if let Err(error) = self.update(job) {
                        eprintln!("Cannot admit queued download: {error:#}");
                        controls.remove(&id);
                        continue;
                    }
                    let rx = controls.get(&id).unwrap().cancel.subscribe();
                    let manager = self.clone();
                    tokio::spawn(async move {
                        let outcome = network::run(&manager, &id, rx).await;
                        let mut controls = manager.0.controls.lock().unwrap();
                        if let Ok(mut job) = manager.get(&id) {
                            job.speed = 0.0;
                            job.connections = 0;
                            if let Err(error) = outcome
                                && job.status != Status::Paused
                                && job.status != Status::Cancelled
                            {
                                job.status = Status::Failed;
                                job.error = Some(format!("{error:#}"));
                            }
                            if let Err(error) = manager.update(job) {
                                eprintln!("Cannot persist final state: {error}");
                            }
                        }
                        let resume = controls
                            .remove(&id)
                            .is_some_and(|control| control.resume_requested);
                        manager.0.meter.lock().unwrap().remove(&id);
                        if resume && let Err(error) = manager.enqueue(&id, &mut controls) {
                            eprintln!("Cannot resume download: {error:#}");
                        }
                        manager.0.queue_changed.notify_one();
                    });
                }
            }
            // Scheduled jobs share one timer; completions and user actions wake immediately.
            tokio::select! {
                _ = self.0.queue_changed.notified() => (),
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => (),
            }
        }
    }
    pub fn pause(&self, id: &str) -> Result<()> {
        self.stop(id, Status::Paused)
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        self.stop(id, Status::Cancelled)
    }
    fn stop(&self, id: &str, status: Status) -> Result<()> {
        let mut controls = self.0.controls.lock().unwrap();
        let mut job = self.get(id)?;
        if job.status == Status::Completed {
            bail!("Completed downloads cannot be stopped");
        }
        let queued = job.status == Status::Queued;
        job.status = status;
        job.speed = 0.0;
        self.update(job)?;
        if let Some(control) = controls.get_mut(id) {
            control.resume_requested = false;
            let _ = control.cancel.send(true);
        }
        if queued {
            controls.remove(id);
        }
        self.0.queue_changed.notify_one();
        Ok(())
    }
    pub fn restart(&self, id: &str) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        if controls.contains_key(id) {
            bail!("Pause and wait before restarting");
        }
        let mut job = self.get(id)?;
        if job.status == Status::Completed {
            bail!("Add a new download to preserve the completed file");
        }
        let staging = staging_path(&job);
        if staging.exists() {
            std::fs::remove_file(staging)?;
        }
        job.downloaded = 0;
        job.segments.clear();
        job.validator = None;
        job.resumable = false;
        job.sha256 = None;
        job.completed = None;
        job.size = None;
        job.status = Status::Paused;
        self.update(job)?;
        drop(controls);
        self.start(id)
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        if controls.contains_key(id) {
            bail!("Pause and wait before removing");
        }
        let job = self.get(id)?;
        let staging = staging_path(&job);
        if staging.exists() {
            std::fs::remove_file(staging)?;
        }
        self.0.store.lock().unwrap().delete(id)?;
        self.0.jobs.lock().unwrap().remove(id);
        Ok(())
    }
    pub fn is_running(&self) -> bool {
        !self.0.controls.lock().unwrap().is_empty()
            || self.0.dispatcher_running.load(Ordering::SeqCst)
    }
    /// Re-reads a completed file and compares SHA-256 to a provided or recorded digest.
    pub fn verify(&self, id: &str, expected: Option<&str>) -> Result<()> {
        use sha2::{Digest, Sha256};
        use std::io::Read;
        let job = self.get(id)?;
        anyhow::ensure!(
            job.status == Status::Completed,
            "Only completed files can be verified"
        );
        let expected = expected
            .or(job.sha256.as_deref())
            .context("No recorded checksum")?;
        anyhow::ensure!(
            expected.len() == 64 && expected.bytes().all(|c| c.is_ascii_hexdigit()),
            "SHA-256 must contain exactly 64 hexadecimal characters"
        );
        let mut file = std::fs::File::open(&job.destination)?;
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 64 * 1024];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest.update(&buffer[..n]);
        }
        anyhow::ensure!(
            format!("{:x}", digest.finalize()) == expected.to_lowercase(),
            "SHA-256 mismatch; file does not match the expected checksum"
        );
        Ok(())
    }
    pub fn configure(&self, id: &str, priority: i32, bytes_per_second: u64) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        if controls.contains_key(id) {
            bail!("Pause before changing transfer settings");
        }
        let mut job = self.get(id)?;
        job.priority = priority;
        job.bytes_per_second = bytes_per_second;
        self.update(job)
    }
    /// Changes the rate without silently changing queue priority.
    pub fn limit(&self, id: &str, bytes_per_second: u64) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        anyhow::ensure!(
            !controls.contains_key(id),
            "Pause before changing transfer settings"
        );
        let mut job = self.get(id)?;
        job.bytes_per_second = bytes_per_second;
        self.update(job)
    }
    /// Changes classification and a UTC Unix start time without moving partial data.
    pub fn schedule(&self, id: &str, category: &str, scheduled_at: Option<u64>) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        anyhow::ensure!(
            !controls.contains_key(id),
            "Pause and wait before scheduling"
        );
        let mut job = self.get(id)?;
        anyhow::ensure!(
            job.status != Status::Completed,
            "Completed downloads cannot be scheduled"
        );
        job.category = sanitize_filename(category);
        job.scheduled_at = scheduled_at;
        self.update(job)
    }
}

pub fn category_for(filename: &str) -> &'static str {
    match std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "mkv" | "webm" | "mov" | "avi" => "Video",
        "mp3" | "flac" | "wav" | "ogg" | "m4a" => "Audio",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => "Images",
        "pdf" | "txt" | "doc" | "docx" | "xlsx" | "csv" | "epub" => "Documents",
        "zip" | "7z" | "rar" | "tar" | "gz" => "Archives",
        "exe" | "msi" | "msix" => "Applications",
        _ => "Other",
    }
}

pub fn sanitize_filename(raw: &str) -> String {
    let mut name: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    name = name.trim_matches([' ', '.']).to_owned();
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_uppercase();
    if name.is_empty()
        || [
            "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM1", "COM2", "COM3", "COM4",
            "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
            "LPT7", "LPT8", "LPT9", "COM¹", "COM²", "COM³", "LPT¹", "LPT²", "LPT³",
        ]
        .contains(&stem.as_str())
    {
        name = format!("download_{name}");
    }
    // Leave room for collision prefixes on byte-limited filesystems, preserving
    // short extensions and truncating only at a valid UTF-8 character boundary.
    if name.len() > 180 {
        let suffix = name
            .rsplit_once('.')
            .filter(|(_, ext)| !ext.is_empty() && ext.len() <= 20)
            .map(|(_, ext)| format!(".{ext}"))
            .unwrap_or_default();
        let mut end = 180 - suffix.len();
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name.truncate(end);
        name.push_str(&suffix);
    }
    name.trim_end_matches([' ', '.']).to_owned()
}
pub(crate) fn staging_path(job: &Job) -> PathBuf {
    job.destination
        .with_file_name(format!(".strata-{}.part", job.id))
}
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
