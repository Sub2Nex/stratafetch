//! Versioned, size-bounded OS-local protocol. Windows rejects remote pipe clients.
use crate::{Command,Reply,execute};
use anyhow::{Result,ensure};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::path::Path;
use strata_core::Manager;
use tokio::io::{AsyncRead,AsyncReadExt,AsyncWrite,AsyncWriteExt};
const MAX:usize=1024*1024;
#[derive(Serialize,Deserialize)]
struct Request {version:u32,command:Command}
fn endpoint(directory:&Path)->String {let canonical=directory.canonicalize().unwrap_or_else(|_|directory.to_path_buf());let key=format!("{:x}",Sha256::digest(canonical.to_string_lossy().as_bytes()));
    #[cfg(windows)] {format!("\\\\.\\pipe\\stratafetch-{}",&key[..32])}
    #[cfg(unix)] {directory.join(format!("ipc-{}.sock",&key[..12])).to_string_lossy().into_owned()}
}
async fn write_frame<T:serde::Serialize>(stream:&mut (impl AsyncWrite+Unpin),value:&T)->Result<()> {let bytes=serde_json::to_vec(value)?;ensure!(bytes.len()<=MAX,"IPC response exceeds limit");stream.write_u32_le(bytes.len() as u32).await?;stream.write_all(&bytes).await?;stream.flush().await?;Ok(())}
async fn read_frame<T:serde::de::DeserializeOwned>(stream:&mut (impl AsyncRead+Unpin))->Result<T> {let length=stream.read_u32_le().await? as usize;ensure!(length<=MAX,"IPC message exceeds limit");let mut bytes=vec![0;length];stream.read_exact(&mut bytes).await?;Ok(serde_json::from_slice(&bytes)?)}
async fn exchange(mut stream:impl AsyncRead+AsyncWrite+Unpin,command:Command)->Result<Reply> {write_frame(&mut stream,&Request {version:1,command}).await?;read_frame(&mut stream).await}
pub async fn call(directory:&Path,command:Command)->Result<Reply> {
    #[cfg(windows)] {let stream=tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint(directory))?;tokio::time::timeout(std::time::Duration::from_secs(15),exchange(stream,command)).await?}
    #[cfg(unix)] {let stream=tokio::net::UnixStream::connect(endpoint(directory)).await?;tokio::time::timeout(std::time::Duration::from_secs(15),exchange(stream,command)).await?}
}
async fn handle(mut stream:impl AsyncRead+AsyncWrite+Unpin,manager:Manager)->Result<()> {let request:Request=tokio::time::timeout(std::time::Duration::from_secs(5),read_frame(&mut stream)).await??;ensure!(request.version==1,"Unsupported IPC version");let reply=execute(&manager,request.command);write_frame(&mut stream,&reply).await}
pub async fn serve(directory:&Path,manager:Manager)->Result<()> {
    let capacity=std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    #[cfg(windows)] {
        use tokio::net::windows::named_pipe::ServerOptions;
        let name=endpoint(directory);let mut server=ServerOptions::new().first_pipe_instance(true).reject_remote_clients(true).create(&name)?;
        loop {server.connect().await?;let next=ServerOptions::new().reject_remote_clients(true).create(&name)?;let connected=std::mem::replace(&mut server,next);let manager=manager.clone();let permit=capacity.clone().acquire_owned().await?;tokio::spawn(async move {let _permit=permit;let _=handle(connected,manager).await;});}
    }
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let name=endpoint(directory);if Path::new(&name).exists(){std::fs::remove_file(&name)?;}
        let listener=tokio::net::UnixListener::bind(&name)?;std::fs::set_permissions(&name,std::fs::Permissions::from_mode(0o600))?;
        loop {let (connected,_)=listener.accept().await?;let manager=manager.clone();let permit=capacity.clone().acquire_owned().await?;tokio::spawn(async move {let _permit=permit;let _=handle(connected,manager).await;});}
    }
}
