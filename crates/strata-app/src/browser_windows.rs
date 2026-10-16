//! Windows native-messaging registration without PowerShell or policy changes.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use windows_sys::Win32::System::Registry::*;

const HOST: &str = "org.stratafetch.host";
fn roots() -> Vec<String> {
    ["Google\\Chrome", "Microsoft\\Edge", "Mozilla"]
        .iter()
        .map(|browser| format!("Software\\{browser}\\NativeMessagingHosts\\{HOST}"))
        .collect()
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn status(code: u32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(code as i32).into())
    }
}
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn get_value(path: &str) -> Result<Option<PathBuf>> {
    let mut key = std::ptr::null_mut();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            KEY_READ | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if result == 2 {
        return Ok(None);
    }
    status(result)?;
    let key = Key(key);
    let mut bytes = 0;
    let mut kind = 0;
    let result = unsafe {
        RegQueryValueExW(
            key.0,
            std::ptr::null(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if result == 2 {
        return Ok(None);
    }
    status(result)?;
    ensure!(
        kind == REG_SZ && bytes <= 64 * 1024 && bytes.is_multiple_of(2),
        "Invalid native-host registry value"
    );
    let mut buffer = vec![0u16; bytes as usize / 2 + 1];
    status(unsafe {
        RegQueryValueExW(
            key.0,
            std::ptr::null(),
            std::ptr::null(),
            &mut kind,
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    })?;
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    Ok(Some(String::from_utf16(&buffer[..length])?.into()))
}
fn set_value(path: &str, value: &Path) -> Result<()> {
    let mut key = std::ptr::null_mut();
    status(unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_WOW64_64KEY,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    })?;
    let key = Key(key);
    let data = wide(
        value
            .to_str()
            .context("Native host path is not valid Unicode")?,
    );
    status(unsafe {
        RegSetValueExW(
            key.0,
            std::ptr::null(),
            0,
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * 2) as u32,
        )
    })
}
fn delete_key(path: &str) -> Result<()> {
    let code =
        unsafe { RegDeleteKeyExW(HKEY_CURRENT_USER, wide(path).as_ptr(), KEY_WOW64_64KEY, 0) };
    if code == 2 { Ok(()) } else { status(code) }
}
fn normal_path(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    let text = path
        .to_str()
        .context("Native host path is not valid Unicode")?;
    Ok(if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}").into()
    } else {
        text.strip_prefix("\\\\?\\").unwrap_or(text).into()
    })
}
fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    use std::os::windows::fs::MetadataExt;
    ensure!(
        metadata.is_file()
            && metadata.file_attributes() & 0x400 == 0
            && metadata.len() <= 128 * 1024,
        "Invalid browser registration file"
    );
    Ok(Some(std::fs::read(path)?))
}
fn json_file(path: &Path, value: &impl Serialize) -> Result<()> {
    if path.exists() {
        read(path)?;
    }
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing manifest directory")?)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.flush()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct Record {
    binary: PathBuf,
    managed: Vec<String>,
    baseline: Option<Vec<String>>,
}
fn origins(state: &Path) -> Result<Vec<String>> {
    read(&state.join("browser-origins.json"))?
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()
        .map(|list| list.unwrap_or_default())
        .map_err(Into::into)
}
fn checked_manifests(keys: &[String], binary: &Path) -> Result<Vec<String>> {
    let mut managed = Vec::new();
    for key in keys {
        if let Some(path) = get_value(key)? {
            let bytes = read(&path)?.context("Native-host registration points to a missing manifest; existing registration preserved")?;
            let manifest: Value = serde_json::from_slice(&bytes)?;
            let previous = PathBuf::from(
                manifest["path"]
                    .as_str()
                    .context("Invalid native-host manifest path")?,
            );
            ensure!(
                manifest["name"] == HOST && normal_path(&previous)? == binary,
                "Another installation owns browser registration; unregister it first"
            );
            for field in ["allowed_origins", "allowed_extensions"] {
                managed.extend(
                    manifest[field]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            }
        }
    }
    Ok(managed)
}
pub fn register(state: &Path, binary: &Path, chrome: &[String], firefox: &str) -> Result<()> {
    register_in(state, binary, chrome, firefox, &roots())
}
fn register_in(
    state: &Path,
    binary: &Path,
    chrome: &[String],
    firefox: &str,
    keys: &[String],
) -> Result<()> {
    ensure!(
        !chrome.is_empty()
            && chrome.len() <= 3
            && chrome
                .iter()
                .all(|id| id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c)))
            && firefox == "stratafetch@local",
        "Invalid bundled browser IDs"
    );
    std::fs::create_dir_all(state)?;
    let binary = normal_path(binary)?;
    checked_manifests(keys, &binary)?;
    let record_path = state.join("browser-registration-windows.json");
    let old = read(&record_path)?
        .map(|bytes| serde_json::from_slice::<Record>(&bytes))
        .transpose()?;
    let mut baseline = origins(state)?;
    if let Some(record) = &old {
        ensure!(
            normal_path(&record.binary)? == binary,
            "Another installation owns the native bridge"
        );
        baseline.retain(|origin| !record.managed.contains(origin));
        baseline.extend(record.baseline.clone().unwrap_or_default());
    }
    baseline.sort();
    baseline.dedup();
    let mut managed: Vec<_> = chrome
        .iter()
        .map(|id| format!("chrome-extension://{id}/"))
        .collect();
    managed.push(firefox.into());
    managed.sort();
    managed.dedup();
    let mut all = baseline.clone();
    all.extend(managed.clone());
    all.sort();
    all.dedup();
    let chromium = state.join("chromium-host.json");
    let firefox_path = state.join("firefox-host.json");
    let allowed: Vec<_> = all
        .iter()
        .filter(|origin| origin.starts_with("chrome-extension://") && origin.ends_with('/'))
        .collect();
    json_file(
        &chromium,
        &json!({"name":HOST,"description":"StrataFetch native download bridge","path":binary,"type":"stdio","allowed_origins":allowed}),
    )?;
    json_file(
        &firefox_path,
        &json!({"name":HOST,"description":"StrataFetch native download bridge","path":binary,"type":"stdio","allowed_extensions":[firefox]}),
    )?;
    json_file(&state.join("browser-origins.json"), &all)?;
    json_file(
        &record_path,
        &Record {
            binary,
            managed,
            baseline: Some(baseline),
        },
    )?;
    for (index, key) in keys.iter().enumerate() {
        set_value(key, if index == 2 { &firefox_path } else { &chromium })?;
    }
    Ok(())
}
pub fn unregister(state: &Path, binary: &Path) -> Result<()> {
    unregister_in(state, binary, &roots())
}
fn unregister_in(state: &Path, binary: &Path, keys: &[String]) -> Result<()> {
    let binary = normal_path(binary)?;
    let mut managed = checked_manifests(keys, &binary)?;
    let record_path = state.join("browser-registration-windows.json");
    let record = read(&record_path)?
        .map(|bytes| serde_json::from_slice::<Record>(&bytes))
        .transpose()?;
    let mut remaining = origins(state)?;
    if let Some(record) = record {
        ensure!(
            normal_path(&record.binary)? == binary,
            "Another installation owns the native bridge"
        );
        managed.extend(record.managed);
        remaining.retain(|origin| !managed.contains(origin));
        remaining.extend(record.baseline.unwrap_or_default());
    } else {
        remaining.retain(|origin| !managed.contains(origin));
    }
    remaining.sort();
    remaining.dedup();
    for key in keys {
        delete_key(key)?;
    }
    for name in [
        "chromium-host.json",
        "firefox-host.json",
        "browser-registration-windows.json",
    ] {
        let path = state.join(name);
        if read(&path)?.is_some() {
            std::fs::remove_file(path)?;
        }
    }
    let origins_path = state.join("browser-origins.json");
    if remaining.is_empty() {
        if read(&origins_path)?.is_some() {
            std::fs::remove_file(origins_path)?;
        }
    } else {
        json_file(&origins_path, &remaining)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_registration_roundtrip_preserves_other_origins_and_refuses_other_installations() {
        let temp = tempfile::tempdir().unwrap();
        let state = temp.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let binary = temp.path().join("stratafetch-browser-host.exe");
        std::fs::write(&binary, b"test fixture").unwrap();
        let root = format!(
            "Software\\StrataFetch-Test-{}",
            temp.path().file_name().unwrap().to_string_lossy()
        );
        let keys: Vec<_> = ["Chrome", "Edge", "Firefox"]
            .iter()
            .map(|browser| format!("{root}\\{browser}"))
            .collect();
        assert!(keys.iter().all(|key| get_value(key).unwrap().is_none()));
        json_file(&state.join("browser-origins.json"), &vec!["manual-origin"]).unwrap();
        std::fs::write(state.join("history-marker"), b"preserved").unwrap();
        let chrome = vec!["a".repeat(32), "b".repeat(32), "c".repeat(32)];
        register_in(&state, &binary, &chrome, "stratafetch@local", &keys).unwrap();
        register_in(&state, &binary, &chrome, "stratafetch@local", &keys).unwrap();
        assert_eq!(origins(&state).unwrap().len(), 5);
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(state.join("chromium-host.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["allowed_origins"].as_array().unwrap().len(), 3);
        let other = temp.path().join("other.exe");
        std::fs::write(&other, b"other").unwrap();
        assert!(unregister_in(&state, &other, &keys).is_err());
        assert!(get_value(&keys[0]).unwrap().is_some());
        unregister_in(&state, &binary, &keys).unwrap();
        assert_eq!(origins(&state).unwrap(), vec!["manual-origin"]);
        assert!(keys.iter().all(|key| get_value(key).unwrap().is_none()));
        assert_eq!(
            std::fs::read(state.join("history-marker")).unwrap(),
            b"preserved"
        );
        delete_key(&root).unwrap();
    }
}
