//! Local-first download engine. See [`Manager`] and [`Options`].
pub mod network;
pub mod storage;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Semaphore, watch};
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
    pub segments: Vec<Segment>,
    pub error: Option<String>,
    pub sha256: Option<String>,
    pub created: u64,
    pub completed: Option<u64>,
    pub priority: i32,
    pub bytes_per_second: u64,
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

pub(crate) struct Inner {
    pub jobs: Mutex<HashMap<String, Job>>,
    pub store: Mutex<storage::Store>,
    pub controls: Mutex<HashMap<String, watch::Sender<bool>>>,
    pub slots: Arc<Semaphore>,
    pub options: Options,
    pub client: reqwest::Client,
    pub meter: Mutex<HashMap<String, (std::time::Instant, u64)>>,
}

#[derive(Clone)]
pub struct Manager(pub(crate) Arc<Inner>);

impl Manager {
    /// Opens persistent state, with an exclusive process lock. Interrupted work is paused.
    pub fn open(directory: &Path, options: Options) -> Result<Self> {
        std::fs::create_dir_all(directory)?;
        let store = storage::Store::open(directory)?;
        let mut jobs = store.load()?;
        for job in jobs.values_mut() {
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
            .user_agent("StrataFetch/0.1.0");
        if let Some(proxy) = &options.proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        Ok(Self(Arc::new(Inner {
            jobs: Mutex::new(jobs),
            store: Mutex::new(store),
            controls: Mutex::new(HashMap::new()),
            slots: Arc::new(Semaphore::new(options.concurrent_jobs.clamp(1, 32))),
            client: builder.build()?,
            options,
            meter: Mutex::new(HashMap::new()),
        })))
    }

    pub fn list(&self) -> Vec<Job> {
        let mut jobs: Vec<_> = self.0.jobs.lock().unwrap().values().cloned().collect();
        jobs.sort_by_key(|j| (std::cmp::Reverse(j.priority), j.created));
        jobs
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
        let url = url::Url::parse(raw.trim()).context("Enter a valid HTTP or HTTPS URL")?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            bail!("Only HTTP and HTTPS URLs are supported");
        }
        if !url.username().is_empty() || url.password().is_some() {
            bail!("Credentials in URLs are not supported");
        }
        std::fs::create_dir_all(directory)?;
        let directory = directory.canonicalize()?;
        let filename = sanitize_filename(name.unwrap_or_else(|| {
            url.path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .unwrap_or("download")
        }));
        let jobs = self.list();
        let mut destination = directory.join(&filename);
        for index in 1..10000 {
            if !destination.exists() && !jobs.iter().any(|j| j.destination == destination) {
                break;
            }
            destination = directory.join(format!("{index}-{filename}"));
        }
        if destination.exists() || jobs.iter().any(|j| j.destination == destination) {
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
            segments: vec![],
            error: None,
            sha256: None,
            created: now(),
            completed: None,
            priority: 0,
            bytes_per_second: 0,
        };
        self.update(job.clone())?;
        Ok(job)
    }
    /// Starts a cooperative transfer on the current Tokio runtime.
    pub fn start(&self, id: &str) -> Result<()> {
        let mut controls = self.0.controls.lock().unwrap();
        if controls.contains_key(id) {
            return Ok(());
        }
        let mut job = self.get(id)?;
        if job.status == Status::Completed {
            bail!("Download already completed");
        }
        job.status = Status::Queued;
        job.error = None;
        self.update(job)?;
        let (tx, rx) = watch::channel(false);
        controls.insert(id.to_owned(), tx);
        drop(controls);
        let manager = self.clone();
        let id = id.to_owned();
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
            controls.remove(&id);
            manager.0.meter.lock().unwrap().remove(&id);
        });
        Ok(())
    }
    pub fn pause(&self, id: &str) -> Result<()> {
        self.stop(id, Status::Paused)
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        self.stop(id, Status::Cancelled)
    }
    fn stop(&self, id: &str, status: Status) -> Result<()> {
        let controls = self.0.controls.lock().unwrap();
        let mut job = self.get(id)?;
        if job.status == Status::Completed {
            bail!("Completed downloads cannot be stopped");
        }
        job.status = status;
        job.speed = 0.0;
        self.update(job)?;
        if let Some(control) = controls.get(id) {
            let _ = control.send(true);
        }
        Ok(())
    }
    pub fn restart(&self, id: &str) -> Result<()> {
        if self.0.controls.lock().unwrap().contains_key(id) {
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
        job.size = None;
        job.status = Status::Paused;
        self.update(job)?;
        self.start(id)
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        if self.0.controls.lock().unwrap().contains_key(id) {
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
        if self.0.controls.lock().unwrap().contains_key(id) {
            bail!("Pause before changing transfer settings");
        }
        let mut job = self.get(id)?;
        job.priority = priority;
        job.bytes_per_second = bytes_per_second;
        self.update(job)
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
        .take(180)
        .collect();
    name = name.trim_matches([' ', '.']).to_owned();
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if name.is_empty()
        || [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
    {
        name = format!("download_{name}");
    }
    name
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
