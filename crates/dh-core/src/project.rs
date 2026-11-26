use crate::component::{ComponentDef, InstanceOverrides, OpeningSource};
use crate::params::{PrinterPreset, ScalePreset, WindowParams, WindowStyle};
use crate::template::PieceTemplate;
use crate::units::DisplayUnit;
use crate::wall::{ProjectWall, WallOpening};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    SinglePiece,
    #[default]
    MultiWall,
}

impl ProjectKind {
    pub fn label(self) -> &'static str {
        match self {
            ProjectKind::SinglePiece => "Single Piece",
            ProjectKind::MultiWall => "House / Multi-Wall",
        }
    }
}

/// A standalone window or door object inside a `SinglePiece` project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandalonePiece {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub notes: String,
    pub params: WindowParams,
}

impl StandalonePiece {
    pub fn new(name: impl Into<String>, params: WindowParams) -> Self {
        Self {
            id: new_id("piece"),
            name: name.into(),
            notes: String::new(),
            params,
        }
    }
}

/// Root manifest stored in `dollhouse.project.json` inside the project folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub kind: ProjectKind,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub scale_preset: ScalePreset,
    #[serde(default)]
    pub display_unit: DisplayUnit,
    #[serde(default = "default_wall_thickness")]
    pub default_wall_thickness: f64,
    #[serde(default = "default_acetate_thickness")]
    pub default_acetate_thickness: f64,
    #[serde(default)]
    pub default_printer_preset: PrinterPreset,
    #[serde(default)]
    pub active_wall_id: Option<String>,
    #[serde(default)]
    pub active_piece_id: Option<String>,
    #[serde(default)]
    pub wall_files: Vec<String>,
    #[serde(default)]
    pub component_files: Vec<String>,
    #[serde(default)]
    pub piece_files: Vec<String>,
}

fn default_wall_thickness() -> f64 {
    9.0
}

fn default_acetate_thickness() -> f64 {
    0.40
}

/// Complete in-memory project aggregate mapped to a folder on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub manifest: ProjectManifest,
    #[serde(default)]
    pub components: Vec<ComponentDef>,
    #[serde(default)]
    pub walls: Vec<ProjectWall>,
    #[serde(default)]
    pub pieces: Vec<StandalonePiece>,
    #[serde(default)]
    pub project_templates: Vec<PieceTemplate>,
}

impl Project {
    pub fn new_single_piece(
        name: impl Into<String>,
        piece_name: impl Into<String>,
        mut params: WindowParams,
        scale_preset: ScalePreset,
        display_unit: DisplayUnit,
    ) -> Self {
        params.apply_scale_preset(scale_preset);
        let piece = StandalonePiece::new(piece_name, params.clone());
        let pid = piece.id.clone();
        Self {
            manifest: ProjectManifest {
                schema_version: SCHEMA_VERSION,
                id: new_id("proj"),
                name: name.into(),
                kind: ProjectKind::SinglePiece,
                notes: String::new(),
                scale_preset,
                display_unit,
                default_wall_thickness: params.wall_thickness,
                default_acetate_thickness: params.acetate_thickness,
                default_printer_preset: PrinterPreset::Standard04,
                active_wall_id: None,
                active_piece_id: Some(pid),
                wall_files: Vec::new(),
                component_files: Vec::new(),
                piece_files: Vec::new(),
            },
            components: Vec::new(),
            walls: Vec::new(),
            pieces: vec![piece],
            project_templates: Vec::new(),
        }
    }

    pub fn new_multi_wall(
        name: impl Into<String>,
        with_sample_facade: bool,
        scale_preset: ScalePreset,
        display_unit: DisplayUnit,
    ) -> Self {
        let (wall, components) = if with_sample_facade {
            let (mut w, comp) = ProjectWall::new_sample_facade();
            if scale_preset != ScalePreset::Scale1_12 {
                let factor = scale_preset.scale_factor();
                w.width = (w.width * factor).max(120.0);
                w.height = (w.height * factor).max(100.0);
                w.wall_thickness = (w.wall_thickness * factor).clamp(3.2, 24.0);
                let mut c = comp;
                c.params.apply_scale_preset(scale_preset);
                c.params.wall_thickness = w.wall_thickness;
                for op in &mut w.openings {
                    op.x_from_left *= factor;
                    op.y_from_bottom *= factor;
                    if let OpeningSource::Unique { params } = &mut op.source {
                        params.apply_scale_preset(scale_preset);
                        params.wall_thickness = w.wall_thickness;
                    }
                }
                (w, vec![c])
            } else {
                (w, vec![comp])
            }
        } else {
            let mut w = ProjectWall::new_default("Front Wall");
            let factor = scale_preset.scale_factor();
            w.width = (w.width * factor).max(120.0);
            w.height = (w.height * factor).max(100.0);
            w.wall_thickness = (w.wall_thickness * factor).clamp(3.2, 24.0);
            (w, Vec::new())
        };

        let wid = wall.id.clone();
        let wt = wall.wall_thickness;
        let at = wall.acetate_thickness;
        Self {
            manifest: ProjectManifest {
                schema_version: SCHEMA_VERSION,
                id: new_id("proj"),
                name: name.into(),
                kind: ProjectKind::MultiWall,
                notes: String::new(),
                scale_preset,
                display_unit,
                default_wall_thickness: wt,
                default_acetate_thickness: at,
                default_printer_preset: PrinterPreset::Standard04,
                active_wall_id: Some(wid),
                active_piece_id: None,
                wall_files: Vec::new(),
                component_files: Vec::new(),
                piece_files: Vec::new(),
            },
            components,
            walls: vec![wall],
            pieces: Vec::new(),
            project_templates: Vec::new(),
        }
    }

    pub fn ensure_valid_selection(&mut self) {
        match self.manifest.kind {
            ProjectKind::SinglePiece => {
                if self.pieces.is_empty() {
                    let mut p = WindowParams::default();
                    WindowStyle::GeorgianGrid.apply_style_defaults(&mut p);
                    self.pieces.push(StandalonePiece::new("Window 1", p));
                }
                let valid = self
                    .manifest
                    .active_piece_id
                    .as_ref()
                    .is_some_and(|id| self.pieces.iter().any(|p| &p.id == id));
                if !valid {
                    self.manifest.active_piece_id = Some(self.pieces[0].id.clone());
                }
            }
            ProjectKind::MultiWall => {
                if self.walls.is_empty() {
                    let w = ProjectWall::new_default("Wall 1");
                    self.manifest.active_wall_id = Some(w.id.clone());
                    self.walls.push(w);
                }
                let valid = self
                    .manifest
                    .active_wall_id
                    .as_ref()
                    .is_some_and(|id| self.walls.iter().any(|w| &w.id == id));
                if !valid {
                    self.manifest.active_wall_id = Some(self.walls[0].id.clone());
                }
            }
        }
    }

    pub fn active_wall(&self) -> Option<&ProjectWall> {
        let id = self.manifest.active_wall_id.as_ref()?;
        self.walls.iter().find(|w| &w.id == id)
    }

    pub fn active_wall_mut(&mut self) -> Option<&mut ProjectWall> {
        let id = self.manifest.active_wall_id.clone()?;
        self.walls.iter_mut().find(|w| w.id == id)
    }

    pub fn active_piece(&self) -> Option<&StandalonePiece> {
        let id = self.manifest.active_piece_id.as_ref()?;
        self.pieces.iter().find(|p| &p.id == id)
    }

    pub fn active_piece_mut(&mut self) -> Option<&mut StandalonePiece> {
        let id = self.manifest.active_piece_id.clone()?;
        self.pieces.iter_mut().find(|p| p.id == id)
    }

    pub fn component(&self, id: &str) -> Option<&ComponentDef> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn component_mut(&mut self, id: &str) -> Option<&mut ComponentDef> {
        self.components.iter_mut().find(|c| c.id == id)
    }

    /// Returns `(total_instances, wall_count)` for a shared component across all walls.
    pub fn component_usage(&self, component_id: &str) -> (usize, usize) {
        let mut instances = 0usize;
        let mut wall_ids: HashSet<&str> = HashSet::new();
        for w in &self.walls {
            for op in &w.openings {
                if op.component_id() == Some(component_id) {
                    instances += 1;
                    wall_ids.insert(&w.id);
                }
            }
        }
        (instances, wall_ids.len())
    }

    /// Creates a new shared component definition in the project.
    pub fn create_component(&mut self, name: impl Into<String>, params: WindowParams) -> String {
        let comp = ComponentDef::new(name, params);
        let id = comp.id.clone();
        self.components.push(comp);
        id
    }

    /// Promotes a unique wall opening into a shared ComponentDef and links that opening to it.
    /// If the opening is already linked, returns its existing component id.
    pub fn promote_opening_to_component(
        &mut self,
        wall_id: &str,
        opening_id: &str,
        comp_name: Option<&str>,
    ) -> Option<String> {
        let wall_idx = self.walls.iter().position(|w| w.id == wall_id)?;
        let op_idx = self.walls[wall_idx]
            .openings
            .iter()
            .position(|o| o.id == opening_id)?;

        if let Some(cid) = self.walls[wall_idx].openings[op_idx].component_id() {
            return Some(cid.to_string());
        }

        let op_name = self.walls[wall_idx].openings[op_idx].name.clone();
        let params = self.walls[wall_idx].openings[op_idx].resolve_params(&self.components);
        let final_name = comp_name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&op_name)
            .to_string();

        let comp = ComponentDef::new(final_name, params.clone());
        let cid = comp.id.clone();
        self.components.push(comp);

        let wall = &mut self.walls[wall_idx];
        let overrides = if wall.sync_common_settings {
            InstanceOverrides {
                wall_thickness: Some(wall.wall_thickness),
                acetate_thickness: Some(wall.acetate_thickness),
                ..Default::default()
            }
        } else {
            InstanceOverrides::default()
        };
        wall.openings[op_idx].source = OpeningSource::Component {
            component_id: cid.clone(),
            overrides,
        };
        Some(cid)
    }

    /// Places a linked instance of an existing ComponentDef onto a wall.
    pub fn place_component_instance(
        &mut self,
        wall_id: &str,
        component_id: &str,
        name: Option<&str>,
        x_from_left: f64,
        y_from_bottom: f64,
    ) -> Option<String> {
        let comp = self.component(component_id)?.clone();
        let wall_idx = self.walls.iter().position(|w| w.id == wall_id)?;
        let wall_w = self.walls[wall_idx].width;
        let wall_h = self.walls[wall_idx].height;
        let sync = self.walls[wall_idx].sync_common_settings;
        let wt = self.walls[wall_idx].wall_thickness;
        let at = self.walls[wall_idx].acetate_thickness;
        let next_idx = self.walls[wall_idx].openings.len() + 1;

        let inst_name = name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("{} #{next_idx}", comp.name));

        let overrides = if sync {
            InstanceOverrides {
                wall_thickness: Some(wt),
                acetate_thickness: Some(at),
                ..Default::default()
            }
        } else {
            InstanceOverrides::default()
        };
        let mut op =
            WallOpening::new_instance(inst_name, x_from_left, y_from_bottom, comp.id, overrides);
        op.clamp_to_wall(wall_w, wall_h, &self.components);
        let oid = op.id.clone();
        self.walls[wall_idx].openings.push(op);
        Some(oid)
    }

    /// Unlinks an opening from its shared ComponentDef into a unique independent opening.
    pub fn make_opening_unique(&mut self, wall_id: &str, opening_id: &str) -> bool {
        let Some(wall) = self.walls.iter_mut().find(|w| w.id == wall_id) else {
            return false;
        };
        let Some(op) = wall.openings.iter_mut().find(|o| o.id == opening_id) else {
            return false;
        };
        if !op.is_component() {
            return false;
        }
        let resolved = op.resolve_params(&self.components);
        op.source = OpeningSource::Unique { params: resolved };
        true
    }

    /// Deletes a shared component after detaching all existing instances so nothing breaks.
    pub fn delete_component(&mut self, component_id: &str) {
        let pairs: Vec<(String, String)> = self
            .walls
            .iter()
            .flat_map(|w| {
                w.openings
                    .iter()
                    .filter(|o| o.component_id() == Some(component_id))
                    .map(move |o| (w.id.clone(), o.id.clone()))
            })
            .collect();
        for (wid, oid) in pairs {
            self.make_opening_unique(&wid, &oid);
        }
        self.components.retain(|c| c.id != component_id);
    }

    /// Creates a new wall inside a MultiWall project and makes it active.
    pub fn add_wall(
        &mut self,
        name: &str,
        width: f64,
        height: f64,
        wall_thickness: f64,
        acetate_thickness: f64,
    ) -> String {
        let trimmed = name.trim();
        let final_name = if trimmed.is_empty() {
            format!("Wall {}", self.walls.len() + 1)
        } else {
            trimmed.to_string()
        };
        let mut wall = ProjectWall::new_default(final_name);
        wall.width = width.clamp(40.0, 2000.0);
        wall.height = height.clamp(40.0, 2000.0);
        wall.wall_thickness = wall_thickness.clamp(3.0, 35.0);
        wall.acetate_thickness = acetate_thickness.clamp(0.1, 5.0);
        let id = wall.id.clone();
        self.manifest.kind = ProjectKind::MultiWall;
        self.manifest.active_wall_id = Some(id.clone());
        self.walls.push(wall);
        id
    }

    /// Promotes a `SinglePiece` project into a `MultiWall` project, placing the piece onto a new wall.
    pub fn promote_single_piece_to_multi_wall(&mut self) -> Option<String> {
        if self.manifest.kind == ProjectKind::MultiWall {
            return self.manifest.active_wall_id.clone();
        }
        let piece = self
            .active_piece()
            .cloned()
            .or_else(|| self.pieces.first().cloned())?;
        let mut wall = ProjectWall::new_default("Front Wall");
        wall.wall_thickness = piece.params.wall_thickness;
        wall.acetate_thickness = piece.params.acetate_thickness;
        wall.width = (piece.params.opening_width * 3.0).clamp(240.0, 1000.0);
        wall.height = (piece.params.opening_height * 2.2).clamp(200.0, 1000.0);

        let x = ((wall.width - piece.params.opening_width) * 0.5).max(0.0);
        let y = if piece.params.style.is_door() {
            0.0
        } else {
            ((wall.height - piece.params.opening_height) * 0.45).max(0.0)
        };
        let opening = WallOpening::new_unique(piece.name, x, y, piece.params);
        let oid = opening.id.clone();
        wall.openings.push(opening);

        let wid = wall.id.clone();
        self.walls.push(wall);
        self.pieces.clear();
        self.manifest.kind = ProjectKind::MultiWall;
        self.manifest.active_piece_id = None;
        self.manifest.active_wall_id = Some(wid);
        Some(oid)
    }
}

pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "item".to_string()
    } else {
        trimmed.to_lowercase()
    }
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn new_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{}_{}", now_epoch(), seq)
}
