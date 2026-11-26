use crate::canvas2d::CanvasDragPayload;
use crate::icons::{VectorIcon, draw_style_thumbnail, paint_icon};
use crate::theme::StudioPalette;
use dh_core::{DisplayUnit, PieceTemplate, Project, ProjectKind, WindowStyle};
use eframe::egui::{self, CornerRadius, Margin, RichText, Sense, Stroke, Vec2};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExplorerSelection {
    #[default]
    None,
    Piece(String),
    Wall(String),
    Opening {
        wall_id: String,
        opening_id: String,
    },
    Component(String),
}

#[derive(Debug, Clone)]
pub enum ExplorerAction {
    SelectWall(String),
    SelectOpening {
        wall_id: String,
        opening_id: String,
        toggle: bool,
    },
    OpenOpeningIn3D {
        wall_id: String,
        opening_id: String,
    },
    SelectComponent(String),
    SelectPiece(String),
    AddBlankWall,
    DuplicateWall(String),
    DeleteWall(String),
    AddStandalonePiece(WindowStyle),
    DeleteStandalonePiece(String),
    PromoteSinglePieceToMultiWall,
    PlaceComponentOnActiveWall(String),
    DuplicateOpening {
        wall_id: String,
        opening_id: String,
    },
    DeleteOpening {
        wall_id: String,
        opening_id: String,
    },
    PromoteOpeningToComponent {
        wall_id: String,
        opening_id: String,
    },
    MakeOpeningUnique {
        wall_id: String,
        opening_id: String,
    },
    DeleteComponent(String),
    InsertTemplateOnActiveWall(PieceTemplate),
    ApplyTemplateStyleOnly(PieceTemplate),
    SaveSelectionAsTemplate,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_project_explorer(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    active_component_id: Option<&str>,
    selected_opening_ids: &[String],
    personal_templates: &[PieceTemplate],
    starter_templates: &[PieceTemplate],
    unit: DisplayUnit,
) -> Vec<ExplorerAction> {
    let mut actions = Vec::new();
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

    egui::ScrollArea::vertical()
        .id_salt("project_explorer_scroll")
        .show(ui, |ui| {
            match project.manifest.kind {
                ProjectKind::SinglePiece => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("PIECES").small().strong().color(pal.text_muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("+ Add").clicked() {
                                actions.push(ExplorerAction::AddStandalonePiece(
                                    WindowStyle::GeorgianGrid,
                                ));
                            }
                        });
                    });
                    ui.add_space(4.0);

                    for piece in &project.pieces {
                        let active = project.manifest.active_piece_id.as_deref() == Some(piece.id.as_str());
                        let fill = if active {
                            pal.accent.linear_multiply(0.18)
                        } else {
                            pal.surface_card
                        };
                        let resp = egui::Frame::NONE
                            .fill(fill)
                            .stroke(Stroke::new(1.0, if active { pal.accent } else { pal.border }))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (tr, _) = ui.allocate_exact_size(
                                        Vec2::splat(26.0),
                                        Sense::hover(),
                                    );
                                    draw_style_thumbnail(
                                        ui.painter(),
                                        tr,
                                        piece.params.style,
                                        active,
                                        pal,
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&piece.name).strong().size(13.0));
                                        ui.label(
                                            RichText::new(unit.format_dims(
                                                piece.params.opening_width,
                                                piece.params.opening_height,
                                            ))
                                            .small()
                                            .color(pal.text_muted),
                                        );
                                    });
                                });
                            })
                            .response
                            .interact(Sense::click());
                        if resp.clicked() {
                            actions.push(ExplorerAction::SelectPiece(piece.id.clone()));
                        }
                        if project.pieces.len() > 1 {
                            resp.context_menu(|ui| {
                                if ui.button("Delete Piece").clicked() {
                                    actions.push(ExplorerAction::DeleteStandalonePiece(
                                        piece.id.clone(),
                                    ));
                                    ui.close();
                                }
                            });
                        }
                        ui.add_space(4.0);
                    }

                    ui.add_space(8.0);
                    if ui
                        .add(
                            egui::Button::new("Promote to House / Multi-Wall")
                                .min_size(Vec2::new(ui.available_width(), 28.0)),
                        )
                        .clicked()
                    {
                        actions.push(ExplorerAction::PromoteSinglePieceToMultiWall);
                    }
                }
                ProjectKind::MultiWall => {
                    // 1) WALLS TREE
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("WALLS").small().strong().color(pal.text_muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("+ Wall").clicked() {
                                actions.push(ExplorerAction::AddBlankWall);
                            }
                        });
                    });
                    ui.add_space(4.0);

                    let active_wall_id = project.manifest.active_wall_id.as_deref();
                    for wall in &project.walls {
                        let is_active_wall = active_wall_id == Some(wall.id.as_str());
                        egui::Frame::NONE
                            .fill(if is_active_wall {
                                pal.surface_card
                            } else {
                                pal.surface_panel
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if is_active_wall { pal.accent } else { pal.border },
                            ))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                let header_resp = ui
                                    .horizontal(|ui| {
                                        let (ir, _) = ui.allocate_exact_size(
                                            Vec2::splat(16.0),
                                            Sense::hover(),
                                        );
                                        paint_icon(ui.painter(), ir, VectorIcon::Wall, pal.accent);
                                        ui.label(
                                            RichText::new(&wall.name)
                                                .strong()
                                                .size(13.2)
                                                .color(if is_active_wall {
                                                    pal.accent
                                                } else {
                                                    pal.text_primary
                                                }),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{}",
                                                        wall.openings.len()
                                                    ))
                                                    .small()
                                                    .color(pal.text_muted),
                                                );
                                            },
                                        );
                                    })
                                    .response
                                    .interact(Sense::click());

                                if header_resp.clicked() {
                                    actions.push(ExplorerAction::SelectWall(wall.id.clone()));
                                }
                                header_resp.context_menu(|ui| {
                                    if ui.button("Duplicate Wall").clicked() {
                                        actions.push(ExplorerAction::DuplicateWall(wall.id.clone()));
                                        ui.close();
                                    }
                                    if project.walls.len() > 1 && ui.button("Delete Wall").clicked() {
                                        actions.push(ExplorerAction::DeleteWall(wall.id.clone()));
                                        ui.close();
                                    }
                                });

                                if is_active_wall {
                                    ui.add_space(4.0);
                                    for op in &wall.openings {
                                        let selected = active_component_id.is_none()
                                            && selected_opening_ids.contains(&op.id);
                                        let comp_tag = op.component_id().and_then(|cid| {
                                            project
                                                .component(cid)
                                                .map(|c| c.name.clone())
                                        });
                                        let text = if let Some(ref cn) = comp_tag {
                                            format!("{} [{}]", op.name, cn)
                                        } else {
                                            op.name.clone()
                                        };
                                        let r = ui
                                            .horizontal(|ui| {
                                                ui.add_space(10.0);
                                                let (ir, _) = ui.allocate_exact_size(
                                                    Vec2::splat(13.0),
                                                    Sense::hover(),
                                                );
                                                let ic = if comp_tag.is_some() {
                                                    VectorIcon::ComponentLink
                                                } else if op
                                                    .resolve_params(&project.components)
                                                    .style
                                                    .is_door()
                                                {
                                                    VectorIcon::Door
                                                } else {
                                                    VectorIcon::Window
                                                };
                                                paint_icon(
                                                    ui.painter(),
                                                    ir,
                                                    ic,
                                                    if selected { pal.gold } else { pal.text_muted },
                                                );
                                                ui.selectable_label(selected, text)
                                            })
                                            .inner;
                                        if r.clicked() {
                                            actions.push(ExplorerAction::SelectOpening {
                                                wall_id: wall.id.clone(),
                                                opening_id: op.id.clone(),
                                                toggle: ctrl,
                                            });
                                        }
                                        if r.double_clicked() {
                                            actions.push(ExplorerAction::OpenOpeningIn3D {
                                                wall_id: wall.id.clone(),
                                                opening_id: op.id.clone(),
                                            });
                                        }
                                        r.context_menu(|ui| {
                                            if ui.button("Open in Piece 3D").clicked() {
                                                actions.push(ExplorerAction::OpenOpeningIn3D {
                                                    wall_id: wall.id.clone(),
                                                    opening_id: op.id.clone(),
                                                });
                                                ui.close();
                                            }
                                            if ui.button("Duplicate").clicked() {
                                                actions.push(ExplorerAction::DuplicateOpening {
                                                    wall_id: wall.id.clone(),
                                                    opening_id: op.id.clone(),
                                                });
                                                ui.close();
                                            }
                                            if op.is_component() {
                                                if ui.button("Make Unique").clicked() {
                                                    actions.push(
                                                        ExplorerAction::MakeOpeningUnique {
                                                            wall_id: wall.id.clone(),
                                                            opening_id: op.id.clone(),
                                                        },
                                                    );
                                                    ui.close();
                                                }
                                            } else if ui.button("Make Shared Component").clicked() {
                                                actions.push(
                                                    ExplorerAction::PromoteOpeningToComponent {
                                                        wall_id: wall.id.clone(),
                                                        opening_id: op.id.clone(),
                                                    },
                                                );
                                                ui.close();
                                            }
                                            if ui.button("Delete Opening").clicked() {
                                                actions.push(ExplorerAction::DeleteOpening {
                                                    wall_id: wall.id.clone(),
                                                    opening_id: op.id.clone(),
                                                });
                                                ui.close();
                                            }
                                        });
                                    }
                                }
                            });
                        ui.add_space(4.0);
                    }

                    // 2) SHARED COMPONENTS (INSTANCING)
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("SHARED COMPONENTS (DRAG TO WALL)")
                            .small()
                            .strong()
                            .color(pal.text_muted),
                    );
                    ui.add_space(4.0);

                    if project.components.is_empty() {
                        ui.label(
                            RichText::new("Select an opening and click Make Component to link shared edits across walls.")
                                .small()
                                .color(pal.text_muted),
                        );
                    } else {
                        for comp in &project.components {
                            let (inst_n, wall_n) = project.component_usage(&comp.id);
                            let selected = active_component_id == Some(comp.id.as_str());
                            let card = egui::Frame::NONE
                                .fill(if selected {
                                    pal.accent.linear_multiply(0.16)
                                } else {
                                    pal.surface_card
                                })
                                .stroke(Stroke::new(
                                    1.0,
                                    if selected { pal.accent } else { pal.border },
                                ))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (tr, _) = ui.allocate_exact_size(
                                            Vec2::splat(26.0),
                                            Sense::hover(),
                                        );
                                        draw_style_thumbnail(
                                            ui.painter(),
                                            tr,
                                            comp.params.style,
                                            selected,
                                            pal,
                                        );
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&comp.name).strong().size(12.8));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{inst_n}x in {wall_n} wall(s)"
                                                ))
                                                .small()
                                                .color(pal.text_muted),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("+ Place")
                                                    .on_hover_text("Place linked instance on active wall")
                                                    .clicked()
                                                {
                                                    actions.push(
                                                        ExplorerAction::PlaceComponentOnActiveWall(
                                                            comp.id.clone(),
                                                        ),
                                                    );
                                                }
                                            },
                                        );
                                    });
                                });

                            let drag_resp = card.response.interact(Sense::click_and_drag());
                            if drag_resp.drag_started() {
                                egui::DragAndDrop::set_payload(
                                    ui.ctx(),
                                    CanvasDragPayload::Component(comp.id.clone()),
                                );
                            }
                            if drag_resp.clicked() {
                                actions.push(ExplorerAction::SelectComponent(comp.id.clone()));
                            }
                            drag_resp.context_menu(|ui| {
                                if inst_n == 0 && ui.button("Delete Component").clicked() {
                                    actions.push(ExplorerAction::DeleteComponent(comp.id.clone()));
                                    ui.close();
                                } else if inst_n > 0 {
                                    ui.label(
                                        RichText::new("Used on wall (Make Unique first to delete)")
                                            .small()
                                            .color(pal.text_muted),
                                    );
                                }
                            });
                            ui.add_space(4.0);
                        }
                    }
                }
            }

            // 3) TEMPLATES LIBRARY (Personal + Project + Starter)
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("TEMPLATES")
                        .small()
                        .strong()
                        .color(pal.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Save Selection").clicked() {
                        actions.push(ExplorerAction::SaveSelectionAsTemplate);
                    }
                });
            });
            ui.add_space(4.0);

            for (group_lbl, list) in [
                ("Project Templates", project.project_templates.as_slice()),
                ("Personal Library", personal_templates),
                ("Starters", starter_templates),
            ] {
                if list.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(RichText::new(format!("{group_lbl} ({})", list.len())).small())
                    .default_open(group_lbl != "Starters")
                    .show(ui, |ui| {
                        for tpl in list {
                            let item = egui::Frame::NONE
                                .fill(pal.surface_card)
                                .stroke(Stroke::new(1.0, pal.border))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(6, 5))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (tr, _) = ui.allocate_exact_size(
                                            Vec2::splat(22.0),
                                            Sense::hover(),
                                        );
                                        draw_style_thumbnail(
                                            ui.painter(),
                                            tr,
                                            tpl.params.style,
                                            false,
                                            pal,
                                        );
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&tpl.name).size(12.2).strong());
                                            ui.label(
                                                RichText::new(unit.format_dims(
                                                    tpl.params.opening_width,
                                                    tpl.params.opening_height,
                                                ))
                                                .small()
                                                .color(pal.text_muted),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("Style")
                                                    .on_hover_text("Apply styling only (keeps selected size)")
                                                    .clicked()
                                                {
                                                    actions.push(
                                                        ExplorerAction::ApplyTemplateStyleOnly(
                                                            tpl.clone(),
                                                        ),
                                                    );
                                                }
                                                if ui
                                                    .small_button("Insert")
                                                    .on_hover_text("Insert onto wall / apply")
                                                    .clicked()
                                                {
                                                    actions.push(
                                                        ExplorerAction::InsertTemplateOnActiveWall(
                                                            tpl.clone(),
                                                        ),
                                                    );
                                                }
                                            },
                                        );
                                    });
                                });
                            let r = item.response.interact(Sense::click_and_drag());
                            if r.drag_started() {
                                egui::DragAndDrop::set_payload(
                                    ui.ctx(),
                                    CanvasDragPayload::Template(Box::new(tpl.clone())),
                                );
                            }
                            ui.add_space(3.0);
                        }
                    });
            }
        });

    actions
}
