use crate::icons::{VectorIcon, draw_style_thumbnail, paint_icon};
use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{DisplayUnit, PieceTemplate, ProjectKind, ScalePreset, WindowStyle};
use dh_project::{AppPreferences, ThemeMode};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HubTab {
    #[default]
    RecentProjects,
    Templates,
}

#[derive(Debug, Clone, Default)]
pub struct WelcomeState {
    pub tab: HubTab,
    pub search: String,
    pub filter_doors_only: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum WelcomeAction {
    OpenNewProjectWizard {
        initial_kind: Option<ProjectKind>,
        with_sample: bool,
        initial_style: Option<WindowStyle>,
        initial_template: Option<PieceTemplate>,
    },
    OpenExistingFolderPrompt,
    OpenProjectFolder(PathBuf),
    RemoveRecentProject(PathBuf),
    ChangeTheme(ThemeMode),
    ChangeUnit(DisplayUnit),
    ImportPersonalTemplatePrompt,
    ExportPersonalTemplate(PieceTemplate),
    DeletePersonalTemplate(String),
}

pub fn draw_welcome_hub(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    prefs: &AppPreferences,
    personal_templates: &[PieceTemplate],
    starter_templates: &[PieceTemplate],
    state: &mut WelcomeState,
) -> Option<WelcomeAction> {
    let mut action = None;

    egui::Frame::NONE
        .fill(pal.surface_base)
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            let left_w = 300.0_f32.min(full.width() * 0.34);

            ui.horizontal(|ui| {
        // Left Launch Strip
        ui.allocate_ui_with_layout(
            Vec2::new(left_w, full.height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::Frame::NONE
                    .fill(pal.surface_panel)
                    .stroke(Stroke::new(1.0, pal.border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(18))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                            paint_icon(ui.painter(), icon_rect, VectorIcon::Wall, pal.accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new("Dollhouse Studio").size(20.0).strong());
                                ui.label(
                                    RichText::new("1:12 Miniature Architectural CAD")
                                        .small()
                                        .color(pal.text_muted),
                                );
                            });
                        });

                        ui.add_space(14.0);
                        let new_btn = egui::Button::new(
                            RichText::new("New Project Folder...")
                                .strong()
                                .size(13.5)
                                .color(pal.accent_text),
                        )
                        .fill(pal.accent)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 34.0));
                        if ui.add(new_btn).clicked() {
                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                initial_kind: None,
                                with_sample: true,
                                initial_style: None,
                                initial_template: None,
                            });
                        }

                        ui.add_space(6.0);
                        let open_btn = egui::Button::new(RichText::new("Open Project Folder...").size(13.2))
                            .fill(pal.surface_card)
                            .stroke(Stroke::new(1.0, pal.border_strong))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(ui.available_width(), 32.0));
                        if ui.add(open_btn).clicked() {
                            action = Some(WelcomeAction::OpenExistingFolderPrompt);
                        }

                        ui.add_space(16.0);
                        ui.label(RichText::new("QUICK START").small().strong().color(pal.text_muted));
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new("Single Window / Door Project")
                                    .min_size(Vec2::new(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                initial_kind: Some(ProjectKind::SinglePiece),
                                with_sample: false,
                                initial_style: Some(WindowStyle::GeorgianGrid),
                                initial_template: None,
                            });
                        }
                        if ui
                            .add(
                                egui::Button::new("Sample Multi-Wall Facade")
                                    .min_size(Vec2::new(ui.available_width(), 28.0)),
                            )
                            .clicked()
                        {
                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                initial_kind: Some(ProjectKind::MultiWall),
                                with_sample: true,
                                initial_style: None,
                                initial_template: None,
                            });
                        }

                        ui.add_space(18.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label(RichText::new("PREFERENCES").small().strong().color(pal.text_muted));
                        ui.add_space(6.0);

                        let mut theme = prefs.theme;
                        if segmented_pill(
                            ui,
                            pal,
                            &mut theme,
                            &[
                                (ThemeMode::System, "OS"),
                                (ThemeMode::Light, "Light"),
                                (ThemeMode::Dark, "Dark"),
                            ],
                        ) {
                            action = Some(WelcomeAction::ChangeTheme(theme));
                        }

                        ui.add_space(6.0);
                        let mut unit = prefs.display_unit;
                        if segmented_pill(
                            ui,
                            pal,
                            &mut unit,
                            &[
                                (DisplayUnit::Mm, "mm"),
                                (DisplayUnit::Cm, "cm"),
                                (DisplayUnit::Inches, "in"),
                            ],
                        ) {
                            action = Some(WelcomeAction::ChangeUnit(unit));
                        }

                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "Scale Default: {} | MIT Open Source",
                                ScalePreset::Scale1_12.label()
                            ))
                            .small()
                            .color(pal.text_muted),
                        );
                    });
            },
        );

        ui.add_space(16.0);

        // Right Content Browser (Recent Projects + Templates)
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if segmented_pill(
                    ui,
                    pal,
                    &mut state.tab,
                    &[
                        (HubTab::RecentProjects, "Recent Projects"),
                        (HubTab::Templates, "Template Library"),
                    ],
                ) {
                    // Tab switched
                }

                ui.add_space(12.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text("Search projects or templates...")
                        .desired_width(220.0),
                );

                if state.tab == HubTab::Templates {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Import Template...").clicked() {
                            action = Some(WelcomeAction::ImportPersonalTemplatePrompt);
                        }
                    });
                }
            });

            ui.add_space(12.0);
            let q = state.search.trim().to_lowercase();

            egui::ScrollArea::vertical().show(ui, |ui| match state.tab {
                HubTab::RecentProjects => {
                    let entries: Vec<_> = prefs
                        .recent_projects
                        .iter()
                        .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
                        .collect();

                    if entries.is_empty() {
                        egui::Frame::NONE
                            .fill(pal.surface_card)
                            .stroke(Stroke::new(1.0, pal.border))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::same(28))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("No recent projects yet")
                                        .size(16.0)
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "Create a portable project folder on disk using New Project Folder, or open an existing Dollhouse Studio folder.",
                                    )
                                    .color(pal.text_secondary),
                                );
                            });
                    } else {
                        for entry in entries {
                            egui::Frame::NONE
                                .fill(pal.surface_card)
                                .stroke(Stroke::new(1.0, pal.border))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::same(14))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (ir, _) = ui.allocate_exact_size(
                                            Vec2::splat(22.0),
                                            egui::Sense::hover(),
                                        );
                                        let icon = match entry.kind {
                                            ProjectKind::SinglePiece => VectorIcon::Window,
                                            ProjectKind::MultiWall => VectorIcon::Wall,
                                        };
                                        paint_icon(ui.painter(), ir, icon, pal.accent);

                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&entry.name)
                                                        .size(15.0)
                                                        .strong(),
                                                );
                                                ui.label(
                                                    RichText::new(format!("[{}]", entry.kind.label()))
                                                        .small()
                                                        .color(pal.accent),
                                                );
                                            });
                                            let detail = match entry.kind {
                                                ProjectKind::SinglePiece => {
                                                    format!("{} piece(s)", entry.piece_count)
                                                }
                                                ProjectKind::MultiWall => {
                                                    format!("{} wall(s)", entry.wall_count)
                                                }
                                            };
                                            ui.label(
                                                RichText::new(format!(
                                                    "{detail}  |  {}",
                                                    entry.path.display()
                                                ))
                                                .small()
                                                .color(pal.text_muted),
                                            );
                                        });

                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("Remove")
                                                    .on_hover_text("Remove from Recent list (folder stays on disk)")
                                                    .clicked()
                                                {
                                                    action = Some(WelcomeAction::RemoveRecentProject(
                                                        entry.path.clone(),
                                                    ));
                                                }
                                                let open_btn = egui::Button::new(
                                                    RichText::new("Open")
                                                        .strong()
                                                        .color(pal.accent_text),
                                                )
                                                .fill(pal.accent);
                                                if ui.add(open_btn).clicked() {
                                                    action = Some(WelcomeAction::OpenProjectFolder(
                                                        entry.path.clone(),
                                                    ));
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(8.0);
                        }
                    }
                }
                HubTab::Templates => {
                    ui.horizontal(|ui| {
                        let all_sel = state.filter_doors_only.is_none();
                        if ui.selectable_label(all_sel, "All").clicked() {
                            state.filter_doors_only = None;
                        }
                        if ui
                            .selectable_label(state.filter_doors_only == Some(false), "Windows")
                            .clicked()
                        {
                            state.filter_doors_only = Some(false);
                        }
                        if ui
                            .selectable_label(state.filter_doors_only == Some(true), "Doors")
                            .clicked()
                        {
                            state.filter_doors_only = Some(true);
                        }
                    });
                    ui.add_space(10.0);

                    if !personal_templates.is_empty() {
                        ui.label(
                            RichText::new("YOUR PERSONAL TEMPLATES")
                                .small()
                                .strong()
                                .color(pal.text_muted),
                        );
                        ui.add_space(6.0);
                        for tpl in personal_templates {
                            if !matches_template_filter(tpl, &q, state.filter_doors_only) {
                                continue;
                            }
                            if let Some(a) = draw_template_card(ui, pal, tpl, prefs.display_unit, true) {
                                action = Some(a);
                            }
                            ui.add_space(6.0);
                        }
                        ui.add_space(12.0);
                    }

                    ui.label(
                        RichText::new("BUILT-IN ARCHITECTURAL STARTERS")
                            .small()
                            .strong()
                            .color(pal.text_muted),
                    );
                    ui.add_space(6.0);
                    for tpl in starter_templates {
                        if !matches_template_filter(tpl, &q, state.filter_doors_only) {
                            continue;
                        }
                        if let Some(a) = draw_template_card(ui, pal, tpl, prefs.display_unit, false) {
                            action = Some(a);
                        }
                        ui.add_space(6.0);
                    }
                }
            });
        });
            });
        });

    let _ = Color32::TRANSPARENT;
    action
}

fn matches_template_filter(tpl: &PieceTemplate, q: &str, doors_only: Option<bool>) -> bool {
    if let Some(d) = doors_only
        && tpl.params.style.is_door() != d
    {
        return false;
    }
    if q.is_empty() {
        return true;
    }
    tpl.name.to_lowercase().contains(q)
        || tpl.params.style.name().to_lowercase().contains(q)
        || tpl.tags.iter().any(|t| t.to_lowercase().contains(q))
}

fn draw_template_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    tpl: &PieceTemplate,
    unit: DisplayUnit,
    is_personal: bool,
) -> Option<WelcomeAction> {
    let mut action = None;
    egui::Frame::NONE
        .fill(pal.surface_card)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (tr, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
                draw_style_thumbnail(ui.painter(), tr, tpl.params.style, false, pal);

                ui.vertical(|ui| {
                    ui.label(RichText::new(&tpl.name).strong().size(14.0));
                    ui.label(
                        RichText::new(format!(
                            "{} | {}",
                            tpl.params.style.name(),
                            unit.format_dims(tpl.params.opening_width, tpl.params.opening_height)
                        ))
                        .small()
                        .color(pal.text_secondary),
                    );
                    if !tpl.notes.is_empty() {
                        ui.label(RichText::new(&tpl.notes).small().color(pal.text_muted));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_personal {
                        if ui.small_button("Delete").clicked() {
                            action = Some(WelcomeAction::DeletePersonalTemplate(tpl.id.clone()));
                        }
                        if ui.small_button("Export").clicked() {
                            action = Some(WelcomeAction::ExportPersonalTemplate(tpl.clone()));
                        }
                    }
                    let use_btn = egui::Button::new(
                        RichText::new("New Project").strong().color(pal.accent_text),
                    )
                    .fill(pal.accent);
                    if ui.add(use_btn).clicked() {
                        action = Some(WelcomeAction::OpenNewProjectWizard {
                            initial_kind: Some(ProjectKind::SinglePiece),
                            with_sample: false,
                            initial_style: None,
                            initial_template: Some(tpl.clone()),
                        });
                    }
                });
            });
        });
    action
}
