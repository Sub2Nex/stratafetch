use eframe::egui;

use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

use strata_core::{Job, Settings, Status};

/// Core interface labels; advanced explanations currently remain in English.
pub fn text(swedish: bool, label: &str) -> &str {
    if !swedish {
        return label;
    }
    match label {
        "File" => "Arkiv",
        "Downloads" => "Hämtningar",
        "Tools" => "Verktyg",
        "View" => "Visa",
        "Add URL" => "Lägg till URL",
        "Resume" => "Återuppta",
        "Pause" => "Pausa",
        "Stop" => "Stoppa",
        "Stop all" => "Stoppa alla",
        "Remove" => "Ta bort",
        "Clear finished" => "Rensa färdiga",
        "Scheduler" => "Schemaläggare",
        "Start queue" => "Starta kö",
        "Stop queue" => "Stoppa kö",
        "Settings" => "Inställningar",
        "Search downloads" => "Sök hämtningar",
        "All downloads" => "Alla hämtningar",
        "Unfinished" => "Ofärdiga",
        "All unfinished" => "Alla ofärdiga",
        "Finished" => "Färdiga",
        "Downloading" => "Hämtar",
        "Queued" => "I kö",
        "Paused" => "Pausad",
        "Failed" => "Misslyckad",
        "Stopped" => "Stoppad",
        "Complete" => "Färdig",
        "Categories" => "Kategorier",
        "Queues" => "Köer",
        "General" => "Allmänt",
        "Compressed" => "Komprimerat",
        "Documents" => "Dokument",
        "Music" => "Musik",
        "Programs" => "Program",
        "Images" => "Bilder",
        "Main queue" => "Huvudkö",
        "Scheduled" => "Schemalagda",
        "Name" => "Namn",
        "Size" => "Storlek",
        "Progress" => "Förlopp",
        "Speed" => "Hastighet",
        "Time left" => "Tid kvar",
        "Connections" => "Anslutningar",
        "Added (UTC)" => "Tillagd (UTC)",
        "Category" => "Kategori",
        "Source" => "Källa",
        "Downloaded" => "Hämtat",
        _ => label,
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Column {
    Name,

    Size,

    Status,

    Progress,

    Speed,

    Eta,

    Connections,

    Added,

    Category,

    Source,

    Downloaded,
}

impl Column {
    pub const ALL: [Self; 11] = [
        Self::Name,
        Self::Size,
        Self::Status,
        Self::Progress,
        Self::Speed,
        Self::Eta,
        Self::Connections,
        Self::Added,
        Self::Category,
        Self::Source,
        Self::Downloaded,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Name => "Name",

            Self::Size => "Size",

            Self::Status => "Status",

            Self::Progress => "Progress",

            Self::Speed => "Speed",

            Self::Eta => "Time left",

            Self::Connections => "Connections",

            Self::Added => "Added (UTC)",

            Self::Category => "Category",

            Self::Source => "Source",

            Self::Downloaded => "Downloaded",
        }
    }

    pub fn width(self) -> f32 {
        match self {
            Self::Name => 250.0,

            Self::Progress => 120.0,

            Self::Added => 142.0,

            Self::Source => 170.0,

            Self::Status => 100.0,

            _ => 88.0,
        }
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Completion {
    #[default]
    None,

    Message,

    OpenFolder,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub folder: String,

    pub dark: bool,

    pub swedish: bool,

    pub sidebar: bool,

    pub open_completed: bool,

    pub columns: Vec<Column>,

    pub descriptions: HashMap<String, String>,

    pub completion: HashMap<String, Completion>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            folder: strata_app::download_dir().to_string_lossy().into(),

            dark: true,

            swedish: false,

            sidebar: true,

            open_completed: false,

            columns: Column::ALL[..8].to_vec(),

            descriptions: Default::default(),

            completion: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Filter {
    All,

    Unfinished,

    Status(Status),

    Category(String),

    Queue,

    Scheduled,
}

impl Filter {
    pub fn matches(&self, j: &Job) -> bool {
        match self {
            Self::All => true,

            Self::Unfinished => j.status != Status::Completed,

            Self::Status(s) => j.status == *s,

            Self::Category(c) => j.category == *c,

            Self::Queue => matches!(j.status, Status::Queued | Status::Paused),

            Self::Scheduled => j.scheduled_at.is_some() && j.status != Status::Completed,
        }
    }
}

#[derive(Default)]
pub struct Selection {
    pub ids: HashSet<String>,

    pub anchor: Option<String>,

    pub cursor: Option<String>,
}

impl Selection {
    pub fn click(&mut self, id: &str, ordered: &[&str], ctrl: bool, shift: bool) {
        if shift
            && let (Some(a), Some(b)) = (
                self.anchor
                    .as_ref()
                    .and_then(|a| ordered.iter().position(|id| *id == a)),
                ordered.iter().position(|other| *other == id),
            )
        {
            if !ctrl {
                self.ids.clear();
            }

            for id in &ordered[a.min(b)..=a.max(b)] {
                self.ids.insert((*id).to_owned());
            }
        } else if ctrl {
            if !self.ids.remove(id) {
                self.ids.insert(id.to_owned());
            }

            self.anchor = Some(id.to_owned());
        } else {
            self.ids.clear();

            self.ids.insert(id.to_owned());

            self.anchor = Some(id.to_owned());
        }

        self.cursor = Some(id.to_owned());
    }

    pub fn all(&mut self, ordered: &[&str]) {
        self.ids = ordered.iter().map(|id| (*id).to_owned()).collect();

        self.cursor = ordered.first().map(|id| (*id).to_owned());

        self.anchor = self.cursor.clone();
    }

    pub fn retain(&mut self, jobs: &[Job]) {
        let existing: HashSet<_> = jobs.iter().map(|j| j.id.as_str()).collect();

        self.ids.retain(|id| existing.contains(id.as_str()));

        if self
            .cursor
            .as_ref()
            .is_some_and(|id| !existing.contains(id.as_str()))
        {
            self.cursor = None;
        }
    }
}

#[derive(Default)]
pub struct AddDialog {
    pub form_id: u64,
    pub probe_id: u64,
    pub open: bool,
    pub focus_url: bool,

    pub batch: bool,

    pub url: String,

    pub filename: String,

    pub folder: String,

    pub category: String,

    pub description: String,

    pub metadata: Option<strata_core::metadata::Metadata>,

    pub probing: bool,

    pub probe_url: String,

    pub error: Option<String>,
}

pub struct TransferDraft {
    pub id: String,

    pub rate: u64,

    pub priority: i32,

    pub category: String,

    pub delay_minutes: u64,
}

#[derive(Clone, Copy)]
pub enum FolderTarget {
    Add(u64),
    Preferences(u64),
}

pub struct State {
    pub prefs: Preferences,

    pub selection: Selection,

    pub filter: Filter,

    pub search: String,

    pub search_focused: bool,

    pub sort: Column,

    pub descending: bool,

    pub visible: Vec<usize>,

    pub dirty: bool,

    pub scroll_to: Option<usize>,

    pub add: AddDialog,

    pub transfer: Option<String>,

    pub transfer_tab: usize,

    pub draft: Option<TransferDraft>,

    pub settings_open: bool,
    pub settings_id: u64,
    pub choose_preferences_folder: bool,

    pub settings_page: usize,

    pub settings: Settings,

    pub scheduler_open: bool,

    pub statistics_open: bool,

    pub columns_open: bool,

    pub remove: Option<Vec<String>>,

    pub message: Option<String>,

    pub history_path: String,
}

impl State {
    /// Ignore a late native-dialog result after closing the form or editing its path.
    pub fn apply_folder(&mut self, target: FolderTarget, initial: &str, path: std::path::PathBuf) {
        let (open, field) = match target {
            FolderTarget::Add(id) => (
                self.add.open && id == self.add.form_id,
                &mut self.add.folder,
            ),
            FolderTarget::Preferences(id) => (
                self.settings_open && id == self.settings_id,
                &mut self.prefs.folder,
            ),
        };
        if open && field == initial {
            if let Some(path) = path.to_str() {
                *field = path.to_owned();
            } else {
                self.message = Some("This directory path is not valid UTF-8. Choose another directory or enter a supported path.".into());
            }
        }
    }
    pub fn new(mut prefs: Preferences) -> Self {
        let mut columns = HashSet::new();
        prefs.columns.retain(|column| columns.insert(*column));
        if !prefs.columns.contains(&Column::Name) {
            prefs.columns.insert(0, Column::Name);
        }
        Self {
            prefs,

            selection: Default::default(),

            filter: Filter::All,

            search: String::new(),

            search_focused: false,

            sort: Column::Added,

            descending: true,

            visible: Vec::new(),

            dirty: true,

            scroll_to: None,

            add: Default::default(),

            transfer: None,

            transfer_tab: 0,

            draft: None,

            settings_open: false,
            settings_id: 0,
            choose_preferences_folder: false,

            settings_page: 0,

            settings: Settings::default(),

            scheduler_open: false,

            statistics_open: false,

            columns_open: false,

            remove: None,

            message: None,

            history_path: strata_app::download_dir()
                .join("stratafetch-history.json")
                .to_string_lossy()
                .into(),
        }
    }

    pub fn show_add(&mut self) {
        self.add = AddDialog {
            form_id: self.add.form_id.wrapping_add(1),
            open: true,
            focus_url: true,

            folder: self.prefs.folder.clone(),

            ..Default::default()
        };
    }

    pub fn show_settings(&mut self) {
        if !self.settings_open {
            self.settings_id = self.settings_id.wrapping_add(1);
        }
        self.settings_open = true;
    }
    pub fn apply_metadata(
        &mut self,
        form_id: u64,
        probe_id: u64,
        url: &str,
        result: Result<strata_core::metadata::Metadata, String>,
    ) {
        if !self.add.open
            || self.add.form_id != form_id
            || self.add.probe_id != probe_id
            || self.add.probe_url != url
        {
            return;
        }
        self.add.probing = false;
        if self.add.url.trim() == url {
            match result {
                Ok(metadata) => {
                    if self.add.filename.is_empty() {
                        self.add.filename = metadata.filename.clone();
                    }
                    self.add.metadata = Some(metadata);
                }
                Err(error) => self.add.error = Some(error),
            }
        }
    }
    pub fn table_keyboard(&self, ctx: &egui::Context) -> bool {
        !self.search_focused
            && !self.add.open
            && !self.settings_open
            && !self.scheduler_open
            && !self.statistics_open
            && !self.columns_open
            && self.transfer.is_none()
            && self.remove.is_none()
            && self.message.is_none()
            && !egui::Popup::is_any_open(ctx)
    }

    pub fn selected<'a>(&self, jobs: &'a [Job]) -> Vec<&'a Job> {
        jobs.iter()
            .filter(|j| self.selection.ids.contains(&j.id))
            .collect()
    }
}

#[derive(Clone)]
pub enum Action {
    Add,

    Resume,

    Pause,

    Stop,

    StopAll,

    StopQueue,

    StartQueue,

    Remove,

    RemoveCompleted,

    Details(String),

    OpenFile(String),

    OpenFolder(String),

    Verify(String),

    Restart(String),

    Settings,

    Scheduler,

    Statistics,

    Columns,
}

pub fn name(job: &Job) -> String {
    job.destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into()
}

pub fn bytes(value: f64) -> String {
    let value = value.max(0.0);

    if value >= 1_073_741_824.0 {
        format!("{:.2} GiB", value / 1_073_741_824.0)
    } else if value >= 1_048_576.0 {
        format!("{:.1} MiB", value / 1_048_576.0)
    } else if value >= 1024.0 {
        format!("{:.1} KiB", value / 1024.0)
    } else {
        format!("{value:.0} B")
    }
}

pub fn percent(j: &Job) -> f32 {
    j.size
        .filter(|n| *n > 0)
        .map(|n| (j.downloaded as f64 / n as f64).clamp(0.0, 1.0) as f32)
        .unwrap_or(if j.status == Status::Completed {
            1.0
        } else {
            0.0
        })
}

pub fn eta(j: &Job) -> Option<f64> {
    j.size
        .filter(|_| j.speed > 0.0 && j.status == Status::Downloading)
        .map(|size| size.saturating_sub(j.downloaded) as f64 / j.speed)
}

pub fn duration(seconds: f64) -> String {
    if seconds >= 3600.0 {
        format!("{}h {}m", seconds as u64 / 3600, seconds as u64 % 3600 / 60)
    } else if seconds >= 60.0 {
        format!("{}m {}s", seconds as u64 / 60, seconds as u64 % 60)
    } else {
        format!("{seconds:.0}s")
    }
}

pub fn added(timestamp: u64) -> String {
    time::OffsetDateTime::from_unix_timestamp(timestamp as i64)
        .map(|d| {
            format!(
                "{}-{:02}-{:02} {:02}:{:02}",
                d.year(),
                u8::from(d.month()),
                d.day(),
                d.hour(),
                d.minute()
            )
        })
        .unwrap_or_else(|_| "Unknown".into())
}

pub fn status(s: Status) -> &'static str {
    match s {
        Status::Completed => "Complete",

        Status::Downloading => "Downloading",

        Status::Queued => "Queued",

        Status::Paused => "Paused",

        Status::Cancelled => "Stopped",

        Status::Failed => "Failed",
    }
}

pub fn modifiers(ctx: &egui::Context) -> (bool, bool) {
    ctx.input(|i| {
        let modifiers = i
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                egui::Event::PointerButton {
                    pressed: false,
                    modifiers,
                    ..
                } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or(i.modifiers);
        (modifiers.ctrl || modifiers.command, modifiers.shift)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn late_folder_dialog_results_preserve_closed_forms_and_manual_edits() {
        use super::{FolderTarget, State};
        let mut state = State::new(Default::default());
        state.add.open = true;
        state.add.folder = "original".into();
        state.apply_folder(FolderTarget::Add(0), "original", "selected".into());
        assert_eq!(state.add.folder, "selected");
        state.add.folder = "typed manually".into();
        state.apply_folder(FolderTarget::Add(0), "original", "stale".into());
        assert_eq!(state.add.folder, "typed manually");
        state.settings_open = false;
        state.prefs.folder = "original".into();
        state.apply_folder(FolderTarget::Preferences(0), "original", "stale".into());
        assert_eq!(state.prefs.folder, "original");
        state.settings_open = true;
        state.apply_folder(FolderTarget::Preferences(0), "original", "selected".into());
        assert_eq!(state.prefs.folder, "selected");
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let path = std::ffi::OsString::from_vec(b"/tmp/non-utf8-\xff".to_vec());
            state.apply_folder(FolderTarget::Preferences(0), "selected", path.into());
            assert_eq!(state.prefs.folder, "selected");
            assert!(state.message.as_ref().unwrap().contains("UTF-8"));
        }
    }

    use super::*;

    #[test]
    fn selection_toggle_range_and_single_share_one_model() {
        let mut s = Selection::default();

        let rows = ["a", "b", "c", "d"];

        s.click("a", &rows, false, false);

        s.click("c", &rows, true, false);

        assert_eq!(s.ids.len(), 2);

        s.click("d", &rows, false, true);

        assert_eq!(s.ids, HashSet::from(["c".into(), "d".into()]));

        s.click("b", &rows, false, false);

        assert_eq!(s.ids, HashSet::from(["b".into()]));
    }

    #[test]
    fn range_uses_current_sorted_view() {
        let mut s = Selection::default();

        let rows = ["d", "c", "b", "a"];

        s.click("c", &rows, false, false);

        s.click("a", &rows, false, true);

        assert_eq!(s.ids, HashSet::from(["c".into(), "b".into(), "a".into()]));
    }

    #[test]
    fn select_all_replaces_hidden_selection() {
        let mut s = Selection::default();

        s.click("hidden", &["hidden"], false, false);

        s.all(&["visible"]);

        assert_eq!(s.ids, HashSet::from(["visible".into()]));
    }
}
#[cfg(test)]
mod reopened_forms {
    use super::*;
    #[test]
    fn old_chooser_results_do_not_change_a_reopened_add_form() {
        let mut state = State::new(Default::default());
        state.show_add();
        let initial = state.add.folder.clone();
        let target = FolderTarget::Add(state.add.form_id);
        state.add.open = false;
        state.show_add();
        state.apply_folder(target, &initial, "old selection".into());
        assert_eq!(state.add.folder, initial);
    }
}
#[cfg(test)]
mod async_result_order {
    use super::*;
    #[test]
    fn stale_metadata_does_not_overwrite_a_newer_probe_or_form() {
        let mut state = State::new(Default::default());
        state.show_add();
        state.add.url = "https://example.org/file".into();
        state.add.probe_url = state.add.url.clone();
        state.add.probe_id = 2;
        state.add.probing = true;
        let id = state.add.form_id;
        let metadata = strata_core::metadata::identity(&state.add.url).unwrap();
        state.apply_metadata(id, 1, "https://example.org/file", Ok(metadata.clone()));
        assert!(state.add.probing);
        assert!(state.add.metadata.is_none());
        state.apply_metadata(id, 2, "https://example.org/file", Ok(metadata.clone()));
        assert!(!state.add.probing);
        assert!(state.add.metadata.is_some());
        state.show_add();
        state.add.url = "https://example.org/file".into();
        state.add.probe_url = state.add.url.clone();
        state.add.probe_id = 2;
        state.add.probing = true;
        state.apply_metadata(id, 2, "https://example.org/file", Ok(metadata));
        assert!(state.add.probing);
        assert!(state.add.metadata.is_none());
    }
    #[test]
    fn old_chooser_results_do_not_change_reopened_preferences() {
        let mut state = State::new(Default::default());
        state.show_settings();
        let target = FolderTarget::Preferences(state.settings_id);
        let initial = state.prefs.folder.clone();
        state.settings_open = false;
        state.show_settings();
        state.apply_folder(target, &initial, "old selection".into());
        assert_eq!(state.prefs.folder, initial);
    }
}
