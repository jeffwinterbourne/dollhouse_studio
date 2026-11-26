use crate::canvas2d::{
    Blueprint2DState, BlueprintAction, PhotoCanvasTool, draw_blueprint_2d,
    quick_bar::QuickBarAction,
};
use crate::explorer::{ExplorerAction, draw_project_explorer};
use crate::inspector::{
    InspectorAction, InspectorResult, InspectorState, draw_standalone_piece_inspector,
    draw_wall_inspector,
};
use crate::photo_cache::WallPhotoTextureCache;
use crate::ribbon::{ActiveViewport, RibbonAction, RibbonState, draw_top_ribbon};
use crate::theme::{StudioPalette, apply_studio_theme};
use crate::welcome::{WelcomeAction, WelcomeState, draw_welcome_hub};
use crate::wizards::{
    NewProjectWizardState, OnboardingOutcome, OnboardingState, PdfExportModalState, PdfExportScope,
    PhotoWizardState, SaveTemplateModalState, draw_new_project_wizard, draw_onboarding_tour,
    draw_pdf_export_wizard, draw_photo_setup_wizard, draw_save_template_modal,
};
use dh_core::{
    ComponentDef, DisplayUnit, OpeningSource, PieceTemplate, Project, ProjectKind, StandalonePiece,
    TemplateScope, UndoHistory, WallOpening, WindowParams, sanitize_filename,
};
use dh_export::{
    BedSizePreset, PdfExportOptions, export_piece_pdf, export_project_booklet_pdf, export_wall_pdf,
    export_wall_svg, export_window_svg, pack_project_onto_build_plates,
    pack_wall_onto_build_plates, write_binary_stl,
};
use dh_geometry::{GeneratedWindow, WallMeshCache, build_window};
use dh_project::{
    AppPreferences, ProjectStore, ThemeMode, delete_personal_template, export_template_file,
    import_template_file, import_wall_photo, list_personal_templates, save_personal_template,
    starter_templates,
};
use dh_render::{Viewport3DEvent, Viewport3DState, draw_viewport};
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct CrashRecoveryPrompt {
    project_dir: PathBuf,
    recovery_epoch: u64,
    recovered_project: Project,
}

#[derive(Debug, Clone)]
struct ToastNotice {
    message: String,
    is_error: bool,
    expires_at: Instant,
}

pub struct DollhouseStudioApp {
    pub show_hub_overlay: bool,
    pub prefs: AppPreferences,
    pub palette: StudioPalette,
    pub personal_templates: Vec<PieceTemplate>,
    pub starter_templates: Vec<PieceTemplate>,

    pub project: Project,
    pub project_dir: Option<PathBuf>,
    pub dirty: bool,
    pub undo: UndoHistory<Project>,
    pending_undo_label: Option<&'static str>,
    last_autosave_at: Instant,

    pub viewport: ActiveViewport,
    pub selected_opening_ids: Vec<String>,
    pub active_component_id: Option<String>,

    pub welcome_state: WelcomeState,
    pub ribbon_state: RibbonState,
    pub canvas2d: Blueprint2DState,
    pub inspector_state: InspectorState,

    pub new_project_wizard: NewProjectWizardState,
    pub onboarding: OnboardingState,
    pub save_template_modal: SaveTemplateModalState,
    pub pdf_export_modal: PdfExportModalState,
    pub photo_wizard: PhotoWizardState,
    crash_recovery_prompt: Option<CrashRecoveryPrompt>,

    pub viewport3d: Arc<Mutex<Viewport3DState>>,
    wall_mesh_cache: WallMeshCache,
    photo_cache: WallPhotoTextureCache,
    wizard_photo_cache: WallPhotoTextureCache,
    cached_piece_key: Option<(String, WindowParams)>,
    cached_piece_mesh: Option<Arc<GeneratedWindow>>,
    last_wall_mesh_hash: Option<u64>,
    last_photo_sync_key: Option<(Option<String>, u64, bool, u32, u64)>,
    pub last_explorer_width: f32,
    pub last_inspector_width: f32,

    toast: Option<ToastNotice>,
}

impl DollhouseStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::new_headless();
        app.palette = apply_studio_theme(&cc.egui_ctx, app.prefs.theme);
        app.sync_3d_viewport(&cc.egui_ctx);
        app
    }

    pub fn new_headless() -> Self {
        let prefs = AppPreferences::load_or_default();
        let personal_dir = AppPreferences::personal_templates_dir();
        let personal_templates = list_personal_templates(&personal_dir);
        let starter_templates = starter_templates();
        let project = Project::new_multi_wall(
            "Untitled Dollhouse",
            true,
            dh_core::ScalePreset::Scale1_12,
            prefs.display_unit,
        );
        let undo = UndoHistory::new(project.clone());
        let unit = prefs.display_unit;
        let last_root = prefs.last_projects_root.clone();
        let onboarding_open = !prefs.has_seen_onboarding;

        Self {
            show_hub_overlay: true,
            palette: StudioPalette::for_mode(prefs.theme == ThemeMode::Dark),
            prefs,
            personal_templates,
            starter_templates,
            project,
            project_dir: None,
            dirty: false,
            undo,
            pending_undo_label: None,
            last_autosave_at: Instant::now(),
            viewport: ActiveViewport::Blueprint2D,
            selected_opening_ids: Vec::new(),
            active_component_id: None,
            welcome_state: WelcomeState::default(),
            ribbon_state: RibbonState::default(),
            canvas2d: Blueprint2DState::new(),
            inspector_state: InspectorState::default(),
            new_project_wizard: NewProjectWizardState::closed(unit, last_root),
            onboarding: OnboardingState {
                open: onboarding_open,
                ..OnboardingState::default()
            },
            save_template_modal: SaveTemplateModalState::default(),
            pdf_export_modal: PdfExportModalState::default(),
            photo_wizard: PhotoWizardState::default(),
            crash_recovery_prompt: None,
            viewport3d: Arc::new(Mutex::new(Viewport3DState::default())),
            wall_mesh_cache: WallMeshCache::default(),
            photo_cache: WallPhotoTextureCache::default(),
            wizard_photo_cache: WallPhotoTextureCache::default(),
            cached_piece_key: None,
            cached_piece_mesh: None,
            last_wall_mesh_hash: None,
            last_photo_sync_key: None,
            last_explorer_width: 0.0,
            last_inspector_width: 0.0,
            toast: None,
        }
    }

    pub fn new_headless_with_project(mut project: Project, dir: Option<PathBuf>) -> Self {
        let mut app = Self::new_headless();
        project.ensure_valid_selection();
        app.project = project.clone();
        app.project_dir = dir;
        app.dirty = false;
        app.undo.reset(project);
        app.show_hub_overlay = false;
        app.onboarding.open = false;
        app.viewport = match app.project.manifest.kind {
            ProjectKind::SinglePiece => ActiveViewport::Piece3D,
            ProjectKind::MultiWall => ActiveViewport::Blueprint2D,
        };
        app
    }

    fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.toast = Some(ToastNotice {
            message: msg.into(),
            is_error,
            expires_at: Instant::now() + Duration::from_secs(4),
        });
    }

    fn commit_undo(&mut self, label: impl Into<String>) {
        let comps = self.project.components.clone();
        for wall in &mut self.project.walls {
            wall.sync_anchored_dimensions(&comps);
        }
        self.project.ensure_valid_selection();
        self.undo.commit(label, self.project.clone());
        self.dirty = true;
    }

    fn mark_live_edit(&mut self, label: &'static str) {
        self.dirty = true;
        if self.pending_undo_label.is_none() {
            self.pending_undo_label = Some(label);
        }
    }

    fn flush_pending_undo(&mut self, ctx: &egui::Context) {
        if let Some(lbl) = self.pending_undo_label {
            let dragging = ctx.input(|i| i.pointer.any_down());
            if !dragging {
                self.pending_undo_label = None;
                self.commit_undo(lbl);
            }
        }
    }

    fn save_preferences(&self) {
        let _ = self.prefs.save();
    }

    pub fn set_project(&mut self, mut project: Project, dir: Option<PathBuf>) {
        project.ensure_valid_selection();
        self.viewport = match project.manifest.kind {
            ProjectKind::SinglePiece => ActiveViewport::Piece3D,
            ProjectKind::MultiWall => ActiveViewport::Blueprint2D,
        };
        self.selected_opening_ids.clear();
        self.active_component_id = None;
        self.canvas2d.edit_photo_mode = false;
        self.canvas2d.fitted_wall_id = None;
        self.photo_cache.clear();
        self.last_wall_mesh_hash = None;
        self.last_photo_sync_key = None;
        self.cached_piece_key = None;

        if let Some(ref pdir) = dir {
            let (wc, pc) = (project.walls.len(), project.pieces.len());
            self.prefs
                .record_recent(pdir, &project.manifest.name, project.manifest.kind, wc, pc);
            self.save_preferences();
        }

        self.project = project.clone();
        self.project_dir = dir;
        self.dirty = false;
        self.undo.reset(project);
        self.show_hub_overlay = false;
    }

    fn save_current_project(&mut self) {
        self.project.ensure_valid_selection();
        if let Some(dir) = self.project_dir.clone() {
            match ProjectStore::save(&dir, &mut self.project) {
                Ok(()) => {
                    ProjectStore::clear_autosave(&dir);
                    self.dirty = false;
                    self.undo.set_baseline(self.project.clone());
                    self.prefs.record_recent(
                        &dir,
                        &self.project.manifest.name,
                        self.project.manifest.kind,
                        self.project.walls.len(),
                        self.project.pieces.len(),
                    );
                    self.save_preferences();
                    self.show_toast("Saved project folder", false);
                }
                Err(e) => self.show_toast(format!("Save failed: {e}"), true),
            }
        } else if let Some(chosen) = rfd::FileDialog::new()
            .set_title("Choose Empty Folder for Project")
            .pick_folder()
        {
            match ProjectStore::create_in_dir(&chosen, &mut self.project) {
                Ok(()) => {
                    self.project_dir = Some(chosen.clone());
                    self.dirty = false;
                    self.undo.set_baseline(self.project.clone());
                    self.prefs.record_recent(
                        &chosen,
                        &self.project.manifest.name,
                        self.project.manifest.kind,
                        self.project.walls.len(),
                        self.project.pieces.len(),
                    );
                    self.save_preferences();
                    self.show_toast("Saved project folder", false);
                }
                Err(e) => self.show_toast(format!("Save failed: {e}"), true),
            }
        }
    }

    fn open_project_folder(&mut self, dir: PathBuf) {
        match ProjectStore::open_dir(&dir) {
            Ok(loaded) => {
                let recovery_opt = if loaded.has_crash_recovery {
                    ProjectStore::load_autosave(&dir).ok().map(|(epoch, proj)| {
                        CrashRecoveryPrompt {
                            project_dir: dir.clone(),
                            recovery_epoch: epoch,
                            recovered_project: proj,
                        }
                    })
                } else {
                    None
                };
                self.set_project(loaded.project, Some(loaded.dir));
                self.crash_recovery_prompt = recovery_opt;
            }
            Err(e) => self.show_toast(format!("Open failed: {e}"), true),
        }
    }

    fn active_unit(&self) -> DisplayUnit {
        if !self.show_hub_overlay {
            self.project.manifest.display_unit
        } else {
            self.prefs.display_unit
        }
    }

    fn current_selected_params_for_3d(&self) -> Option<(String, WindowParams)> {
        if self.project.manifest.kind == ProjectKind::SinglePiece {
            return self
                .project
                .active_piece()
                .map(|p| (p.id.clone(), p.params.clone()));
        }
        if let Some(ref cid) = self.active_component_id
            && let Some(c) = self.project.component(cid)
        {
            return Some((c.id.clone(), c.params.clone()));
        }
        let wall = self.project.active_wall()?;
        if let Some(first_id) = self.selected_opening_ids.first()
            && let Some(op) = wall.openings.iter().find(|o| &o.id == first_id)
        {
            return Some((op.id.clone(), op.resolve_params(&self.project.components)));
        }
        wall.openings
            .first()
            .map(|op| (op.id.clone(), op.resolve_params(&self.project.components)))
    }

    fn sync_3d_viewport(&mut self, ctx: &egui::Context) {
        let dark = self.palette.dark_mode;

        // 1) Update Wall 3D built cache & photo texture if in MultiWall mode.
        if self.project.manifest.kind == ProjectKind::MultiWall {
            if let Some(wall) = self.project.active_wall().cloned() {
                let fast_preview = self.canvas2d.photo.fast_preview;
                let has_photo = wall.photo.has_photo();
                let xform_hash = wall.photo.transform_hash();
                let photo_asset = wall.photo.asset_path.clone();

                let mut photo_ver = 0u64;
                if has_photo {
                    if let Some((gpu, _)) = self.photo_cache.ensure_loaded(
                        ctx,
                        self.project_dir.as_deref(),
                        photo_asset.as_deref(),
                        &wall.photo,
                        fast_preview,
                    ) {
                        photo_ver = gpu.version;
                    }
                    if self.canvas2d.edit_photo_mode
                        && self.canvas2d.photo.tool == PhotoCanvasTool::PerspectivePins
                    {
                        self.photo_cache.ensure_oriented_loaded(
                            ctx,
                            self.project_dir.as_deref(),
                            photo_asset.as_deref(),
                            wall.photo.rotation_deg,
                            fast_preview,
                        );
                    }
                } else {
                    self.photo_cache.clear();
                }

                let bounds_mm = [
                    wall.photo.offset_x_mm as f32,
                    wall.photo.offset_y_mm as f32,
                    wall.photo.width_mm as f32,
                    wall.photo.height_mm as f32,
                ];
                let photo_sync_key = (
                    photo_asset,
                    xform_hash,
                    wall.photo.show_in_3d,
                    wall.photo.opacity.to_bits(),
                    photo_ver,
                );

                // Rebuild 3D wall meshes only when wall geometry changes cleanly
                let wall_hash = quick_wall_hash(&wall, &self.project.components);
                let built_wall = if self.last_wall_mesh_hash != Some(wall_hash) {
                    let built = Arc::new(
                        self.wall_mesh_cache
                            .build_wall(&wall, &self.project.components),
                    );
                    self.last_wall_mesh_hash = Some(wall_hash);
                    Some(built)
                } else {
                    None
                };

                let mut st = self.viewport3d.lock().unwrap();
                st.settings.dark_mode = dark;
                st.project_wall_mode = self.viewport == ActiveViewport::Wall3D;
                st.wall_selected_ids = self.selected_opening_ids.iter().cloned().collect();
                st.hovered_opening_id = self.canvas2d.openings.hovered_opening_id.clone();
                if let Some(bw) = built_wall {
                    st.set_project_wall(bw);
                }
                if self.last_photo_sync_key.as_ref() != Some(&photo_sync_key)
                    || st.project_wall_mode
                {
                    st.set_wall_photo(
                        self.photo_cache.shared_gpu_data(),
                        bounds_mm,
                        wall.photo.show_in_3d && has_photo,
                        wall.photo.opacity,
                    );
                    self.last_photo_sync_key = Some(photo_sync_key);
                }
            }
        } else {
            let mut st = self.viewport3d.lock().unwrap();
            st.settings.dark_mode = dark;
            st.project_wall_mode = false;
        }

        // 2) Update Piece 3D cached GeneratedWindow when params change.
        if let Some((id, params)) = self.current_selected_params_for_3d() {
            let need_rebuild = self
                .cached_piece_key
                .as_ref()
                .is_none_or(|(cid, cp)| cid != &id || cp != &params);
            if need_rebuild {
                let built = Arc::new(build_window(&params));
                self.cached_piece_key = Some((id, params));
                self.cached_piece_mesh = Some(Arc::clone(&built));
                let mut st = self.viewport3d.lock().unwrap();
                st.set_window(built);
            }
        }
    }

    fn process_3d_events(&mut self) {
        let events: Vec<Viewport3DEvent> = {
            let mut st = self.viewport3d.lock().unwrap();
            std::mem::take(&mut st.events)
        };
        for ev in events {
            match ev {
                Viewport3DEvent::PickedOpening { id, toggle } => {
                    self.active_component_id = None;
                    if let Some(oid) = id {
                        if toggle {
                            if let Some(pos) =
                                self.selected_opening_ids.iter().position(|x| x == &oid)
                            {
                                self.selected_opening_ids.remove(pos);
                            } else {
                                self.selected_opening_ids.push(oid);
                            }
                        } else {
                            self.selected_opening_ids = vec![oid];
                        }
                    } else if !toggle {
                        self.selected_opening_ids.clear();
                    }
                }
                Viewport3DEvent::DoubleClickedOpening(id) => {
                    self.active_component_id = None;
                    self.selected_opening_ids = vec![id];
                    self.viewport = ActiveViewport::Piece3D;
                }
                Viewport3DEvent::MoveOpening {
                    id,
                    x_from_left,
                    y_from_bottom,
                } => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        let ww = wall.width;
                        let wh = wall.height;
                        if let Some(op) = wall.openings.iter_mut().find(|o| o.id == id)
                            && !op.lock_position
                        {
                            op.x_from_left = x_from_left;
                            op.y_from_bottom = y_from_bottom;
                            op.clamp_to_wall(ww, wh, &comps);
                            self.mark_live_edit("Move opening in 3D");
                        }
                    }
                }
                Viewport3DEvent::ResizeOpening {
                    id,
                    x_from_left,
                    y_from_bottom,
                    width,
                    height,
                } => {
                    self.resize_opening_by_id(&id, x_from_left, y_from_bottom, width, height);
                    self.mark_live_edit("Resize opening in 3D");
                }
                Viewport3DEvent::InteractionFinished(lbl) => {
                    self.pending_undo_label = None;
                    self.commit_undo(lbl);
                }
            }
        }
    }

    fn resize_opening_by_id(
        &mut self,
        opening_id: &str,
        x_from_left: f64,
        y_from_bottom: f64,
        width: f64,
        height: f64,
    ) {
        let Some(wid) = self.project.manifest.active_wall_id.clone() else {
            return;
        };
        let mut linked_comp_id: Option<String> = None;
        if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == wid)
            && let Some(op) = wall.openings.iter_mut().find(|o| o.id == opening_id)
        {
            if !op.lock_position {
                op.x_from_left = x_from_left;
                op.y_from_bottom = y_from_bottom;
            }
            let lock_w = op.lock_width;
            let lock_h = op.lock_height;
            match &mut op.source {
                OpeningSource::Unique { params } => {
                    if !lock_w {
                        params.opening_width = width.clamp(18.0, 400.0);
                    }
                    if !lock_h {
                        params.opening_height = height.clamp(18.0, 400.0);
                    }
                }
                OpeningSource::Component { component_id, .. } => {
                    if !lock_w || !lock_h {
                        linked_comp_id = Some(component_id.clone());
                    }
                }
            }
        }
        if let Some(cid) = linked_comp_id
            && let Some(comp) = self.project.component_mut(&cid)
        {
            comp.params.opening_width = width.clamp(18.0, 400.0);
            comp.params.opening_height = height.clamp(18.0, 400.0);
        }
        let comps = self.project.components.clone();
        if let Some(wall) = self.project.active_wall_mut() {
            let ww = wall.width;
            let wh = wall.height;
            if let Some(op) = wall.openings.iter_mut().find(|o| o.id == opening_id) {
                op.clamp_to_wall(ww, wh, &comps);
            }
            wall.sync_anchored_dimensions(&comps);
        }
    }

    fn insert_params_onto_active_wall(
        &mut self,
        name: &str,
        mut params: WindowParams,
        pos_mm: Option<[f64; 2]>,
    ) {
        let comps = self.project.components.clone();
        let Some(wall) = self.project.active_wall_mut() else {
            return;
        };
        if wall.sync_common_settings {
            params.wall_thickness = wall.wall_thickness;
            params.acetate_thickness = wall.acetate_thickness;
        }
        let (x, y) = match pos_mm {
            Some([px, py]) => (px, py),
            None => {
                let idx = wall.openings.len();
                let px =
                    30.0 + ((idx as f64 * 46.0) % (wall.width - params.opening_width).max(20.0));
                let py = if params.style.is_door() {
                    0.0
                } else {
                    (wall.height - params.opening_height) * 0.5
                };
                (px, py)
            }
        };
        let mut op = WallOpening::new_unique(name, x, y, params);
        op.clamp_to_wall(wall.width, wall.height, &comps);
        let oid = op.id.clone();
        wall.openings.push(op);
        self.active_component_id = None;
        self.selected_opening_ids = vec![oid];
        self.commit_undo("Add wall opening");
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.show_hub_overlay
            || self.onboarding.open
            || self.photo_wizard.open
            || self.new_project_wizard.open
        {
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save_current_project();
            return;
        }
        if !typing
            && ctx
                .input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            || (!typing && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Y)))
        {
            self.perform_redo();
            return;
        }
        if !typing && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.perform_undo();
            return;
        }
        if !typing
            && ctx
                .input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
        {
            self.delete_selected_openings();
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.canvas2d.dims.cancel_draft();
            self.canvas2d.dims.active_draw_mode = false;
            self.canvas2d.edit_photo_mode = false;
            self.canvas2d.photo.fast_preview = false;
        }
        if !typing && self.project.manifest.kind == ProjectKind::MultiWall {
            if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
                self.viewport = ActiveViewport::Blueprint2D;
            } else if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
                self.viewport = ActiveViewport::Wall3D;
            } else if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
                self.viewport = ActiveViewport::Piece3D;
            }
        }
    }

    fn perform_undo(&mut self) {
        if let Some((lbl, prev)) = self.undo.undo(self.project.clone()) {
            self.project = prev;
            self.project.ensure_valid_selection();
            self.dirty = true;
            self.selected_opening_ids.retain(|id| {
                self.project
                    .active_wall()
                    .is_some_and(|w| w.openings.iter().any(|o| &o.id == id))
            });
            self.show_toast(format!("Undid: {lbl}"), false);
        }
    }

    fn perform_redo(&mut self) {
        if let Some((lbl, next)) = self.undo.redo(self.project.clone()) {
            self.project = next;
            self.project.ensure_valid_selection();
            self.dirty = true;
            self.show_toast(format!("Redid: {lbl}"), false);
        }
    }

    fn delete_selected_openings(&mut self) {
        if self.selected_opening_ids.is_empty() {
            return;
        }
        let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
        if let Some(wall) = self.project.active_wall_mut() {
            let before = wall.openings.len();
            wall.openings.retain(|o| !ids.contains(&o.id));
            if wall.openings.len() != before {
                self.selected_opening_ids.clear();
                self.commit_undo("Delete selected opening(s)");
            }
        }
    }

    fn duplicate_selected_openings(&mut self) {
        let Some(wid) = self.project.manifest.active_wall_id.clone() else {
            return;
        };
        let selected = self.selected_opening_ids.clone();
        if selected.is_empty() {
            return;
        }
        let comps = self.project.components.clone();
        let mut new_ids = Vec::new();
        if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == wid) {
            let ww = wall.width;
            let wh = wall.height;
            let clones: Vec<WallOpening> = wall
                .openings
                .iter()
                .filter(|o| selected.contains(&o.id))
                .cloned()
                .collect();
            for mut cl in clones {
                cl.id = dh_core::new_id("open");
                cl.name = format!("{} Copy", cl.name);
                cl.x_from_left += 18.0;
                cl.clamp_to_wall(ww, wh, &comps);
                new_ids.push(cl.id.clone());
                wall.openings.push(cl);
            }
        }
        if !new_ids.is_empty() {
            self.selected_opening_ids = new_ids;
            self.commit_undo("Duplicate opening(s)");
        }
    }

    fn open_save_template_for_selection(&mut self) {
        if let Some((_, params)) = self.current_selected_params_for_3d() {
            let sug_name = if let Some(p) = self.project.active_piece() {
                p.name.clone()
            } else if let Some(cid) = &self.active_component_id {
                self.project
                    .component(cid)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Custom Template".into())
            } else if let (Some(w), Some(oid)) = (
                self.project.active_wall(),
                self.selected_opening_ids.first(),
            ) {
                w.openings
                    .iter()
                    .find(|o| &o.id == oid)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Custom Template".into())
            } else {
                "Custom Template".into()
            };
            self.save_template_modal.open_for(&sug_name, params);
        } else {
            self.show_toast(
                "Select an opening or piece first to save as template.",
                true,
            );
        }
    }

    fn trigger_import_wall_photo(&mut self) {
        let Some(wid) = self.project.manifest.active_wall_id.clone() else {
            return;
        };
        let Some(pdir) = self.project_dir.clone() else {
            self.show_toast(
                "Save the project folder first before importing photos.",
                true,
            );
            return;
        };
        let Some(src) = rfd::FileDialog::new()
            .set_title("Import Wall Reference Photo")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
        else {
            return;
        };

        match import_wall_photo(&pdir, &src) {
            Ok(rel_path) => {
                if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == wid) {
                    wall.photo.asset_path = Some(rel_path);
                    wall.photo.show_in_blueprint = true;
                    wall.photo.show_in_3d = true;
                    wall.photo.reset_edits();
                    if let Ok(dec) = dh_project::load_rgba_image(&src, 2048) {
                        wall.photo.pixel_size = [dec.width_px, dec.height_px];
                    }
                    let ww = wall.width;
                    let wh = wall.height;
                    wall.photo.fit_to_wall(ww, wh);
                    self.photo_wizard.open_for(&wid, &wall.photo);
                    self.viewport = ActiveViewport::Blueprint2D;
                    self.commit_undo("Import wall reference photo");
                }
            }
            Err(e) => self.show_toast(format!("Photo import failed: {e}"), true),
        }
    }

    fn default_export_subdir(&self, sub: &str) -> PathBuf {
        let base = self
            .project_dir
            .as_deref()
            .map(ProjectStore::default_exports_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let dir = base.join(sub);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn run_pdf_export(&mut self, scope: PdfExportScope, opts: PdfExportOptions) {
        let default_name = match scope {
            PdfExportScope::ActivePiece => format!(
                "{}_template.pdf",
                sanitize_filename(&self.project.manifest.name)
            ),
            PdfExportScope::ActiveWall => {
                let wname = self
                    .project
                    .active_wall()
                    .map(|w| w.name.as_str())
                    .unwrap_or("wall");
                format!("{}_1to1.pdf", sanitize_filename(wname))
            }
            PdfExportScope::ProjectBooklet => {
                format!(
                    "{}_booklet.pdf",
                    sanitize_filename(&self.project.manifest.name)
                )
            }
        };
        let dir = self.default_export_subdir("pdf");
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export 1:1 Scale PDF")
            .set_directory(&dir)
            .set_file_name(&default_name)
            .add_filter("PDF Document", &["pdf"])
            .save_file()
        else {
            return;
        };

        let result = match scope {
            PdfExportScope::ActivePiece => {
                if let Some((_, params)) = self.current_selected_params_for_3d() {
                    let built = build_window(&params);
                    export_piece_pdf(&built, &self.project.manifest.name, &opts, &path)
                } else {
                    return;
                }
            }
            PdfExportScope::ActiveWall => {
                if let Some(wall) = self.project.active_wall() {
                    export_wall_pdf(wall, &self.project.components, &opts, &path)
                } else {
                    return;
                }
            }
            PdfExportScope::ProjectBooklet => {
                export_project_booklet_pdf(&self.project, &opts, &path)
            }
        };

        match result {
            Ok(()) => self.show_toast(format!("Exported PDF: {}", path.display()), false),
            Err(e) => self.show_toast(format!("PDF export error: {e}"), true),
        }
    }

    /// Core UI rendering entry point shared identically by `eframe::App::ui` and headless tests.
    pub fn draw_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.palette = apply_studio_theme(&ctx, self.prefs.theme);
        let pal = self.palette;

        // Clear expired toast
        if let Some(ref t) = self.toast
            && Instant::now() > t.expires_at
        {
            self.toast = None;
        }

        // Silent periodic .autosave every 45s while dirty
        if !self.show_hub_overlay
            && self.dirty
            && self.project_dir.is_some()
            && self.last_autosave_at.elapsed() >= Duration::from_secs(45)
            && let Some(ref dir) = self.project_dir
        {
            let _ = ProjectStore::write_autosave(dir, &self.project);
            self.last_autosave_at = Instant::now();
        }

        // Always render live workspace behind floating Hub & onboarding overlays.
        self.handle_shortcuts(&ctx);
        self.process_3d_events();
        self.sync_3d_viewport(&ctx);

        // 1) Top Ribbon
        let ribbon_actions = egui::Panel::top("studio_top_ribbon")
            .resizable(false)
            .show(ui, |ui| {
                let (view_settings, wall_solid) = {
                    let st = self.viewport3d.lock().unwrap();
                    (st.settings.clone(), st.wall_solid)
                };
                draw_top_ribbon(
                    ui,
                    &pal,
                    &self.project,
                    self.project_dir.is_some(),
                    self.dirty,
                    &self.undo,
                    self.project.manifest.display_unit,
                    self.prefs.theme,
                    self.viewport,
                    self.selected_opening_ids.len(),
                    self.canvas2d.edit_photo_mode,
                    self.canvas2d.photo.tool,
                    self.canvas2d.dims.active_draw_mode,
                    self.canvas2d.show_grid,
                    self.canvas2d.show_rulers,
                    self.canvas2d.show_architrave_outlines,
                    &view_settings,
                    wall_solid,
                    &mut self.ribbon_state,
                )
            })
            .inner;
        self.handle_ribbon_actions(ribbon_actions);

        // 2) Left CAD Explorer
        let explorer_resp = egui::Panel::left("studio_explorer")
            .resizable(false)
            .exact_size(256.0)
            .show(ui, |ui| {
                draw_project_explorer(
                    ui,
                    &pal,
                    &self.project,
                    self.active_component_id.as_deref(),
                    &self.selected_opening_ids,
                    &self.personal_templates,
                    &self.starter_templates,
                    self.project.manifest.display_unit,
                )
            });
        self.last_explorer_width = explorer_resp.response.rect.width();
        self.handle_explorer_actions(explorer_resp.inner);

        // 3) Right Contextual Inspector
        let insp_resp = egui::Panel::right("studio_inspector")
            .resizable(false)
            .exact_size(326.0)
            .show(ui, |ui| {
                let unit = self.project.manifest.display_unit;
                let (mut ext_col, mut door_col) = {
                    let st = self.viewport3d.lock().unwrap();
                    (st.settings.filament_color, st.settings.door_color)
                };

                let res = match self.project.manifest.kind {
                    ProjectKind::SinglePiece => {
                        if let Some(piece) = self.project.active_piece_mut() {
                            draw_standalone_piece_inspector(
                                ui,
                                &pal,
                                piece,
                                unit,
                                &mut ext_col,
                                &mut door_col,
                            )
                        } else {
                            InspectorResult::default()
                        }
                    }
                    ProjectKind::MultiWall => {
                        let Some(wid) = self.project.manifest.active_wall_id.clone() else {
                            return InspectorResult::default();
                        };
                        let w_idx = self.project.walls.iter().position(|w| w.id == wid);
                        if let Some(idx) = w_idx {
                            let wall = &mut self.project.walls[idx];
                            let comps = &mut self.project.components;
                            draw_wall_inspector(
                                ui,
                                &pal,
                                wall,
                                comps,
                                &self.selected_opening_ids,
                                self.active_component_id.as_deref(),
                                unit,
                                &mut ext_col,
                                &mut door_col,
                                &mut self.inspector_state,
                            )
                        } else {
                            InspectorResult::default()
                        }
                    }
                };

                {
                    let mut st = self.viewport3d.lock().unwrap();
                    st.settings.filament_color = ext_col;
                    st.settings.door_color = door_col;
                }
                res
            });
        self.last_inspector_width = insp_resp.response.rect.width();
        self.handle_inspector_result(insp_resp.inner);

        // 4) Central Canvas (Blueprint 2D / Wall 3D / Piece 3D)
        egui::CentralPanel::default().show(ui, |ui| {
            let effective_vp = if self.project.manifest.kind == ProjectKind::SinglePiece {
                ActiveViewport::Piece3D
            } else {
                self.viewport
            };

            match effective_vp {
                ActiveViewport::Blueprint2D => {
                    let unit = self.project.manifest.display_unit;
                    let Some(wid) = self.project.manifest.active_wall_id.clone() else {
                        return;
                    };
                    if let Some(idx) = self.project.walls.iter().position(|w| w.id == wid) {
                        let comps = self.project.components.clone();
                        let actions = draw_blueprint_2d(
                            ui,
                            &pal,
                            &mut self.project.walls[idx],
                            &comps,
                            &self.selected_opening_ids,
                            unit,
                            &self.photo_cache,
                            &mut self.canvas2d,
                        );
                        self.handle_blueprint_actions(actions);
                    }
                }
                ActiveViewport::Wall3D | ActiveViewport::Piece3D => {
                    draw_viewport(ui, Arc::clone(&self.viewport3d));
                }
            }
        });

        self.flush_pending_undo(&ctx);

        // Floating Project & Template Hub overlay (non-destructive over live studio workspace).
        if self.show_hub_overlay
            && !self.new_project_wizard.open
            && !self.onboarding.open
            && let Some(act) = draw_welcome_hub(
                &ctx,
                &pal,
                &self.prefs,
                &self.personal_templates,
                &self.starter_templates,
                &mut self.welcome_state,
            )
        {
            self.handle_welcome_action(act);
        }

        // Skippable 4-step Onboarding Quick Tour
        if let Some(OnboardingOutcome::Dismissed { remember_seen }) =
            draw_onboarding_tour(&ctx, &pal, &mut self.onboarding)
            && remember_seen
        {
            self.prefs.has_seen_onboarding = true;
            self.save_preferences();
        }

        // Floating Wizards & Modals
        if let Some(req) = draw_new_project_wizard(&ctx, &pal, &mut self.new_project_wizard) {
            let mut proj = match req.kind {
                ProjectKind::SinglePiece => {
                    let params = if let Some(ref tpl) = req.initial_template {
                        tpl.params.clone()
                    } else {
                        let mut p = WindowParams::default();
                        req.initial_style.apply_style_defaults(&mut p);
                        p
                    };
                    Project::new_single_piece(&req.name, "Main Piece", params, req.scale, req.unit)
                }
                ProjectKind::MultiWall => {
                    Project::new_multi_wall(&req.name, req.with_sample_facade, req.scale, req.unit)
                }
            };
            match ProjectStore::create_in_dir(&req.project_dir, &mut proj) {
                Ok(()) => {
                    self.prefs.last_projects_root = req.project_dir.parent().map(PathBuf::from);
                    self.set_project(proj, Some(req.project_dir));
                    self.show_toast("Created project folder", false);
                }
                Err(e) => self.show_toast(format!("Create project failed: {e}"), true),
            }
        }

        if let Some(req) = draw_save_template_modal(
            &ctx,
            &pal,
            self.active_unit(),
            &mut self.save_template_modal,
        ) {
            let tpl = PieceTemplate::from_params(req.name, req.notes, req.tags, req.params);
            match req.scope {
                TemplateScope::Personal => {
                    let dir = AppPreferences::personal_templates_dir();
                    match save_personal_template(&dir, &tpl) {
                        Ok(_) => {
                            self.personal_templates = list_personal_templates(&dir);
                            self.show_toast("Saved to Personal Template Library", false);
                        }
                        Err(e) => self.show_toast(format!("Template save failed: {e}"), true),
                    }
                }
                TemplateScope::Project => {
                    self.project.project_templates.push(tpl);
                    self.commit_undo("Save template to project");
                    self.show_toast("Saved template to Project Folder", false);
                }
            }
        }

        if let Some((scope, opts)) =
            draw_pdf_export_wizard(&ctx, &pal, &self.project, &mut self.pdf_export_modal)
        {
            self.run_pdf_export(scope, opts);
        }

        if self.photo_wizard.open {
            if let Some(wall) = self
                .project
                .walls
                .iter()
                .find(|w| w.id == self.photo_wizard.wall_id)
                .cloned()
            {
                if let Some((cfg, open_canvas_handles, opt_wall_size)) = draw_photo_setup_wizard(
                    &ctx,
                    &pal,
                    self.project_dir.as_deref(),
                    wall.width,
                    wall.height,
                    self.project.manifest.display_unit,
                    &mut self.wizard_photo_cache,
                    &mut self.photo_wizard,
                ) {
                    let comps = self.project.components.clone();
                    if let Some(w_mut) = self.project.walls.iter_mut().find(|w| w.id == wall.id) {
                        w_mut.photo = cfg;
                        if let Some((nw, nh)) = opt_wall_size {
                            w_mut.width = nw;
                            w_mut.height = nh;
                            for op in &mut w_mut.openings {
                                op.clamp_to_wall(nw, nh, &comps);
                            }
                            self.canvas2d.fitted_wall_id = None;
                        }
                    }
                    self.canvas2d.edit_photo_mode = open_canvas_handles;
                    self.commit_undo("Calibrate wall photo");
                }
            } else {
                self.photo_wizard.open = false;
            }
        }

        // Silent .autosave crash recovery modal
        if let Some(ref prompt) = self.crash_recovery_prompt.clone() {
            egui::Window::new("Unsaved Session Recovery Found")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(&ctx, |ui| {
                    ui.label(
                        RichText::new(
                            "A newer unsaved autosave snapshot was found from an unexpected exit.",
                        )
                        .color(pal.text_primary),
                    );
                    ui.label(
                        RichText::new(format!("Snapshot timestamp: {}", prompt.recovery_epoch))
                            .small()
                            .color(pal.text_muted),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let rec_btn = egui::Button::new(
                            RichText::new("Recover Unsaved Work")
                                .strong()
                                .color(pal.accent_text),
                        )
                        .fill(pal.accent);
                        if ui.add(rec_btn).clicked() {
                            self.project = prompt.recovered_project.clone();
                            self.project.ensure_valid_selection();
                            self.dirty = true;
                            self.undo.reset(self.project.clone());
                            self.crash_recovery_prompt = None;
                            self.show_toast("Recovered autosave snapshot", false);
                        }
                        if ui.button("Discard Snapshot").clicked() {
                            ProjectStore::clear_autosave(&prompt.project_dir);
                            self.crash_recovery_prompt = None;
                        }
                    });
                });
        }

        // Bottom-center toast banner
        if let Some(ref t) = self.toast {
            egui::Area::new(ui.scope_id().with("studio_toast"))
                .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -20.0))
                .order(egui::Order::Tooltip)
                .show(&ctx, |ui| {
                    egui::Frame::NONE
                        .fill(if t.is_error {
                            pal.danger
                        } else {
                            pal.hud_glass
                        })
                        .stroke(Stroke::new(1.0, pal.border_strong))
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::symmetric(14, 8))
                        .show(ui, |ui| {
                            ui.label(RichText::new(&t.message).strong().color(if t.is_error {
                                egui::Color32::WHITE
                            } else {
                                pal.text_primary
                            }));
                        });
                });
        }
    }

    fn open_project_dir_in_os(&mut self) {
        let Some(ref dir) = self.project_dir else {
            self.show_toast("Save the project first to create its folder on disk.", true);
            return;
        };
        #[cfg(target_os = "windows")]
        let res = std::process::Command::new("explorer").arg(dir).spawn();
        #[cfg(target_os = "macos")]
        let res = std::process::Command::new("open").arg(dir).spawn();
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let res = std::process::Command::new("xdg-open").arg(dir).spawn();

        if let Err(e) = res {
            self.show_toast(format!("Could not open folder: {e}"), true);
        }
    }

    fn handle_welcome_action(&mut self, act: WelcomeAction) {
        match act {
            WelcomeAction::CloseHub => {
                self.show_hub_overlay = false;
            }
            WelcomeAction::OpenGettingStartedTour => {
                self.onboarding.open_tour();
            }
            WelcomeAction::OpenNewProjectWizard {
                initial_kind,
                with_sample,
                initial_style,
                initial_template,
            } => {
                self.new_project_wizard.open_with(
                    self.prefs.display_unit,
                    self.prefs.last_projects_root.clone(),
                    initial_kind,
                    with_sample,
                    initial_style,
                    initial_template,
                );
            }
            WelcomeAction::OpenExistingFolderPrompt => {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Open Dollhouse Studio Project Folder")
                    .pick_folder()
                {
                    self.open_project_folder(dir);
                }
            }
            WelcomeAction::OpenProjectFolder(dir) => {
                self.open_project_folder(dir);
            }
            WelcomeAction::RemoveRecentProject(dir) => {
                self.prefs.remove_recent(&dir);
                self.save_preferences();
            }
            WelcomeAction::ChangeTheme(t) => {
                self.prefs.theme = t;
                self.save_preferences();
            }
            WelcomeAction::ChangeUnit(u) => {
                self.prefs.display_unit = u;
                if self.project_dir.is_none() && !self.dirty {
                    self.project.manifest.display_unit = u;
                }
                self.save_preferences();
            }
            WelcomeAction::ImportPersonalTemplatePrompt => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Import Template File")
                    .add_filter("Dollhouse Template", &["dhtemplate.json", "json"])
                    .pick_file()
                {
                    match import_template_file(&path) {
                        Ok(tpl) => {
                            let dir = AppPreferences::personal_templates_dir();
                            let _ = save_personal_template(&dir, &tpl);
                            self.personal_templates = list_personal_templates(&dir);
                            self.show_toast("Imported template into Personal Library", false);
                        }
                        Err(e) => self.show_toast(format!("Import failed: {e}"), true),
                    }
                }
            }
            WelcomeAction::ExportPersonalTemplate(tpl) => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(format!("{}.dhtemplate.json", sanitize_filename(&tpl.name)))
                    .save_file()
                {
                    match export_template_file(&tpl, &path) {
                        Ok(()) => self.show_toast("Exported .dhtemplate.json", false),
                        Err(e) => self.show_toast(format!("Export failed: {e}"), true),
                    }
                }
            }
            WelcomeAction::DeletePersonalTemplate(id) => {
                let dir = AppPreferences::personal_templates_dir();
                let _ = delete_personal_template(&dir, &id);
                self.personal_templates = list_personal_templates(&dir);
            }
        }
    }

    fn handle_ribbon_actions(&mut self, actions: Vec<RibbonAction>) {
        for act in actions {
            match act {
                RibbonAction::OpenHubOverlay => {
                    self.show_hub_overlay = true;
                }
                RibbonAction::OpenGettingStartedTour => {
                    self.onboarding.open_tour();
                }
                RibbonAction::NewProject => {
                    self.new_project_wizard.open_with(
                        self.prefs.display_unit,
                        self.prefs.last_projects_root.clone(),
                        None,
                        true,
                        None,
                        None,
                    );
                }
                RibbonAction::OpenFolder => {
                    if let Some(dir) = rfd::FileDialog::new()
                        .set_title("Open Dollhouse Studio Project Folder")
                        .pick_folder()
                    {
                        self.open_project_folder(dir);
                    }
                }
                RibbonAction::OpenProjectDirInOs => {
                    self.open_project_dir_in_os();
                }
                RibbonAction::SaveProject => self.save_current_project(),
                RibbonAction::PromoteToMultiWall => {
                    if self.project.promote_single_piece_to_multi_wall().is_some() {
                        self.viewport = ActiveViewport::Blueprint2D;
                        self.commit_undo("Promote to Multi-Wall house");
                    }
                }
                RibbonAction::Undo => self.perform_undo(),
                RibbonAction::Redo => self.perform_redo(),
                RibbonAction::ChangeUnit(u) => {
                    self.project.manifest.display_unit = u;
                    self.prefs.display_unit = u;
                    self.save_preferences();
                    self.dirty = true;
                }
                RibbonAction::ChangeTheme(t) => {
                    self.prefs.theme = t;
                    self.save_preferences();
                }
                RibbonAction::SetViewport(vp) => {
                    self.viewport = vp;
                    if vp != ActiveViewport::Blueprint2D {
                        self.canvas2d.edit_photo_mode = false;
                        self.canvas2d.dims.active_draw_mode = false;
                    }
                }
                RibbonAction::AddOpeningStyle(style) => {
                    let mut p = WindowParams::default();
                    style.apply_style_defaults(&mut p);
                    self.insert_params_onto_active_wall(style.name(), p, None);
                }
                RibbonAction::SelectOrApplyStyle(style) => {
                    if self.project.manifest.kind == ProjectKind::SinglePiece {
                        if let Some(piece) = self.project.active_piece_mut() {
                            piece.params.style = style;
                            style.apply_style_defaults(&mut piece.params);
                            self.commit_undo("Change style");
                        }
                    } else {
                        let mut p = WindowParams::default();
                        style.apply_style_defaults(&mut p);
                        self.insert_params_onto_active_wall(style.name(), p, None);
                    }
                }
                RibbonAction::NewBlankWall => {
                    let next = self.project.walls.len() + 1;
                    let wt = self.project.manifest.default_wall_thickness;
                    let at = self.project.manifest.default_acetate_thickness;
                    self.project
                        .add_wall(&format!("Wall {next}"), 380.0, 240.0, wt, at);
                    self.selected_opening_ids.clear();
                    self.canvas2d.fitted_wall_id = None;
                    self.commit_undo("Add blank wall");
                }
                RibbonAction::DuplicateActiveWall => {
                    if let Some(mut copy) = self.project.active_wall().cloned() {
                        copy.id = dh_core::new_id("wall");
                        copy.name = format!("{} Copy", copy.name);
                        for op in &mut copy.openings {
                            op.id = dh_core::new_id("open");
                        }
                        let new_id = copy.id.clone();
                        self.project.walls.push(copy);
                        self.project.manifest.active_wall_id = Some(new_id);
                        self.selected_opening_ids.clear();
                        self.commit_undo("Duplicate wall");
                    }
                }
                RibbonAction::AlignCenterH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.center_openings_horizontally(&ids, &comps);
                        self.commit_undo("Center openings horizontally");
                    }
                }
                RibbonAction::AlignDistributeH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.distribute_openings_horizontally(&ids, &comps);
                        self.commit_undo("Distribute openings");
                    }
                }
                RibbonAction::AlignMirrorH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.mirror_openings_horizontally(&ids, &comps);
                        self.commit_undo("Mirror openings");
                    }
                }
                RibbonAction::SnapDoorFloor => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    if let Some(wall) = self.project.active_wall_mut() {
                        for op in &mut wall.openings {
                            if ids.contains(&op.id) && !op.lock_position {
                                op.y_from_bottom = 0.0;
                            }
                        }
                        self.commit_undo("Snap opening to floor");
                    }
                }
                RibbonAction::ToggleArchitraves(v) => {
                    self.canvas2d.show_architrave_outlines = v;
                }
                RibbonAction::SaveSelectionAsTemplate => self.open_save_template_for_selection(),
                RibbonAction::MakeSelectedComponent => {
                    if let (Some(wid), Some(oid)) = (
                        self.project.manifest.active_wall_id.clone(),
                        self.selected_opening_ids.first().cloned(),
                    ) && let Some(cid) =
                        self.project.promote_opening_to_component(&wid, &oid, None)
                    {
                        self.active_component_id = Some(cid);
                        self.commit_undo("Promote opening to shared Component");
                    }
                }
                RibbonAction::ApplyScalePreset(sc) => {
                    self.project.manifest.scale_preset = sc;
                    if self.project.manifest.kind == ProjectKind::SinglePiece
                        && let Some(p) = self.project.active_piece_mut()
                    {
                        p.params.apply_scale_preset(sc);
                    }
                    self.commit_undo("Change scale preset");
                }
                RibbonAction::SetFilamentExterior(fc) => {
                    self.viewport3d.lock().unwrap().settings.filament_color = fc;
                }
                RibbonAction::SetFilamentDoor(fc) => {
                    self.viewport3d.lock().unwrap().settings.door_color = fc;
                }
                RibbonAction::SetFilamentWall(fc) => {
                    self.viewport3d.lock().unwrap().settings.wall_color = fc;
                }
                RibbonAction::ImportWallPhoto => self.trigger_import_wall_photo(),
                RibbonAction::SetPhotoEditMode(on) => {
                    if on {
                        self.viewport = ActiveViewport::Blueprint2D;
                        self.canvas2d.dims.active_draw_mode = false;
                    }
                    self.canvas2d.edit_photo_mode = on;
                }
                RibbonAction::SetPhotoTool(t) => {
                    self.viewport = ActiveViewport::Blueprint2D;
                    self.canvas2d.edit_photo_mode = true;
                    self.canvas2d.photo.tool = t;
                }
                RibbonAction::OpenPhotoWizard => {
                    if let Some(wall) = self.project.active_wall()
                        && wall.photo.has_photo()
                    {
                        self.photo_wizard.open_for(&wall.id, &wall.photo);
                    }
                }
                RibbonAction::FitPhotoToWall => {
                    if let Some(wall) = self.project.active_wall_mut() {
                        let (w, h) = (wall.width, wall.height);
                        wall.photo.fit_to_wall(w, h);
                        self.commit_undo("Fit photo to wall");
                    }
                }
                RibbonAction::MatchPhotoExactWall => {
                    if let Some(wall) = self.project.active_wall_mut() {
                        let (w, h) = (wall.width, wall.height);
                        wall.photo.match_wall_exact(w, h);
                        self.commit_undo("Stretch photo to exact wall");
                    }
                }
                RibbonAction::ClearWallPhoto => {
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.photo.asset_path = None;
                        wall.photo.reset_edits();
                        self.canvas2d.edit_photo_mode = false;
                        self.commit_undo("Remove wall photo");
                    }
                }
                RibbonAction::TogglePhoto2D(v) => {
                    if let Some(w) = self.project.active_wall_mut() {
                        w.photo.show_in_blueprint = v;
                        self.dirty = true;
                    }
                }
                RibbonAction::TogglePhoto3D(v) => {
                    if let Some(w) = self.project.active_wall_mut() {
                        w.photo.show_in_3d = v;
                        self.dirty = true;
                    }
                }
                RibbonAction::SetPhotoOpacity(v) => {
                    if let Some(w) = self.project.active_wall_mut() {
                        w.photo.opacity = v;
                        self.dirty = true;
                    }
                }
                RibbonAction::ToggleDrawDimension(v) => {
                    if v {
                        self.viewport = ActiveViewport::Blueprint2D;
                        self.canvas2d.edit_photo_mode = false;
                    }
                    self.canvas2d.dims.active_draw_mode = v;
                }
                RibbonAction::AutoDimensionWall => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.generate_auto_dimensions(&comps);
                        self.commit_undo("Auto-dimension wall");
                    }
                }
                RibbonAction::ClearDimensions => {
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.dimensions.clear();
                        self.commit_undo("Clear wall dimensions");
                    }
                }
                RibbonAction::ToggleGrid(v) => self.canvas2d.show_grid = v,
                RibbonAction::ToggleRulers(v) => self.canvas2d.show_rulers = v,
                RibbonAction::CameraPreset(preset) => {
                    self.viewport3d.lock().unwrap().camera.set_preset(preset);
                }
                RibbonAction::ResetCamera => {
                    self.viewport3d
                        .lock()
                        .unwrap()
                        .camera
                        .set_preset(dh_render::CameraPreset::Isometric);
                }
                RibbonAction::SetWallSolid(s) => {
                    self.viewport3d.lock().unwrap().wall_solid = s;
                }
                RibbonAction::SetPieceViewMode(vm) => {
                    self.viewport = ActiveViewport::Piece3D;
                    self.viewport3d.lock().unwrap().settings.view_mode = vm;
                }
                RibbonAction::SetExplodeDistance(d) => {
                    self.viewport3d.lock().unwrap().settings.explode_distance = d;
                }
                RibbonAction::SetDoorOpenDeg(a) => {
                    self.viewport3d.lock().unwrap().settings.door_open_deg = a;
                }
                RibbonAction::SetWireframe(w) => {
                    self.viewport3d.lock().unwrap().settings.show_wireframe = w;
                }
                RibbonAction::SetHighlightOverhangs(o) => {
                    self.viewport3d.lock().unwrap().settings.highlight_overhangs = o;
                }
                RibbonAction::ExportBookletPdf => self.pdf_export_modal.open_for(
                    &self.project,
                    self.project.manifest.display_unit,
                    PdfExportScope::ProjectBooklet,
                ),
                RibbonAction::ExportWallPdf => self.pdf_export_modal.open_for(
                    &self.project,
                    self.project.manifest.display_unit,
                    PdfExportScope::ActiveWall,
                ),
                RibbonAction::ExportPiecePdf => self.pdf_export_modal.open_for(
                    &self.project,
                    self.project.manifest.display_unit,
                    PdfExportScope::ActivePiece,
                ),
                RibbonAction::ExportPieceStlPlate => self.export_piece_stl(0),
                RibbonAction::ExportPieceStlExterior => self.export_piece_stl(1),
                RibbonAction::ExportPieceStlInterior => self.export_piece_stl(2),
                RibbonAction::PackWallPlatesStl(bed) => self.export_packed_plates(false, bed),
                RibbonAction::PackProjectPlatesStl(bed) => self.export_packed_plates(true, bed),
                RibbonAction::ExportSvgCutSheet => self.export_svg_sheet(),
            }
        }
    }

    fn export_piece_stl(&mut self, which: usize) {
        let Some((_, params)) = self.current_selected_params_for_3d() else {
            self.show_toast("No active window/door selected.", true);
            return;
        };
        let built = build_window(&params);
        let (mesh, suffix, lbl) = match which {
            1 => (built.printable_exterior_assembly(), "exterior", "Exterior"),
            2 => (built.interior_half.clone(), "interior", "Interior"),
            _ => (built.print_plate.clone(), "plate", "Print Plate"),
        };
        let dir = self.default_export_subdir("stl");
        let name = format!(
            "{}_{suffix}.stl",
            sanitize_filename(&self.project.manifest.name)
        );
        if let Some(path) = rfd::FileDialog::new()
            .set_directory(&dir)
            .set_file_name(&name)
            .add_filter("STL Mesh", &["stl"])
            .save_file()
        {
            match write_binary_stl(&mesh, &path, lbl) {
                Ok(()) => self.show_toast(format!("Exported STL: {}", path.display()), false),
                Err(e) => self.show_toast(format!("STL export failed: {e}"), true),
            }
        }
    }

    fn export_packed_plates(&mut self, whole_project: bool, bed: BedSizePreset) {
        let (bw, bh) = bed.dimensions_mm();
        let plates = if whole_project {
            pack_project_onto_build_plates(&self.project, bw, bh, 5.0)
        } else if let Some(wall) = self.project.active_wall() {
            pack_wall_onto_build_plates(wall, &self.project.components, bw, bh, 5.0)
        } else {
            Vec::new()
        };
        if plates.is_empty() {
            self.show_toast("Nothing to pack into build plates.", true);
            return;
        }
        let dir = self.default_export_subdir("plates");
        let Some(base_dir) = rfd::FileDialog::new()
            .set_title("Choose Output Folder for Packed Build Plate STLs")
            .set_directory(&dir)
            .pick_folder()
        else {
            return;
        };
        let prefix = sanitize_filename(&self.project.manifest.name);
        for (idx, plate) in plates.iter().enumerate() {
            let p = base_dir.join(format!("{prefix}_plate_{}.stl", idx + 1));
            if let Err(e) = write_binary_stl(plate, &p, &format!("Plate {}", idx + 1)) {
                self.show_toast(format!("Failed writing plate {}: {e}", idx + 1), true);
                return;
            }
        }
        self.show_toast(
            format!("Exported {} packed build plate STL(s)", plates.len()),
            false,
        );
    }

    fn export_svg_sheet(&mut self) {
        let dir = self.default_export_subdir("svg");
        let name = format!(
            "{}_1to1.svg",
            sanitize_filename(&self.project.manifest.name)
        );
        let Some(path) = rfd::FileDialog::new()
            .set_directory(&dir)
            .set_file_name(&name)
            .add_filter("SVG Vector", &["svg"])
            .save_file()
        else {
            return;
        };
        let res = if self.project.manifest.kind == ProjectKind::MultiWall
            && self.viewport != ActiveViewport::Piece3D
        {
            if let Some(wall) = self.project.active_wall() {
                export_wall_svg(wall, &self.project.components, &path)
            } else {
                return;
            }
        } else if let Some((_, params)) = self.current_selected_params_for_3d() {
            let built = build_window(&params);
            export_window_svg(&built, &self.project.manifest.name, &path)
        } else {
            return;
        };
        match res {
            Ok(()) => self.show_toast(format!("Exported 1:1 SVG: {}", path.display()), false),
            Err(e) => self.show_toast(format!("SVG export failed: {e}"), true),
        }
    }

    fn handle_explorer_actions(&mut self, actions: Vec<ExplorerAction>) {
        for act in actions {
            match act {
                ExplorerAction::SelectWall(wid) => {
                    self.project.manifest.active_wall_id = Some(wid);
                    self.selected_opening_ids.clear();
                    self.active_component_id = None;
                    self.canvas2d.edit_photo_mode = false;
                    self.canvas2d.fitted_wall_id = None;
                }
                ExplorerAction::SelectOpening {
                    wall_id,
                    opening_id,
                    toggle,
                } => {
                    self.project.manifest.active_wall_id = Some(wall_id);
                    self.active_component_id = None;
                    if toggle {
                        if let Some(pos) = self
                            .selected_opening_ids
                            .iter()
                            .position(|x| x == &opening_id)
                        {
                            self.selected_opening_ids.remove(pos);
                        } else {
                            self.selected_opening_ids.push(opening_id);
                        }
                    } else {
                        self.selected_opening_ids = vec![opening_id];
                    }
                }
                ExplorerAction::OpenOpeningIn3D {
                    wall_id,
                    opening_id,
                } => {
                    self.project.manifest.active_wall_id = Some(wall_id);
                    self.active_component_id = None;
                    self.selected_opening_ids = vec![opening_id];
                    self.viewport = ActiveViewport::Piece3D;
                }
                ExplorerAction::SelectComponent(cid) => {
                    self.active_component_id = Some(cid);
                }
                ExplorerAction::SelectPiece(pid) => {
                    self.project.manifest.active_piece_id = Some(pid);
                }
                ExplorerAction::AddBlankWall => {
                    let n = self.project.walls.len() + 1;
                    let wt = self.project.manifest.default_wall_thickness;
                    let at = self.project.manifest.default_acetate_thickness;
                    self.project
                        .add_wall(&format!("Wall {n}"), 380.0, 240.0, wt, at);
                    self.selected_opening_ids.clear();
                    self.commit_undo("Add wall");
                }
                ExplorerAction::DuplicateWall(wid) => {
                    if let Some(mut copy) = self.project.walls.iter().find(|w| w.id == wid).cloned()
                    {
                        copy.id = dh_core::new_id("wall");
                        copy.name = format!("{} Copy", copy.name);
                        for op in &mut copy.openings {
                            op.id = dh_core::new_id("open");
                        }
                        let nid = copy.id.clone();
                        self.project.walls.push(copy);
                        self.project.manifest.active_wall_id = Some(nid);
                        self.selected_opening_ids.clear();
                        self.commit_undo("Duplicate wall");
                    }
                }
                ExplorerAction::DeleteWall(wid) => {
                    if self.project.walls.len() > 1 {
                        self.project.walls.retain(|w| w.id != wid);
                        self.project.ensure_valid_selection();
                        self.selected_opening_ids.clear();
                        self.commit_undo("Delete wall");
                    }
                }
                ExplorerAction::AddStandalonePiece(style) => {
                    let mut p = WindowParams::default();
                    style.apply_style_defaults(&mut p);
                    let np =
                        StandalonePiece::new(format!("Piece {}", self.project.pieces.len() + 1), p);
                    let nid = np.id.clone();
                    self.project.pieces.push(np);
                    self.project.manifest.active_piece_id = Some(nid);
                    self.commit_undo("Add piece");
                }
                ExplorerAction::DeleteStandalonePiece(pid) => {
                    if self.project.pieces.len() > 1 {
                        self.project.pieces.retain(|p| p.id != pid);
                        self.project.ensure_valid_selection();
                        self.commit_undo("Delete piece");
                    }
                }
                ExplorerAction::PromoteSinglePieceToMultiWall => {
                    if self.project.promote_single_piece_to_multi_wall().is_some() {
                        self.viewport = ActiveViewport::Blueprint2D;
                        self.commit_undo("Promote to Multi-Wall house");
                    }
                }
                ExplorerAction::PlaceComponentOnActiveWall(cid) => {
                    if let Some(wid) = self.project.manifest.active_wall_id.clone()
                        && let Some(oid) = self
                            .project
                            .place_component_instance(&wid, &cid, None, 32.0, 56.0)
                    {
                        self.active_component_id = None;
                        self.selected_opening_ids = vec![oid];
                        self.commit_undo("Place component instance");
                    }
                }
                ExplorerAction::DuplicateOpening {
                    wall_id,
                    opening_id,
                } => {
                    self.project.manifest.active_wall_id = Some(wall_id);
                    self.selected_opening_ids = vec![opening_id];
                    self.duplicate_selected_openings();
                }
                ExplorerAction::DeleteOpening {
                    wall_id,
                    opening_id,
                } => {
                    if let Some(w) = self.project.walls.iter_mut().find(|w| w.id == wall_id) {
                        w.openings.retain(|o| o.id != opening_id);
                        self.selected_opening_ids.retain(|x| x != &opening_id);
                        self.commit_undo("Delete opening");
                    }
                }
                ExplorerAction::PromoteOpeningToComponent {
                    wall_id,
                    opening_id,
                } => {
                    if let Some(cid) =
                        self.project
                            .promote_opening_to_component(&wall_id, &opening_id, None)
                    {
                        self.active_component_id = Some(cid);
                        self.commit_undo("Make shared Component");
                    }
                }
                ExplorerAction::MakeOpeningUnique {
                    wall_id,
                    opening_id,
                } => {
                    if self.project.make_opening_unique(&wall_id, &opening_id) {
                        self.active_component_id = None;
                        self.commit_undo("Make instance unique");
                    }
                }
                ExplorerAction::DeleteComponent(cid) => {
                    self.project.delete_component(&cid);
                    if self.active_component_id.as_deref() == Some(cid.as_str()) {
                        self.active_component_id = None;
                    }
                    self.commit_undo("Delete component");
                }
                ExplorerAction::InsertTemplateOnActiveWall(tpl) => {
                    if self.project.manifest.kind == ProjectKind::SinglePiece {
                        if let Some(p) = self.project.active_piece_mut() {
                            p.params = tpl.params;
                            self.commit_undo("Apply template");
                        }
                    } else {
                        self.insert_params_onto_active_wall(&tpl.name, tpl.params, None);
                    }
                }
                ExplorerAction::ApplyTemplateStyleOnly(tpl) => {
                    if self.project.manifest.kind == ProjectKind::SinglePiece {
                        if let Some(p) = self.project.active_piece_mut() {
                            p.params.apply_style_only_from(&tpl.params);
                            self.commit_undo("Apply template style");
                        }
                    } else if let Some(ref cid) = self.active_component_id {
                        if let Some(comp) = self.project.component_mut(cid) {
                            comp.params.apply_style_only_from(&tpl.params);
                            self.commit_undo("Apply template style to component");
                        }
                    } else if let Some(oid) = self.selected_opening_ids.first().cloned() {
                        let mut linked_cid: Option<String> = None;
                        if let Some(w) = self.project.active_wall_mut()
                            && let Some(op) = w.openings.iter_mut().find(|o| o.id == oid)
                        {
                            match &mut op.source {
                                OpeningSource::Unique { params } => {
                                    params.apply_style_only_from(&tpl.params);
                                }
                                OpeningSource::Component { component_id, .. } => {
                                    linked_cid = Some(component_id.clone());
                                }
                            }
                        }
                        if let Some(cid) = linked_cid
                            && let Some(c) = self.project.component_mut(&cid)
                        {
                            c.params.apply_style_only_from(&tpl.params);
                        }
                        self.commit_undo("Apply template style");
                    }
                }
                ExplorerAction::SaveSelectionAsTemplate => self.open_save_template_for_selection(),
            }
        }
    }

    fn handle_inspector_result(&mut self, res: InspectorResult) {
        if let Some(lbl) = res.immediate_commit_label {
            self.pending_undo_label = None;
            self.commit_undo(lbl);
        } else if res.live_changed {
            self.mark_live_edit(res.drag_commit_label.unwrap_or("Edit properties"));
        }

        if let Some(act) = res.action {
            match act {
                InspectorAction::PromoteOpeningToComponent(oid) => {
                    if let Some(wid) = self.project.manifest.active_wall_id.clone()
                        && self
                            .project
                            .promote_opening_to_component(&wid, &oid, None)
                            .is_some()
                    {
                        self.commit_undo("Promote opening to Component");
                    }
                }
                InspectorAction::MakeOpeningUnique(oid) => {
                    if let Some(wid) = self.project.manifest.active_wall_id.clone()
                        && self.project.make_opening_unique(&wid, &oid)
                    {
                        self.commit_undo("Make instance unique");
                    }
                }
                InspectorAction::SaveAsTemplate => self.open_save_template_for_selection(),
                InspectorAction::AutoDimensionWall => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.generate_auto_dimensions(&comps);
                        self.commit_undo("Auto-dimension wall");
                    }
                }
                InspectorAction::SyncCommonWallSettings => {
                    if let Some(wall) = self.project.active_wall_mut() {
                        if wall.sync_common_settings {
                            wall.apply_common_settings_to_openings();
                        }
                        self.commit_undo("Sync wall common settings");
                    }
                }
                InspectorAction::EditPhotoOnCanvas => {
                    self.viewport = ActiveViewport::Blueprint2D;
                    self.canvas2d.edit_photo_mode = true;
                }
                InspectorAction::OpenPhotoWizard => {
                    if let Some(wall) = self.project.active_wall()
                        && wall.photo.has_photo()
                    {
                        self.photo_wizard.open_for(&wall.id, &wall.photo);
                    }
                }
                InspectorAction::DeleteSelectedOpenings => self.delete_selected_openings(),
                InspectorAction::AlignSelectedCenterH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.center_openings_horizontally(&ids, &comps);
                        self.commit_undo("Center openings horizontally");
                    }
                }
                InspectorAction::AlignSelectedDistributeH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.distribute_openings_horizontally(&ids, &comps);
                        self.commit_undo("Distribute openings");
                    }
                }
                InspectorAction::AlignSelectedMirrorH => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.mirror_openings_horizontally(&ids, &comps);
                        self.commit_undo("Mirror openings");
                    }
                }
            }
        }
    }

    fn handle_blueprint_actions(&mut self, actions: Vec<BlueprintAction>) {
        for act in actions {
            match act {
                BlueprintAction::SelectOpenings(ids) => {
                    self.active_component_id = None;
                    self.selected_opening_ids = ids;
                }
                BlueprintAction::DoubleClickOpen3D(id) => {
                    self.active_component_id = None;
                    self.selected_opening_ids = vec![id];
                    self.viewport = ActiveViewport::Piece3D;
                }
                BlueprintAction::MoveOpeningsAbsolute(moves) => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        let ww = wall.width;
                        let wh = wall.height;
                        for (id, nx, ny) in moves {
                            if let Some(op) = wall.openings.iter_mut().find(|o| o.id == id)
                                && !op.lock_position
                            {
                                op.x_from_left = nx;
                                op.y_from_bottom = ny;
                                op.clamp_to_wall(ww, wh, &comps);
                            }
                        }
                        wall.sync_anchored_dimensions(&comps);
                        self.dirty = true;
                    }
                }
                BlueprintAction::ResizeOpening {
                    opening_id,
                    x_from_left,
                    y_from_bottom,
                    width,
                    height,
                } => {
                    self.resize_opening_by_id(
                        &opening_id,
                        x_from_left,
                        y_from_bottom,
                        width,
                        height,
                    );
                    self.dirty = true;
                }
                BlueprintAction::ResizeWall { width, height } => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.width = width;
                        wall.height = height;
                        for op in &mut wall.openings {
                            op.clamp_to_wall(wall.width, wall.height, &comps);
                        }
                        wall.sync_anchored_dimensions(&comps);
                        self.dirty = true;
                    }
                }
                BlueprintAction::DropStyle { style, x_mm, y_mm } => {
                    let mut p = WindowParams::default();
                    style.apply_style_defaults(&mut p);
                    self.insert_params_onto_active_wall(style.name(), p, Some([x_mm, y_mm]));
                }
                BlueprintAction::DropComponent {
                    component_id,
                    x_mm,
                    y_mm,
                } => {
                    if let Some(wid) = self.project.manifest.active_wall_id.clone()
                        && let Some(oid) = self.project.place_component_instance(
                            &wid,
                            &component_id,
                            None,
                            x_mm,
                            y_mm,
                        )
                    {
                        self.active_component_id = None;
                        self.selected_opening_ids = vec![oid];
                        self.commit_undo("Drop component instance");
                    }
                }
                BlueprintAction::DropTemplate {
                    template,
                    x_mm,
                    y_mm,
                } => {
                    let tpl = *template;
                    self.insert_params_onto_active_wall(&tpl.name, tpl.params, Some([x_mm, y_mm]));
                }
                BlueprintAction::QuickAction(qa) => match qa {
                    QuickBarAction::Duplicate => self.duplicate_selected_openings(),
                    QuickBarAction::PromoteToComponent => {
                        if let (Some(wid), Some(oid)) = (
                            self.project.manifest.active_wall_id.clone(),
                            self.selected_opening_ids.first().cloned(),
                        ) && self
                            .project
                            .promote_opening_to_component(&wid, &oid, None)
                            .is_some()
                        {
                            self.commit_undo("Promote opening to Component");
                        }
                    }
                    QuickBarAction::MakeUnique => {
                        if let (Some(wid), Some(oid)) = (
                            self.project.manifest.active_wall_id.clone(),
                            self.selected_opening_ids.first().cloned(),
                        ) && self.project.make_opening_unique(&wid, &oid)
                        {
                            self.commit_undo("Make instance unique");
                        }
                    }
                    QuickBarAction::SaveAsTemplate => self.open_save_template_for_selection(),
                    QuickBarAction::ToggleLock => {
                        if let (Some(wall), Some(oid)) = (
                            self.project.active_wall_mut(),
                            self.selected_opening_ids.first().cloned(),
                        ) && let Some(op) = wall.openings.iter_mut().find(|o| o.id == oid)
                        {
                            let next = !(op.lock_position || op.lock_width || op.lock_height);
                            op.lock_position = next;
                            op.lock_width = next;
                            op.lock_height = next;
                            self.commit_undo(if next {
                                "Lock opening"
                            } else {
                                "Unlock opening"
                            });
                        }
                    }
                    QuickBarAction::Inspect3D => {
                        self.viewport = ActiveViewport::Piece3D;
                    }
                    QuickBarAction::Delete => self.delete_selected_openings(),
                },
                BlueprintAction::DriveDimension { dim_id, target_mm } => {
                    let Some(wid) = self.project.manifest.active_wall_id.clone() else {
                        continue;
                    };
                    let mut comps = self.project.components.clone();
                    let res =
                        if let Some(wall) = self.project.walls.iter_mut().find(|w| w.id == wid) {
                            wall.apply_driving_dimension(&dim_id, target_mm, &mut comps)
                        } else {
                            Err("Active wall not found".to_string())
                        };
                    match res {
                        Ok(lbl) => {
                            self.project.components = comps;
                            self.commit_undo(lbl);
                        }
                        Err(msg) => self.show_toast(msg, true),
                    }
                }
                BlueprintAction::ImportWallPhoto => self.trigger_import_wall_photo(),
                BlueprintAction::AutoDimensionWall => {
                    let comps = self.project.components.clone();
                    if let Some(wall) = self.project.active_wall_mut() {
                        wall.generate_auto_dimensions(&comps);
                        self.commit_undo("Auto-dimension wall");
                    }
                }
                BlueprintAction::SnapSelectedToFloor => {
                    let ids: HashSet<String> = self.selected_opening_ids.iter().cloned().collect();
                    if let Some(wall) = self.project.active_wall_mut() {
                        for op in &mut wall.openings {
                            if ids.contains(&op.id) && !op.lock_position {
                                op.y_from_bottom = 0.0;
                            }
                        }
                        self.commit_undo("Snap opening to floor");
                    }
                }
                BlueprintAction::ToggleArchitraves(v) => {
                    self.canvas2d.show_architrave_outlines = v;
                }
                BlueprintAction::OpenPhotoWizard => {
                    if let Some(wall) = self.project.active_wall()
                        && wall.photo.has_photo()
                    {
                        self.photo_wizard.open_for(&wall.id, &wall.photo);
                    }
                }
                BlueprintAction::CommitUndo(lbl) => {
                    self.commit_undo(lbl);
                }
            }
        }
    }
}

impl eframe::App for DollhouseStudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_ui(ui);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        if let Some(gl_ctx) = gl
            && let Ok(mut st) = self.viewport3d.lock()
        {
            st.destroy_gl(gl_ctx);
        }
    }
}

fn quick_wall_hash(wall: &dh_core::ProjectWall, components: &[ComponentDef]) -> u64 {
    #[inline]
    fn mix(h: &mut u64, v: u64) {
        *h ^= v;
        *h = h.wrapping_mul(0x100000001b3);
    }
    let mut h: u64 = 0xcbf29ce484222325;
    mix(&mut h, wall.width.to_bits());
    mix(&mut h, wall.height.to_bits());
    mix(&mut h, wall.wall_thickness.to_bits());
    mix(&mut h, wall.acetate_thickness.to_bits());
    mix(&mut h, wall.openings.len() as u64);
    for op in &wall.openings {
        mix(&mut h, op.x_from_left.to_bits());
        mix(&mut h, op.y_from_bottom.to_bits());
        let p = op.resolve_params(components);
        mix(&mut h, p.style as u64);
        mix(&mut h, p.opening_width.to_bits());
        mix(&mut h, p.opening_height.to_bits());
        mix(&mut h, p.wall_thickness.to_bits());
        mix(&mut h, p.trim_width.to_bits());
        mix(&mut h, p.trim_thickness.to_bits());
        mix(&mut h, p.architrave_style as u64);
        mix(&mut h, p.groove_style as u64);
        mix(&mut h, p.shutter_style as u64);
        mix(&mut h, p.header_style as u64);
        mix(&mut h, p.flush_top_mode as u64);
        mix(&mut h, u64::from(p.flush_bottom_sill));
        mix(&mut h, u64::from(p.flush_left_jamb));
        mix(&mut h, u64::from(p.flush_right_jamb));
        mix(&mut h, u64::from(p.sill_enabled));
        mix(&mut h, u64::from(p.flower_box_enabled));
        mix(&mut h, u64::from(p.double_doors));
        mix(&mut h, u64::from(p.hinge_right));
        mix(&mut h, p.bars_x as u64);
        mix(&mut h, p.bars_y as u64);
        mix(&mut h, p.arch_spokes as u64);
        mix(&mut h, p.window_opening_mode as u64);
    }
    h
}
