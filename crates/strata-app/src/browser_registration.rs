//! Per-user native messaging registration for Linux and macOS.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const HOST: &str = "org.stratafetch.host";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Chrome,
    Chromium,
    Edge,
    Firefox,
}
#[derive(Clone, Copy)]
pub enum Platform {
    Linux,
    Macos,
}
pub struct Paths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub state: PathBuf,
    pub platform: Platform,
}
impl Paths {
    pub fn current() -> Result<Self> {
        let base = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("User directories unavailable"))?;
        Ok(Self {
            home: base.home_dir().into(),
            config: base.config_dir().into(),
            state: crate::state_dir(),
            platform: if cfg!(target_os = "macos") {
                Platform::Macos
            } else {
                Platform::Linux
            },
        })
    }
    pub fn manifest(&self, browser: Browser) -> PathBuf {
        let folder = match self.platform {
            Platform::Linux => match browser {
                Browser::Chrome => self.config.join("google-chrome/NativeMessagingHosts"),
                Browser::Chromium => self.config.join("chromium/NativeMessagingHosts"),
                Browser::Edge => self.config.join("microsoft-edge/NativeMessagingHosts"),
                Browser::Firefox => self.home.join(".mozilla/native-messaging-hosts"),
            },
            Platform::Macos => self
                .home
                .join("Library/Application Support")
                .join(match browser {
                    Browser::Chrome => "Google/Chrome/NativeMessagingHosts",
                    Browser::Chromium => "Chromium/NativeMessagingHosts",
                    Browser::Edge => "Microsoft Edge/NativeMessagingHosts",
                    Browser::Firefox => "Mozilla/NativeMessagingHosts",
                }),
        };
        folder.join(format!("{HOST}.json"))
    }
}
#[derive(Serialize, Deserialize)]
struct Record {
    binary: PathBuf,
    browsers: BTreeMap<Browser, String>,
    baseline_origins: Vec<String>,
}
fn origins(browsers: &BTreeMap<Browser, String>) -> Vec<String> {
    browsers
        .iter()
        .map(|(browser, id)| {
            if *browser == Browser::Firefox {
                id.clone()
            } else {
                format!("chrome-extension://{id}/")
            }
        })
        .collect()
}
fn existing(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "Refusing non-regular registration file: {}",
                path.display()
            );
            Ok(Some(std::fs::read(path)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn validate_manifest(path: &Path, binary: &Path) -> Result<()> {
    if let Some(bytes) = existing(path)? {
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        ensure!(
            value["name"] == HOST && value["path"].as_str() == binary.to_str(),
            "Another installation owns {}; registration preserved",
            path.display()
        );
    }
    Ok(())
}
fn load_origins(path: &Path) -> Result<Vec<String>> {
    existing(path)?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()
        .map(|list| list.unwrap_or_default())
        .map_err(Into::into)
}
fn merge_origins(mut current: Vec<String>, record: &Record, remove: &[String]) -> Vec<String> {
    current.retain(|origin| !remove.contains(origin));
    current.extend(record.baseline_origins.iter().cloned());
    current.extend(origins(&record.browsers));
    current.sort();
    current.dedup();
    current
}
pub fn register(paths: &Paths, binary: &Path, browsers: BTreeMap<Browser, String>) -> Result<()> {
    ensure!(
        !browsers.is_empty(),
        "Select at least one browser using --chrome-id, --chromium-id, --edge-id or --firefox"
    );
    for (browser, id) in &browsers {
        ensure!(
            if *browser == Browser::Firefox {
                id == "stratafetch@local"
            } else {
                id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c))
            },
            "Invalid extension ID for {browser:?}"
        );
    }
    let binary = binary.canonicalize()?;
    let record_path = paths.state.join("browser-registration.json");
    let origins_path = paths.state.join("browser-origins.json");
    let current_origins = load_origins(&origins_path)?;
    let mut record = if let Some(bytes) = existing(&record_path)? {
        serde_json::from_slice::<Record>(&bytes)?
    } else {
        Record {
            binary: binary.clone(),
            browsers: BTreeMap::new(),
            baseline_origins: current_origins.clone(),
        }
    };
    ensure!(
        record.binary == binary,
        "Another installation owns the browser registration; unregister it before moving the native host"
    );
    let previous_origins = origins(&record.browsers);
    for browser in record.browsers.keys().chain(browsers.keys()) {
        validate_manifest(&paths.manifest(*browser), &binary)?;
    }
    record.browsers.extend(browsers.clone());
    let allowed: Vec<_> = origins(&record.browsers)
        .into_iter()
        .filter(|origin| origin.starts_with("chrome-extension://"))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    // Each installed Chromium-family browser accepts both local development
    // and explicitly configured store identities. Refresh existing manifests
    // as well when another store identity is added.
    for (browser, id) in &record.browsers {
        let mut manifest = serde_json::json!({ "name": HOST, "description": "StrataFetch native download bridge", "path": binary, "type": "stdio" });
        if *browser == Browser::Firefox {
            manifest["allowed_extensions"] = serde_json::json!([id]);
        } else {
            manifest["allowed_origins"] = serde_json::json!(allowed);
        }
        crate::unix_files::write_atomic(
            &paths.manifest(*browser),
            &serde_json::to_vec_pretty(&manifest)?,
        )?;
    }
    let merged = merge_origins(current_origins, &record, &previous_origins);
    crate::unix_files::write_atomic(&origins_path, &serde_json::to_vec_pretty(&merged)?)?;
    crate::unix_files::write_atomic(&record_path, &serde_json::to_vec_pretty(&record)?)?;
    Ok(())
}
pub fn unregister(paths: &Paths, binary: &Path) -> Result<()> {
    let record_path = paths.state.join("browser-registration.json");
    let Some(bytes) = existing(&record_path)? else {
        anyhow::bail!("No managed browser registration found; existing files were preserved");
    };
    let mut record: Record = serde_json::from_slice(&bytes)?;
    ensure!(
        record.binary == binary.canonicalize()?,
        "Another installation owns browser registration; files preserved"
    );
    for browser in record.browsers.keys() {
        validate_manifest(&paths.manifest(*browser), &record.binary)?;
    }
    let origins_path = paths.state.join("browser-origins.json");
    let managed = origins(&record.browsers);
    let current = load_origins(&origins_path)?;
    for browser in record.browsers.keys() {
        let path = paths.manifest(*browser);
        if existing(&path)?.is_some() {
            std::fs::remove_file(path)?;
        }
    }
    record.browsers.clear();
    let remaining = merge_origins(current, &record, &managed);
    crate::unix_files::write_atomic(&origins_path, &serde_json::to_vec_pretty(&remaining)?)?;
    std::fs::remove_file(record_path)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(dir: &Path) -> (Paths, PathBuf) {
        let binary = dir.join("stratafetch-browser-host");
        std::fs::write(&binary, b"fixture").unwrap();
        (
            Paths {
                home: dir.join("home"),
                config: dir.join("config"),
                state: dir.join("state"),
                platform: Platform::Linux,
            },
            binary,
        )
    }
    #[test]
    fn store_and_local_chromium_origins_work_in_every_registered_browser() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, binary) = fixture(dir.path());
        register(
            &paths,
            &binary,
            BTreeMap::from([
                (Browser::Chrome, "b".repeat(32)),
                (Browser::Chromium, "a".repeat(32)),
                (Browser::Edge, "c".repeat(32)),
                (Browser::Firefox, "stratafetch@local".into()),
            ]),
        )
        .unwrap();
        let allowed: Vec<_> = ["a", "b", "c"]
            .map(|id| format!("chrome-extension://{}/", id.repeat(32)))
            .into();
        for browser in [Browser::Chrome, Browser::Chromium, Browser::Edge] {
            let manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(paths.manifest(browser)).unwrap()).unwrap();
            assert_eq!(manifest["allowed_origins"], serde_json::json!(allowed));
        }
        assert_eq!(
            load_origins(&paths.state.join("browser-origins.json"))
                .unwrap()
                .len(),
            4
        );
    }
    #[test]
    fn register_update_and_uninstall_preserve_history_and_manual_origins() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, binary) = fixture(dir.path());
        std::fs::create_dir_all(&paths.state).unwrap();
        std::fs::write(paths.state.join("history.sqlite"), b"history").unwrap();
        std::fs::write(
            paths.state.join("browser-origins.json"),
            br#"["manual-origin"]"#,
        )
        .unwrap();
        let first = "a".repeat(32);
        let second = "b".repeat(32);
        register(
            &paths,
            &binary,
            BTreeMap::from([
                (Browser::Chrome, first.clone()),
                (Browser::Firefox, "stratafetch@local".into()),
            ]),
        )
        .unwrap();
        register(
            &paths,
            &binary,
            BTreeMap::from([(Browser::Chrome, second.clone())]),
        )
        .unwrap();
        let list = load_origins(&paths.state.join("browser-origins.json")).unwrap();
        assert!(!list.contains(&format!("chrome-extension://{first}/")));
        assert!(list.contains(&format!("chrome-extension://{second}/")));
        assert!(list.contains(&"stratafetch@local".into()));
        unregister(&paths, &binary).unwrap();
        assert_eq!(
            load_origins(&paths.state.join("browser-origins.json")).unwrap(),
            vec!["manual-origin"]
        );
        assert_eq!(
            std::fs::read(paths.state.join("history.sqlite")).unwrap(),
            b"history"
        );
        assert!(!paths.manifest(Browser::Chrome).exists());
        assert!(!paths.manifest(Browser::Firefox).exists());
    }
    #[test]
    fn registration_refuses_foreign_and_symlink_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, binary) = fixture(dir.path());
        let path = paths.manifest(Browser::Chrome);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let foreign = br#"{"name":"org.stratafetch.host","path":"/another/installation"}"#;
        std::fs::write(&path, foreign).unwrap();
        let browsers = BTreeMap::from([(Browser::Chrome, "a".repeat(32))]);
        assert!(register(&paths, &binary, browsers.clone()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), foreign);
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&binary, &path).unwrap();
        assert!(register(&paths, &binary, browsers).is_err());
        assert_eq!(std::fs::read(&binary).unwrap(), b"fixture");
    }
    #[test]
    fn macos_browser_paths_include_application_support() {
        let dir = tempfile::tempdir().unwrap();
        let (mut paths, _) = fixture(dir.path());
        paths.platform = Platform::Macos;
        assert_eq!(paths.manifest(Browser::Chrome), paths.home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts/org.stratafetch.host.json"));
        assert_eq!(
            paths.manifest(Browser::Firefox),
            paths.home.join(
                "Library/Application Support/Mozilla/NativeMessagingHosts/org.stratafetch.host.json"
            )
        );
    }
}
