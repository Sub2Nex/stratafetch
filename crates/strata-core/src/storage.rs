//! Transactional SQLite snapshots; the lock prevents competing engine processes.
use crate::Job;
use anyhow::{Context, Result};
use fs2::FileExt;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::Path,
};

/// Repairs a crash between atomic publication and the completion checkpoint.
pub(crate) fn recover_published(job: &mut Job) -> Result<bool> {
    if job.status == crate::Status::Completed {
        return Ok(false);
    }
    let stage = crate::staging_path(job);
    if !job.destination.exists()
        || !stage.exists()
        || !same_file::is_same_file(&stage, &job.destination)?
    {
        return Ok(false);
    }
    let valid = (|| -> Result<String> {
        use std::io::{Read, Seek};
        anyhow::ensure!(
            job.size == Some(job.downloaded),
            "Incomplete published size"
        );
        let mut file = File::open(&stage)?;
        anyhow::ensure!(
            file.metadata()?.len() == job.downloaded,
            "Published file size changed"
        );
        let mut buffer = vec![0; 64 * 1024];
        for segment in &job.segments {
            file.seek(std::io::SeekFrom::Start(segment.start))?;
            let mut left = segment.done;
            let mut hash = Sha256::new();
            while left > 0 {
                let limit = left.min(buffer.len() as u64) as usize;
                file.read_exact(&mut buffer[..limit])?;
                hash.update(&buffer[..limit]);
                left -= limit as u64;
            }
            anyhow::ensure!(
                Some(format!("{:x}", hash.finalize())) == segment.sha256,
                "Published data checksum changed"
            );
        }
        file.seek(std::io::SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        Ok(format!("{:x}", hash.finalize()))
    })();
    match valid {
        Ok(hash) => {
            job.status = crate::Status::Completed;
            job.sha256 = Some(hash);
            job.completed = Some(crate::now());
            job.error = None;
            job.speed = 0.0;
            job.connections = 0;
        }
        Err(error) => {
            job.status = crate::Status::Failed;
            job.error = Some(format!("Publication recovery failed: {error:#}"));
        }
    }
    Ok(true)
}

fn private_file(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT: inspect the entry, rather than its target.
        options.custom_flags(0x00200000);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file(),
        "State must be a regular file: {}",
        path.display()
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        anyhow::ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "State reparse points are not supported"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}
pub struct Store {
    connection: Connection,
    _lock: File,
}
impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        let lock = private_file(&directory.join("engine.lockfile"))?;
        lock.try_lock_exclusive()
            .context("StrataFetch is already running with this state directory")?;
        let database = directory.join("state.sqlite");
        // SQLite sidecars inherit database permissions. Tighten existing sidecars
        // too, and refuse links before opening a profile through SQLite's VFS.
        let _database_file = private_file(&database)?;
        for name in ["state.sqlite-wal", "state.sqlite-shm"] {
            let path = directory.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(_) => {
                    private_file(&path)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => return Err(error.into()),
            }
        }
        let connection = Connection::open(database)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        anyhow::ensure!(
            version <= 2,
            "State database was created by a newer StrataFetch version"
        );
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS jobs (id TEXT PRIMARY KEY, json TEXT NOT NULL, digest TEXT NOT NULL); CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL); PRAGMA user_version=2; COMMIT;")?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }
    pub fn load(&self) -> Result<HashMap<String, Job>> {
        let mut statement = self.connection.prepare("SELECT id,json,digest FROM jobs")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut jobs = HashMap::new();
        for row in rows {
            let (id, json, digest) = row?;
            anyhow::ensure!(
                format!("{:x}", Sha256::digest(json.as_bytes())) == digest,
                "State checksum mismatch for {id}"
            );
            jobs.insert(id, serde_json::from_str(&json)?);
        }
        Ok(jobs)
    }
    pub fn settings(&self) -> Result<Option<crate::Settings>> {
        use rusqlite::OptionalExtension;
        let json: Option<String> = self
            .connection
            .query_row("SELECT json FROM settings WHERE id=1", [], |row| row.get(0))
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(Into::into))
            .transpose()
    }
    pub fn save_settings(&self, settings: &crate::Settings) -> Result<()> {
        self.connection.execute("INSERT INTO settings(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json", [serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn save(&self, job: &Job) -> Result<()> {
        let json = serde_json::to_string(job)?;
        let digest = format!("{:x}", Sha256::digest(json.as_bytes()));
        self.connection.execute("INSERT INTO jobs(id,json,digest) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET json=excluded.json,digest=excluded.digest", params![job.id, json, digest])?;
        Ok(())
    }
    pub fn delete(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM jobs WHERE id=?1", [id])?;
        Ok(())
    }
}
