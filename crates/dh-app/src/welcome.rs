use crate::icons::{VectorIcon, draw_style_thumbnail, paint_brand_mark, paint_icon};
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
    CloseHub,
    OpenGettingStartedTour,
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

/// Renders the Project & Template Hub as a clean centered floating pane over a translucent scrim,
/// keeping the live Dollhouse Studio workspace visible underneath.
#[allow(clippy::too_many_arguments)]
pub fn draw_welcome_hub(
    ctx: &egui::Context,
    pal: &StudioPalette,
    prefs: &AppPreferences,
    personal_templates: &[PieceTemplate],
    starter_templates: &[PieceTemplate],
    state: &mut WelcomeState,
) -> Option<WelcomeAction> {
    let mut action = None;
    let screen = ctx.content_rect();

    // Soft modal scrim over main studio UI (click outside or press Esc to continue editing).
    egui::Area::new(egui::Id::unique("hub_scrim_area"))
        .fixed_pos(screen.left_top())
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let (r, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            let scrim = if pal.dark_mode {
                Color32::from_rgba_unmultiplied(10, 14, 22, 168)
            } else {
                Color32::from_rgba_unmultiplied(24, 34, 52, 118)
            };
            ui.painter().rect_filled(r, 0.0, scrim);
            if resp.clicked() {
                action = Some(WelcomeAction::CloseHub);
            }
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        action = Some(WelcomeAction::CloseHub);
    }

    let modal_w = (screen.width() - 64.0).clamp(620.0, 960.0);
    let modal_h = (screen.height() - 64.0).clamp(440.0, 620.0);
    let modal_pos = screen.center() - Vec2::new(modal_w * 0.5, modal_h * 0.5);

    egui::Area::new(egui::Id::unique("hub_floating_card"))
        .fixed_pos(modal_pos)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(pal.surface_panel)
                .stroke(Stroke::new(1.5, pal.border_strong))
                .corner_radius(CornerRadius::same(12))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 2,
                    color: Color32::from_rgba_unmultiplied(0, 0, 0, 90),
                })
                .inner_margin(Margin::same(18))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(modal_w - 36.0, modal_h - 36.0));
                    ui.set_max_size(Vec2::new(modal_w - 36.0, modal_h - 36.0));

                    // Top Floating Hub Header
                    ui.horizontal(|ui| {
                        let (mark_r, _) =
                            ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                        paint_brand_mark(ui.painter(), mark_r);
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Dollhouse Studio").size(19.0).strong());
                            ui.label(
                                RichText::new(
                                    "Projects, Templates & Quick Start  |  1:12 Miniature Architectural CAD",
                                )
                                .small()
                                .color(pal.text_muted),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Continue to Canvas ->")
                                            .strong()
                                            .color(pal.accent_text),
                                    )
                                    .fill(pal.accent)
                                    .corner_radius(CornerRadius::same(6)),
                                )
                                .clicked()
                            {
                                action = Some(WelcomeAction::CloseHub);
                            }
                            if ui
                                .button("Quick Tour...")
                                .on_hover_text("Interactive walkthrough of key concepts")
                                .clicked()
                            {
                                action = Some(WelcomeAction::OpenGettingStartedTour);
                            }
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    let body_h = (modal_h - 118.0).max(280.0);
                    let left_w = 278.0;

                    ui.horizontal_top(|ui| {
                        // Left Action Sidebar inside floating card
                        ui.allocate_ui_with_layout(
                            Vec2::new(left_w, body_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::Frame::NONE
                                    .fill(pal.surface_card)
                                    .stroke(Stroke::new(1.0, pal.border))
                                    .corner_radius(CornerRadius::same(9))
                                    .inner_margin(Margin::same(14))
                                    .show(ui, |ui| {
                                        if hub_action_row(
                                            ui,
                                            pal,
                                            VectorIcon::Plus,
                                            "New Project Folder...",
                                            true,
                                        ) {
                                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                                initial_kind: None,
                                                with_sample: true,
                                                initial_style: None,
                                                initial_template: None,
                                            });
                                        }
                                        ui.add_space(6.0);
                                        if hub_action_row(
                                            ui,
                                            pal,
                                            VectorIcon::Folder,
                                            "Open Project Folder...",
                                            false,
                                        ) {
                                            action = Some(WelcomeAction::OpenExistingFolderPrompt);
                                        }

                                        ui.add_space(14.0);
                                        ui.label(
                                            RichText::new("QUICK START TEMPLATES")
                                                .small()
                                                .strong()
                                                .color(pal.text_muted),
                                        );
                                        ui.add_space(6.0);
                                        if hub_action_row(
                                            ui,
                                            pal,
                                            VectorIcon::Window,
                                            "Single Window / Door Study",
                                            false,
                                        ) {
                                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                                initial_kind: Some(ProjectKind::SinglePiece),
                                                with_sample: false,
                                                initial_style: Some(WindowStyle::GeorgianGrid),
                                                initial_template: None,
                                            });
                                        }
                                        ui.add_space(5.0);
                                        if hub_action_row(
                                            ui,
                                            pal,
                                            VectorIcon::Wall,
                                            "Sample House Wall Facade",
                                            false,
                                        ) {
                                            action = Some(WelcomeAction::OpenNewProjectWizard {
                                                initial_kind: Some(ProjectKind::MultiWall),
                                                with_sample: true,
                                                initial_style: None,
                                                initial_template: None,
                                            });
                                        }

                                        ui.add_space(14.0);
                                        ui.separator();
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("STUDIO DEFAULTS")
                                                .small()
                                                .strong()
                                                .color(pal.text_muted),
                                        );
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

                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(format!(
                                                "Scale Default: {} | MIT",
                                                ScalePreset::Scale1_12.label()
                                            ))
                                            .small()
                                            .color(pal.text_muted),
                                        );
                                    });
                            },
                        );

                        ui.add_space(12.0);

                        // Right Recent Projects & Template Browser inside floating pane
                        ui.allocate_ui_with_layout(
                            Vec2::new((modal_w - left_w - 56.0).max(260.0), body_h),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.horizontal(|ui| {
                                    segmented_pill(
                                        ui,
                                        pal,
                                        &mut state.tab,
                                        &[
                                            (HubTab::RecentProjects, "Recent Projects"),
                                            (HubTab::Templates, "Templates"),
                                        ],
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut state.search)
                                            .hint_text("Search...")
                                            .desired_width(160.0),
                                    );
                                    if state.tab == HubTab::Templates
                                        && ui.small_button("Import...").clicked()
                                    {
                                        action = Some(WelcomeAction::ImportPersonalTemplatePrompt);
                                    }
                                });

                                ui.add_space(8.0);
                                let q = state.search.trim().to_lowercase();

                                egui::ScrollArea::vertical()
                                    .max_height((body_h - 36.0).max(200.0))
                                    .show(ui, |ui| match state.tab {
                                        HubTab::RecentProjects => {
                                            let entries: Vec<_> = prefs
                                                .recent_projects
                                                .iter()
                                                .filter(|e| {
                                                    q.is_empty()
                                                        || e.name.to_lowercase().contains(&q)
                                                })
                                                .collect();
                                            if entries.is_empty() {
                                                egui::Frame::NONE
                                                    .fill(pal.surface_card)
                                                    .stroke(Stroke::new(1.0, pal.border))
                                                    .corner_radius(CornerRadius::same(8))
                                                    .inner_margin(Margin::same(18))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new("No saved projects yet")
                                                                .strong()
                                                                .size(14.5),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            RichText::new(
                                                                "Click 'Continue to Canvas' to sketch immediately in the starter facade, or 'New Project Folder' to create a folder on disk.",
                                                            )
                                                            .color(pal.text_secondary),
                                                        );
                                                    });
                                            } else {
                                                for entry in entries {
                                                    egui::Frame::NONE
                                                        .fill(pal.surface_card)
                                                        .stroke(Stroke::new(1.0, pal.border))
                                                        .corner_radius(CornerRadius::same(7))
                                                        .inner_margin(Margin::same(10))
                                                        .show(ui, |ui| {
                                                            ui.horizontal(|ui| {
                                                                let (ir, _) = ui.allocate_exact_size(
                                                                    Vec2::splat(20.0),
                                                                    egui::Sense::hover(),
                                                                );
                                                                let ic = match entry.kind {
                                                                    ProjectKind::SinglePiece => VectorIcon::Window,
                                                                    ProjectKind::MultiWall => VectorIcon::Wall,
                                                                };
                                                                paint_icon(ui.painter(), ir, ic, pal.accent);
                                                                ui.vertical(|ui| {
                                                                    ui.horizontal(|ui| {
                                                                        ui.label(RichText::new(&entry.name).strong().size(13.8));
                                                                        ui.label(RichText::new(entry.kind.label()).small().color(pal.accent));
                                                                    });
                                                                    let detail = match entry.kind {
                                                                        ProjectKind::SinglePiece => format!("{} piece(s)", entry.piece_count),
                                                                        ProjectKind::MultiWall => format!("{} wall(s)", entry.wall_count),
                                                                    };
                                                                    ui.label(RichText::new(format!("{detail} | {}", entry.path.display())).small().color(pal.text_muted));
                                                                });
                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    if ui.small_button("Remove").clicked() {
                                                                        action = Some(WelcomeAction::RemoveRecentProject(entry.path.clone()));
                                                                    }
                                                                    let b = egui::Button::new(RichText::new("Open").strong().color(pal.accent_text)).fill(pal.accent);
                                                                    if ui.add(b).clicked() {
                                                                        action = Some(WelcomeAction::OpenProjectFolder(entry.path.clone()));
                                                                    }
                                                                });
                                                            });
                                                        });
                                                    ui.add_space(6.0);
                                                }
                                            }
                                        }
                                        HubTab::Templates => {
                                            ui.horizontal(|ui| {
                                                if ui.selectable_label(state.filter_doors_only.is_none(), "All").clicked() {
                                                    state.filter_doors_only = None;
                                                }
                                                if ui.selectable_label(state.filter_doors_only == Some(false), "Windows").clicked() {
                                                    state.filter_doors_only = Some(false);
                                                }
                                                if ui.selectable_label(state.filter_doors_only == Some(true), "Doors").clicked() {
                                                    state.filter_doors_only = Some(true);
                                                }
                                            });
                                            ui.add_space(6.0);
                                            for (hdr, list, is_pers) in [
                                                ("PERSONAL LIBRARY", personal_templates, true),
                                                ("BUILT-IN STARTERS", starter_templates, false),
                                            ] {
                                                if list.is_empty() {
                                                    continue;
                                                }
                                                ui.label(RichText::new(hdr).small().strong().color(pal.text_muted));
                                                ui.add_space(4.0);
                                                for tpl in list {
                                                    if !matches_template_filter(tpl, &q, state.filter_doors_only) {
                                                        continue;
                                                    }
                                                    if let Some(a) = draw_template_card(ui, pal, tpl, prefs.display_unit, is_pers) {
                                                        action = Some(a);
                                                    }
                                                    ui.add_space(5.0);
                                                }
                                                ui.add_space(8.0);
                                            }
                                        }
                                    });
                            },
                        );
                    });
                });
        });

    action
}

fn hub_action_row(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    icon: VectorIcon,
    label: &str,
    primary: bool,
) -> bool {
    let size = Vec2::new(ui.available_width(), 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if primary {
        pal.accent
    } else if resp.hovered() {
        pal.surface_elevated
    } else {
        pal.surface_panel
    };
    let fg = if primary {
        pal.accent_text
    } else {
        pal.text_primary
    };
    let border = if primary {
        pal.accent
    } else {
        pal.border_strong
    };
    ui.painter().rect_filled(rect, 6.0, bg);
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let ir = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        Vec2::splat(16.0),
    );
    paint_icon(ui.painter(), ir, icon, fg);
    ui.painter().text(
        egui::pos2(rect.left() + 30.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        fg,
    );
    resp.clicked()
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
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (tr, _) = ui.allocate_exact_size(Vec2::splat(38.0), egui::Sense::hover());
                draw_style_thumbnail(ui.painter(), tr, tpl.params.style, false, pal);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&tpl.name).strong().size(13.2));
                    ui.label(
                        RichText::new(format!(
                            "{} | {}",
                            tpl.params.style.name(),
                            unit.format_dims(tpl.params.opening_width, tpl.params.opening_height)
                        ))
                        .small()
                        .color(pal.text_secondary),
                    );
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
                    let use_btn =
                        egui::Button::new(RichText::new("Use").strong().color(pal.accent_text))
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
