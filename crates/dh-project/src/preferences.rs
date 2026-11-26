use crate::store::{read_json, write_json_atomic};
use dh_core::{DisplayUnit, ProjectKind, now_epoch};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::System => "Follow OS",
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BedSizePref {
    Mini180,
    Standard220,
    #[default]
    Bambu256,
    Large300,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PaperSizePref {
    #[default]
    A4,
    UsLetter,
    A3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PdfInstanceModePref {
    #[default]
    UniqueWithQty,
    AllInstances,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProjectEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: ProjectKind,
    pub wall_count: usize,
    pub piece_count: usize,
    pub last_opened_epoch: u64,
}

/// OS-stored application preferences (recent projects, UI theme, default units/export settings).
/// Kept cleanly separated from individual project folders on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppPreferences {
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default)]
    pub display_unit: DisplayUnit,
    #[serde(default)]
    pub default_bed: BedSizePref,
    #[serde(default)]
    pub default_paper: PaperSizePref,
    #[serde(default)]
    pub default_pdf_instance_mode: PdfInstanceModePref,
    #[serde(default)]
    pub last_projects_root: Option<PathBuf>,
    #[serde(default)]
    pub recent_projects: Vec<RecentProjectEntry>,
    #[serde(default)]
    pub has_seen_onboarding: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            display_unit: DisplayUnit::Mm,
            default_bed: BedSizePref::Bambu256,
            default_paper: PaperSizePref::A4,
            default_pdf_instance_mode: PdfInstanceModePref::UniqueWithQty,
            last_projects_root: None,
            recent_projects: Vec::new(),
            has_seen_onboarding: false,
        }
    }
}

impl AppPreferences {
    pub fn config_dir() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("DollhouseStudio");
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("dollhouse_studio");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("dollhouse_studio");
        }
        PathBuf::from(".dollhouse_studio_config")
    }

    pub fn preferences_path() -> PathBuf {
        Self::config_dir().join("preferences.json")
    }

    pub fn personal_templates_dir() -> PathBuf {
        Self::config_dir().join("templates")
    }

    pub fn load_or_default() -> Self {
        let path = Self::preferences_path();
        if path.is_file()
            && let Ok(mut prefs) = read_json::<AppPreferences>(&path)
        {
            prefs.prune_missing_recent();
            return prefs;
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        write_json_atomic(&Self::preferences_path(), self)
    }

    pub fn record_recent(
        &mut self,
        dir: &Path,
        name: &str,
        kind: ProjectKind,
        wall_count: usize,
        piece_count: usize,
    ) {
        self.recent_projects.retain(|r| r.path != dir);
        self.recent_projects.insert(
            0,
            RecentProjectEntry {
                path: dir.to_path_buf(),
                name: name.to_string(),
                kind,
                wall_count,
                piece_count,
                last_opened_epoch: now_epoch(),
            },
        );
        self.recent_projects.truncate(15);
        if let Some(parent) = dir.parent() {
            self.last_projects_root = Some(parent.to_path_buf());
        }
    }

    pub fn remove_recent(&mut self, dir: &Path) {
        self.recent_projects.retain(|r| r.path != dir);
    }

    pub fn prune_missing_recent(&mut self) {
        self.recent_projects
            .retain(|r| crate::store::ProjectStore::is_project_dir(&r.path));
    }
}
