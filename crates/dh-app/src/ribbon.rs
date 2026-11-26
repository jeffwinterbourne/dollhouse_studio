use crate::canvas2d::{CanvasDragPayload, PhotoCanvasTool};
use crate::icons::{VectorIcon, draw_style_thumbnail, paint_icon};
use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{
    DisplayUnit, FilamentColor, Project, ProjectKind, ScalePreset, UndoHistory, ViewMode,
    WindowStyle,
};
use dh_export::BedSizePreset;
use dh_project::ThemeMode;
use dh_render::{CameraPreset, ViewportSettings};
use eframe::egui::{self, Color32, CornerRadius, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveViewport {
    #[default]
    Blueprint2D,
    Wall3D,
    Piece3D,
}

impl ActiveViewport {
    pub fn label(self) -> &'static str {
        match self {
            ActiveViewport::Blueprint2D => "Blueprint 2D",
            ActiveViewport::Wall3D => "Wall 3D",
            ActiveViewport::Piece3D => "Piece 3D",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonTab {
    File,
    #[default]
    Home,
    Design,
    Photo,
    Annotate,
    View,
    Export,
}

impl RibbonTab {
    pub const ALL: [RibbonTab; 7] = [
        RibbonTab::File,
        RibbonTab::Home,
        RibbonTab::Design,
        RibbonTab::Photo,
        RibbonTab::Annotate,
        RibbonTab::View,
        RibbonTab::Export,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RibbonTab::File => "File",
            RibbonTab::Home => "Home",
            RibbonTab::Design => "Design",
            RibbonTab::Photo => "Photo",
            RibbonTab::Annotate => "Annotate",
            RibbonTab::View => "View",
            RibbonTab::Export => "Export",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesignStyleFilter {
    #[default]
    All,
    Windows,
    Doors,
}

pub struct RibbonState {
    pub active_tab: RibbonTab,
    pub style_filter: DesignStyleFilter,
    pub bed_preset: BedSizePreset,
}

impl Default for RibbonState {
    fn default() -> Self {
        Self {
            active_tab: RibbonTab::Home,
            style_filter: DesignStyleFilter::All,
            bed_preset: BedSizePreset::Bambu256,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RibbonAction {
    GoHomeHub,
    NewProject,
    OpenFolder,
    SaveProject,
    PromoteToMultiWall,
    Undo,
    Redo,
    ChangeUnit(DisplayUnit),
    ChangeTheme(ThemeMode),
    SetViewport(ActiveViewport),

    // Home / Wall editing
    AddOpeningStyle(WindowStyle),
    SelectOrApplyStyle(WindowStyle),
    NewBlankWall,
    DuplicateActiveWall,
    AlignCenterH,
    AlignDistributeH,
    AlignMirrorH,
    SnapDoorFloor,

    // Design
    SaveSelectionAsTemplate,
    MakeSelectedComponent,
    ApplyScalePreset(ScalePreset),
    SetFilamentExterior(FilamentColor),
    SetFilamentDoor(FilamentColor),
    SetFilamentWall(FilamentColor),

    // Photo
    ImportWallPhoto,
    SetPhotoEditMode(bool),
    SetPhotoTool(PhotoCanvasTool),
    OpenPhotoWizard,
    FitPhotoToWall,
    MatchPhotoExactWall,
    ClearWallPhoto,
    TogglePhoto2D(bool),
    TogglePhoto3D(bool),
    SetPhotoOpacity(f32),

    // Annotate
    ToggleDrawDimension(bool),
    AutoDimensionWall,
    ClearDimensions,
    ToggleGrid(bool),
    ToggleRulers(bool),

    // View / 3D
    CameraPreset(CameraPreset),
    ResetCamera,
    SetWallSolid(bool),
    SetPieceViewMode(ViewMode),
    SetExplodeDistance(f32),
    SetDoorOpenDeg(f32),
    SetWireframe(bool),
    SetHighlightOverhangs(bool),

    // Export
    ExportBookletPdf,
    ExportWallPdf,
    ExportPiecePdf,
    ExportPieceStlPlate,
    ExportPieceStlExterior,
    ExportPieceStlInterior,
    PackWallPlatesStl(BedSizePreset),
    PackProjectPlatesStl(BedSizePreset),
    ExportSvgCutSheet,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_top_ribbon(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    dirty: bool,
    undo: &UndoHistory<Project>,
    unit: DisplayUnit,
    theme_mode: ThemeMode,
    viewport: ActiveViewport,
    selected_count: usize,
    photo_edit_mode: bool,
    photo_tool: PhotoCanvasTool,
    draw_dim_mode: bool,
    show_grid: bool,
    show_rulers: bool,
    view3d: &ViewportSettings,
    wall_solid: bool,
    state: &mut RibbonState,
) -> Vec<RibbonAction> {
    let mut actions = Vec::new();
    let is_multi = project.manifest.kind == ProjectKind::MultiWall;

    // 1) Quick Access Header Bar
    egui::Frame::NONE
        .fill(pal.surface_ribbon)
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Hub")
                    .on_hover_text("Return to Start / Project Hub")
                    .clicked()
                {
                    actions.push(RibbonAction::GoHomeHub);
                }
                ui.separator();

                let title = if dirty {
                    format!("{}*", project.manifest.name)
                } else {
                    project.manifest.name.clone()
                };
                ui.label(RichText::new(title).strong().size(14.0));
                ui.label(
                    RichText::new(project.manifest.scale_preset.short_label())
                        .small()
                        .color(pal.accent),
                );
                ui.separator();

                let save_btn = egui::Button::new(
                    RichText::new("Save")
                        .strong()
                        .color(if dirty { pal.accent_text } else { pal.text_primary }),
                )
                .fill(if dirty { pal.accent } else { pal.surface_card })
                .corner_radius(CornerRadius::same(5));
                if ui
                    .add(save_btn)
                    .on_hover_text("Save Project Folder (Ctrl+S)")
                    .clicked()
                {
                    actions.push(RibbonAction::SaveProject);
                }

                let undo_tip = undo
                    .undo_label()
                    .map(|l| format!("Undo: {l} (Ctrl+Z)"))
                    .unwrap_or_else(|| "Nothing to undo".to_string());
                if ui
                    .add_enabled(undo.can_undo(), egui::Button::new("Undo"))
                    .on_hover_text(undo_tip)
                    .clicked()
                {
                    actions.push(RibbonAction::Undo);
                }

                let redo_tip = undo
                    .redo_label()
                    .map(|l| format!("Redo: {l} (Ctrl+Y)"))
                    .unwrap_or_else(|| "Nothing to redo".to_string());
                if ui
                    .add_enabled(undo.can_redo(), egui::Button::new("Redo"))
                    .on_hover_text(redo_tip)
                    .clicked()
                {
                    actions.push(RibbonAction::Redo);
                }

                ui.separator();

                // Primary Viewport Switcher
                if is_multi {
                    let mut vp = viewport;
                    if segmented_pill(
                        ui,
                        pal,
                        &mut vp,
                        &[
                            (ActiveViewport::Blueprint2D, "Blueprint 2D"),
                            (ActiveViewport::Wall3D, "Wall 3D"),
                            (ActiveViewport::Piece3D, "Piece 3D"),
                        ],
                    ) {
                        actions.push(RibbonAction::SetViewport(vp));
                    }
                } else {
                    ui.label(RichText::new("Piece 3D View").strong().color(pal.accent));
                    if ui
                        .small_button("Promote to House / Multi-Wall")
                        .on_hover_text("Expand this single object into a multi-wall house project")
                        .clicked()
                    {
                        actions.push(RibbonAction::PromoteToMultiWall);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut t = theme_mode;
                    if segmented_pill(
                        ui,
                        pal,
                        &mut t,
                        &[
                            (ThemeMode::System, "OS"),
                            (ThemeMode::Light, "Light"),
                            (ThemeMode::Dark, "Dark"),
                        ],
                    ) {
                        actions.push(RibbonAction::ChangeTheme(t));
                    }

                    let mut u = unit;
                    if segmented_pill(
                        ui,
                        pal,
                        &mut u,
                        &[
                            (DisplayUnit::Mm, "mm"),
                            (DisplayUnit::Cm, "cm"),
                            (DisplayUnit::Inches, "in"),
                        ],
                    ) {
                        actions.push(RibbonAction::ChangeUnit(u));
                    }
                });
            });

            ui.add_space(4.0);

            // 2) Ribbon Tab Selector Strip
            ui.horizontal(|ui| {
                for tab in RibbonTab::ALL {
                    if !is_multi && matches!(tab, RibbonTab::Photo | RibbonTab::Annotate) {
                        continue;
                    }
                    let active = state.active_tab == tab;
                    let text = RichText::new(tab.label())
                        .size(13.0)
                        .strong()
                        .color(if active { pal.accent } else { pal.text_secondary });
                    let btn = egui::Button::new(text)
                        .fill(if active { pal.surface_card } else { Color32::TRANSPARENT })
                        .stroke(if active {
                            Stroke::new(1.0, pal.border)
                        } else {
                            Stroke::NONE
                        })
                        .corner_radius(CornerRadius {
                            nw: 6,
                            ne: 6,
                            sw: 0,
                            se: 0,
                        });
                    if ui.add(btn).clicked() {
                        state.active_tab = tab;
                    }
                }
            });
        });

    // 3) Contextual Ribbon Command Strip
    egui::Frame::NONE
        .fill(pal.surface_card)
        .stroke(Stroke::new(1.0, pal.border))
        .inner_margin(Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| match state.active_tab {
                RibbonTab::File => {
                    if ui.button("New Project Folder...").clicked() {
                        actions.push(RibbonAction::NewProject);
                    }
                    if ui.button("Open Project Folder...").clicked() {
                        actions.push(RibbonAction::OpenFolder);
                    }
                    if ui.button("Save Project (Ctrl+S)").clicked() {
                        actions.push(RibbonAction::SaveProject);
                    }
                    ui.separator();
                    if !is_multi && ui.button("Promote to House / Multi-Wall").clicked() {
                        actions.push(RibbonAction::PromoteToMultiWall);
                    }
                    if ui.button("Close to Start Hub").clicked() {
                        actions.push(RibbonAction::GoHomeHub);
                    }
                }
                RibbonTab::Home => {
                    if is_multi {
                        ribbon_group(ui, pal, "Insert Opening", |ui| {
                            if ribbon_icon_button(ui, pal, VectorIcon::Window, "Add Window") {
                                actions.push(RibbonAction::AddOpeningStyle(WindowStyle::GeorgianGrid));
                            }
                            if ribbon_icon_button(ui, pal, VectorIcon::Door, "Add Door") {
                                actions.push(RibbonAction::AddOpeningStyle(WindowStyle::DoorColonial4Panel));
                            }
                        });
                        ui.separator();
                        ribbon_group(ui, pal, "Walls", |ui| {
                            if ribbon_icon_button(ui, pal, VectorIcon::Wall, "New Wall") {
                                actions.push(RibbonAction::NewBlankWall);
                            }
                            if ui.button("Duplicate Wall").clicked() {
                                actions.push(RibbonAction::DuplicateActiveWall);
                            }
                        });
                        ui.separator();
                        ribbon_group(ui, pal, "Arrange Selection", |ui| {
                            let en = selected_count > 0;
                            if ui.add_enabled(en, egui::Button::new("Center H")).clicked() {
                                actions.push(RibbonAction::AlignCenterH);
                            }
                            if ui
                                .add_enabled(selected_count >= 2, egui::Button::new("Distribute"))
                                .clicked()
                            {
                                actions.push(RibbonAction::AlignDistributeH);
                            }
                            if ui.add_enabled(en, egui::Button::new("Mirror")).clicked() {
                                actions.push(RibbonAction::AlignMirrorH);
                            }
                            if ui
                                .add_enabled(en, egui::Button::new("Snap Floor"))
                                .on_hover_text("Drop selected opening bottom to floor (Y = 0)")
                                .clicked()
                            {
                                actions.push(RibbonAction::SnapDoorFloor);
                            }
                        });
                    } else {
                        ribbon_group(ui, pal, "Templates & Reuse", |ui| {
                            if ui.button("Save As Template...").clicked() {
                                actions.push(RibbonAction::SaveSelectionAsTemplate);
                            }
                            if ui.button("Promote to Multi-Wall").clicked() {
                                actions.push(RibbonAction::PromoteToMultiWall);
                            }
                        });
                    }
                }
                RibbonTab::Design => {
                    ribbon_group(ui, pal, "Filter", |ui| {
                        let mut f = state.style_filter;
                        if segmented_pill(
                            ui,
                            pal,
                            &mut f,
                            &[
                                (DesignStyleFilter::All, "All"),
                                (DesignStyleFilter::Windows, "Win"),
                                (DesignStyleFilter::Doors, "Door"),
                            ],
                        ) {
                            state.style_filter = f;
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::horizontal()
                        .id_salt("design_style_strip")
                        .max_width((ui.available_width() - 320.0).max(200.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let styles: &[WindowStyle] = match state.style_filter {
                                    DesignStyleFilter::All => &WindowStyle::ALL,
                                    DesignStyleFilter::Windows => &WindowStyle::ALL_WINDOWS,
                                    DesignStyleFilter::Doors => &WindowStyle::ALL_DOORS,
                                };
                                for &style in styles {
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::splat(40.0),
                                        Sense::click_and_drag(),
                                    );
                                    draw_style_thumbnail(ui.painter(), rect, style, false, pal);
                                    if resp.drag_started() {
                                        egui::DragAndDrop::set_payload(
                                            ui.ctx(),
                                            CanvasDragPayload::Style(style),
                                        );
                                    }
                                    let resp = resp.on_hover_text(format!(
                                        "{} (click to add/apply, or drag onto Blueprint 2D)",
                                        style.name()
                                    ));
                                    if resp.clicked() {
                                        actions.push(RibbonAction::SelectOrApplyStyle(style));
                                    }
                                }
                            });
                        });

                    ui.separator();
                    ribbon_group(ui, pal, "Actions & Scale", |ui| {
                        if ui.button("Save Template...").clicked() {
                            actions.push(RibbonAction::SaveSelectionAsTemplate);
                        }
                        if is_multi && selected_count == 1 && ui.button("Make Component").clicked() {
                            actions.push(RibbonAction::MakeSelectedComponent);
                        }
                        egui::ComboBox::from_id_salt("ribbon_scale")
                            .selected_text(project.manifest.scale_preset.short_label())
                            .width(68.0)
                            .show_ui(ui, |ui| {
                                for sc in ScalePreset::ALL {
                                    if ui
                                        .selectable_label(project.manifest.scale_preset == sc, sc.label())
                                        .clicked()
                                    {
                                        actions.push(RibbonAction::ApplyScalePreset(sc));
                                    }
                                }
                            });
                    });
                }
                RibbonTab::Photo => {
                    let wall_opt = project.active_wall();
                    let has_photo = wall_opt.is_some_and(|w| w.photo.has_photo());
                    ribbon_group(ui, pal, "Photo Import", |ui| {
                        if ribbon_icon_button(ui, pal, VectorIcon::Photo, "Import Photo...") {
                            actions.push(RibbonAction::ImportWallPhoto);
                        }
                        if has_photo && ui.button("Wizard...").clicked() {
                            actions.push(RibbonAction::OpenPhotoWizard);
                        }
                    });
                    ui.separator();
                    if let Some(wall) = wall_opt {
                        if has_photo {
                            ribbon_group(ui, pal, "Mouse Tools", |ui| {
                                let mut edit = photo_edit_mode;
                                if ui.toggle_value(&mut edit, "Edit on Canvas").changed() {
                                    actions.push(RibbonAction::SetPhotoEditMode(edit));
                                }
                                for tool in [
                                    PhotoCanvasTool::MoveStretch,
                                    PhotoCanvasTool::Rotate,
                                    PhotoCanvasTool::PerspectivePins,
                                    PhotoCanvasTool::Crop,
                                    PhotoCanvasTool::TwoPointCalibrate,
                                ] {
                                    let active = photo_edit_mode && photo_tool == tool;
                                    if ui.selectable_label(active, tool.label()).clicked() {
                                        actions.push(RibbonAction::SetPhotoEditMode(true));
                                        actions.push(RibbonAction::SetPhotoTool(tool));
                                    }
                                }
                            });
                            ui.separator();
                            ribbon_group(ui, pal, "Visibility & Fit", |ui| {
                                let mut s2 = wall.photo.show_in_blueprint;
                                if ui.checkbox(&mut s2, "2D").changed() {
                                    actions.push(RibbonAction::TogglePhoto2D(s2));
                                }
                                let mut s3 = wall.photo.show_in_3d;
                                if ui.checkbox(&mut s3, "3D").changed() {
                                    actions.push(RibbonAction::TogglePhoto3D(s3));
                                }
                                let mut op = wall.photo.opacity;
                                ui.spacing_mut().slider_width = 72.0;
                                if ui.add(egui::Slider::new(&mut op, 0.1..=1.0).show_value(false)).changed() {
                                    actions.push(RibbonAction::SetPhotoOpacity(op));
                                }
                                if ui.button("Fit Wall").clicked() {
                                    actions.push(RibbonAction::FitPhotoToWall);
                                }
                                if ui.button("Exact").clicked() {
                                    actions.push(RibbonAction::MatchPhotoExactWall);
                                }
                                if ui.button("Clear").clicked() {
                                    actions.push(RibbonAction::ClearWallPhoto);
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new("Import a reference photo of a dollhouse wall to calibrate, rectify perspective pins, and trace openings.")
                                    .color(pal.text_secondary),
                            );
                        }
                    }
                }
                RibbonTab::Annotate => {
                    ribbon_group(ui, pal, "Dimensions", |ui| {
                        let mut d = draw_dim_mode;
                        if ui.toggle_value(&mut d, "Draw Dimension").changed() {
                            actions.push(RibbonAction::ToggleDrawDimension(d));
                        }
                        if ui.button("Auto-Dimension Wall").clicked() {
                            actions.push(RibbonAction::AutoDimensionWall);
                        }
                        if ui.button("Clear All").clicked() {
                            actions.push(RibbonAction::ClearDimensions);
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "Canvas Guides", |ui| {
                        let mut g = show_grid;
                        if ui.checkbox(&mut g, "Grid").changed() {
                            actions.push(RibbonAction::ToggleGrid(g));
                        }
                        let mut r = show_rulers;
                        if ui.checkbox(&mut r, "Rulers").changed() {
                            actions.push(RibbonAction::ToggleRulers(r));
                        }
                    });
                }
                RibbonTab::View => {
                    ribbon_group(ui, pal, "Camera Presets", |ui| {
                        for (preset, lbl) in [
                            (CameraPreset::Isometric, "Iso"),
                            (CameraPreset::Front, "Front"),
                            (CameraPreset::Back, "Back"),
                            (CameraPreset::Left, "Left"),
                            (CameraPreset::Right, "Right"),
                            (CameraPreset::Top, "Top"),
                        ] {
                            if ui.small_button(lbl).clicked() {
                                actions.push(RibbonAction::CameraPreset(preset));
                            }
                        }
                        if ui.small_button("Reset").clicked() {
                            actions.push(RibbonAction::ResetCamera);
                        }
                    });
                    ui.separator();
                    if is_multi && viewport == ActiveViewport::Wall3D {
                        ribbon_group(ui, pal, "Wall 3D", |ui| {
                            let mut sol = wall_solid;
                            if ui.checkbox(&mut sol, "Solid Wall Cutaway").changed() {
                                actions.push(RibbonAction::SetWallSolid(sol));
                            }
                        });
                        ui.separator();
                    }
                    ribbon_group(ui, pal, "Piece 3D Sandwich", |ui| {
                        for vm in ViewMode::ALL {
                            if ui
                                .selectable_label(view3d.view_mode == vm, vm.short_label())
                                .clicked()
                            {
                                actions.push(RibbonAction::SetPieceViewMode(vm));
                            }
                        }
                        let mut ex = view3d.explode_distance;
                        ui.spacing_mut().slider_width = 72.0;
                        if ui
                            .add(egui::Slider::new(&mut ex, 0.0..=60.0).text("Explode"))
                            .changed()
                        {
                            actions.push(RibbonAction::SetExplodeDistance(ex));
                        }
                        let mut da = view3d.door_open_deg;
                        if ui
                            .add(egui::Slider::new(&mut da, 0.0..=110.0).text("Open"))
                            .changed()
                        {
                            actions.push(RibbonAction::SetDoorOpenDeg(da));
                        }
                    });
                    ui.separator();
                    let mut wf = view3d.show_wireframe;
                    if ui.checkbox(&mut wf, "Wire").changed() {
                        actions.push(RibbonAction::SetWireframe(wf));
                    }
                    let mut ov = view3d.highlight_overhangs;
                    if ui.checkbox(&mut ov, "Overhangs").changed() {
                        actions.push(RibbonAction::SetHighlightOverhangs(ov));
                    }
                }
                RibbonTab::Export => {
                    ribbon_group(ui, pal, "1:1 Scale PDFs", |ui| {
                        if ribbon_icon_button(ui, pal, VectorIcon::ExportPdf, "PDF Booklet...") {
                            actions.push(RibbonAction::ExportBookletPdf);
                        }
                        if is_multi && ui.button("Active Wall PDF...").clicked() {
                            actions.push(RibbonAction::ExportWallPdf);
                        }
                        if ui.button("Active Piece PDF...").clicked() {
                            actions.push(RibbonAction::ExportPiecePdf);
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "3D Print STL", |ui| {
                        if ribbon_icon_button(ui, pal, VectorIcon::ExportStl, "Piece Plate STL") {
                            actions.push(RibbonAction::ExportPieceStlPlate);
                        }
                        if ui.small_button("Ext STL").clicked() {
                            actions.push(RibbonAction::ExportPieceStlExterior);
                        }
                        if ui.small_button("Int STL").clicked() {
                            actions.push(RibbonAction::ExportPieceStlInterior);
                        }
                        egui::ComboBox::from_id_salt("ribbon_bed")
                            .selected_text(state.bed_preset.short_label())
                            .width(66.0)
                            .show_ui(ui, |ui| {
                                for bp in BedSizePreset::ALL {
                                    ui.selectable_value(&mut state.bed_preset, bp, bp.label());
                                }
                            });
                        if is_multi && ui.button("Pack Wall STL").clicked() {
                            actions.push(RibbonAction::PackWallPlatesStl(state.bed_preset));
                        }
                        if ui.button("Pack Project STL").clicked() {
                            actions.push(RibbonAction::PackProjectPlatesStl(state.bed_preset));
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "CAD / Laser", |ui| {
                        if ui.button("Export 1:1 SVG...").clicked() {
                            actions.push(RibbonAction::ExportSvgCutSheet);
                        }
                    });
                }
            });
        });

    actions
}

fn ribbon_group(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.vertical(|ui| {
        ui.horizontal(add_contents);
        ui.label(RichText::new(title).size(10.5).color(pal.text_muted));
    });
}

fn ribbon_icon_button(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    icon: VectorIcon,
    label: &str,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let btn = egui::Button::new(RichText::new(label).size(12.8))
            .fill(pal.surface_panel)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(CornerRadius::same(5));
        let resp = ui.add(btn);
        let ir = Rect::from_min_size(
            Pos2::new(resp.rect.left() - 2.0, resp.rect.center().y - 7.0),
            Vec2::splat(14.0),
        );
        let _ = (ir, icon, paint_icon as fn(&egui::Painter, Rect, VectorIcon, Color32));
        if resp.clicked() {
            clicked = true;
        }
    });
    clicked
}
