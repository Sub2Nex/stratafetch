//! Local browser setup and a single desktop window per profile.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug)]
pub enum Browser {
    Chrome,
    Edge,
    Firefox,
}
impl Browser {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chrome => "Chrome",
            Self::Edge => "Edge",
            Self::Firefox => "Firefox",
        }
    }
    pub fn folder(self) -> &'static str {
        if matches!(self, Self::Firefox) {
            "firefox"
        } else {
            "chromium"
        }
    }
    pub fn page(self) -> &'static str {
        match self {
            Self::Chrome => "chrome://extensions/",
            Self::Edge => "edge://extensions/",
            Self::Firefox => "about:debugging#/runtime/this-firefox",
        }
    }
}

#[derive(Deserialize)]
struct Ids {
    chromium: String,
    firefox: String,
}
#[derive(Deserialize)]
struct StoreIdentity {
    id: String,
}
#[derive(Deserialize)]
struct StoreIds {
    chrome: Option<StoreIdentity>,
    edge: Option<StoreIdentity>,
}
fn production_ids(local: &str, config: &str) -> Result<(String, String)> {
    let stores: StoreIds = serde_json::from_str(config)?;
    let validate = |identity: Option<StoreIdentity>| -> Result<String> {
        match identity {
            None => Ok(local.into()),
            Some(identity) => {
                ensure!(
                    identity.id != local
                        && identity.id.len() == 32
                        && identity.id.bytes().all(|c| (b'a'..=b'p').contains(&c)),
                    "Invalid production extension ID; preserve the local development identity"
                );
                Ok(identity.id)
            }
        }
    };
    Ok((validate(stores.chrome)?, validate(stores.edge)?))
}
pub fn chromium_id() -> String {
    serde_json::from_str::<Ids>(include_str!("../../../extensions/local-ids.json"))
        .expect("bundled extension IDs")
        .chromium
}

pub fn package_root() -> Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let directory = executable
        .parent()
        .context("Missing executable directory")?;
    if directory
        .join("extensions/chromium/manifest.json")
        .is_file()
    {
        return Ok(directory.into());
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ensure!(
        source.join("extensions/chromium/manifest.json").is_file(),
        "Install the full StrataFetch setup package to use local browser integration"
    );
    Ok(source.canonicalize()?)
}

pub fn extension_folder(browser: Browser) -> Result<PathBuf> {
    Ok(package_root()?.join("extensions").join(browser.folder()))
}

pub fn browser_components_warning() -> Option<String> {
    component_warning_at(std::env::current_exe().ok()?.parent()?)
}
fn component_warning_at(directory: &Path) -> Option<String> {
    if !directory
        .join("extensions/chromium/manifest.json")
        .is_file()
    {
        return None;
    }
    let metadata = std::fs::read_to_string(directory.join("BUILD.txt")).unwrap_or_default();
    if metadata
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        == Some(env!("CARGO_PKG_VERSION"))
    {
        return None;
    }
    Some("The desktop was updated, but its browser components need the matching full setup package. Automatic handoff may be unavailable until the native host and extensions are updated.".into())
}

pub fn register_local() -> Result<()> {
    let directory = std::env::current_exe()?
        .parent()
        .context("Missing executable directory")?
        .to_owned();
    let binary = directory.join(if cfg!(windows) {
        "stratafetch-browser-host.exe"
    } else {
        "stratafetch-browser-host"
    });
    ensure!(
        binary.is_file(),
        "Install the full setup package; the native browser host is missing"
    );
    let folder = package_root()?;
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        folder.join("extensions/chromium/manifest.json"),
    )?)?;
    ensure!(
        manifest["version"] == "0.2.0",
        "Browser components need updating. Install the full Windows setup or matching Linux package, then try Setup again"
    );
    let mut version = Command::new(&binary);
    version.arg("--version").stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        version.creation_flags(0x08000000);
    }
    let output = version.output()?;
    ensure!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).trim()
                == concat!("stratafetch-browser-host ", env!("CARGO_PKG_VERSION")),
        "The browser host needs updating. Install the full setup package rather than updating only the desktop executable"
    );
    let ids: Ids = serde_json::from_str(include_str!("../../../extensions/local-ids.json"))?;
    let (chrome, edge) = production_ids(
        &ids.chromium,
        include_str!("../../../extensions/store-ids.json"),
    )?;
    #[cfg(windows)]
    {
        crate::browser_windows::register(
            &crate::state_dir(),
            &binary,
            &[ids.chromium.clone(), chrome, edge],
            &ids.firefox,
        )?;
    }
    #[cfg(unix)]
    {
        use crate::browser_registration::{self, Browser as Registered, Paths};
        browser_registration::register(
            &Paths::current()?,
            &binary,
            std::collections::BTreeMap::from([
                (Registered::Chrome, chrome),
                (Registered::Chromium, ids.chromium.clone()),
                (Registered::Edge, edge),
                (Registered::Firefox, ids.firefox),
            ]),
        )?;
    }
    Ok(())
}

pub fn unregister_local() -> Result<()> {
    let binary = std::env::current_exe()?
        .parent()
        .context("Missing executable directory")?
        .join(if cfg!(windows) {
            "stratafetch-browser-host.exe"
        } else {
            "stratafetch-browser-host"
        });
    #[cfg(windows)]
    crate::browser_windows::unregister(&crate::state_dir(), &binary)?;
    #[cfg(unix)]
    crate::browser_registration::unregister(
        &crate::browser_registration::Paths::current()?,
        &binary,
    )?;
    Ok(())
}

pub fn setup(browser: Browser) -> Result<()> {
    register_local()?;
    let folder = extension_folder(browser)?;
    let executable = browser_executable(browser)
        .context("Browser not found. Install it, then use this setup button again")?;
    Command::new(executable)
        .arg(browser.page())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    open::that(folder)?;
    Ok(())
}

fn browser_executable(browser: Browser) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let relative = match browser {
            Browser::Chrome => "Google/Chrome/Application/chrome.exe",
            Browser::Edge => "Microsoft/Edge/Application/msedge.exe",
            Browser::Firefox => "Mozilla Firefox/firefox.exe",
        };
        ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
            .iter()
            .filter_map(std::env::var_os)
            .map(|root| PathBuf::from(root).join(relative))
            .find(|path| path.is_file())
    }
    #[cfg(target_os = "linux")]
    {
        let names: &[&str] = match browser {
            Browser::Chrome => &["google-chrome", "chromium", "chromium-browser"],
            Browser::Edge => &["microsoft-edge", "microsoft-edge-stable"],
            Browser::Firefox => &["firefox"],
        };
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .flat_map(|folder| names.iter().map(move |name| folder.join(name)))
            .find(|path| path.is_file())
    }
    #[cfg(target_os = "macos")]
    {
        let name = match browser {
            Browser::Chrome => "Google Chrome.app/Contents/MacOS/Google Chrome",
            Browser::Edge => "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            Browser::Firefox => "Firefox.app/Contents/MacOS/firefox",
        };
        [
            PathBuf::from("/Applications"),
            directories::BaseDirs::new()?
                .home_dir()
                .join("Applications"),
        ]
        .iter()
        .map(|root| root.join(name))
        .find(|path| path.is_file())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attention {
    pub id: Option<String>,
    pub setup: bool,
}
pub fn request_attention(directory: &Path, id: Option<String>, setup: bool) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, &Attention { id, setup })?;
    file.flush()?;
    file.persist(directory.join("desktop-attention.json"))
        .map_err(|error| error.error)?;
    Ok(())
}
pub fn take_attention(directory: &Path) -> Result<Option<Attention>> {
    let path = directory.join("desktop-attention.json");
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 4096, "Invalid desktop attention request");
    std::fs::remove_file(path)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
pub fn desktop_lock(directory: &Path, setup: bool) -> Result<Option<File>> {
    use fs2::FileExt;
    std::fs::create_dir_all(directory)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = options.open(directory.join("desktop.lockfile"))?;
    ensure!(file.metadata()?.is_file(), "Invalid desktop lock file");
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            file.metadata()?.file_attributes() & 0x400 == 0,
            "Invalid desktop lock reparse point"
        );
    }
    match file.try_lock_exclusive() {
        Ok(()) => {
            if setup {
                request_attention(directory, None, true)?;
            }
            Ok(Some(file))
        }
        Err(error)
            if error.kind() == std::io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
        {
            request_attention(directory, None, setup)?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
pub fn launch_desktop(setup: bool) -> Result<()> {
    let binary = std::env::current_exe()?
        .parent()
        .context("Missing executable directory")?
        .join(if cfg!(windows) {
            "stratafetch-desktop.exe"
        } else {
            "stratafetch-desktop"
        });
    let mut command = Command::new(binary);
    command
        .arg(if setup {
            "--browser-setup"
        } else {
            "--browser-handoff"
        })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn().context("Cannot open StrataFetch desktop")?;
    Ok(())
}

pub fn confirm_download(name: &str, source: &str, destination: &Path) -> Result<bool> {
    // Browser native hosts may start with SW_HIDE. A fresh GUI process gets its
    // own normal startup visibility, rather than hiding a modal in that host.
    let binary = std::env::current_exe()?
        .parent()
        .context("Missing executable directory")?
        .join(if cfg!(windows) {
            "stratafetch-desktop.exe"
        } else {
            "stratafetch-desktop"
        });
    let mut command = Command::new(binary);
    command
        .arg("--confirm-browser")
        .arg(name)
        .arg(source)
        .arg(destination)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let result = command
        .status()
        .context("Cannot open download confirmation")?;
    confirmation_result(result.code())
}

// A normal desktop launch exits with 0. That must never count as consent if an
// older or mismatched desktop does not understand the confirmation arguments.
pub const CONFIRM_ACCEPTED: i32 = 42;
pub const CONFIRM_DECLINED: i32 = 10;
fn confirmation_result(code: Option<i32>) -> Result<bool> {
    ensure!(
        matches!(code, Some(CONFIRM_ACCEPTED | CONFIRM_DECLINED)),
        "Download confirmation could not open; install matching browser components"
    );
    Ok(code == Some(CONFIRM_ACCEPTED))
}

pub fn confirmation_if_requested() -> Option<Result<bool>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--confirm-browser")) {
        return None;
    }
    Some((|| {
        let name = args.next().context("Missing download name")?;
        let source = args.next().context("Missing download source")?;
        let destination = args.next().context("Missing download directory")?;
        ensure!(args.next().is_none(), "Invalid confirmation arguments");
        crate::browser_confirmation::show(
            name.to_string_lossy().into(),
            source.to_string_lossy().into(),
            PathBuf::from(destination).display().to_string(),
        )
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_only_upgrade_cannot_silently_keep_older_browser_components() {
        let dir = tempfile::tempdir().unwrap();
        assert!(component_warning_at(dir.path()).is_none());
        std::fs::create_dir_all(dir.path().join("extensions/chromium")).unwrap();
        std::fs::write(dir.path().join("extensions/chromium/manifest.json"), "{}").unwrap();
        std::fs::write(dir.path().join("BUILD.txt"), "StrataFetch 0.1.0-beta.4\n").unwrap();
        assert!(component_warning_at(dir.path()).is_some());
        std::fs::write(
            dir.path().join("BUILD.txt"),
            concat!("StrataFetch ", env!("CARGO_PKG_VERSION"), "\n"),
        )
        .unwrap();
        assert!(component_warning_at(dir.path()).is_none());
    }
    #[test]
    fn production_ids_are_separate_and_local_ids_survive() {
        let local = "a".repeat(32);
        let chrome = "b".repeat(32);
        let edge = "c".repeat(32);
        assert_eq!(
            production_ids(&local, r#"{"chrome":null,"edge":null}"#).unwrap(),
            (local.clone(), local.clone())
        );
        let config = serde_json::json!({"chrome":{"id":chrome},"edge":{"id":edge}}).to_string();
        assert_eq!(production_ids(&local, &config).unwrap(), (chrome, edge));
        assert!(
            production_ids(
                &local,
                &serde_json::json!({"chrome":{"id":local},"edge":null}).to_string()
            )
            .is_err()
        );
        assert!(production_ids(&local, r#"{"chrome":{"id":"invalid"},"edge":null}"#).is_err());
    }
    #[test]
    fn only_explicit_confirmation_counts_as_approval() {
        assert!(confirmation_result(Some(CONFIRM_ACCEPTED)).unwrap());
        assert!(!confirmation_result(Some(CONFIRM_DECLINED)).unwrap());
        for code in [None, Some(0), Some(1), Some(2)] {
            assert!(confirmation_result(code).is_err());
        }
    }
    #[test]
    fn second_desktop_focuses_the_existing_window_and_lock_is_reusable() {
        let dir = tempfile::tempdir().unwrap();
        let first = desktop_lock(dir.path(), false).unwrap().unwrap();
        assert!(desktop_lock(dir.path(), true).unwrap().is_none());
        assert!(take_attention(dir.path()).unwrap().unwrap().setup);
        assert!(take_attention(dir.path()).unwrap().is_none());
        request_attention(dir.path(), Some("download-id".into()), false).unwrap();
        assert_eq!(
            take_attention(dir.path()).unwrap().unwrap().id.as_deref(),
            Some("download-id")
        );
        drop(first);
        assert!(desktop_lock(dir.path(), false).unwrap().is_some());
    }
}
