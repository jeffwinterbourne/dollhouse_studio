use crate::canvas2d::CanvasDragPayload;
use crate::icons::{VectorIcon, draw_style_thumbnail, paint_icon};
use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{DisplayUnit, PieceTemplate, Project, ProjectKind, WindowParams, WindowStyle};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryTab {
    #[default]
    Styles,
    Components,
    Templates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryFilter {
    #[default]
    All,
    Windows,
    Doors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GalleryViewMode {
    #[default]
    Grid,
    List,
}

#[derive(Debug, Clone, Default)]
pub struct GalleryState {
    pub tab: GalleryTab,
    pub filter: GalleryFilter,
    pub view_mode: GalleryViewMode,
    pub search: String,
}

#[derive(Debug, Clone)]
pub enum GalleryAction {
    HideGallery,
    InsertStyle(WindowStyle),
    ApplyStyleToSelected(WindowStyle),
    SelectComponent(String),
    PlaceComponentOnActiveWall(String),
    DeleteComponent(String),
    InsertTemplate(PieceTemplate),
    ApplyTemplateStyleOnly(PieceTemplate),
    SaveSelectionAsTemplate,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_visual_gallery(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    active_component_id: Option<&str>,
    has_selection: bool,
    personal_templates: &[PieceTemplate],
    starter_templates: &[PieceTemplate],
    unit: DisplayUnit,
    state: &mut GalleryState,
) -> Vec<GalleryAction> {
    let mut actions = Vec::new();
    let is_multi = project.manifest.kind == ProjectKind::MultiWall;
    if !is_multi && state.tab == GalleryTab::Components {
        state.tab = GalleryTab::Styles;
    }

    // 1) Compact Header Bar ("Library", Grid/List toggle, Hide button)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Library")
                .strong()
                .size(13.5)
                .color(pal.text_primary),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if gallery_icon_btn(ui, pal, VectorIcon::ChevronLeft, false)
                .on_hover_text("Hide Visual Library pane")
                .clicked()
            {
                actions.push(GalleryAction::HideGallery);
            }
            if gallery_icon_btn(
                ui,
                pal,
                VectorIcon::ListView,
                state.view_mode == GalleryViewMode::List,
            )
            .on_hover_text("List view")
            .clicked()
            {
                state.view_mode = GalleryViewMode::List;
            }
            if gallery_icon_btn(
                ui,
                pal,
                VectorIcon::GridView,
                state.view_mode == GalleryViewMode::Grid,
            )
            .on_hover_text("Visual grid view")
            .clicked()
            {
                state.view_mode = GalleryViewMode::Grid;
            }
        });
    });
    ui.add_space(4.0);

    // 2) Tab Bar (Styles | Components | Templates)
    if is_multi {
        let mut t = state.tab;
        if segmented_pill(
            ui,
            pal,
            &mut t,
            &[
                (GalleryTab::Styles, "Styles"),
                (GalleryTab::Components, "Comp"),
                (GalleryTab::Templates, "Templates"),
            ],
        ) {
            state.tab = t;
        }
    } else {
        let mut t = state.tab;
        if segmented_pill(
            ui,
            pal,
            &mut t,
            &[
                (GalleryTab::Styles, "Styles"),
                (GalleryTab::Templates, "Templates"),
            ],
        ) {
            state.tab = t;
        }
    }
    ui.add_space(4.0);

    // 3) Search + Filter Row (bounded to ui.available_width() so it never forces pane wider)
    let row_w = ui.available_width().max(120.0);
    ui.add(
        egui::TextEdit::singleline(&mut state.search)
            .desired_width(row_w)
            .hint_text("Search styles, templates..."),
    );
    if state.tab != GalleryTab::Components {
        ui.add_space(3.0);
        let mut f = state.filter;
        if segmented_pill(
            ui,
            pal,
            &mut f,
            &[
                (GalleryFilter::All, "All"),
                (GalleryFilter::Windows, "Windows"),
                (GalleryFilter::Doors, "Doors"),
            ],
        ) {
            state.filter = f;
        }
    }

    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);

    let q = state.search.trim().to_ascii_lowercase();

    egui::ScrollArea::vertical()
        .id_salt("visual_gallery_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| match state.tab {
            GalleryTab::Styles => {
                draw_styles_gallery(
                    ui,
                    pal,
                    is_multi,
                    has_selection,
                    unit,
                    state,
                    &q,
                    &mut actions,
                );
            }
            GalleryTab::Components => {
                draw_components_gallery(
                    ui,
                    pal,
                    project,
                    active_component_id,
                    state,
                    &q,
                    &mut actions,
                );
            }
            GalleryTab::Templates => {
                draw_templates_gallery(
                    ui,
                    pal,
                    is_multi,
                    has_selection,
                    project,
                    personal_templates,
                    starter_templates,
                    unit,
                    state,
                    &q,
                    &mut actions,
                );
            }
        });

    actions
}

#[allow(clippy::too_many_arguments)]
fn draw_styles_gallery(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    is_multi: bool,
    has_selection: bool,
    unit: DisplayUnit,
    state: &GalleryState,
    query: &str,
    actions: &mut Vec<GalleryAction>,
) {
    let styles: Vec<WindowStyle> = match state.filter {
        GalleryFilter::All => WindowStyle::ALL.to_vec(),
        GalleryFilter::Windows => WindowStyle::ALL_WINDOWS.to_vec(),
        GalleryFilter::Doors => WindowStyle::ALL_DOORS.to_vec(),
    }
    .into_iter()
    .filter(|s| {
        query.is_empty()
            || s.name().to_ascii_lowercase().contains(query)
            || s.subtitle().to_ascii_lowercase().contains(query)
    })
    .collect();

    if styles.is_empty() {
        ui.label(
            RichText::new("No matching styles.")
                .small()
                .color(pal.text_muted),
        );
        return;
    }

    ui.label(
        RichText::new(if is_multi {
            "Drag onto Blueprint 2D or click to add"
        } else {
            "Click any style to apply to piece"
        })
        .small()
        .color(pal.text_muted),
    );
    ui.add_space(4.0);

    match state.view_mode {
        GalleryViewMode::Grid => {
            let avail_w = ui.available_width().max(150.0);
            let gap = 6.0;
            let cols = ((avail_w + gap) / 118.0).floor().clamp(1.0, 4.0) as usize;
            let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
                .floor()
                .max(88.0);
            let card_h = 114.0;

            for chunk in styles.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &style in chunk {
                        draw_style_grid_card(
                            ui,
                            pal,
                            style,
                            card_w,
                            card_h,
                            is_multi,
                            has_selection,
                            unit,
                            actions,
                        );
                    }
                });
                ui.add_space(gap);
            }
        }
        GalleryViewMode::List => {
            let card_w = ui.available_width().max(150.0);
            for style in styles {
                draw_style_list_card(
                    ui,
                    pal,
                    style,
                    card_w,
                    is_multi,
                    has_selection,
                    unit,
                    actions,
                );
                ui.add_space(4.0);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_style_grid_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    style: WindowStyle,
    card_w: f32,
    card_h: f32,
    is_multi: bool,
    has_selection: bool,
    unit: DisplayUnit,
    actions: &mut Vec<GalleryAction>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click_and_drag());
    let hovered = resp.hovered();
    let fill = if hovered {
        pal.surface_elevated
    } else {
        pal.surface_card
    };
    let border = if hovered { pal.accent } else { pal.border };

    ui.painter().rect_filled(rect, 7.0, fill);
    ui.painter()
        .rect_stroke(rect, 7.0, Stroke::new(1.0, border), StrokeKind::Inside);

    let thumb_sz = (card_w - 22.0).clamp(54.0, 72.0);
    let thumb_rect = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + 8.0 + thumb_sz * 0.5),
        Vec2::splat(thumb_sz),
    );
    draw_style_thumbnail(ui.painter(), thumb_rect, style, hovered, pal);

    let clip_p = ui
        .painter()
        .with_clip_rect(rect.shrink2(Vec2::new(5.0, 2.0)));
    clip_p.text(
        Pos2::new(rect.center().x, thumb_rect.bottom() + 4.0),
        Align2::CENTER_TOP,
        style.name(),
        FontId::proportional(11.2),
        pal.text_primary,
    );

    let mut def_p = WindowParams::default();
    style.apply_style_defaults(&mut def_p);
    let dim_txt = unit.format_dims(def_p.opening_width, def_p.opening_height);
    clip_p.text(
        Pos2::new(rect.center().x, rect.bottom() - 5.0),
        Align2::CENTER_BOTTOM,
        dim_txt,
        FontId::proportional(9.8),
        pal.text_muted,
    );

    // Optional hover quick "Apply" badge in MultiWall when an opening is selected
    let apply_pill = Rect::from_min_size(
        Pos2::new(rect.right() - 44.0, rect.top() + 4.0),
        Vec2::new(40.0, 17.0),
    );
    let mut clicked_apply_pill = false;
    if hovered && is_multi && has_selection {
        let ptr_over_apply = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| apply_pill.contains(p));
        ui.painter().rect_filled(
            apply_pill,
            4.0,
            if ptr_over_apply {
                pal.accent
            } else {
                pal.hud_glass
            },
        );
        ui.painter().rect_stroke(
            apply_pill,
            4.0,
            Stroke::new(1.0, pal.accent),
            StrokeKind::Inside,
        );
        ui.painter().text(
            apply_pill.center(),
            Align2::CENTER_CENTER,
            "Apply",
            FontId::proportional(9.8),
            if ptr_over_apply {
                pal.accent_text
            } else {
                pal.text_primary
            },
        );
        if resp.clicked() && ptr_over_apply {
            clicked_apply_pill = true;
        }
    }

    if resp.drag_started() {
        egui::DragAndDrop::set_payload(ui.ctx(), CanvasDragPayload::Style(style));
    } else if clicked_apply_pill {
        actions.push(GalleryAction::ApplyStyleToSelected(style));
    } else if resp.clicked() {
        if is_multi {
            actions.push(GalleryAction::InsertStyle(style));
        } else {
            actions.push(GalleryAction::ApplyStyleToSelected(style));
        }
    }

    resp.clone()
        .on_hover_text(format!("{}\n{}", style.name(), style.subtitle()));
    resp.context_menu(|ui| {
        if is_multi {
            if ui.button("Add onto Active Wall").clicked() {
                actions.push(GalleryAction::InsertStyle(style));
                ui.close();
            }
            if has_selection && ui.button("Apply Style to Selected").clicked() {
                actions.push(GalleryAction::ApplyStyleToSelected(style));
                ui.close();
            }
        } else if ui.button("Apply Style to Active Piece").clicked() {
            actions.push(GalleryAction::ApplyStyleToSelected(style));
            ui.close();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_style_list_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    style: WindowStyle,
    card_w: f32,
    is_multi: bool,
    has_selection: bool,
    unit: DisplayUnit,
    actions: &mut Vec<GalleryAction>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 62.0), Sense::click_and_drag());
    let hovered = resp.hovered();
    ui.painter().rect_filled(
        rect,
        6.0,
        if hovered {
            pal.surface_elevated
        } else {
            pal.surface_card
        },
    );
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, if hovered { pal.accent } else { pal.border }),
        StrokeKind::Inside,
    );

    let thumb_rect = Rect::from_min_size(rect.min + Vec2::new(6.0, 6.0), Vec2::splat(50.0));
    draw_style_thumbnail(ui.painter(), thumb_rect, style, hovered, pal);

    let text_x = thumb_rect.right() + 9.0;
    let clip_p = ui.painter().with_clip_rect(Rect::from_min_max(
        Pos2::new(text_x, rect.top() + 3.0),
        Pos2::new(rect.right() - 6.0, rect.bottom() - 3.0),
    ));
    clip_p.text(
        Pos2::new(text_x, rect.top() + 10.0),
        Align2::LEFT_TOP,
        style.name(),
        FontId::proportional(12.6),
        pal.text_primary,
    );
    clip_p.text(
        Pos2::new(text_x, rect.top() + 27.0),
        Align2::LEFT_TOP,
        style.subtitle(),
        FontId::proportional(10.5),
        pal.text_secondary,
    );
    let mut def_p = WindowParams::default();
    style.apply_style_defaults(&mut def_p);
    clip_p.text(
        Pos2::new(text_x, rect.bottom() - 7.0),
        Align2::LEFT_BOTTOM,
        unit.format_dims(def_p.opening_width, def_p.opening_height),
        FontId::proportional(9.8),
        pal.text_muted,
    );

    let apply_pill = Rect::from_min_size(
        Pos2::new(rect.right() - 48.0, rect.bottom() - 22.0),
        Vec2::new(42.0, 17.0),
    );
    let mut clicked_apply_pill = false;
    if hovered && is_multi && has_selection {
        let ptr_over_apply = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| apply_pill.contains(p));
        ui.painter().rect_filled(
            apply_pill,
            4.0,
            if ptr_over_apply {
                pal.accent
            } else {
                pal.hud_glass
            },
        );
        ui.painter().rect_stroke(
            apply_pill,
            4.0,
            Stroke::new(1.0, pal.accent),
            StrokeKind::Inside,
        );
        ui.painter().text(
            apply_pill.center(),
            Align2::CENTER_CENTER,
            "Apply",
            FontId::proportional(9.8),
            if ptr_over_apply {
                pal.accent_text
            } else {
                pal.text_primary
            },
        );
        if resp.clicked() && ptr_over_apply {
            clicked_apply_pill = true;
        }
    }

    if resp.drag_started() {
        egui::DragAndDrop::set_payload(ui.ctx(), CanvasDragPayload::Style(style));
    } else if clicked_apply_pill {
        actions.push(GalleryAction::ApplyStyleToSelected(style));
    } else if resp.clicked() {
        if is_multi {
            actions.push(GalleryAction::InsertStyle(style));
        } else {
            actions.push(GalleryAction::ApplyStyleToSelected(style));
        }
    }

    resp.context_menu(|ui| {
        if is_multi {
            if ui.button("Add onto Active Wall").clicked() {
                actions.push(GalleryAction::InsertStyle(style));
                ui.close();
            }
            if has_selection && ui.button("Apply Style to Selected").clicked() {
                actions.push(GalleryAction::ApplyStyleToSelected(style));
                ui.close();
            }
        } else if ui.button("Apply Style to Active Piece").clicked() {
            actions.push(GalleryAction::ApplyStyleToSelected(style));
            ui.close();
        }
    });
}

fn draw_components_gallery(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    project: &Project,
    active_component_id: Option<&str>,
    state: &GalleryState,
    query: &str,
    actions: &mut Vec<GalleryAction>,
) {
    let comps: Vec<_> = project
        .components
        .iter()
        .filter(|c| query.is_empty() || c.name.to_ascii_lowercase().contains(query))
        .collect();

    if comps.is_empty() {
        ui.label(
            RichText::new("Select an opening and click 'Make Component' to create shared linked instances across walls.")
                .small()
                .color(pal.text_muted),
        );
        return;
    }

    let avail_w = ui.available_width().max(150.0);
    if state.view_mode == GalleryViewMode::Grid {
        let gap = 6.0;
        let cols = ((avail_w + gap) / 118.0).floor().clamp(1.0, 4.0) as usize;
        let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
            .floor()
            .max(88.0);
        for chunk in comps.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &comp in chunk {
                    let (inst_n, wall_n) = project.component_usage(&comp.id);
                    let sel = active_component_id == Some(comp.id.as_str());
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(card_w, 114.0), Sense::click_and_drag());
                    ui.painter().rect_filled(
                        rect,
                        7.0,
                        if sel {
                            pal.accent.linear_multiply(0.18)
                        } else if resp.hovered() {
                            pal.surface_elevated
                        } else {
                            pal.surface_card
                        },
                    );
                    ui.painter().rect_stroke(
                        rect,
                        7.0,
                        Stroke::new(
                            1.0,
                            if sel || resp.hovered() {
                                pal.accent
                            } else {
                                pal.border
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    let thumb_sz = (card_w - 22.0).clamp(52.0, 68.0);
                    let tr = Rect::from_center_size(
                        Pos2::new(rect.center().x, rect.top() + 8.0 + thumb_sz * 0.5),
                        Vec2::splat(thumb_sz),
                    );
                    draw_style_thumbnail(
                        ui.painter(),
                        tr,
                        comp.params.style,
                        sel || resp.hovered(),
                        pal,
                    );
                    let cp = ui
                        .painter()
                        .with_clip_rect(rect.shrink2(Vec2::new(5.0, 2.0)));
                    cp.text(
                        Pos2::new(rect.center().x, tr.bottom() + 4.0),
                        Align2::CENTER_TOP,
                        &comp.name,
                        FontId::proportional(11.2),
                        pal.text_primary,
                    );
                    cp.text(
                        Pos2::new(rect.center().x, rect.bottom() - 5.0),
                        Align2::CENTER_BOTTOM,
                        format!("{inst_n}x / {wall_n}w"),
                        FontId::proportional(9.8),
                        pal.text_muted,
                    );
                    if resp.drag_started() {
                        egui::DragAndDrop::set_payload(
                            ui.ctx(),
                            CanvasDragPayload::Component(comp.id.clone()),
                        );
                    } else if resp.clicked() {
                        actions.push(GalleryAction::SelectComponent(comp.id.clone()));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Place Instance on Active Wall").clicked() {
                            actions
                                .push(GalleryAction::PlaceComponentOnActiveWall(comp.id.clone()));
                            ui.close();
                        }
                        if inst_n == 0 && ui.button("Delete Component").clicked() {
                            actions.push(GalleryAction::DeleteComponent(comp.id.clone()));
                            ui.close();
                        }
                    });
                }
            });
            ui.add_space(gap);
        }
    } else {
        for comp in comps {
            let (inst_n, wall_n) = project.component_usage(&comp.id);
            let sel = active_component_id == Some(comp.id.as_str());
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(avail_w, 58.0), Sense::click_and_drag());
            ui.painter().rect_filled(
                rect,
                6.0,
                if sel {
                    pal.accent.linear_multiply(0.18)
                } else if resp.hovered() {
                    pal.surface_elevated
                } else {
                    pal.surface_card
                },
            );
            ui.painter().rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, if sel { pal.accent } else { pal.border }),
                StrokeKind::Inside,
            );
            let tr = Rect::from_min_size(rect.min + Vec2::new(6.0, 6.0), Vec2::splat(46.0));
            draw_style_thumbnail(ui.painter(), tr, comp.params.style, sel, pal);
            let tx = tr.right() + 8.0;
            let cp = ui.painter().with_clip_rect(Rect::from_min_max(
                Pos2::new(tx, rect.top() + 3.0),
                Pos2::new(rect.right() - 6.0, rect.bottom() - 3.0),
            ));
            cp.text(
                Pos2::new(tx, rect.top() + 10.0),
                Align2::LEFT_TOP,
                &comp.name,
                FontId::proportional(12.2),
                pal.text_primary,
            );
            cp.text(
                Pos2::new(tx, rect.bottom() - 10.0),
                Align2::LEFT_BOTTOM,
                format!("{inst_n}x in {wall_n} wall(s) • Drag or double-click to place"),
                FontId::proportional(10.0),
                pal.text_muted,
            );
            if resp.drag_started() {
                egui::DragAndDrop::set_payload(
                    ui.ctx(),
                    CanvasDragPayload::Component(comp.id.clone()),
                );
            } else if resp.double_clicked() {
                actions.push(GalleryAction::PlaceComponentOnActiveWall(comp.id.clone()));
            } else if resp.clicked() {
                actions.push(GalleryAction::SelectComponent(comp.id.clone()));
            }
            resp.context_menu(|ui| {
                if ui.button("Place Instance on Active Wall").clicked() {
                    actions.push(GalleryAction::PlaceComponentOnActiveWall(comp.id.clone()));
                    ui.close();
                }
                if inst_n == 0 && ui.button("Delete Component").clicked() {
                    actions.push(GalleryAction::DeleteComponent(comp.id.clone()));
                    ui.close();
                }
            });
            ui.add_space(4.0);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_templates_gallery(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    is_multi: bool,
    has_selection: bool,
    project: &Project,
    personal_templates: &[PieceTemplate],
    starter_templates: &[PieceTemplate],
    unit: DisplayUnit,
    state: &GalleryState,
    query: &str,
    actions: &mut Vec<GalleryAction>,
) {
    ui.horizontal(|ui| {
        if ui
            .small_button("+ Save Selection")
            .on_hover_text("Save current window or door setup as reusable template")
            .clicked()
        {
            actions.push(GalleryAction::SaveSelectionAsTemplate);
        }
    });
    ui.add_space(4.0);

    let matches_filter = |tpl: &PieceTemplate| {
        let cat_ok = match state.filter {
            GalleryFilter::All => true,
            GalleryFilter::Windows => !tpl.params.style.is_door(),
            GalleryFilter::Doors => tpl.params.style.is_door(),
        };
        let q_ok = query.is_empty()
            || tpl.name.to_ascii_lowercase().contains(query)
            || tpl.params.style.name().to_ascii_lowercase().contains(query);
        cat_ok && q_ok
    };

    for (group_lbl, list) in [
        ("Project Templates", project.project_templates.as_slice()),
        ("Personal Library", personal_templates),
        ("Starter Presets", starter_templates),
    ] {
        let filtered: Vec<&PieceTemplate> = list.iter().filter(|t| matches_filter(t)).collect();
        if filtered.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(
            RichText::new(format!("{group_lbl} ({})", filtered.len())).small(),
        )
        .default_open(group_lbl != "Starter Presets" || !query.is_empty())
        .show(ui, |ui| {
            let avail_w = ui.available_width().max(150.0);
            if state.view_mode == GalleryViewMode::Grid {
                let gap = 6.0;
                let cols = ((avail_w + gap) / 118.0).floor().clamp(1.0, 4.0) as usize;
                let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
                    .floor()
                    .max(88.0);
                for chunk in filtered.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &tpl in chunk {
                            draw_template_grid_card(
                                ui,
                                pal,
                                tpl,
                                card_w,
                                is_multi,
                                has_selection,
                                unit,
                                actions,
                            );
                        }
                    });
                    ui.add_space(gap);
                }
            } else {
                for &tpl in &filtered {
                    draw_template_list_card(
                        ui,
                        pal,
                        tpl,
                        avail_w,
                        is_multi,
                        has_selection,
                        unit,
                        actions,
                    );
                    ui.add_space(4.0);
                }
            }
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_template_grid_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    tpl: &PieceTemplate,
    card_w: f32,
    is_multi: bool,
    has_selection: bool,
    unit: DisplayUnit,
    actions: &mut Vec<GalleryAction>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 114.0), Sense::click_and_drag());
    let hovered = resp.hovered();
    ui.painter().rect_filled(
        rect,
        7.0,
        if hovered {
            pal.surface_elevated
        } else {
            pal.surface_card
        },
    );
    ui.painter().rect_stroke(
        rect,
        7.0,
        Stroke::new(1.0, if hovered { pal.accent } else { pal.border }),
        StrokeKind::Inside,
    );

    let thumb_sz = (card_w - 22.0).clamp(52.0, 68.0);
    let tr = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.top() + 8.0 + thumb_sz * 0.5),
        Vec2::splat(thumb_sz),
    );
    draw_style_thumbnail(ui.painter(), tr, tpl.params.style, hovered, pal);

    let cp = ui
        .painter()
        .with_clip_rect(rect.shrink2(Vec2::new(5.0, 2.0)));
    cp.text(
        Pos2::new(rect.center().x, tr.bottom() + 4.0),
        Align2::CENTER_TOP,
        &tpl.name,
        FontId::proportional(11.2),
        pal.text_primary,
    );
    cp.text(
        Pos2::new(rect.center().x, rect.bottom() - 5.0),
        Align2::CENTER_BOTTOM,
        unit.format_dims(tpl.params.opening_width, tpl.params.opening_height),
        FontId::proportional(9.8),
        pal.text_muted,
    );

    if resp.drag_started() {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            CanvasDragPayload::Template(Box::new(tpl.clone())),
        );
    } else if resp.clicked() {
        actions.push(GalleryAction::InsertTemplate(tpl.clone()));
    }

    resp.context_menu(|ui| {
        if ui
            .button(if is_multi {
                "Insert onto Active Wall"
            } else {
                "Apply Template"
            })
            .clicked()
        {
            actions.push(GalleryAction::InsertTemplate(tpl.clone()));
            ui.close();
        }
        if has_selection && ui.button("Apply Styling Only to Selected").clicked() {
            actions.push(GalleryAction::ApplyTemplateStyleOnly(tpl.clone()));
            ui.close();
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_template_list_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    tpl: &PieceTemplate,
    card_w: f32,
    is_multi: bool,
    has_selection: bool,
    unit: DisplayUnit,
    actions: &mut Vec<GalleryAction>,
) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(card_w, 58.0), Sense::click_and_drag());
    let hovered = resp.hovered();
    ui.painter().rect_filled(
        rect,
        6.0,
        if hovered {
            pal.surface_elevated
        } else {
            pal.surface_card
        },
    );
    ui.painter().rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0, if hovered { pal.accent } else { pal.border }),
        StrokeKind::Inside,
    );
    let tr = Rect::from_min_size(rect.min + Vec2::new(6.0, 6.0), Vec2::splat(46.0));
    draw_style_thumbnail(ui.painter(), tr, tpl.params.style, hovered, pal);
    let tx = tr.right() + 8.0;
    let cp = ui.painter().with_clip_rect(Rect::from_min_max(
        Pos2::new(tx, rect.top() + 3.0),
        Pos2::new(rect.right() - 6.0, rect.bottom() - 3.0),
    ));
    cp.text(
        Pos2::new(tx, rect.top() + 10.0),
        Align2::LEFT_TOP,
        &tpl.name,
        FontId::proportional(12.2),
        pal.text_primary,
    );
    cp.text(
        Pos2::new(tx, rect.bottom() - 10.0),
        Align2::LEFT_BOTTOM,
        unit.format_dims(tpl.params.opening_width, tpl.params.opening_height),
        FontId::proportional(10.0),
        pal.text_muted,
    );

    if resp.drag_started() {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            CanvasDragPayload::Template(Box::new(tpl.clone())),
        );
    } else if resp.clicked() {
        actions.push(GalleryAction::InsertTemplate(tpl.clone()));
    }

    resp.context_menu(|ui| {
        if ui
            .button(if is_multi {
                "Insert onto Active Wall"
            } else {
                "Apply Template"
            })
            .clicked()
        {
            actions.push(GalleryAction::InsertTemplate(tpl.clone()));
            ui.close();
        }
        if has_selection && ui.button("Apply Styling Only to Selected").clicked() {
            actions.push(GalleryAction::ApplyTemplateStyleOnly(tpl.clone()));
            ui.close();
        }
    });
}

fn gallery_icon_btn(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    icon: VectorIcon,
    active: bool,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        4.0,
        if active {
            pal.accent
        } else if resp.hovered() {
            pal.surface_elevated
        } else {
            pal.surface_card
        },
    );
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, if active { pal.accent } else { pal.border }),
        StrokeKind::Inside,
    );
    paint_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(13.5)),
        icon,
        if active {
            pal.accent_text
        } else {
            pal.text_primary
        },
    );
    resp
}
