//! Shared command interface for CLI, desktop, and browser native messaging.
pub mod ipc;
pub mod native;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strata_core::{Job, Manager, Settings, Status};

pub fn state_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("STRATAFETCH_STATE_DIR") {
        return PathBuf::from(path);
    }
    directories::ProjectDirs::from("org", "StrataFetch", "StrataFetch")
        .expect("User data directory unavailable")
        .data_local_dir()
        .to_path_buf()
}
pub fn download_dir() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|d| d.download_dir().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    List,
    Settings {
        settings: Settings,
    },
    Verify {
        id: String,
        sha256: Option<String>,
    },
    Add {
        url: String,
        directory: PathBuf,
        name: Option<String>,
    },
    Start {
        id: String,
    },
    Pause {
        id: String,
    },
    Cancel {
        id: String,
    },
    Restart {
        id: String,
    },
    Remove {
        id: String,
    },
    Configure {
        id: String,
        priority: i32,
        bytes_per_second: u64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub version: u32,
    pub jobs: Vec<Job>,
    pub error: Option<String>,
    pub accepted_id: Option<String>,
    #[serde(default)]
    pub settings: Settings,
}
pub fn execute(manager: &Manager, command: Command) -> Reply {
    let mut accepted_id = match &command {
        Command::Verify { id, .. }
        | Command::Start { id }
        | Command::Pause { id }
        | Command::Cancel { id }
        | Command::Restart { id }
        | Command::Remove { id }
        | Command::Configure { id, .. } => Some(id.clone()),
        _ => None,
    };
    let result: Result<()> = (|| {
        match command {
            Command::List => (),
            Command::Settings { settings } => manager.set_settings(settings)?,
            Command::Verify { id, sha256 } => manager.verify(&id, sha256.as_deref())?,
            Command::Add {
                url,
                directory,
                name,
            } => {
                let job = manager.add(&url, &directory, name.as_deref())?;
                accepted_id = Some(job.id.clone());
                manager.start(&job.id)?;
            }
            Command::Start { id } => manager.start(&id)?,
            Command::Pause { id } => manager.pause(&id)?,
            Command::Cancel { id } => manager.cancel(&id)?,
            Command::Restart { id } => manager.restart(&id)?,
            Command::Remove { id } => manager.remove(&id)?,
            Command::Configure {
                id,
                priority,
                bytes_per_second,
            } => manager.configure(&id, priority, bytes_per_second)?,
        };
        Ok(())
    })();
    Reply {
        version: 1,
        jobs: manager.list(),
        error: result.err().map(|e| format!("{e:#}")),
        accepted_id,
        settings: manager.settings(),
    }
}
pub async fn shutdown(manager: &Manager) {
    for job in manager.list() {
        if matches!(job.status, Status::Downloading | Status::Queued) {
            let _ = manager.pause(&job.id);
        }
    }
    for _ in 0..200 {
        if !manager.is_running() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}
pub fn check_reply(reply: Reply) -> Result<Reply> {
    if let Some(error) = &reply.error {
        bail!("{error}");
    }
    Ok(reply)
}
