//! Explicit, per-user Linux desktop-menu installation; no autostart or URL interception.
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
const ID: &str = "org.stratafetch.StrataFetch";
const ICON: &[u8] = include_bytes!("../../../assets/stratafetch.svg");
fn escape_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
fn entry(binary: &Path, icon: &Path) -> Result<String> {
    let binary = binary
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Desktop executable path must be UTF-8"))?;
    ensure!(
        !binary.chars().any(char::is_control),
        "Executable path contains control characters"
    );
    let quoted = binary
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=StrataFetch\nComment=Local HTTP download manager\nExec=\"{}\"\nTryExec={}\nIcon={}\nTerminal=false\nCategories=Network;FileTransfer;\nStartupNotify=true\nStartupWMClass={ID}\n",
        escape_value(&quoted),
        escape_value(binary),
        escape_value(&icon.to_string_lossy())
    ))
}
fn check_owned(path: &Path, content: &[u8]) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file()
                    && !metadata.file_type().is_symlink()
                    && std::fs::read(path)? == content,
                "Existing desktop integration differs; file preserved: {}",
                path.display()
            );
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}
pub fn install(data: &Path, desktop: &Path, uninstall: bool) -> Result<PathBuf> {
    let desktop = desktop.canonicalize()?;
    let icon = data.join(format!("icons/hicolor/scalable/apps/{ID}.svg"));
    let launcher = data.join(format!("applications/{ID}.desktop"));
    let content = entry(&desktop, &icon)?;
    // Validate both before changing either; preserve another installation/user edits.
    let existing_launcher = check_owned(&launcher, content.as_bytes())?;
    let existing_icon = check_owned(&icon, ICON)?;
    if uninstall {
        if existing_launcher {
            std::fs::remove_file(&launcher)?;
        }
        if existing_icon {
            std::fs::remove_file(&icon)?;
        }
    } else {
        crate::unix_files::write_atomic(&icon, ICON)?;
        crate::unix_files::write_atomic(&launcher, content.as_bytes())?;
    }
    Ok(launcher)
}
pub fn current(uninstall: bool) -> Result<()> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("User data directory unavailable"))?;
    let desktop = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("stratafetch-desktop");
    let path = install(base.data_dir(), &desktop, uninstall)?;
    println!(
        "Desktop menu entry {}: {}. Download history was preserved.",
        if uninstall { "removed" } else { "installed" },
        path.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_handles_spaced_paths_and_preserves_user_edits() {
        let dir = tempfile::tempdir().unwrap();
        let desktop = dir.path().join("app with spaces");
        std::fs::write(&desktop, b"fixture").unwrap();
        let data = dir.path().join("data");
        let launcher = install(&data, &desktop, false).unwrap();
        let content = std::fs::read_to_string(&launcher).unwrap();
        assert!(content.contains(&format!("Exec=\"{}\"", desktop.display())));
        install(&data, &desktop, false).unwrap();
        std::fs::write(&launcher, b"user edited file").unwrap();
        assert!(install(&data, &desktop, true).is_err());
        assert_eq!(std::fs::read(&launcher).unwrap(), b"user edited file");
        std::fs::write(&launcher, content).unwrap();
        install(&data, &desktop, true).unwrap();
        assert!(!launcher.exists());
        assert!(desktop.exists());
    }
    #[test]
    fn exec_escapes_desktop_field_codes_and_shell_metacharacters() {
        let value = entry(
            Path::new("/app/100% $special`\"name"),
            Path::new("/icons/icon.svg"),
        )
        .unwrap();
        assert!(value.contains("100%% \\\\$special\\\\`\\\\\"name"));
    }
}
