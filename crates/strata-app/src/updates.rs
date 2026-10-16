//! GitHub release discovery and checksum-verified portable desktop updates.
use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf, time::Duration};

const API: &str = "https://api.github.com/repos/Sub2Nex/stratafetch/releases?per_page=100";
pub const RELEASES: &str = "https://github.com/Sub2Nex/stratafetch/releases";
const MAX_BINARY: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

impl Release {
    fn asset(&self, name: &str) -> Result<&Asset> {
        let mut matching = self.assets.iter().filter(|a| a.name == name);
        let asset = matching
            .next()
            .context("This release has no compatible update asset")?;
        ensure!(matching.next().is_none(), "Duplicate release asset");
        let prefix = format!(
            "https://github.com/Sub2Nex/stratafetch/releases/download/{}/",
            self.tag_name
        );
        ensure!(
            asset.browser_download_url.starts_with(&prefix),
            "Unexpected release asset URL"
        );
        Ok(asset)
    }

    pub fn can_install(&self) -> bool {
        cfg!(all(target_os = "windows", target_arch = "x86_64"))
            && self.asset("stratafetch-desktop.exe").is_ok()
            && self.asset("SHA256SUMS").is_ok()
    }
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("StrataFetch/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(300))
        .build()?)
}

async fn fetch(client: &reqwest::Client, url: &str, max: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    ensure!(
        response.content_length().unwrap_or(0) <= max as u64,
        "Update response too large"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= max,
            "Update response too large"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn select_release(releases: Vec<Release>, current: &str) -> Result<Option<Release>> {
    let current = Version::parse(current)?;
    let mut candidates = Vec::new();
    for release in releases {
        if release.draft {
            continue;
        }
        let Ok(version) = Version::parse(release.tag_name.trim_start_matches('v')) else {
            continue;
        };
        // Stable users stay on stable; preview users can graduate to stable.
        if current.pre.is_empty() && (release.prerelease || !version.pre.is_empty()) {
            continue;
        }
        if version > current {
            candidates.push((version, release));
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(candidates.pop().map(|(_, release)| release))
}

pub async fn check() -> Result<Option<Release>> {
    let client = client()?;
    let body = fetch(&client, API, 4 * 1024 * 1024).await?;
    select_release(serde_json::from_slice(&body)?, env!("CARGO_PKG_VERSION"))
}

fn expected_hash(sums: &str, name: &str) -> Result<String> {
    let hashes: Vec<_> = sums
        .lines()
        .filter_map(|line| {
            let (hash, file) = line.split_once(char::is_whitespace)?;
            (file.trim().trim_start_matches('*') == name).then_some(hash)
        })
        .collect();
    ensure!(hashes.len() == 1, "Missing or duplicate desktop checksum");
    let hash = hashes[0];
    ensure!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid SHA-256 checksum"
    );
    Ok(hash.to_ascii_lowercase())
}

/// The staged executable is also the helper, so no shell script or quoting is needed.
pub struct PreparedUpdate {
    directory: tempfile::TempDir,
    executable: PathBuf,
}

pub async fn download(release: &Release) -> Result<PreparedUpdate> {
    ensure!(
        release.can_install(),
        "In-app installation is available for Windows x64 portable desktops only"
    );
    let client = client()?;
    let sums = fetch(
        &client,
        &release.asset("SHA256SUMS")?.browser_download_url,
        64 * 1024,
    )
    .await?;
    let expected = expected_hash(std::str::from_utf8(&sums)?, "stratafetch-desktop.exe")?;
    let binary = fetch(
        &client,
        &release
            .asset("stratafetch-desktop.exe")?
            .browser_download_url,
        MAX_BINARY,
    )
    .await?;
    ensure!(
        format!("{:x}", Sha256::digest(&binary)) == expected,
        "Update checksum mismatch; installation cancelled"
    );
    ensure!(
        binary.starts_with(b"MZ"),
        "Update is not a Windows executable"
    );
    let current = std::env::current_exe()?;
    let directory = tempfile::Builder::new()
        .prefix(".stratafetch-update-")
        .tempdir_in(current.parent().context("No executable directory")?)?;
    let executable = directory.path().join("stratafetch-desktop.exe");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&executable)?;
    file.write_all(&binary)?;
    file.sync_all()?;
    Ok(PreparedUpdate {
        directory,
        executable,
    })
}

impl PreparedUpdate {
    pub fn launch(self) -> Result<()> {
        let target = std::env::current_exe()?;
        let mut command = std::process::Command::new(&self.executable);
        command
            .arg("--apply-update")
            .arg(target)
            .arg(std::process::id().to_string());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.spawn().context("Cannot start update helper")?;
        let _ = self.directory.keep();
        Ok(())
    }
}

/// Invoked before eframe starts. Wait for graceful engine shutdown before replacing files.
pub fn apply_update_if_requested() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--apply-update")) {
        return None;
    }
    Some((|| {
        let target = PathBuf::from(args.next().context("Missing update target")?);
        let pid: u32 = args
            .next()
            .context("Missing parent PID")?
            .to_str()
            .context("Invalid PID")?
            .parse()?;
        ensure!(args.next().is_none(), "Unexpected update arguments");
        apply(target, pid)
    })())
}

#[cfg(windows)]
fn apply(target: PathBuf, pid: u32) -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, GetLastError},
        System::Threading::{OpenProcess, WaitForSingleObject},
    };
    let source = std::env::current_exe()?;
    let staging = source.parent().context("No staging directory")?;
    ensure!(
        staging
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(".stratafetch-update-"))
            && staging.parent() == target.parent(),
        "Invalid update staging location"
    );
    // SYNCHRONIZE access only; no elevation or modification of the parent process.
    unsafe {
        let process = OpenProcess(0x00100000, 0, pid);
        if !process.is_null() {
            let status = WaitForSingleObject(process, 120_000);
            CloseHandle(process);
            ensure!(status == 0, "Timed out waiting for StrataFetch to close");
        } else {
            ensure!(
                GetLastError() == ERROR_INVALID_PARAMETER,
                "Cannot wait for the running desktop to exit"
            );
        }
    }
    let backup = staging.join("previous.exe");
    // Windows can briefly retain image locks after the process signals exit.
    let mut moved = false;
    for _ in 0..100 {
        if std::fs::rename(&target, &backup).is_ok() {
            moved = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    ensure!(
        moved,
        "Cannot replace desktop executable; old version retained"
    );
    replace_and_restart(&source, &target, &backup, |path| {
        std::process::Command::new(path).spawn()?;
        Ok(())
    })
}

#[cfg(any(windows, test))]
fn replace_and_restart(
    source: &std::path::Path,
    target: &std::path::Path,
    backup: &std::path::Path,
    mut restart: impl FnMut(&std::path::Path) -> Result<()>,
) -> Result<()> {
    let install = (|| -> Result<()> {
        std::fs::copy(source, target)?;
        std::fs::OpenOptions::new()
            .write(true)
            .open(target)?
            .sync_all()?;
        restart(target)?;
        Ok(())
    })();
    if install.is_err() {
        let _ = std::fs::remove_file(target);
        std::fs::rename(backup, target)
            .context("Restore previous executable from update directory")?;
        let _ = restart(target);
    }
    install
}

#[cfg(not(windows))]
fn apply(_target: PathBuf, _pid: u32) -> Result<()> {
    anyhow::bail!("In-app installation is available on Windows x64 only")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: tag.contains('-'),
            assets: vec![],
        }
    }
    #[test]
    fn versions_and_channels() {
        let tags = || {
            vec![
                release("v1.0.9"),
                release("v1.0.10"),
                release("v2.0.0-beta.1"),
                release("bad"),
            ]
        };
        assert_eq!(
            select_release(tags(), "1.0.0").unwrap().unwrap().tag_name,
            "v1.0.10"
        );
        assert_eq!(
            select_release(tags(), "1.0.0-beta.1")
                .unwrap()
                .unwrap()
                .tag_name,
            "v2.0.0-beta.1"
        );
        assert!(select_release(tags(), "2.0.0").unwrap().is_none());
        let mut draft = release("v3.0.0");
        draft.draft = true;
        assert!(select_release(vec![draft], "1.0.0").unwrap().is_none());
    }
    #[test]
    fn checksum_requires_exact_unique_filename() {
        let hash = "a".repeat(64);
        assert_eq!(
            expected_hash(
                &format!("{hash}  stratafetch-desktop.exe\n"),
                "stratafetch-desktop.exe"
            )
            .unwrap(),
            hash
        );
        assert!(expected_hash(&format!("{hash}  other.exe"), "stratafetch-desktop.exe").is_err());
        assert!(expected_hash("oops  stratafetch-desktop.exe", "stratafetch-desktop.exe").is_err());
        assert!(expected_hash(&format!("{hash}  x\n{hash}  x"), "x").is_err());
    }
    #[test]
    fn rejects_external_asset_url() {
        let mut r = release("v1.0.1");
        r.assets.push(Asset {
            name: "SHA256SUMS".into(),
            browser_download_url: "https://example.org/SHA256SUMS".into(),
        });
        assert!(r.asset("SHA256SUMS").is_err());
    }

    #[test]
    fn failed_restart_restores_previous_executable() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("new.exe");
        let target = dir.path().join("app.exe");
        let backup = dir.path().join("previous.exe");
        std::fs::write(&source, b"new version").unwrap();
        std::fs::write(&backup, b"old version").unwrap();
        let mut attempts = 0;
        let result = replace_and_restart(&source, &target, &backup, |path| {
            attempts += 1;
            if attempts == 1 {
                anyhow::bail!("simulated launch failure");
            }
            assert_eq!(std::fs::read(path)?, b"old version");
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(attempts, 2);
        assert_eq!(std::fs::read(&target).unwrap(), b"old version");
    }

    #[test]
    fn successful_restart_retains_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("new.exe");
        let target = dir.path().join("app.exe");
        let backup = dir.path().join("previous.exe");
        std::fs::write(&source, b"new version").unwrap();
        std::fs::write(&backup, b"old version").unwrap();
        replace_and_restart(&source, &target, &backup, |path| {
            assert_eq!(std::fs::read(path)?, b"new version");
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read(backup).unwrap(), b"old version");
    }
}
