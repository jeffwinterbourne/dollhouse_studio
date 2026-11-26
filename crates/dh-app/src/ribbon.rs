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
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Margin, Rect, RichText, Sense, Stroke, Vec2,
};

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
    OpenHubOverlay,
    OpenGettingStartedTour,
    NewProject,
    OpenFolder,
    OpenProjectDirInOs,
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
    ToggleArchitraves(bool),

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
    has_project_dir: bool,
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
    show_architraves: bool,
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
                if ribbon_pill_btn(ui, pal, VectorIcon::Hub, "Hub", false, true)
                    .on_hover_text("Open Floating Project & Template Hub")
                    .clicked()
                {
                    actions.push(RibbonAction::OpenHubOverlay);
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

                if ribbon_pill_btn(ui, pal, VectorIcon::Save, "Save", dirty, true)
                    .on_hover_text("Save Project Folder (Ctrl+S)")
                    .clicked()
                {
                    actions.push(RibbonAction::SaveProject);
                }

                let undo_tip = undo
                    .undo_label()
                    .map(|l| format!("Undo: {l} (Ctrl+Z)"))
                    .unwrap_or_else(|| "Nothing to undo".to_string());
                if ribbon_pill_btn(ui, pal, VectorIcon::Undo, "Undo", false, undo.can_undo())
                    .on_hover_text(undo_tip)
                    .clicked()
                {
                    actions.push(RibbonAction::Undo);
                }

                let redo_tip = undo
                    .redo_label()
                    .map(|l| format!("Redo: {l} (Ctrl+Y)"))
                    .unwrap_or_else(|| "Nothing to redo".to_string());
                if ribbon_pill_btn(ui, pal, VectorIcon::Redo, "Redo", false, undo.can_redo())
                    .on_hover_text(redo_tip)
                    .clicked()
                {
                    actions.push(RibbonAction::Redo);
                }

                ui.separator();

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
                    if ribbon_pill_btn(
                        ui,
                        pal,
                        VectorIcon::Wall,
                        "Promote to Multi-Wall",
                        false,
                        true,
                    )
                    .clicked()
                    {
                        actions.push(RibbonAction::PromoteToMultiWall);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ribbon_pill_btn(ui, pal, VectorIcon::HelpTour, "Tour", false, true)
                        .on_hover_text("Open interactive Getting Started Walkthrough")
                        .clicked()
                    {
                        actions.push(RibbonAction::OpenGettingStartedTour);
                    }

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

            // 2) Ribbon Tab Selector Strip (defaults to Home)
            ui.horizontal(|ui| {
                for tab in RibbonTab::ALL {
                    if !is_multi && matches!(tab, RibbonTab::Photo | RibbonTab::Annotate) {
                        continue;
                    }
                    let active = state.active_tab == tab;
                    let text = RichText::new(tab.label())
                        .size(13.0)
                        .strong()
                        .color(if active {
                            pal.accent
                        } else {
                            pal.text_secondary
                        });
                    let btn = egui::Button::new(text)
                        .fill(if active {
                            pal.surface_card
                        } else {
                            Color32::TRANSPARENT
                        })
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

    // 3) Contextual Ribbon Command Strip with Vector Icons
    egui::Frame::NONE
        .fill(pal.surface_card)
        .stroke(Stroke::new(1.0, pal.border))
        .inner_margin(Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| match state.active_tab {
                RibbonTab::File => {
                    ribbon_group(ui, pal, "Project", |ui| {
                        if ribbon_tile(ui, pal, VectorIcon::Plus, "New Project", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::NewProject);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::Folder, "Open Folder", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::OpenFolder);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::Save, "Save", dirty, true).clicked() {
                            actions.push(RibbonAction::SaveProject);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::Hub, "Project Hub", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::OpenHubOverlay);
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "Workspace & Share", |ui| {
                        if ribbon_tile(
                            ui,
                            pal,
                            VectorIcon::OpenExternal,
                            "Show Folder",
                            false,
                            has_project_dir,
                        )
                        .on_hover_text("Reveal project folder on disk")
                        .clicked()
                        {
                            actions.push(RibbonAction::OpenProjectDirInOs);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::ExportPdf, "Booklet PDF", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ExportBookletPdf);
                        }
                        if !is_multi
                            && ribbon_tile(ui, pal, VectorIcon::Wall, "Make House", false, true)
                                .clicked()
                        {
                            actions.push(RibbonAction::PromoteToMultiWall);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::HelpTour, "Quick Tour", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::OpenGettingStartedTour);
                        }
                    });
                }
                RibbonTab::Home => {
                    if is_multi {
                        ribbon_group(ui, pal, "Insert Opening", |ui| {
                            if ribbon_tile(ui, pal, VectorIcon::Window, "Add Window", false, true)
                                .clicked()
                            {
                                actions.push(RibbonAction::AddOpeningStyle(
                                    WindowStyle::GeorgianGrid,
                                ));
                            }
                            if ribbon_tile(ui, pal, VectorIcon::Door, "Add Door", false, true)
                                .clicked()
                            {
                                actions.push(RibbonAction::AddOpeningStyle(
                                    WindowStyle::DoorColonial4Panel,
                                ));
                            }
                            if ribbon_tile(
                                ui,
                                pal,
                                VectorIcon::Photo,
                                "Wall Photo",
                                false,
                                true,
                            )
                            .on_hover_text("Import reference wall photo")
                            .clicked()
                            {
                                actions.push(RibbonAction::ImportWallPhoto);
                            }
                        });
                        ui.separator();
                        ribbon_group(ui, pal, "Walls & Tolerances", |ui| {
                            if ribbon_tile(ui, pal, VectorIcon::Wall, "New Wall", false, true)
                                .clicked()
                            {
                                actions.push(RibbonAction::NewBlankWall);
                            }
                            if ribbon_tile(
                                ui,
                                pal,
                                VectorIcon::Duplicate,
                                "Copy Wall",
                                false,
                                true,
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::DuplicateActiveWall);
                            }
                            if ribbon_tile(
                                ui,
                                pal,
                                VectorIcon::Architrave,
                                "Architraves",
                                show_architraves,
                                true,
                            )
                            .on_hover_text(
                                "Toggle outer trim/architrave outlines & spacing overlap warnings on Blueprint",
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::ToggleArchitraves(!show_architraves));
                            }
                        });
                        ui.separator();
                        ribbon_group(ui, pal, "Arrange Selection", |ui| {
                            let en = selected_count > 0;
                            if ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::AlignCenter,
                                "Center H",
                                false,
                                en,
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::AlignCenterH);
                            }
                            if ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::Distribute,
                                "Distribute",
                                false,
                                selected_count >= 2,
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::AlignDistributeH);
                            }
                            if ribbon_pill_btn(ui, pal, VectorIcon::Mirror, "Mirror", false, en)
                                .clicked()
                            {
                                actions.push(RibbonAction::AlignMirrorH);
                            }
                            if ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::SnapFloor,
                                "Floor Y=0",
                                false,
                                en,
                            )
                            .on_hover_text("Drop selected opening bottom to floor (Y = 0)")
                            .clicked()
                            {
                                actions.push(RibbonAction::SnapDoorFloor);
                            }
                        });
                    } else {
                        ribbon_group(ui, pal, "Piece & Reuse", |ui| {
                            if ribbon_tile(
                                ui,
                                pal,
                                VectorIcon::Template,
                                "Save Template",
                                false,
                                true,
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::SaveSelectionAsTemplate);
                            }
                            if ribbon_tile(
                                ui,
                                pal,
                                VectorIcon::Wall,
                                "Promote House",
                                false,
                                true,
                            )
                            .clicked()
                            {
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
                        .max_width((ui.available_width() - 340.0).max(200.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let styles: &[WindowStyle] = match state.style_filter {
                                    DesignStyleFilter::All => &WindowStyle::ALL,
                                    DesignStyleFilter::Windows => &WindowStyle::ALL_WINDOWS,
                                    DesignStyleFilter::Doors => &WindowStyle::ALL_DOORS,
                                };
                                for &style in styles {
                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::splat(42.0),
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
                    ribbon_group(ui, pal, "Reuse & Scale", |ui| {
                        if ribbon_pill_btn(
                            ui,
                            pal,
                            VectorIcon::Template,
                            "Save Template",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            actions.push(RibbonAction::SaveSelectionAsTemplate);
                        }
                        if is_multi
                            && ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::ComponentLink,
                                "Make Component",
                                false,
                                selected_count == 1,
                            )
                            .clicked()
                        {
                            actions.push(RibbonAction::MakeSelectedComponent);
                        }
                        egui::ComboBox::from_id_salt("ribbon_scale")
                            .selected_text(project.manifest.scale_preset.short_label())
                            .width(68.0)
                            .show_ui(ui, |ui| {
                                for sc in ScalePreset::ALL {
                                    if ui
                                        .selectable_label(
                                            project.manifest.scale_preset == sc,
                                            sc.label(),
                                        )
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
                        if ribbon_tile(ui, pal, VectorIcon::Photo, "Import Photo", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ImportWallPhoto);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::Wizard, "Setup Wizard", false, has_photo)
                            .on_hover_text("Open large pan/zoom Photo Setup Wizard")
                            .clicked()
                        {
                            actions.push(RibbonAction::OpenPhotoWizard);
                        }
                    });
                    ui.separator();
                    if let Some(wall) = wall_opt {
                        if has_photo {
                            ribbon_group(ui, pal, "Direct Canvas Tools", |ui| {
                                if ribbon_pill_btn(
                                    ui,
                                    pal,
                                    VectorIcon::Blueprint,
                                    "Edit on Canvas",
                                    photo_edit_mode,
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(RibbonAction::SetPhotoEditMode(!photo_edit_mode));
                                }
                                for (tool, icon) in [
                                    (PhotoCanvasTool::MoveStretch, VectorIcon::FitView),
                                    (PhotoCanvasTool::Rotate, VectorIcon::Rotate),
                                    (PhotoCanvasTool::PerspectivePins, VectorIcon::Perspective),
                                    (PhotoCanvasTool::Crop, VectorIcon::Crop),
                                    (PhotoCanvasTool::TwoPointCalibrate, VectorIcon::Caliper),
                                ] {
                                    let active = photo_edit_mode && photo_tool == tool;
                                    if ribbon_pill_btn(ui, pal, icon, tool.label(), active, true)
                                        .clicked()
                                    {
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
                                if ui
                                    .add(egui::Slider::new(&mut op, 0.1..=1.0).show_value(false))
                                    .changed()
                                {
                                    actions.push(RibbonAction::SetPhotoOpacity(op));
                                }
                                if ribbon_pill_btn(
                                    ui,
                                    pal,
                                    VectorIcon::FitView,
                                    "Fit",
                                    false,
                                    true,
                                )
                                .clicked()
                                {
                                    actions.push(RibbonAction::FitPhotoToWall);
                                }
                                if ui.small_button("Exact").clicked() {
                                    actions.push(RibbonAction::MatchPhotoExactWall);
                                }
                                if ribbon_pill_btn(ui, pal, VectorIcon::Trash, "Clear", false, true)
                                    .clicked()
                                {
                                    actions.push(RibbonAction::ClearWallPhoto);
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new(
                                    "Import a wall photo to orient, rectify 4-corner perspective, calibrate scale with exact dimensions, and trace openings.",
                                )
                                .color(pal.text_secondary),
                            );
                        }
                    }
                }
                RibbonTab::Annotate => {
                    ribbon_group(ui, pal, "Associative Dimensions", |ui| {
                        if ribbon_tile(
                            ui,
                            pal,
                            VectorIcon::Ruler,
                            "Draw Dim",
                            draw_dim_mode,
                            true,
                        )
                        .on_hover_text(
                            "Click any wall or opening edges/corners to add live linked dimensions (double-click any dimension to drive size)",
                        )
                        .clicked()
                        {
                            actions.push(RibbonAction::ToggleDrawDimension(!draw_dim_mode));
                        }
                        if ribbon_tile(ui, pal, VectorIcon::AutoDim, "Auto-Dim", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::AutoDimensionWall);
                        }
                        if ribbon_tile(ui, pal, VectorIcon::Trash, "Clear Dims", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ClearDimensions);
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "Blueprint Guides", |ui| {
                        let mut g = show_grid;
                        if ribbon_pill_btn(ui, pal, VectorIcon::Grid, "Grid", g, true).clicked() {
                            g = !g;
                            actions.push(RibbonAction::ToggleGrid(g));
                        }
                        let mut r = show_rulers;
                        if ribbon_pill_btn(ui, pal, VectorIcon::Scale, "Rulers", r, true).clicked()
                        {
                            r = !r;
                            actions.push(RibbonAction::ToggleRulers(r));
                        }
                        if ribbon_pill_btn(
                            ui,
                            pal,
                            VectorIcon::Architrave,
                            "Architraves",
                            show_architraves,
                            true,
                        )
                        .clicked()
                        {
                            actions.push(RibbonAction::ToggleArchitraves(!show_architraves));
                        }
                    });
                }
                RibbonTab::View => {
                    ribbon_group(ui, pal, "3D Camera", |ui| {
                        for (preset, lbl) in [
                            (CameraPreset::Isometric, "Iso"),
                            (CameraPreset::Front, "Ext Front"),
                            (CameraPreset::Back, "Int Back"),
                            (CameraPreset::Left, "Left"),
                            (CameraPreset::Right, "Right"),
                            (CameraPreset::Top, "Top"),
                        ] {
                            if ribbon_pill_btn(ui, pal, VectorIcon::Camera, lbl, false, true)
                                .clicked()
                            {
                                actions.push(RibbonAction::CameraPreset(preset));
                            }
                        }
                        if ribbon_pill_btn(ui, pal, VectorIcon::FitView, "Reset", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ResetCamera);
                        }
                    });
                    ui.separator();
                    if is_multi {
                        ribbon_group(ui, pal, "Overlays", |ui| {
                            if ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::Architrave,
                                "Architraves",
                                show_architraves,
                                true,
                            )
                            .clicked()
                            {
                                actions.push(RibbonAction::ToggleArchitraves(!show_architraves));
                            }
                            if viewport == ActiveViewport::Wall3D {
                                let mut sol = wall_solid;
                                if ui.checkbox(&mut sol, "Solid Wall").changed() {
                                    actions.push(RibbonAction::SetWallSolid(sol));
                                }
                            }
                        });
                        ui.separator();
                    }
                    ribbon_group(ui, pal, "Piece 3D Assembly", |ui| {
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
                    if ribbon_pill_btn(
                        ui,
                        pal,
                        VectorIcon::Wireframe,
                        "Wire",
                        view3d.show_wireframe,
                        true,
                    )
                    .clicked()
                    {
                        actions.push(RibbonAction::SetWireframe(!view3d.show_wireframe));
                    }
                    if ribbon_pill_btn(
                        ui,
                        pal,
                        VectorIcon::Overhang,
                        "Overhangs",
                        view3d.highlight_overhangs,
                        true,
                    )
                    .clicked()
                    {
                        actions.push(RibbonAction::SetHighlightOverhangs(
                            !view3d.highlight_overhangs,
                        ));
                    }
                }
                RibbonTab::Export => {
                    ribbon_group(ui, pal, "1:1 Scale PDFs", |ui| {
                        if ribbon_tile(ui, pal, VectorIcon::ExportPdf, "Booklet PDF", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ExportBookletPdf);
                        }
                        if is_multi
                            && ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::ExportPdf,
                                "Active Wall",
                                false,
                                true,
                            )
                            .clicked()
                        {
                            actions.push(RibbonAction::ExportWallPdf);
                        }
                        if ribbon_pill_btn(
                            ui,
                            pal,
                            VectorIcon::ExportPdf,
                            "Active Piece",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            actions.push(RibbonAction::ExportPiecePdf);
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "3D Print STL", |ui| {
                        if ribbon_tile(ui, pal, VectorIcon::ExportStl, "Piece Plate", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ExportPieceStlPlate);
                        }
                        if ribbon_pill_btn(ui, pal, VectorIcon::ExportStl, "Ext STL", false, true)
                            .clicked()
                        {
                            actions.push(RibbonAction::ExportPieceStlExterior);
                        }
                        if ribbon_pill_btn(ui, pal, VectorIcon::ExportStl, "Int STL", false, true)
                            .clicked()
                        {
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
                        if is_multi
                            && ribbon_pill_btn(
                                ui,
                                pal,
                                VectorIcon::PackPlates,
                                "Pack Wall",
                                false,
                                true,
                            )
                            .clicked()
                        {
                            actions.push(RibbonAction::PackWallPlatesStl(state.bed_preset));
                        }
                        if ribbon_pill_btn(
                            ui,
                            pal,
                            VectorIcon::PackPlates,
                            "Pack Project",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            actions.push(RibbonAction::PackProjectPlatesStl(state.bed_preset));
                        }
                    });
                    ui.separator();
                    ribbon_group(ui, pal, "CAD / Laser", |ui| {
                        if ribbon_tile(ui, pal, VectorIcon::Svg, "1:1 SVG", false, true).clicked()
                        {
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

fn ribbon_tile(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    icon: VectorIcon,
    label: &str,
    active: bool,
    enabled: bool,
) -> egui::Response {
    let w = (label.len() as f32 * 6.5 + 22.0).max(58.0);
    let size = Vec2::new(w, 46.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    let bg = if !enabled {
        pal.surface_base
    } else if active {
        pal.accent
    } else if resp.hovered() {
        pal.surface_elevated
    } else {
        pal.surface_panel
    };
    let fg = if !enabled {
        pal.text_muted
    } else if active {
        pal.accent_text
    } else {
        pal.text_primary
    };
    let border = if active {
        pal.accent
    } else if resp.hovered() && enabled {
        pal.border_strong
    } else {
        pal.border
    };

    ui.painter().rect_filled(rect, 6.0, bg);
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 14.5),
        Vec2::splat(17.0),
    );
    paint_icon(ui.painter(), icon_rect, icon, fg);
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 8.5),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(11.3),
        fg,
    );
    resp
}

fn ribbon_pill_btn(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    icon: VectorIcon,
    label: &str,
    active: bool,
    enabled: bool,
) -> egui::Response {
    let w = (label.len() as f32 * 6.8 + 30.0).max(52.0);
    let size = Vec2::new(w, 25.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    let bg = if !enabled {
        pal.surface_base
    } else if active {
        pal.accent
    } else if resp.hovered() {
        pal.surface_elevated
    } else {
        pal.surface_panel
    };
    let fg = if !enabled {
        pal.text_muted
    } else if active {
        pal.accent_text
    } else {
        pal.text_primary
    };
    let border = if active {
        pal.accent
    } else if resp.hovered() && enabled {
        pal.border_strong
    } else {
        pal.border
    };

    ui.painter().rect_filled(rect, 5.0, bg);
    ui.painter().rect_stroke(
        rect,
        5.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        Vec2::splat(14.0),
    );
    paint_icon(ui.painter(), icon_rect, icon, fg);
    ui.painter().text(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.2),
        fg,
    );
    resp
}
