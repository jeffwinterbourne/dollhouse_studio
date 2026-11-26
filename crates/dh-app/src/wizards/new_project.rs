use crate::icons::draw_style_thumbnail;
use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{
    DisplayUnit, PieceTemplate, ProjectKind, ScalePreset, WindowStyle, sanitize_filename,
};
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NewProjectWizardState {
    pub open: bool,
    pub name: String,
    pub parent_dir: PathBuf,
    pub kind: ProjectKind,
    pub scale: ScalePreset,
    pub unit: DisplayUnit,
    pub with_sample_facade: bool,
    pub initial_style: WindowStyle,
    pub initial_template: Option<PieceTemplate>,
    pub error_msg: Option<String>,
}

impl NewProjectWizardState {
    pub fn default_root() -> PathBuf {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return PathBuf::from(profile).join("Documents").join("DollhouseProjects");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Documents").join("DollhouseProjects");
        }
        PathBuf::from("DollhouseProjects")
    }

    pub fn closed(default_unit: DisplayUnit, saved_root: Option<PathBuf>) -> Self {
        Self {
            open: false,
            name: "Georgian Dollhouse".to_string(),
            parent_dir: saved_root.unwrap_or_else(Self::default_root),
            kind: ProjectKind::MultiWall,
            scale: ScalePreset::Scale1_12,
            unit: default_unit,
            with_sample_facade: true,
            initial_style: WindowStyle::GeorgianGrid,
            initial_template: None,
            error_msg: None,
        }
    }

    pub fn open_with(
        &mut self,
        default_unit: DisplayUnit,
        saved_root: Option<PathBuf>,
        kind: Option<ProjectKind>,
        with_sample: bool,
        style: Option<WindowStyle>,
        template: Option<PieceTemplate>,
    ) {
        self.open = true;
        self.error_msg = None;
        if let Some(root) = saved_root {
            self.parent_dir = root;
        }
        self.kind = kind.unwrap_or(ProjectKind::MultiWall);
        self.unit = default_unit;
        self.with_sample_facade = with_sample;
        self.initial_style = style.unwrap_or(WindowStyle::GeorgianGrid);
        self.initial_template = template.clone();
        self.name = if let Some(t) = template {
            format!("{} Study", t.name)
        } else if self.kind == ProjectKind::SinglePiece {
            "Parlor Window".to_string()
        } else {
            "My Dollhouse".to_string()
        };
    }

    pub fn resolved_folder(&self) -> PathBuf {
        let folder = sanitize_filename(&self.name);
        self.parent_dir.join(if folder.is_empty() {
            "dollhouse_project".to_string()
        } else {
            folder
        })
    }
}

#[derive(Debug, Clone)]
pub struct NewProjectRequest {
    pub name: String,
    pub project_dir: PathBuf,
    pub kind: ProjectKind,
    pub scale: ScalePreset,
    pub unit: DisplayUnit,
    pub with_sample_facade: bool,
    pub initial_style: WindowStyle,
    pub initial_template: Option<PieceTemplate>,
}

pub fn draw_new_project_wizard(
    ctx: &egui::Context,
    pal: &StudioPalette,
    state: &mut NewProjectWizardState,
) -> Option<NewProjectRequest> {
    if !state.open {
        return None;
    }

    let mut request = None;
    let mut open = state.open;

    egui::Window::new("New Project Folder")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(560.0, 480.0))
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Every project lives in its own organized folder on disk.")
                    .color(pal.text_secondary),
            );
            ui.add_space(10.0);

            egui::Grid::new("new_proj_grid")
                .num_columns(2)
                .spacing([12.0, 10.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Project Name").strong());
                    ui.add(egui::TextEdit::singleline(&mut state.name).desired_width(340.0));
                    ui.end_row();

                    ui.label(RichText::new("Parent Folder").strong());
                    ui.horizontal(|ui| {
                        let mut p_str = state.parent_dir.to_string_lossy().to_string();
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut p_str).desired_width(250.0));
                        if ui.button("Browse...").clicked()
                            && let Some(chosen) = rfd::FileDialog::new()
                                .set_title("Choose Parent Folder for New Project")
                                .set_directory(&state.parent_dir)
                                .pick_folder()
                            {
                                state.parent_dir = chosen;
                            }
                    });
                    ui.end_row();

                    ui.label(RichText::new("Project Folder").strong());
                    ui.label(
                        RichText::new(state.resolved_folder().to_string_lossy().to_string())
                            .small()
                            .color(pal.accent),
                    );
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.label(RichText::new("Project Scope").strong());
            segmented_pill(
                ui,
                pal,
                &mut state.kind,
                &[
                    (ProjectKind::MultiWall, "House / Multi-Wall"),
                    (ProjectKind::SinglePiece, "Single Window / Door"),
                ],
            );

            ui.add_space(8.0);
            match state.kind {
                ProjectKind::MultiWall => {
                    ui.checkbox(
                        &mut state.with_sample_facade,
                        "Start with sample Front Facade wall (two linked Parlor window instances + Front Door)",
                    );
                }
                ProjectKind::SinglePiece => {
                    if let Some(ref tpl) = state.initial_template {
                        ui.label(
                            RichText::new(format!("Using Template: {}", tpl.name))
                                .color(pal.accent),
                        );
                    } else {
                        ui.label(RichText::new("Starter Piece Style").small().color(pal.text_muted));
                        egui::ScrollArea::horizontal().max_height(70.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for style in WindowStyle::ALL {
                                    let sel = state.initial_style == style;
                                    let (r, resp) = ui.allocate_exact_size(
                                        Vec2::splat(46.0),
                                        egui::Sense::click(),
                                    );
                                    draw_style_thumbnail(ui.painter(), r, style, sel, pal);
                                    let resp = resp.on_hover_text(style.name());
                                    if resp.clicked() {
                                        state.initial_style = style;
                                    }
                                }
                            });
                        });
                    }
                }
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Default Scale").strong());
                    egui::ComboBox::from_id_salt("new_proj_scale")
                        .selected_text(state.scale.label())
                        .show_ui(ui, |ui| {
                            for sc in ScalePreset::ALL {
                                ui.selectable_value(&mut state.scale, sc, sc.label());
                            }
                        });
                });
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Display Unit").strong());
                    segmented_pill(
                        ui,
                        pal,
                        &mut state.unit,
                        &[
                            (DisplayUnit::Mm, "mm"),
                            (DisplayUnit::Cm, "cm"),
                            (DisplayUnit::Inches, "in"),
                        ],
                    );
                });
            });

            if let Some(ref err) = state.error_msg {
                ui.add_space(8.0);
                ui.label(RichText::new(err).color(pal.danger));
            }

            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let create_btn = egui::Button::new(
                    RichText::new("Create Project Folder")
                        .strong()
                        .color(pal.accent_text),
                )
                .fill(pal.accent)
                .min_size(Vec2::new(150.0, 30.0));
                if ui.add(create_btn).clicked() {
                    let clean_name = state.name.trim().to_string();
                    if clean_name.is_empty() {
                        state.error_msg = Some("Please enter a project name.".to_string());
                    } else {
                        let target = state.resolved_folder();
                        if target.exists() {
                            state.error_msg = Some(format!(
                                "Folder already exists: {}. Rename or choose another location.",
                                target.display()
                            ));
                        } else {
                            request = Some(NewProjectRequest {
                                name: clean_name,
                                project_dir: target,
                                kind: state.kind,
                                scale: state.scale,
                                unit: state.unit,
                                with_sample_facade: state.with_sample_facade,
                                initial_style: state.initial_style,
                                initial_template: state.initial_template.clone(),
                            });
                            state.open = false;
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    state.open = false;
                }
            });
        });

    if !open {
        state.open = false;
    }
    let _ = Margin::same(0);
    let _ = CornerRadius::same(0);
    let _ = Stroke::NONE;
    request
}
