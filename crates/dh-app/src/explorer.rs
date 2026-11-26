use crate::canvas2d::CanvasDragPayload;
use crate::icons::{VectorIcon, draw_style_thumbnail, paint_icon};
use crate::theme::StudioPalette;
use dh_core::{DisplayUnit, Project, ProjectKind, WindowStyle};
use eframe::egui::{
    self, CornerRadius, Id, Margin, Order, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BrowserState {
    pub open: bool,
    pub collapsed: bool,
    pub wall_expanded: HashMap<String, bool>,
    pub components_expanded: bool,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            open: true,
            collapsed: false,
            wall_expanded: HashMap::new(),
            components_expanded: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExplorerAction {
    HideBrowser,
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
}

/// Renders a translucent Fusion-360-style collapsible Scene Browser floating over the top-left
/// of `viewport_rect` (clearing engineering rulers and auto-minimizing during photo edit handles).
#[allow(clippy::too_many_arguments)]
pub fn draw_floating_browser(
    ctx: &egui::Context,
    pal: &StudioPalette,
    viewport_rect: Rect,
    rulers_visible: bool,
    photo_edit_mode: bool,
    project: &Project,
    active_component_id: Option<&str>,
    selected_opening_ids: &[String],
    unit: DisplayUnit,
    state: &mut BrowserState,
) -> Vec<ExplorerAction> {
    let mut actions = Vec::new();
    if !state.open || viewport_rect.width() < 260.0 || viewport_rect.height() < 160.0 {
        return actions;
    }

    let x_pad = if rulers_visible { 30.0 } else { 12.0 };
    let y_pad = if rulers_visible { 30.0 } else { 12.0 };
    let anchor_pos = Pos2::new(viewport_rect.left() + x_pad, viewport_rect.top() + y_pad);
    let effective_collapsed = state.collapsed || photo_edit_mode;
    let ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

    if effective_collapsed {
        let summary = match project.manifest.kind {
            ProjectKind::SinglePiece => project
                .active_piece()
                .map(|p| format!("{} ({})", p.name, project.pieces.len()))
                .unwrap_or_else(|| "Pieces".to_string()),
            ProjectKind::MultiWall => project
                .active_wall()
                .map(|w| format!("{} ({})", w.name, w.openings.len()))
                .unwrap_or_else(|| format!("Walls ({})", project.walls.len())),
        };
        egui::Area::new(Id::unique("floating_scene_browser"))
            .fixed_pos(anchor_pos)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                let pill_resp = egui::Frame::NONE
                    .fill(pal.hud_glass)
                    .stroke(Stroke::new(1.0, pal.border_strong))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (cr, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                            paint_icon(ui.painter(), cr, VectorIcon::ChevronRight, pal.text_muted);
                            let (ir, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                            paint_icon(ui.painter(), ir, VectorIcon::BrowserTree, pal.accent);
                            ui.label(
                                RichText::new(if photo_edit_mode {
                                    format!("Browser • {summary} (Photo Mode)")
                                } else {
                                    format!("Browser • {summary}")
                                })
                                .small()
                                .strong()
                                .color(pal.text_primary),
                            );
                        });
                    })
                    .response
                    .interact(Sense::click());
                if pill_resp.clicked() && !photo_edit_mode {
                    state.collapsed = false;
                }
            });
        return actions;
    }

    let panel_w = 224.0_f32.min((viewport_rect.width() - x_pad - 16.0).max(160.0));
    let max_h = (viewport_rect.height() - y_pad - 18.0).clamp(150.0, 440.0);

    egui::Area::new(Id::unique("floating_scene_browser"))
        .fixed_pos(anchor_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(pal.hud_glass)
                .stroke(Stroke::new(1.0, pal.border_strong))
                .corner_radius(CornerRadius::same(8))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 14,
                    spread: 1,
                    color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 58),
                })
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    ui.set_max_height(max_h);

                    // Top Browser Header Bar
                    ui.horizontal(|ui| {
                        let (cr, cr_resp) =
                            ui.allocate_exact_size(Vec2::splat(15.0), Sense::click());
                        paint_icon(ui.painter(), cr, VectorIcon::ChevronDown, pal.text_muted);
                        if cr_resp.on_hover_text("Collapse Browser").clicked() {
                            state.collapsed = true;
                        }
                        let (ir, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                        paint_icon(ui.painter(), ir, VectorIcon::BrowserTree, pal.accent);
                        ui.label(
                            RichText::new("Browser")
                                .strong()
                                .size(12.6)
                                .color(pal.text_primary),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("×")
                                .on_hover_text("Hide Browser overlay (reopen from top toolbar)")
                                .clicked()
                            {
                                actions.push(ExplorerAction::HideBrowser);
                            }
                            let add_lbl = match project.manifest.kind {
                                ProjectKind::SinglePiece => "+ Piece",
                                ProjectKind::MultiWall => "+ Wall",
                            };
                            if ui.small_button(add_lbl).clicked() {
                                match project.manifest.kind {
                                    ProjectKind::SinglePiece => {
                                        actions.push(ExplorerAction::AddStandalonePiece(
                                            WindowStyle::GeorgianGrid,
                                        ))
                                    }
                                    ProjectKind::MultiWall => {
                                        actions.push(ExplorerAction::AddBlankWall)
                                    }
                                }
                            }
                        });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .id_salt("floating_browser_scroll")
                        .max_height((max_h - 44.0).max(90.0))
                        .auto_shrink([false, true])
                        .show(ui, |ui| match project.manifest.kind {
                            ProjectKind::SinglePiece => {
                                draw_single_piece_tree(ui, pal, project, unit, &mut actions);
                            }
                            ProjectKind::MultiWall => {
                                draw_multi_wall_tree(
                                    ui,
                                    pal,
                                    project,
                                    active_component_id,
                                    selected_opening_ids,
                                    unit,
                                    ctrl,
                                    state,
                                    &mut actions,
                                );
                            }
                        });
                });
        });

    actions
}

fn draw_single_piece_tree(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    unit: DisplayUnit,
    actions: &mut Vec<ExplorerAction>,
) {
    for piece in &project.pieces {
        let active = project.manifest.active_piece_id.as_deref() == Some(piece.id.as_str());
        let resp = ui
            .horizontal(|ui| {
                let (tr, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                draw_style_thumbnail(ui.painter(), tr, piece.params.style, active, pal);
                ui.add(
                    egui::Label::new(
                        RichText::new(&piece.name)
                            .size(12.2)
                            .strong()
                            .color(if active { pal.accent } else { pal.text_primary }),
                    )
                    .truncate()
                    .sense(Sense::click()),
                )
            })
            .inner;
        let dim_tip = unit.format_dims(piece.params.opening_width, piece.params.opening_height);
        if resp.clone().on_hover_text(dim_tip).clicked() {
            actions.push(ExplorerAction::SelectPiece(piece.id.clone()));
        }
        if project.pieces.len() > 1 {
            resp.context_menu(|ui| {
                if ui.button("Delete Piece").clicked() {
                    actions.push(ExplorerAction::DeleteStandalonePiece(piece.id.clone()));
                    ui.close();
                }
            });
        }
        ui.add_space(2.0);
    }

    ui.add_space(6.0);
    if ui
        .add(
            egui::Button::new(RichText::new("Promote to Multi-Wall").small())
                .min_size(Vec2::new(ui.available_width(), 24.0)),
        )
        .clicked()
    {
        actions.push(ExplorerAction::PromoteSinglePieceToMultiWall);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_multi_wall_tree(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    active_component_id: Option<&str>,
    selected_opening_ids: &[String],
    unit: DisplayUnit,
    ctrl: bool,
    state: &mut BrowserState,
    actions: &mut Vec<ExplorerAction>,
) {
    let active_wall_id = project.manifest.active_wall_id.as_deref();

    for wall in &project.walls {
        let is_active = active_wall_id == Some(wall.id.as_str());
        let expanded = state
            .wall_expanded
            .get(&wall.id)
            .copied()
            .unwrap_or(is_active);

        let row_resp = ui
            .horizontal(|ui| {
                let (cr, cr_resp) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::click());
                paint_icon(
                    ui.painter(),
                    cr,
                    if expanded {
                        VectorIcon::ChevronDown
                    } else {
                        VectorIcon::ChevronRight
                    },
                    pal.text_muted,
                );
                if cr_resp.clicked() {
                    state.wall_expanded.insert(wall.id.clone(), !expanded);
                }

                let (wr, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                paint_icon(
                    ui.painter(),
                    wr,
                    VectorIcon::Wall,
                    if is_active {
                        pal.accent
                    } else {
                        pal.text_muted
                    },
                );

                let lbl_w = (ui.available_width() - 26.0).max(60.0);
                let lbl_resp = ui
                    .allocate_ui_with_layout(
                        Vec2::new(lbl_w, 18.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&wall.name).size(12.2).strong().color(
                                        if is_active {
                                            pal.accent
                                        } else {
                                            pal.text_primary
                                        },
                                    ),
                                )
                                .truncate()
                                .sense(Sense::click()),
                            )
                        },
                    )
                    .inner;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}", wall.openings.len()))
                            .small()
                            .color(pal.text_muted),
                    );
                });
                lbl_resp
            })
            .inner;

        let wall_tip = unit.format_dims(wall.width, wall.height);
        if row_resp.clone().on_hover_text(wall_tip).clicked() {
            actions.push(ExplorerAction::SelectWall(wall.id.clone()));
            state.wall_expanded.insert(wall.id.clone(), true);
        }
        row_resp.context_menu(|ui| {
            if ui.button("Duplicate Wall").clicked() {
                actions.push(ExplorerAction::DuplicateWall(wall.id.clone()));
                ui.close();
            }
            if project.walls.len() > 1 && ui.button("Delete Wall").clicked() {
                actions.push(ExplorerAction::DeleteWall(wall.id.clone()));
                ui.close();
            }
        });

        if expanded {
            for op in &wall.openings {
                let selected = is_active
                    && active_component_id.is_none()
                    && selected_opening_ids.contains(&op.id);
                let is_comp = op.is_component();
                let locked = op.lock_position || op.lock_width || op.lock_height;
                let is_door = op.resolve_params(&project.components).style.is_door();

                let r = ui
                    .horizontal(|ui| {
                        ui.add_space(16.0);
                        let (ir, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        let ic = if is_comp {
                            VectorIcon::ComponentLink
                        } else if is_door {
                            VectorIcon::Door
                        } else {
                            VectorIcon::Window
                        };
                        paint_icon(
                            ui.painter(),
                            ir,
                            ic,
                            if selected {
                                pal.gold
                            } else if is_comp {
                                pal.accent
                            } else {
                                pal.text_muted
                            },
                        );

                        let right_pad = if locked { 16.0 } else { 4.0 };
                        let text_w = (ui.available_width() - right_pad).max(52.0);
                        let sel_r = ui
                            .allocate_ui_with_layout(
                                Vec2::new(text_w, 18.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.selectable_label(
                                        selected,
                                        RichText::new(&op.name).size(11.6),
                                    )
                                },
                            )
                            .inner;
                        if locked {
                            let (lr, _) = ui.allocate_exact_size(Vec2::splat(11.0), Sense::hover());
                            paint_icon(ui.painter(), lr, VectorIcon::Lock, pal.gold);
                        }
                        sel_r
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
                    if is_comp {
                        if ui.button("Make Unique").clicked() {
                            actions.push(ExplorerAction::MakeOpeningUnique {
                                wall_id: wall.id.clone(),
                                opening_id: op.id.clone(),
                            });
                            ui.close();
                        }
                    } else if ui.button("Make Shared Component").clicked() {
                        actions.push(ExplorerAction::PromoteOpeningToComponent {
                            wall_id: wall.id.clone(),
                            opening_id: op.id.clone(),
                        });
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
        ui.add_space(2.0);
    }

    // Shared Components Branch
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);
    let comp_hdr = ui
        .horizontal(|ui| {
            let (cr, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            paint_icon(
                ui.painter(),
                cr,
                if state.components_expanded {
                    VectorIcon::ChevronDown
                } else {
                    VectorIcon::ChevronRight
                },
                pal.text_muted,
            );
            let (ir, _) = ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
            paint_icon(ui.painter(), ir, VectorIcon::ComponentLink, pal.accent);
            ui.add(
                egui::Label::new(
                    RichText::new(format!("Components ({})", project.components.len()))
                        .small()
                        .strong()
                        .color(pal.text_secondary),
                )
                .truncate()
                .sense(Sense::click()),
            )
        })
        .inner;
    if comp_hdr.clicked() {
        state.components_expanded = !state.components_expanded;
    }

    if state.components_expanded {
        if project.components.is_empty() {
            ui.label(
                RichText::new("  None yet (select opening -> Make Component)")
                    .small()
                    .color(pal.text_muted),
            );
        } else {
            for comp in &project.components {
                let (inst_n, _) = project.component_usage(&comp.id);
                let sel = active_component_id == Some(comp.id.as_str());
                let (lbl_resp, add_clicked) = ui
                    .horizontal(|ui| {
                        ui.add_space(12.0);
                        let (tr, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        draw_style_thumbnail(ui.painter(), tr, comp.params.style, sel, pal);
                        let w = (ui.available_width() - 42.0).max(52.0);
                        let lr = ui
                            .allocate_ui_with_layout(
                                Vec2::new(w, 18.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| ui.selectable_label(sel, RichText::new(&comp.name).size(11.4)),
                            )
                            .inner;
                        let mut placed = false;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("+")
                                .on_hover_text("Place linked instance on active wall")
                                .clicked()
                            {
                                placed = true;
                            }
                            ui.label(
                                RichText::new(format!("{inst_n}x"))
                                    .small()
                                    .color(pal.text_muted),
                            );
                        });
                        (lr, placed)
                    })
                    .inner;
                if add_clicked {
                    actions.push(ExplorerAction::PlaceComponentOnActiveWall(comp.id.clone()));
                }
                let drag_r = lbl_resp.interact(Sense::click_and_drag());
                if drag_r.drag_started() {
                    egui::DragAndDrop::set_payload(
                        ui.ctx(),
                        CanvasDragPayload::Component(comp.id.clone()),
                    );
                } else if drag_r.clicked() {
                    actions.push(ExplorerAction::SelectComponent(comp.id.clone()));
                }
                drag_r.context_menu(|ui| {
                    if ui.button("Place Instance on Active Wall").clicked() {
                        actions.push(ExplorerAction::PlaceComponentOnActiveWall(comp.id.clone()));
                        ui.close();
                    }
                    if inst_n == 0 && ui.button("Delete Component").clicked() {
                        actions.push(ExplorerAction::DeleteComponent(comp.id.clone()));
                        ui.close();
                    }
                });
            }
        }
    }
}
