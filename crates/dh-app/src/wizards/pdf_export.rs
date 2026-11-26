use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{DisplayUnit, Project, ProjectKind};
use dh_export::{
    PageOrientation, PaperSize, PdfExportOptions, PdfInstanceMode, estimate_pdf_pages,
};
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfExportScope {
    ActiveWall,
    ProjectBooklet,
    ActivePiece,
}

#[derive(Debug, Clone)]
pub struct PdfExportModalState {
    pub open: bool,
    pub scope: PdfExportScope,
    pub options: PdfExportOptions,
}

impl Default for PdfExportModalState {
    fn default() -> Self {
        Self {
            open: false,
            scope: PdfExportScope::ProjectBooklet,
            options: PdfExportOptions::default(),
        }
    }
}

impl PdfExportModalState {
    pub fn open_for(&mut self, project: &Project, unit: DisplayUnit, scope: PdfExportScope) {
        self.open = true;
        self.scope = match project.manifest.kind {
            ProjectKind::SinglePiece => PdfExportScope::ActivePiece,
            ProjectKind::MultiWall => scope,
        };
        self.options.display_unit = unit;
    }
}

pub fn draw_pdf_export_wizard(
    ctx: &egui::Context,
    pal: &StudioPalette,
    project: &Project,
    state: &mut PdfExportModalState,
) -> Option<(PdfExportScope, PdfExportOptions)> {
    if !state.open {
        return None;
    }

    let mut trigger = None;
    let mut open = state.open;

    egui::Window::new("Export 1:1 Scale PDF Sheets")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(500.0, 440.0))
        .show(ctx, |ui| {
            ui.label(
                RichText::new(
                    "Print at 100% / Actual Size (no page scaling) for exact 1:1 cutting guides.",
                )
                .color(pal.text_secondary),
            );
            ui.add_space(10.0);

            if project.manifest.kind == ProjectKind::MultiWall {
                ui.label(RichText::new("Export Scope").strong());
                segmented_pill(
                    ui,
                    pal,
                    &mut state.scope,
                    &[
                        (PdfExportScope::ActiveWall, "Current Wall"),
                        (PdfExportScope::ProjectBooklet, "Full Project Booklet"),
                    ],
                );
                ui.add_space(8.0);
            }

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Paper Size").strong());
                    egui::ComboBox::from_id_salt("pdf_paper")
                        .selected_text(state.options.paper.label())
                        .show_ui(ui, |ui| {
                            for p in PaperSize::ALL {
                                ui.selectable_value(&mut state.options.paper, p, p.label());
                            }
                        });
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Orientation").strong());
                    egui::ComboBox::from_id_salt("pdf_orient")
                        .selected_text(state.options.orientation.label())
                        .show_ui(ui, |ui| {
                            for o in PageOrientation::ALL {
                                ui.selectable_value(&mut state.options.orientation, o, o.label());
                            }
                        });
                });
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Linked Component Instances on Cut Sheets").strong());
            segmented_pill(
                ui,
                pal,
                &mut state.options.instance_mode,
                &[
                    (PdfInstanceMode::UniqueWithQty, "Unique Parts + Qty Badge"),
                    (PdfInstanceMode::AllInstances, "Every Instance Sheet"),
                ],
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Print Units").strong());
            segmented_pill(
                ui,
                pal,
                &mut state.options.display_unit,
                &[
                    (DisplayUnit::Mm, "mm"),
                    (DisplayUnit::Cm, "cm"),
                    (DisplayUnit::Inches, "in"),
                ],
            );

            ui.add_space(10.0);
            if project.manifest.kind == ProjectKind::MultiWall {
                if state.scope == PdfExportScope::ProjectBooklet {
                    ui.checkbox(
                        &mut state.options.include_cover_bom,
                        "Include Project Cover & Bill of Materials (BOM)",
                    );
                }
                ui.checkbox(
                    &mut state.options.include_wall_tiles,
                    "Include Tiled 1:1 Wall Elevations (with corner alignment marks)",
                );
                ui.checkbox(
                    &mut state.options.include_cutout_and_acetate,
                    "Include Auto-Packed 1:1 Cutout & Acetate Glazing Sheets",
                );

                if state.options.include_wall_tiles {
                    ui.horizontal(|ui| {
                        ui.label("Tile Glue Overlap");
                        ui.add(
                            egui::Slider::new(&mut state.options.tile_overlap_mm, 6.0..=24.0)
                                .suffix(" mm")
                                .max_decimals(0),
                        );
                    });
                }
            }

            ui.add_space(10.0);
            let wall_ref = if state.scope == PdfExportScope::ActiveWall {
                project.active_wall()
            } else {
                None
            };
            let est_pages = if state.scope == PdfExportScope::ActivePiece {
                1
            } else {
                estimate_pdf_pages(project, wall_ref, &state.options)
            };

            egui::Frame::NONE
                .fill(pal.surface_card)
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Estimated Pages:").strong());
                        ui.label(
                            RichText::new(format!("{est_pages} sheet(s)"))
                                .strong()
                                .color(pal.accent),
                        );
                        ui.label(
                            RichText::new(format!(
                                "({} / {})",
                                state.options.paper.label(),
                                state.options.display_unit.short_label()
                            ))
                            .small()
                            .color(pal.text_muted),
                        );
                    });
                });

            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = egui::Button::new(
                    RichText::new("Save PDF File...")
                        .strong()
                        .color(pal.accent_text),
                )
                .fill(pal.accent)
                .min_size(Vec2::new(130.0, 30.0));
                if ui.add(btn).clicked() {
                    trigger = Some((state.scope, state.options.clone()));
                    state.open = false;
                }
                if ui.button("Cancel").clicked() {
                    state.open = false;
                }
            });
        });

    if !open {
        state.open = false;
    }
    trigger
}
