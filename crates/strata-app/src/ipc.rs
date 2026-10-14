//! Versioned, size-bounded OS-local protocol. Windows rejects remote pipe clients.
use crate::{Command, Reply, execute};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use strata_core::Manager;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
const MAX: usize = 1024 * 1024;
#[cfg(windows)]
fn create_pipe(
    name: &str,
    first: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            SECURITY_ATTRIBUTES,
        },
    };
    // Protected DACL: SYSTEM and the object's owner only. OW is the Windows OWNER RIGHTS SID.
    let sddl: Vec<u16> = "D:P(A;;GA;;;SY)(A;;GA;;;OW)"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: NUL-terminated UTF-16 input and valid out-pointer. Windows allocates descriptor.
    let success = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    };
    if success == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    // SAFETY: attributes and allocated security descriptor remain alive through CreateNamedPipeW.
    let result = unsafe {
        tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                name,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )
    };
    // SAFETY: descriptor was allocated by ConvertStringSecurityDescriptor... and is no longer needed.
    unsafe {
        LocalFree(descriptor);
    }
    Ok(result?)
}
#[derive(Serialize, Deserialize)]
struct Request {
    version: u32,
    command: Command,
}
// Pages belong to one immutable command result on one connection. Concurrent
// transfers cannot shift offsets and cause duplicates or missing records.
#[derive(Serialize, Deserialize)]
struct Page {
    version: u32,
    index: usize,
    done: bool,
    reply: Reply,
}
async fn write_pages(stream: &mut (impl AsyncWrite + Unpin), mut reply: Reply) -> Result<()> {
    let jobs = std::mem::take(&mut reply.jobs);
    let mut pending = jobs.into_iter().peekable();
    let mut index = 0;
    loop {
        let mut page_reply = reply.clone();
        let mut bytes = serde_json::to_vec(&page_reply)?.len() + 128;
        while let Some(job) = pending.peek() {
            let size = serde_json::to_vec(job)?.len() + 1;
            ensure!(
                size < MAX / 2,
                "Single download record exceeds IPC page limit"
            );
            if !page_reply.jobs.is_empty()
                && (bytes + size > MAX / 2 || page_reply.jobs.len() == 128)
            {
                break;
            }
            bytes += size;
            page_reply.jobs.push(pending.next().unwrap());
        }
        let done = pending.peek().is_none();
        write_frame(
            stream,
            &Page {
                version: 2,
                index,
                done,
                reply: page_reply,
            },
        )
        .await?;
        if done {
            return Ok(());
        }
        index += 1;
    }
}
async fn read_pages(stream: &mut (impl AsyncRead + Unpin)) -> Result<Reply> {
    let mut result: Option<Reply> = None;
    let mut index = 0;
    loop {
        let page: Page = read_frame(stream).await?;
        ensure!(
            page.version == 2 && page.index == index,
            "Invalid IPC page sequence"
        );
        ensure!(
            page.done || !page.reply.jobs.is_empty(),
            "Empty intermediate IPC page"
        );
        if let Some(reply) = &mut result {
            ensure!(
                reply.version == page.reply.version
                    && reply.error == page.reply.error
                    && reply.accepted_id == page.reply.accepted_id
                    && reply.settings == page.reply.settings,
                "Inconsistent IPC snapshot metadata"
            );
            reply.jobs.extend(page.reply.jobs);
        } else {
            result = Some(page.reply);
        }
        if page.done {
            return Ok(result.unwrap());
        }
        index += 1;
    }
}
#[cfg(windows)]
fn endpoint(directory: &Path) -> Result<String> {
    let canonical = directory
        .canonicalize()
        .unwrap_or_else(|_| directory.to_path_buf());
    let key = format!(
        "{:x}",
        Sha256::digest(canonical.to_string_lossy().as_bytes())
    );
    Ok(format!("\\\\.\\pipe\\stratafetch-{}", &key[..32]))
}
#[cfg(unix)]
fn user_id() -> u32 {
    // SAFETY: geteuid takes no arguments and does not access caller memory.
    unsafe { libc::geteuid() }
}
#[cfg(unix)]
fn endpoint(directory: &Path) -> Result<String> {
    use std::os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    };
    // macOS sun_path is only 104 bytes. State can live in a much longer path;
    // a short, private, per-user location also works across CLI/browser launch environments.
    let root = std::path::PathBuf::from(format!("/tmp/stratafetch-{}", user_id()));
    match std::fs::DirBuilder::new().mode(0o700).create(&root) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error.into()),
    }
    let metadata = std::fs::symlink_metadata(&root)?;
    ensure!(
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && metadata.uid() == user_id()
            && metadata.permissions().mode() & 0o077 == 0,
        "IPC directory must be owned by this user with private permissions: {}",
        root.display()
    );
    let canonical = directory.canonicalize()?;
    let key = format!("{:x}", Sha256::digest(canonical.as_os_str().as_bytes()));
    Ok(root
        .join(format!("ipc-{}.sock", &key[..32]))
        .to_string_lossy()
        .into_owned())
}
#[cfg(unix)]
struct SocketCleanup {
    path: String,
    device: u64,
    inode: u64,
}
#[cfg(unix)]
impl Drop for SocketCleanup {
    fn drop(&mut self) {
        use std::os::unix::fs::MetadataExt;
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path)
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
async fn write_frame<T: serde::Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= MAX, "IPC response exceeds limit");
    stream.write_u32_le(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}
async fn read_frame<T: serde::de::DeserializeOwned>(
    stream: &mut (impl AsyncRead + Unpin),
) -> Result<T> {
    let length = stream.read_u32_le().await? as usize;
    ensure!(length <= MAX, "IPC message exceeds limit");
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
async fn exchange(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    command: Command,
) -> Result<Reply> {
    write_frame(
        &mut stream,
        &Request {
            version: 2,
            command,
        },
    )
    .await?;
    read_pages(&mut stream).await
}
pub async fn call(directory: &Path, command: Command) -> Result<Reply> {
    #[cfg(windows)]
    {
        let stream =
            tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint(directory)?)?;
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            exchange(stream, command),
        )
        .await?
    }
    #[cfg(unix)]
    {
        let stream = tokio::net::UnixStream::connect(endpoint(directory)?).await?;
        ensure!(
            stream.peer_cred()?.uid() == user_id(),
            "IPC service belongs to another user"
        );
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            exchange(stream, command),
        )
        .await?
    }
}
async fn handle(mut stream: impl AsyncRead + AsyncWrite + Unpin, manager: Manager) -> Result<()> {
    let request: Request =
        tokio::time::timeout(std::time::Duration::from_secs(5), read_frame(&mut stream)).await??;
    ensure!(matches!(request.version, 1 | 2), "Unsupported IPC version");
    let version = request.version;
    let reply = tokio::task::spawn_blocking(move || execute(&manager, request.command)).await?;
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        if version == 2 { write_pages(&mut stream, reply).await } else {
            // Retain small legacy replies, with an actionable error for old clients.
            let mut reply = reply;
            if serde_json::to_vec(&reply)?.len() > MAX {
                reply.jobs.clear();
                reply.error = Some("History exceeds legacy IPC limit; upgrade all StrataFetch executables together".into());
            }
            write_frame(&mut stream, &reply).await
        }
    }).await?
}
pub async fn serve(directory: &Path, manager: Manager) -> Result<()> {
    let capacity = std::sync::Arc::new(tokio::sync::Semaphore::new(16));
    #[cfg(windows)]
    {
        let name = endpoint(directory)?;
        let mut server = create_pipe(&name, true)?;
        loop {
            server.connect().await?;
            let next = create_pipe(&name, false)?;
            let connected = std::mem::replace(&mut server, next);
            let manager = manager.clone();
            let permit = capacity.clone().acquire_owned().await?;
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(error) = handle(connected, manager).await {
                    eprintln!("Local control request failed: {error:#}");
                }
            });
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
        let name = endpoint(directory)?;
        match std::fs::symlink_metadata(&name) {
            Ok(metadata) => {
                ensure!(
                    metadata.file_type().is_socket() && metadata.uid() == user_id(),
                    "Refusing to replace a non-socket or foreign IPC endpoint"
                );
                match tokio::net::UnixStream::connect(&name).await {
                    Ok(_) => anyhow::bail!("A local service is already listening"),
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                        ) =>
                    {
                        std::fs::remove_file(&name)?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        let listener = tokio::net::UnixListener::bind(&name)?;
        let metadata = std::fs::symlink_metadata(&name)?;
        let _cleanup = SocketCleanup {
            path: name.clone(),
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        std::fs::set_permissions(&name, std::fs::Permissions::from_mode(0o600))?;
        loop {
            let (connected, _) = listener.accept().await?;
            if connected.peer_cred()?.uid() != user_id() {
                continue;
            }
            let manager = manager.clone();
            let permit = capacity.clone().acquire_owned().await?;
            tokio::spawn(async move {
                let _permit = permit;
                if let Err(error) = handle(connected, manager).await {
                    eprintln!("Local control request failed: {error:#}");
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test]
    async fn unix_long_paths_stale_sockets_and_cleanup() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("long-directory-".repeat(10)).join("state");
        let manager = Manager::open(&state, Default::default()).unwrap();
        let name = endpoint(&state).unwrap();
        assert!(name.len() < 104);
        // Dropping a Unix listener leaves a socket, as does a killed service.
        drop(std::os::unix::net::UnixListener::bind(&name).unwrap());
        let service = tokio::spawn({
            let state = state.clone();
            let manager = manager.clone();
            async move { serve(&state, manager).await }
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if call(&state, Command::List).await.is_ok() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            std::fs::metadata(&name).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            serve(&state, manager.clone()).await.is_err(),
            "must preserve an active listener"
        );
        assert!(call(&state, Command::List).await.is_ok());
        service.abort();
        let _ = service.await;
        assert!(
            !Path::new(&name).exists(),
            "aborting the service cleans up its own socket"
        );
        std::fs::write(&name, b"user file").unwrap();
        assert!(serve(&state, manager).await.is_err());
        assert_eq!(std::fs::read(&name).unwrap(), b"user file");
        std::fs::remove_file(name).unwrap();
    }
    #[tokio::test]
    async fn oversized_ipc_frame_rejected_before_allocation() {
        let (mut sender, mut receiver) = tokio::io::duplex(64);
        sender.write_u32_le(MAX as u32 + 1).await.unwrap();
        assert!(read_frame::<Request>(&mut receiver).await.is_err());
    }
    #[tokio::test]
    async fn unsupported_protocol_version_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::open(dir.path(), strata_core::Options::default()).unwrap();
        let (mut client, server) = tokio::io::duplex(1024);
        write_frame(
            &mut client,
            &Request {
                version: 99,
                command: Command::List,
            },
        )
        .await
        .unwrap();
        assert!(handle(server, manager).await.is_err());
    }

    #[tokio::test]
    async fn truncated_and_reordered_snapshots_are_not_returned() {
        for index in [0, 1] {
            let (mut sender, mut receiver) = tokio::io::duplex(4096);
            let reply = Reply {
                version: 1,
                jobs: vec![],
                error: None,
                accepted_id: None,
                settings: Default::default(),
            };
            write_frame(
                &mut sender,
                &Page {
                    version: 2,
                    index,
                    done: false,
                    reply,
                },
            )
            .await
            .unwrap();
            drop(sender);
            assert!(read_pages(&mut receiver).await.is_err());
        }
    }

    #[tokio::test]
    async fn legacy_clients_still_receive_small_replies() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Manager::open(dir.path(), Default::default()).unwrap();
        let (mut client, server) = tokio::io::duplex(4096);
        let handler = tokio::spawn(handle(server, manager));
        write_frame(
            &mut client,
            &Request {
                version: 1,
                command: Command::List,
            },
        )
        .await
        .unwrap();
        let reply: Reply = read_frame(&mut client).await.unwrap();
        assert!(reply.error.is_none());
        assert!(reply.jobs.is_empty());
        handler.await.unwrap().unwrap();
    }
}
