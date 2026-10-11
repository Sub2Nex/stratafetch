//! Transactional SQLite snapshots; the lock prevents competing engine processes.
use crate::Job;
use anyhow::{Result, Context};
use fs2::FileExt;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs::{File, OpenOptions}, path::Path};

pub struct Store { connection: Connection, _lock: File }
impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(directory.join("engine.lockfile"))?;
        lock.try_lock_exclusive().context("StrataFetch is already running with this state directory")?;
        let connection = Connection::open(directory.join("state.sqlite"))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS jobs (id TEXT PRIMARY KEY, json TEXT NOT NULL, digest TEXT NOT NULL); PRAGMA user_version=1;")?;
        Ok(Self { connection, _lock: lock })
    }
    pub fn load(&self) -> Result<HashMap<String, Job>> {
        let mut statement = self.connection.prepare("SELECT id,json,digest FROM jobs")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?;
        let mut jobs = HashMap::new();
        for row in rows { let (id, json, digest) = row?; anyhow::ensure!(format!("{:x}", Sha256::digest(json.as_bytes())) == digest, "State checksum mismatch for {id}"); jobs.insert(id, serde_json::from_str(&json)?); }
        Ok(jobs)
    }
    pub fn save(&self, job: &Job) -> Result<()> { let json = serde_json::to_string(job)?; let digest = format!("{:x}", Sha256::digest(json.as_bytes())); self.connection.execute("INSERT INTO jobs(id,json,digest) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET json=excluded.json,digest=excluded.digest", params![job.id, json, digest])?; Ok(()) }
    pub fn delete(&self, id: &str) -> Result<()> { self.connection.execute("DELETE FROM jobs WHERE id=?1", [id])?; Ok(()) }
}
