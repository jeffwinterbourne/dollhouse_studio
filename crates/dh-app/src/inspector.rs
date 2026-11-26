use crate::theme::{
    StudioPalette, drag_unit_mm, section_card, slider_ratio_pct, slider_u32, slider_unit_mm,
};
use dh_core::{
    ArchitraveStyle, ComponentDef, DisplayUnit, DoorHandleStyle, FilamentColor, FlushTopMode,
    GrooveStyle, HeaderStyle, InstanceOverrides, OpeningSource, PrinterPreset, ProjectWall,
    ShutterStyle, StandalonePiece, WindowOpeningMode, WindowParams, WindowStyle,
};
use dh_geometry::{build_window, validate_generated_piece};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Stroke};

#[derive(Debug, Clone, Default)]
pub struct InspectorState {
    pub edit_shared_component: bool,
}

#[derive(Debug, Clone)]
pub enum InspectorAction {
    PromoteOpeningToComponent(String),
    MakeOpeningUnique(String),
    SaveAsTemplate,
    AutoDimensionWall,
    SyncCommonWallSettings,
    EditPhotoOnCanvas,
    OpenPhotoWizard,
    DeleteSelectedOpenings,
    AlignSelectedCenterH,
    AlignSelectedDistributeH,
    AlignSelectedMirrorH,
}

#[derive(Default)]
pub struct InspectorResult {
    pub live_changed: bool,
    pub immediate_commit_label: Option<&'static str>,
    pub drag_commit_label: Option<&'static str>,
    pub action: Option<InspectorAction>,
}

/// Renders the contextual properties inspector for an active Standalone Piece, shared ComponentDef,
/// Wall Opening (or multi-selection), or Wall properties.
#[allow(clippy::too_many_arguments)]
pub fn draw_piece_inspector_body(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    name: &mut String,
    params: &mut WindowParams,
    unit: DisplayUnit,
    ext_color: &mut FilamentColor,
    door_color: &mut FilamentColor,
    res: &mut InspectorResult,
) {
    // 1) Core Controls (clean 6-8 controls, never an overwhelming wall)
    section_card(ui, pal, Some("Core Properties"), |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Name").size(12.8));
            if ui
                .add(egui::TextEdit::singleline(name).desired_width(ui.available_width()))
                .changed()
            {
                res.live_changed = true;
                res.drag_commit_label = Some("Rename piece");
            }
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Style").size(12.8));
            egui::ComboBox::from_id_salt("insp_style_combo")
                .selected_text(params.style.name())
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for s in WindowStyle::ALL {
                        if ui.selectable_label(params.style == s, s.name()).clicked() {
                            params.style = s;
                            s.apply_style_defaults(params);
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change piece style");
                        }
                    }
                });
        });

        let w_ch = slider_unit_mm(
            ui,
            pal,
            "Opening Width",
            &mut params.opening_width,
            18.0..=260.0,
            unit,
        );
        if w_ch {
            if params.style == WindowStyle::CircularBullsEye {
                params.opening_height = params.opening_width;
            }
            res.live_changed = true;
            res.drag_commit_label = Some("Resize width");
        }

        let h_ch = slider_unit_mm(
            ui,
            pal,
            "Opening Height",
            &mut params.opening_height,
            18.0..=280.0,
            unit,
        );
        if h_ch {
            if params.style == WindowStyle::CircularBullsEye {
                params.opening_width = params.opening_height;
            }
            res.live_changed = true;
            res.drag_commit_label = Some("Resize height");
        }

        if slider_unit_mm(
            ui,
            pal,
            "Wall Thickness",
            &mut params.wall_thickness,
            3.0..=24.0,
            unit,
        ) {
            res.live_changed = true;
            res.drag_commit_label = Some("Change wall thickness");
        }

        // Dynamic grid / spoke controls matching active style
        if params.style.uses_grid_bars() {
            if slider_u32(ui, pal, "Vertical Bars", &mut params.bars_x, 0..=8) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change vertical glazing bars");
            }
            if slider_u32(ui, pal, "Horizontal Bars", &mut params.bars_y, 0..=8) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change horizontal glazing bars");
            }
        }
        if params.style.uses_radial_spokes()
            && slider_u32(ui, pal, "Radial Spokes", &mut params.arch_spokes, 0..=10)
        {
            res.live_changed = true;
            res.drag_commit_label = Some("Change radial spokes");
        }
        if params.style == WindowStyle::CottageLattice
            && slider_unit_mm(
                ui,
                pal,
                "Diamond Pitch",
                &mut params.diamond_pitch,
                6.0..=28.0,
                unit,
            )
        {
            res.live_changed = true;
            res.drag_commit_label = Some("Change lattice pitch");
        }

        if slider_unit_mm(
            ui,
            pal,
            "Trim Width",
            &mut params.trim_width,
            2.0..=16.0,
            unit,
        ) {
            res.live_changed = true;
            res.drag_commit_label = Some("Change trim width");
        }

        if params.style.supports_sill()
            && ui
                .checkbox(&mut params.sill_enabled, "Exterior Projecting Sill")
                .changed()
        {
            res.live_changed = true;
            res.immediate_commit_label = Some("Toggle sill");
        }

        if params.style.is_door() {
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut params.double_doors, "Double Door")
                    .changed()
                {
                    res.live_changed = true;
                    res.immediate_commit_label = Some("Toggle double doors");
                }
                if !params.double_doors
                    && ui
                        .checkbox(&mut params.hinge_right, "Hinge Right")
                        .changed()
                {
                    res.live_changed = true;
                    res.immediate_commit_label = Some("Flip door hinge");
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Handle").size(12.8));
                egui::ComboBox::from_id_salt("insp_door_handle")
                    .selected_text(params.door_handle_style.name())
                    .show_ui(ui, |ui| {
                        for &hs in DoorHandleStyle::ALL {
                            if ui
                                .selectable_label(params.door_handle_style == hs, hs.name())
                                .clicked()
                            {
                                params.door_handle_style = hs;
                                res.live_changed = true;
                                res.immediate_commit_label = Some("Change door handle");
                            }
                        }
                    });
            });
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let color_ref = if params.style.is_door() {
                door_color
            } else {
                ext_color
            };
            ui.label(RichText::new("Preview").small().color(pal.text_muted));
            for fc in FilamentColor::ALL {
                let rgb = fc.rgb_exterior();
                let c32 = Color32::from_rgb(
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                );
                let active = *color_ref == fc;
                let (r, resp) =
                    ui.allocate_exact_size(egui::Vec2::splat(14.0), egui::Sense::click());
                ui.painter().circle_filled(r.center(), 6.0, c32);
                if active {
                    ui.painter()
                        .circle_stroke(r.center(), 7.5, Stroke::new(1.5, pal.gold));
                }
                if resp.on_hover_text(fc.name()).clicked() {
                    *color_ref = fc;
                }
            }
        });

        ui.add_space(4.0);
        if ui
            .add(
                egui::Button::new("Save As Template...")
                    .min_size(egui::Vec2::new(ui.available_width(), 26.0)),
            )
            .clicked()
        {
            res.action = Some(InspectorAction::SaveAsTemplate);
        }
    });

    // 2) Collapsed Advanced Drawers
    egui::CollapsingHeader::new("Advanced: Trim, Shutters & Flush Edges")
        .default_open(false)
        .show(ui, |ui| {
            egui::ComboBox::from_id_salt("insp_architrave")
                .selected_text(params.architrave_style.name())
                .show_ui(ui, |ui| {
                    for st in ArchitraveStyle::ALL {
                        if ui
                            .selectable_label(params.architrave_style == st, st.name())
                            .clicked()
                        {
                            params.architrave_style = st;
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change architrave style");
                        }
                    }
                });
            egui::ComboBox::from_id_salt("insp_groove")
                .selected_text(params.groove_style.name())
                .show_ui(ui, |ui| {
                    for gr in GrooveStyle::ALL {
                        if ui
                            .selectable_label(params.groove_style == gr, gr.name())
                            .clicked()
                        {
                            params.groove_style = gr;
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change architrave groove");
                        }
                    }
                });
            egui::ComboBox::from_id_salt("insp_shutter")
                .selected_text(params.shutter_style.name())
                .show_ui(ui, |ui| {
                    for sh in ShutterStyle::ALL {
                        if ui
                            .selectable_label(params.shutter_style == sh, sh.name())
                            .clicked()
                        {
                            params.shutter_style = sh;
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change shutters");
                        }
                    }
                });
            egui::ComboBox::from_id_salt("insp_header")
                .selected_text(params.header_style.name())
                .show_ui(ui, |ui| {
                    for hd in HeaderStyle::ALL {
                        if ui
                            .selectable_label(params.header_style == hd, hd.name())
                            .clicked()
                        {
                            params.header_style = hd;
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change classical header");
                        }
                    }
                });
            if params.sill_enabled
                && params.style.supports_sill()
                && ui
                    .checkbox(
                        &mut params.flower_box_enabled,
                        "Attached Planter / Flower Box",
                    )
                    .changed()
            {
                res.live_changed = true;
                res.immediate_commit_label = Some("Toggle flower box");
            }
            if slider_unit_mm(
                ui,
                pal,
                "Trim Depth",
                &mut params.trim_thickness,
                0.8..=6.0,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change trim thickness");
            }
            if slider_unit_mm(
                ui,
                pal,
                "Inner Frame",
                &mut params.frame_width,
                2.0..=10.0,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change inner frame width");
            }
            if slider_unit_mm(
                ui,
                pal,
                "Glazing Bar W",
                &mut params.bar_width,
                0.8..=4.0,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change glazing bar width");
            }
            if ui
                .checkbox(
                    &mut params.match_interior_trim,
                    "Match Interior Trim to Exterior",
                )
                .changed()
            {
                res.live_changed = true;
                res.immediate_commit_label = Some("Toggle interior trim lock");
            }
            if !params.match_interior_trim {
                if slider_unit_mm(
                    ui,
                    pal,
                    "Int Trim W",
                    &mut params.interior_trim_width,
                    1.5..=14.0,
                    unit,
                ) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change interior trim width");
                }
                if slider_unit_mm(
                    ui,
                    pal,
                    "Int Trim Depth",
                    &mut params.interior_trim_thickness,
                    0.8..=5.0,
                    unit,
                ) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change interior trim depth");
                }
            }

            ui.separator();
            ui.label(
                RichText::new("Flush Wall Feature Trimming")
                    .small()
                    .color(pal.text_muted),
            );
            egui::ComboBox::from_id_salt("insp_flush_top")
                .selected_text(params.flush_top_mode.name())
                .show_ui(ui, |ui| {
                    for &fm in FlushTopMode::ALL {
                        if ui
                            .selectable_label(params.flush_top_mode == fm, fm.name())
                            .clicked()
                        {
                            params.flush_top_mode = fm;
                            res.live_changed = true;
                            res.immediate_commit_label = Some("Change flush top mode");
                        }
                    }
                });
            ui.horizontal(|ui| {
                if ui.checkbox(&mut params.flush_bottom_sill, "Sill").changed()
                    || ui.checkbox(&mut params.flush_left_jamb, "Left").changed()
                    || ui.checkbox(&mut params.flush_right_jamb, "Right").changed()
                {
                    res.live_changed = true;
                    res.immediate_commit_label = Some("Toggle flush edge");
                }
            });
        });

    egui::CollapsingHeader::new("Advanced: Doors, Sashes & Hardware")
        .default_open(false)
        .show(ui, |ui| {
            if params.style.supports_opening_sash() {
                egui::ComboBox::from_id_salt("insp_win_open")
                    .selected_text(params.window_opening_mode.name())
                    .show_ui(ui, |ui| {
                        for &wm in WindowOpeningMode::ALL {
                            if ui
                                .selectable_label(params.window_opening_mode == wm, wm.name())
                                .clicked()
                            {
                                params.window_opening_mode = wm;
                                res.live_changed = true;
                                res.immediate_commit_label = Some("Change window sash hinge");
                            }
                        }
                    });
            }
            if params.style.is_door() {
                if slider_unit_mm(
                    ui,
                    pal,
                    "Door Thickness",
                    &mut params.door_thickness,
                    2.5..=7.0,
                    unit,
                ) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change door thickness");
                }
                if slider_ratio_pct(
                    ui,
                    pal,
                    "Lower Panel Ratio",
                    &mut params.lower_panel_ratio,
                    0.0..=0.60,
                ) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change door kick panel");
                }
                if ui
                    .checkbox(&mut params.letterbox_enabled, "Letterbox Slot")
                    .changed()
                {
                    res.live_changed = true;
                    res.immediate_commit_label = Some("Toggle letterbox");
                }
                if ui
                    .checkbox(&mut params.cat_flap_enabled, "Pet / Cat Flap")
                    .changed()
                {
                    res.live_changed = true;
                    res.immediate_commit_label = Some("Toggle cat flap");
                }
            }
        });

    egui::CollapsingHeader::new("Advanced: 3D Print Tolerances")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for pr in PrinterPreset::ALL {
                    if ui.small_button(pr.label()).clicked() {
                        pr.apply(params);
                        res.live_changed = true;
                        res.immediate_commit_label = Some("Apply printer tolerance preset");
                    }
                }
            });
            if slider_unit_mm(
                ui,
                pal,
                "Acetate Thick",
                &mut params.acetate_thickness,
                0.2..=1.2,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change acetate thickness");
            }
            if slider_unit_mm(
                ui,
                pal,
                "Wall Fit Clear",
                &mut params.wall_fit_clearance,
                0.05..=0.8,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change wall fit clearance");
            }
            if slider_unit_mm(
                ui,
                pal,
                "Index Clear",
                &mut params.index_clearance,
                0.05..=0.5,
                unit,
            ) {
                res.live_changed = true;
                res.drag_commit_label = Some("Change index clearance");
            }
        });

    // 3) Compact Printability & Glazing Spec Card
    let specs = params.compute_specs();
    let built = build_window(params);
    let val = validate_generated_piece(&built);
    egui::Frame::NONE
        .fill(pal.surface_card)
        .stroke(Stroke::new(
            1.0,
            if val.is_valid { pal.border } else { pal.danger },
        ))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.label(
                RichText::new(if val.is_valid {
                    "Manifold & Print Ready (0 overhangs)"
                } else {
                    "Print Warning Detected"
                })
                .small()
                .strong()
                .color(if val.is_valid {
                    pal.success
                } else {
                    pal.danger
                }),
            );
            ui.label(
                RichText::new(format!(
                    "Sleeve: {} | Acetate: {}",
                    unit.format_dims(specs.sleeve_outer_width, specs.sleeve_outer_height),
                    unit.format_dims(specs.acetate_cut_width, specs.acetate_cut_height),
                ))
                .small()
                .color(pal.text_secondary),
            );
        });
}

#[allow(clippy::too_many_arguments)]
pub fn draw_wall_inspector(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    wall: &mut ProjectWall,
    components: &mut [ComponentDef],
    selected_ids: &[String],
    active_component_id: Option<&str>,
    unit: DisplayUnit,
    ext_color: &mut FilamentColor,
    door_color: &mut FilamentColor,
    state: &mut InspectorState,
) -> InspectorResult {
    let mut res = InspectorResult::default();

    egui::ScrollArea::vertical()
        .id_salt("inspector_scroll")
        .show(ui, |ui| {
            // Case 1: Editing a shared ComponentDef selected directly in Explorer
            if let Some(cid) = active_component_id {
                if let Some(comp) = components.iter_mut().find(|c| c.id == cid) {
                    egui::Frame::NONE
                        .fill(pal.accent.linear_multiply(0.14))
                        .stroke(Stroke::new(1.0, pal.accent))
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Editing Shared Component Definition")
                                    .strong()
                                    .color(pal.accent),
                            );
                            ui.label(
                                RichText::new(
                                    "Changes here update every linked instance across all walls.",
                                )
                                .small()
                                .color(pal.text_secondary),
                            );
                        });
                    ui.add_space(6.0);
                    draw_piece_inspector_body(
                        ui,
                        pal,
                        &mut comp.name,
                        &mut comp.params,
                        unit,
                        ext_color,
                        door_color,
                        &mut res,
                    );
                }
                return;
            }

            // Case 2: Multiple openings selected
            if selected_ids.len() > 1 {
                section_card(
                    ui,
                    pal,
                    Some(&format!("{} Openings Selected", selected_ids.len())),
                    |ui| {
                        ui.label(
                            RichText::new("Use the alignment tools to space or mirror the group.")
                                .small()
                                .color(pal.text_secondary),
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Center H").clicked() {
                                res.action = Some(InspectorAction::AlignSelectedCenterH);
                            }
                            if ui.button("Distribute").clicked() {
                                res.action = Some(InspectorAction::AlignSelectedDistributeH);
                            }
                            if ui.button("Mirror").clicked() {
                                res.action = Some(InspectorAction::AlignSelectedMirrorH);
                            }
                        });
                        if ui
                            .add(egui::Button::new(
                                RichText::new("Delete Selected").color(pal.danger),
                            ))
                            .clicked()
                        {
                            res.action = Some(InspectorAction::DeleteSelectedOpenings);
                        }
                    },
                );
                return;
            }

            // Case 3: Single opening selected on wall
            if selected_ids.len() == 1 {
                let op_id = selected_ids[0].clone();
                if let Some(op_idx) = wall.openings.iter().position(|o| o.id == op_id) {
                    let comp_id_opt = wall.openings[op_idx]
                        .component_id()
                        .map(ToString::to_string);

                    // Wall Placement Card (X/Y & Locks)
                    section_card(ui, pal, Some("Wall Placement"), |ui| {
                        let op = &mut wall.openings[op_idx];
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut op.name)
                                    .desired_width(ui.available_width()),
                            )
                            .changed()
                        {
                            res.live_changed = true;
                            res.drag_commit_label = Some("Rename opening");
                        }
                        if drag_unit_mm(
                            ui,
                            pal,
                            "X from Left",
                            &mut op.x_from_left,
                            0.0..=wall.width,
                            unit,
                        ) {
                            res.live_changed = true;
                            res.drag_commit_label = Some("Move opening X");
                        }
                        if drag_unit_mm(
                            ui,
                            pal,
                            "Y from Floor",
                            &mut op.y_from_bottom,
                            0.0..=wall.height,
                            unit,
                        ) {
                            res.live_changed = true;
                            res.drag_commit_label = Some("Move opening Y");
                        }
                        op.clamp_to_wall(wall.width, wall.height, components);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Locks:").small().color(pal.text_muted));
                            ui.checkbox(&mut wall.openings[op_idx].lock_position, "Pos");
                            ui.checkbox(&mut wall.openings[op_idx].lock_width, "W");
                            ui.checkbox(&mut wall.openings[op_idx].lock_height, "H");
                        });
                    });

                    // Component Linking & Parameter Card
                    if let Some(cid) = comp_id_opt {
                        egui::Frame::NONE
                            .fill(pal.accent.linear_multiply(0.14))
                            .stroke(Stroke::new(1.0, pal.accent))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                let comp_name = components
                                    .iter()
                                    .find(|c| c.id == cid)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "Component".into());
                                ui.label(
                                    RichText::new(format!("Linked Instance -> {comp_name}"))
                                        .strong()
                                        .color(pal.accent),
                                );
                                ui.horizontal(|ui| {
                                    ui.selectable_value(
                                        &mut state.edit_shared_component,
                                        true,
                                        "Shared Component",
                                    );
                                    ui.selectable_value(
                                        &mut state.edit_shared_component,
                                        false,
                                        "Instance Overrides",
                                    );
                                });
                                if ui.small_button("Make Unique (Unlink)").clicked() {
                                    res.action =
                                        Some(InspectorAction::MakeOpeningUnique(op_id.clone()));
                                }
                            });
                        ui.add_space(6.0);

                        if state.edit_shared_component {
                            if let Some(comp) = components.iter_mut().find(|c| c.id == cid) {
                                draw_piece_inspector_body(
                                    ui,
                                    pal,
                                    &mut comp.name,
                                    &mut comp.params,
                                    unit,
                                    ext_color,
                                    door_color,
                                    &mut res,
                                );
                            }
                        } else if let OpeningSource::Component {
                            ref mut overrides, ..
                        } = wall.openings[op_idx].source
                        {
                            draw_instance_overrides_card(ui, pal, overrides, unit, &mut res);
                        }
                    } else {
                        section_card(ui, pal, Some("Reuse"), |ui| {
                            if ui
                                .button("Promote to Shared Component...")
                                .on_hover_text("Link instances across this or multiple walls")
                                .clicked()
                            {
                                res.action =
                                    Some(InspectorAction::PromoteOpeningToComponent(op_id.clone()));
                            }
                        });
                        let mut tmp_name = wall.openings[op_idx].name.clone();
                        if let OpeningSource::Unique { ref mut params } =
                            wall.openings[op_idx].source
                        {
                            draw_piece_inspector_body(
                                ui,
                                pal,
                                &mut tmp_name,
                                params,
                                unit,
                                ext_color,
                                door_color,
                                &mut res,
                            );
                        }
                        wall.openings[op_idx].name = tmp_name;
                    }
                    return;
                }
            }

            // Case 4: Wall Properties (when no opening selected)
            section_card(ui, pal, Some("Wall Properties"), |ui| {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut wall.name)
                            .desired_width(ui.available_width()),
                    )
                    .changed()
                {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Rename wall");
                }
                if slider_unit_mm(ui, pal, "Wall Width", &mut wall.width, 80.0..=1400.0, unit) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Resize wall width");
                }
                if slider_unit_mm(
                    ui,
                    pal,
                    "Wall Height",
                    &mut wall.height,
                    80.0..=1000.0,
                    unit,
                ) {
                    res.live_changed = true;
                    res.drag_commit_label = Some("Resize wall height");
                }
                if slider_unit_mm(
                    ui,
                    pal,
                    "Wall Thickness",
                    &mut wall.wall_thickness,
                    3.0..=24.0,
                    unit,
                ) {
                    if wall.sync_common_settings {
                        wall.apply_common_settings_to_openings();
                    }
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change wall thickness");
                }
                if slider_unit_mm(
                    ui,
                    pal,
                    "Acetate Thick",
                    &mut wall.acetate_thickness,
                    0.2..=1.2,
                    unit,
                ) {
                    if wall.sync_common_settings {
                        wall.apply_common_settings_to_openings();
                    }
                    res.live_changed = true;
                    res.drag_commit_label = Some("Change wall acetate thickness");
                }
                if ui
                    .checkbox(
                        &mut wall.sync_common_settings,
                        "Sync thickness & acetate to wall openings",
                    )
                    .changed()
                {
                    res.action = Some(InspectorAction::SyncCommonWallSettings);
                    res.immediate_commit_label = Some("Sync wall settings");
                }
                if ui.button("Auto-Dimension All Openings").clicked() {
                    res.action = Some(InspectorAction::AutoDimensionWall);
                }
            });

            section_card(ui, pal, Some("Reference Photo"), |ui| {
                if wall.photo.has_photo() {
                    ui.label(
                        RichText::new(format!(
                            "Calibrated Size: {}",
                            unit.format_dims(wall.photo.width_mm, wall.photo.height_mm)
                        ))
                        .small()
                        .color(pal.text_secondary),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Edit on Canvas").clicked() {
                            res.action = Some(InspectorAction::EditPhotoOnCanvas);
                        }
                        if ui.button("Open Wizard...").clicked() {
                            res.action = Some(InspectorAction::OpenPhotoWizard);
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("No reference photo attached to this wall yet.")
                            .small()
                            .color(pal.text_muted),
                    );
                }
            });

            egui::CollapsingHeader::new("3D House Assembly Placement")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Floor Level");
                        if ui
                            .add(
                                egui::DragValue::new(&mut wall.placement_3d.floor_level).speed(0.1),
                            )
                            .changed()
                        {
                            res.live_changed = true;
                            res.drag_commit_label = Some("Change wall floor level");
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Yaw Rotation");
                        if ui
                            .add(
                                egui::Slider::new(&mut wall.placement_3d.yaw_deg, -180.0..=180.0)
                                    .suffix(" deg"),
                            )
                            .changed()
                        {
                            res.live_changed = true;
                            res.drag_commit_label = Some("Rotate 3D wall placement");
                        }
                    });
                });
        });

    res
}

pub fn draw_standalone_piece_inspector(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    piece: &mut StandalonePiece,
    unit: DisplayUnit,
    ext_color: &mut FilamentColor,
    door_color: &mut FilamentColor,
) -> InspectorResult {
    let mut res = InspectorResult::default();
    egui::ScrollArea::vertical()
        .id_salt("standalone_insp_scroll")
        .show(ui, |ui| {
            draw_piece_inspector_body(
                ui,
                pal,
                &mut piece.name,
                &mut piece.params,
                unit,
                ext_color,
                door_color,
                &mut res,
            );
        });
    res
}

fn draw_instance_overrides_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    overrides: &mut InstanceOverrides,
    unit: DisplayUnit,
    res: &mut InspectorResult,
) {
    section_card(ui, pal, Some("Per-Instance Exceptions"), |ui| {
        let mut wt_ovr = overrides.wall_thickness.is_some();
        if ui
            .checkbox(&mut wt_ovr, "Override Wall Thickness")
            .changed()
        {
            overrides.wall_thickness = if wt_ovr { Some(9.0) } else { None };
            res.live_changed = true;
            res.immediate_commit_label = Some("Toggle wall thickness override");
        }
        if let Some(ref mut wt) = overrides.wall_thickness
            && slider_unit_mm(ui, pal, "Instance Wall", wt, 3.0..=24.0, unit)
        {
            res.live_changed = true;
            res.drag_commit_label = Some("Edit instance wall thickness");
        }

        let mut hinge_ovr = overrides.hinge_right.is_some();
        if ui
            .checkbox(&mut hinge_ovr, "Override Door Hinge Side")
            .changed()
        {
            overrides.hinge_right = if hinge_ovr { Some(true) } else { None };
            res.live_changed = true;
            res.immediate_commit_label = Some("Toggle hinge override");
        }
        if let Some(ref mut hr) = overrides.hinge_right
            && ui.checkbox(hr, "Hinge Right").changed()
        {
            res.live_changed = true;
            res.immediate_commit_label = Some("Flip instance hinge");
        }

        ui.separator();
        ui.label(
            RichText::new("Flush Edge Overrides")
                .small()
                .color(pal.text_muted),
        );
        let mut fs = overrides.flush_bottom_sill.unwrap_or(false);
        let mut fl = overrides.flush_left_jamb.unwrap_or(false);
        let mut fr = overrides.flush_right_jamb.unwrap_or(false);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut fs, "Sill").changed() {
                overrides.flush_bottom_sill = if fs { Some(true) } else { None };
                res.live_changed = true;
                res.immediate_commit_label = Some("Toggle flush sill override");
            }
            if ui.checkbox(&mut fl, "Left").changed() {
                overrides.flush_left_jamb = if fl { Some(true) } else { None };
                res.live_changed = true;
                res.immediate_commit_label = Some("Toggle flush left override");
            }
            if ui.checkbox(&mut fr, "Right").changed() {
                overrides.flush_right_jamb = if fr { Some(true) } else { None };
                res.live_changed = true;
                res.immediate_commit_label = Some("Toggle flush right override");
            }
        });

        if !overrides.is_empty() && ui.small_button("Reset All Overrides").clicked() {
            *overrides = InstanceOverrides::default();
            res.live_changed = true;
            res.immediate_commit_label = Some("Reset instance overrides");
        }
    });
}
