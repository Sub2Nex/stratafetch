//! Portable history exports are recipes, never trusted partial-file or checksum state.
use crate::{Manager, sanitize_filename};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    url: String,
    filename: String,
    category: String,
    priority: i32,
    bytes_per_second: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    version: u32,
    entries: Vec<Entry>,
}
impl Manager {
    /// Creates a new file; existing exports are preserved. URLs may contain private query tokens.
    pub fn export_history(&self, path: &Path) -> Result<()> {
        let entries = self
            .list()
            .into_iter()
            .map(|job| Entry {
                url: job.url,
                filename: job
                    .destination
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                category: job.category,
                priority: job.priority,
                bytes_per_second: job.bytes_per_second,
            })
            .collect();
        let bytes = serde_json::to_vec_pretty(&History {
            version: 1,
            entries,
        })?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "Export exceeds 8 MiB history limit"
        );
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    }
    /// Recreates paused download recipes in an explicit directory with new IDs and no partial data.
    /// Validation is complete before changes; I/O errors can leave an explicitly paused partial import.
    pub fn import_history(&self, path: &Path, directory: &Path) -> Result<()> {
        let file = std::fs::File::open(path)?;
        ensure!(file.metadata()?.is_file(), "History must be a regular file");
        let mut bytes = Vec::new();
        file.take(8 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 8 * 1024 * 1024,
            "History exceeds 8 MiB limit"
        );
        let history: History = serde_json::from_slice(&bytes)?;
        ensure!(history.version == 1, "Unsupported history version");
        ensure!(
            history.entries.len() <= 1000,
            "Import limited to 1000 entries"
        );
        for entry in &history.entries {
            let url = url::Url::parse(&entry.url)?;
            ensure!(
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none(),
                "Unsafe URL in history"
            );
            ensure!(
                entry.filename == sanitize_filename(&entry.filename),
                "Unsafe filename in history"
            );
            ensure!(entry.category.len() <= 180, "Category too long");
        }
        for entry in history.entries {
            let job = self.add(&entry.url, directory, Some(&entry.filename))?;
            self.configure(&job.id, entry.priority, entry.bytes_per_second)?;
            self.schedule(&job.id, &entry.category, None)?;
        }
        Ok(())
    }
}
