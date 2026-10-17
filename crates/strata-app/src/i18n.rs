//! Shared desktop/native language choice. Browser companions receive it on ping.
use std::{collections::HashMap, sync::{OnceLock, atomic::{AtomicBool, AtomicU8, Ordering}}};
use anyhow::Result;

fn catalogue() -> &'static HashMap<String, String> {
    static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();
    STRINGS.get_or_init(|| serde_json::from_str(include_str!("../../../assets/locales/sv.json")).expect("validated Swedish catalogue"))
}
pub fn text(swedish: bool, value: &str) -> &str {
    if swedish { catalogue().get(value).map(String::as_str).unwrap_or(value) } else { value }
}
pub fn preferred_swedish() -> bool {
    if let Ok(language) = std::env::var("STRATAFETCH_LANGUAGE") {
        if matches!(language.as_str(), "sv" | "en") { return language == "sv"; }
    }
    if let Ok(file) = std::fs::File::open(crate::state_dir().join("language.json")) {
        use std::io::Read;
        if let Ok(value) = serde_json::from_reader::<_, serde_json::Value>(file.take(256)) {
            if let Some(language) = value["language"].as_str() { return language == "sv"; }
        }
    }
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" { fn GetUserDefaultUILanguage() -> u16; }
        // GetUserDefaultUILanguage has no pointers or mutable arguments.
        return unsafe { GetUserDefaultUILanguage() } & 0x3ff == 0x1d;
    }
    #[cfg(not(windows))]
    ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(|key| std::env::var(key).ok().filter(|v| !v.is_empty())).is_some_and(|v| v.to_lowercase().starts_with("sv"))
}
fn choice() -> &'static AtomicBool {
    static CHOICE: OnceLock<AtomicBool> = OnceLock::new();
    CHOICE.get_or_init(|| AtomicBool::new(preferred_swedish()))
}
pub fn swedish() -> bool { choice().load(Ordering::Acquire) }
pub fn tr(value: &str) -> &str { text(swedish(), value) }
/// Translate known application errors, preserving verbatim server/OS diagnostics.
pub fn message(value: &str) -> String {
    value.split(": ").map(tr).collect::<Vec<_>>().join(": ")
}
pub fn language() -> &'static str { if swedish() { "sv" } else { "en" } }
pub fn set_swedish(value: bool) { choice().store(value, Ordering::Release); }
pub fn save_preference(value: bool) -> Result<()> {
    static SAVED: AtomicU8 = AtomicU8::new(2);
    let encoded = u8::from(value);
    if SAVED.load(Ordering::Acquire) == encoded { return Ok(()); }
    // Manager creates the private state directory. Never create it with looser permissions.
    let directory = crate::state_dir();
    let mut file = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer(file.as_file_mut(), &serde_json::json!({"language":if value {"sv"} else {"en"}}))?;
    file.as_file().sync_all()?;
    file.persist(directory.join("language.json"))?;
    #[cfg(windows)]
    if std::env::var_os("STRATAFETCH_STATE_DIR").is_none() {
        save_windows_language(value)?;
    }
    SAVED.store(encoded, Ordering::Release);
    Ok(())
}

#[cfg(windows)]
fn save_windows_language(swedish: bool) -> Result<()> {
    use windows_sys::Win32::System::Registry::*;
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let path = wide("Software\\StrataFetch");
    let name = wide("Language");
    let value = wide(if swedish { "sv" } else { "en" });
    let mut key = std::ptr::null_mut();
    // App-owned, per-user preference only. Isolated test profiles never write it.
    let opened = unsafe { RegCreateKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, std::ptr::null(), 0, KEY_SET_VALUE, std::ptr::null(), &mut key, std::ptr::null_mut()) };
    anyhow::ensure!(opened == 0, "Cannot save installer language ({opened})");
    let saved = unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, value.as_ptr().cast(), (value.len()*2) as u32) };
    unsafe { RegCloseKey(key); }
    anyhow::ensure!(saved == 0, "Cannot save installer language ({saved})");
    Ok(())
}

#[macro_export]
macro_rules! localized_format {
    ($english:literal, $swedish:literal $(,)?) => {
        if $crate::i18n::swedish() { format!($swedish) } else { format!($english) }
    };
    ($english:literal, $swedish:literal, $($arguments:tt)*) => {
        if $crate::i18n::swedish() { format!($swedish, $($arguments)*) } else { format!($english, $($arguments)*) }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swedish_catalogue_covers_core_and_advanced_surfaces_and_preserves_user_data() {
        for key in ["Settings", "Download later", "Speed limiter", "Repair browser components…", "Download with StrataFetch", "Capture filters"] {
            assert_ne!(text(true, key), key, "Missing Swedish translation: {key}");
            assert_eq!(text(false, key), key);
        }
        assert_eq!(text(true, "C:\\Downloads\\My File.iso"), "C:\\Downloads\\My File.iso");
        assert_eq!(text(true, "https://example.org/file?token=private"), "https://example.org/file?token=private");
    }
}
