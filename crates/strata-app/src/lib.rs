//! Shared command interface for CLI, desktop, and browser native messaging.
pub mod ipc;
use anyhow::{Result,bail};
use serde::{Deserialize,Serialize};
use std::path::{Path,PathBuf};
use strata_core::{Manager,Job,Status};

pub fn state_dir()->PathBuf {directories::ProjectDirs::from("org","StrataFetch","StrataFetch").expect("User data directory unavailable").data_local_dir().to_path_buf()}
pub fn download_dir()->PathBuf {directories::UserDirs::new().and_then(|d|d.download_dir().map(Path::to_path_buf)).unwrap_or_else(||PathBuf::from("."))}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(tag="command",rename_all="snake_case")]
pub enum Command {
    List, Add {url:String,directory:PathBuf,name:Option<String>},
    Start {id:String},Pause {id:String},Cancel {id:String},Restart {id:String},Remove {id:String},
    Configure {id:String,priority:i32,bytes_per_second:u64},
}
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Reply {pub version:u32,pub jobs:Vec<Job>,pub error:Option<String>}
pub fn execute(manager:&Manager,command:Command)->Reply {
    let result:Result<()>= (|| {match command {
        Command::List=>(),
        Command::Add {url,directory,name}=>{let job=manager.add(&url,&directory,name.as_deref())?;manager.start(&job.id)?;},
        Command::Start {id}=>manager.start(&id)?,Command::Pause {id}=>manager.pause(&id)?,Command::Cancel {id}=>manager.cancel(&id)?,
        Command::Restart {id}=>manager.restart(&id)?,Command::Remove {id}=>manager.remove(&id)?,
        Command::Configure {id,priority,bytes_per_second}=>manager.configure(&id,priority,bytes_per_second)?,
    };Ok(()) })();
    Reply {version:1,jobs:manager.list(),error:result.err().map(|e|format!("{e:#}"))}
}
pub async fn shutdown(manager:&Manager) {for job in manager.list() {if matches!(job.status,Status::Downloading|Status::Queued) {let _=manager.pause(&job.id);}}
    for _ in 0..200 {if !manager.is_running(){break;}tokio::time::sleep(std::time::Duration::from_millis(25)).await;}}
pub fn check_reply(reply:Reply)->Result<Reply>{if let Some(error)=&reply.error {bail!("{error}");}Ok(reply)}
