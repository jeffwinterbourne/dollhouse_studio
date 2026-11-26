use dh_core::{
    ComponentDef, Project, ProjectManifest, ProjectWall, SCHEMA_VERSION, StandalonePiece,
    sanitize_filename,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const PROJECT_MANIFEST_FILE: &str = "dollhouse.project.json";
const AUTOSAVE_DIR: &str = ".autosave";
const AUTOSAVE_FILE: &str = "recovery.json";

#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub project: Project,
    pub dir: PathBuf,
    pub has_crash_recovery: bool,
    pub recovery_epoch: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AutosaveEnvelope {
    saved_at_epoch: u64,
    project: Project,
}

/// Manages disk-folder projects with atomic writes, relative asset structure, and crash recovery.
pub struct ProjectStore;

impl ProjectStore {
    pub fn is_project_dir(dir: &Path) -> bool {
        dir.join(PROJECT_MANIFEST_FILE).is_file()
    }

    /// Initializes the folder layout inside `dir` and writes `project` cleanly to disk.
    pub fn create_in_dir(dir: &Path, project: &mut Project) -> Result<(), String> {
        ensure_project_subdirs(dir)?;
        Self::save(dir, project)
    }

    /// Saves the complete `Project` atomically into `dir` and removes any orphaned item files.
    pub fn save(dir: &Path, project: &mut Project) -> Result<(), String> {
        project.ensure_valid_selection();
        project.manifest.schema_version = SCHEMA_VERSION;
        ensure_project_subdirs(dir)?;

        let mut comp_files = Vec::with_capacity(project.components.len());
        let mut expected_comp_names = HashSet::new();
        for comp in &project.components {
            let fname = item_filename(&comp.name, &comp.id, "component.json");
            let rel = format!("components/{fname}");
            let full = dir.join("components").join(&fname);
            write_json_atomic(&full, comp)?;
            expected_comp_names.insert(fname);
            comp_files.push(rel);
        }
        prune_dir_except(
            &dir.join("components"),
            "component.json",
            &expected_comp_names,
        )?;

        let mut wall_files = Vec::with_capacity(project.walls.len());
        let mut expected_wall_names = HashSet::new();
        for wall in &project.walls {
            let fname = item_filename(&wall.name, &wall.id, "wall.json");
            let rel = format!("walls/{fname}");
            let full = dir.join("walls").join(&fname);
            write_json_atomic(&full, wall)?;
            expected_wall_names.insert(fname);
            wall_files.push(rel);
        }
        prune_dir_except(&dir.join("walls"), "wall.json", &expected_wall_names)?;

        let mut piece_files = Vec::with_capacity(project.pieces.len());
        let mut expected_piece_names = HashSet::new();
        for piece in &project.pieces {
            let fname = item_filename(&piece.name, &piece.id, "piece.json");
            let rel = format!("pieces/{fname}");
            let full = dir.join("pieces").join(&fname);
            write_json_atomic(&full, piece)?;
            expected_piece_names.insert(fname);
            piece_files.push(rel);
        }
        prune_dir_except(&dir.join("pieces"), "piece.json", &expected_piece_names)?;

        let mut expected_tpl_names = HashSet::new();
        for tpl in &project.project_templates {
            let fname = item_filename(&tpl.name, &tpl.id, "template.json");
            let full = dir.join("templates").join(&fname);
            write_json_atomic(&full, tpl)?;
            expected_tpl_names.insert(fname);
        }
        prune_dir_except(&dir.join("templates"), "template.json", &expected_tpl_names)?;

        project.manifest.component_files = comp_files;
        project.manifest.wall_files = wall_files;
        project.manifest.piece_files = piece_files;

        write_json_atomic(&dir.join(PROJECT_MANIFEST_FILE), &project.manifest)?;
        Self::clear_autosave(dir);
        Ok(())
    }

    /// Loads a project folder from `dir`, verifying manifest schema and reporting any `.autosave`.
    pub fn open_dir(dir: &Path) -> Result<LoadedProject, String> {
        let manifest_path = dir.join(PROJECT_MANIFEST_FILE);
        if !manifest_path.is_file() {
            return Err(format!(
                "Not a Dollhouse Studio project folder (missing {PROJECT_MANIFEST_FILE})"
            ));
        }

        let manifest_text = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed reading project manifest: {e}"))?;
        let manifest: ProjectManifest = serde_json::from_str(&manifest_text)
            .map_err(|e| format!("Invalid project manifest JSON: {e}"))?;

        let mut components = Vec::new();
        for rel in &manifest.component_files {
            let p = crate::assets::resolve_asset_path(dir, rel)?;
            if p.is_file() {
                components.push(read_json::<ComponentDef>(&p)?);
            }
        }

        let mut walls = Vec::new();
        for rel in &manifest.wall_files {
            let p = crate::assets::resolve_asset_path(dir, rel)?;
            if p.is_file() {
                walls.push(read_json::<ProjectWall>(&p)?);
            }
        }

        let mut pieces = Vec::new();
        for rel in &manifest.piece_files {
            let p = crate::assets::resolve_asset_path(dir, rel)?;
            if p.is_file() {
                pieces.push(read_json::<StandalonePiece>(&p)?);
            }
        }

        let mut project_templates: Vec<dh_core::PieceTemplate> = Vec::new();
        let tpl_dir = dir.join("templates");
        if tpl_dir.is_dir()
            && let Ok(entries) = fs::read_dir(&tpl_dir)
        {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|n| n.ends_with(".template.json"))
                    && let Ok(tpl) = read_json(&p)
                {
                    project_templates.push(tpl);
                }
            }
            project_templates.sort_by(|a, b| a.name.cmp(&b.name));
        }

        let mut project = Project {
            manifest,
            components,
            walls,
            pieces,
            project_templates,
        };
        project.ensure_valid_selection();

        let autosave_path = dir.join(AUTOSAVE_DIR).join(AUTOSAVE_FILE);
        let (has_crash_recovery, recovery_epoch) = if autosave_path.is_file() {
            let epoch = fs::metadata(&autosave_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            (true, epoch)
        } else {
            (false, None)
        };

        Ok(LoadedProject {
            project,
            dir: dir.to_path_buf(),
            has_crash_recovery,
            recovery_epoch,
        })
    }

    pub fn write_autosave(dir: &Path, project: &Project) -> Result<(), String> {
        let auto_dir = dir.join(AUTOSAVE_DIR);
        fs::create_dir_all(&auto_dir).map_err(|e| format!("Failed creating autosave dir: {e}"))?;
        let env = AutosaveEnvelope {
            saved_at_epoch: dh_core::now_epoch(),
            project: project.clone(),
        };
        write_json_atomic(&auto_dir.join(AUTOSAVE_FILE), &env)
    }

    pub fn load_autosave(dir: &Path) -> Result<(u64, Project), String> {
        let p = dir.join(AUTOSAVE_DIR).join(AUTOSAVE_FILE);
        let env: AutosaveEnvelope = read_json(&p)?;
        let mut proj = env.project;
        proj.ensure_valid_selection();
        Ok((env.saved_at_epoch, proj))
    }

    pub fn clear_autosave(dir: &Path) {
        let p = dir.join(AUTOSAVE_DIR).join(AUTOSAVE_FILE);
        let _ = fs::remove_file(p);
    }

    pub fn default_exports_dir(dir: &Path) -> PathBuf {
        dir.join("exports")
    }
}

fn ensure_project_subdirs(dir: &Path) -> Result<(), String> {
    for sub in [
        "",
        "components",
        "walls",
        "pieces",
        "templates",
        "assets/photos",
        "exports/stl",
        "exports/plates",
        "exports/pdf",
        "exports/svg",
    ] {
        let p = if sub.is_empty() {
            dir.to_path_buf()
        } else {
            dir.join(sub)
        };
        fs::create_dir_all(&p)
            .map_err(|e| format!("Failed creating project directory {}: {e}", p.display()))?;
    }
    Ok(())
}

fn item_filename(name: &str, id: &str, ext: &str) -> String {
    let slug = sanitize_filename(name);
    let short_id = sanitize_filename(id);
    format!("{slug}_{short_id}.{ext}")
}

fn prune_dir_except(
    dir: &Path,
    suffix: &str,
    expected_names: &HashSet<String>,
) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries =
        fs::read_dir(dir).map_err(|e| format!("Failed scanning {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|s| s.to_str())
            && name.ends_with(suffix)
            && !expected_names.contains(name)
        {
            let _ = fs::remove_file(&p);
        }
    }
    Ok(())
}

pub(crate) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed creating parent directory: {e}"))?;
    }
    let json =
        serde_json::to_string_pretty(value).map_err(|e| format!("Failed serializing JSON: {e}"))?;

    let tmp_name = format!(
        "{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("temp")
    );
    let tmp_path = path.with_file_name(tmp_name);
    fs::write(&tmp_path, json)
        .map_err(|e| format!("Failed writing temporary file {}: {e}", tmp_path.display()))?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed committing {}: {e}", path.display()))?;
    Ok(())
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("Failed reading {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("Invalid JSON in {}: {e}", path.display()))
}
