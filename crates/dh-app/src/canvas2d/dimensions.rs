use crate::canvas2d::photo_layer::BlueprintXform;
use crate::theme::StudioPalette;
use dh_core::{
    ComponentDef, CornerKind, DimAnchor, DimOrientation, DisplayUnit, OpeningEdgeKind, ProjectWall,
    WallDimension, WallEdgeKind,
};
use eframe::egui::{
    self, Align2, CornerRadius, FontId, Id, Margin, Order, Pos2, Rect, RichText, Stroke,
    StrokeKind, Vec2,
};

#[derive(Debug, Clone, Default)]
pub struct DimensionToolState {
    pub active_draw_mode: bool,
    draft_start: Option<([f64; 2], DimAnchor)>,
    dragging_dim_offset: Option<String>,
    dragging_endpoint: Option<(String, usize)>,
    pub selected_dim_id: Option<String>,
    pub editing_dim_id: Option<String>,
    pub edit_value_mm: f64,
    edit_focus_requested: bool,
}

impl DimensionToolState {
    pub fn cancel_draft(&mut self) {
        self.draft_start = None;
        self.editing_dim_id = None;
    }

    pub fn is_busy(&self) -> bool {
        self.draft_start.is_some()
            || self.dragging_dim_offset.is_some()
            || self.dragging_endpoint.is_some()
            || self.editing_dim_id.is_some()
    }

    pub fn begin_edit_dim(
        &mut self,
        wall: &ProjectWall,
        components: &[ComponentDef],
        dim_id: &str,
    ) {
        if let Some(dim) = wall.dimensions.iter().find(|d| d.id == dim_id) {
            let (r0, r1) = wall.resolve_dimension_endpoints(dim, components);
            let len = ((r1[0] - r0[0]).powi(2) + (r1[1] - r0[1]).powi(2)).sqrt();
            self.selected_dim_id = Some(dim_id.to_string());
            self.editing_dim_id = Some(dim_id.to_string());
            self.edit_value_mm = len.max(1.0);
            self.edit_focus_requested = true;
        }
    }
}

#[derive(Default)]
pub struct DimensionsOutput {
    pub changed: bool,
    pub commit_label: Option<&'static str>,
    pub consumed_pointer: bool,
    pub drive_request: Option<(String, f64)>,
}

#[derive(Debug, Clone)]
struct SnapTarget {
    pt_mm: [f64; 2],
    anchor: DimAnchor,
    edge_line_mm: Option<([f64; 2], [f64; 2])>,
    snapped: bool,
}

fn is_vert_edge_anchor(a: &DimAnchor) -> bool {
    matches!(
        a,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Left | WallEdgeKind::Right | WallEdgeKind::CenterX
        } | DimAnchor::OpeningEdge {
            edge: OpeningEdgeKind::Left | OpeningEdgeKind::Right | OpeningEdgeKind::CenterX,
            ..
        }
    )
}

fn is_horiz_edge_anchor(a: &DimAnchor) -> bool {
    matches!(
        a,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Bottom | WallEdgeKind::Top | WallEdgeKind::CenterY
        } | DimAnchor::OpeningEdge {
            edge: OpeningEdgeKind::Bottom | OpeningEdgeKind::Top | OpeningEdgeKind::CenterY,
            ..
        }
    )
}

fn infer_orientation(
    a: &DimAnchor,
    b: &DimAnchor,
    p0: [f64; 2],
    p1: [f64; 2],
    shift_held: bool,
) -> DimOrientation {
    if is_vert_edge_anchor(a) && is_vert_edge_anchor(b) {
        return DimOrientation::Horizontal;
    }
    if is_horiz_edge_anchor(a) && is_horiz_edge_anchor(b) {
        return DimOrientation::Vertical;
    }
    if shift_held {
        if (p1[0] - p0[0]).abs() >= (p1[1] - p0[1]).abs() {
            DimOrientation::Horizontal
        } else {
            DimOrientation::Vertical
        }
    } else {
        DimOrientation::Auto
    }
}

fn snap_cad_target(
    raw: [f64; 2],
    wall: &ProjectWall,
    components: &[ComponentDef],
    threshold_mm: f64,
) -> SnapTarget {
    // 1. High-priority corners & centers
    let mut best_pt: Option<([f64; 2], DimAnchor, f64)> = None;
    let mut consider_pt = |pt: [f64; 2], anchor: DimAnchor| {
        let d = ((pt[0] - raw[0]).powi(2) + (pt[1] - raw[1]).powi(2)).sqrt();
        if d <= threshold_mm && best_pt.as_ref().is_none_or(|(_, _, bd)| d < *bd) {
            best_pt = Some((pt, anchor, d));
        }
    };

    consider_pt(
        [0.0, 0.0],
        DimAnchor::WallCorner {
            corner: CornerKind::BottomLeft,
        },
    );
    consider_pt(
        [wall.width, 0.0],
        DimAnchor::WallCorner {
            corner: CornerKind::BottomRight,
        },
    );
    consider_pt(
        [wall.width, wall.height],
        DimAnchor::WallCorner {
            corner: CornerKind::TopRight,
        },
    );
    consider_pt(
        [0.0, wall.height],
        DimAnchor::WallCorner {
            corner: CornerKind::TopLeft,
        },
    );
    consider_pt(
        [wall.width * 0.5, 0.0],
        DimAnchor::WallEdge {
            edge: WallEdgeKind::CenterX,
        },
    );
    consider_pt(
        [wall.width * 0.5, wall.height],
        DimAnchor::WallEdge {
            edge: WallEdgeKind::CenterX,
        },
    );
    consider_pt(
        [0.0, wall.height * 0.5],
        DimAnchor::WallEdge {
            edge: WallEdgeKind::CenterY,
        },
    );
    consider_pt(
        [wall.width, wall.height * 0.5],
        DimAnchor::WallEdge {
            edge: WallEdgeKind::CenterY,
        },
    );

    for op in &wall.openings {
        let w = op.width(components);
        let h = op.height(components);
        let l = op.x_from_left;
        let r = l + w;
        let b = op.y_from_bottom;
        let t = b + h;
        let oid = &op.id;
        consider_pt(
            [l, b],
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::BottomLeft,
            },
        );
        consider_pt(
            [r, b],
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::BottomRight,
            },
        );
        consider_pt(
            [r, t],
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::TopRight,
            },
        );
        consider_pt(
            [l, t],
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::TopLeft,
            },
        );
        consider_pt(
            [l + w * 0.5, b + h * 0.5],
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::CenterX,
            },
        );
    }

    if let Some((pt_mm, anchor, _)) = best_pt {
        return SnapTarget {
            pt_mm,
            anchor,
            edge_line_mm: None,
            snapped: true,
        };
    }

    // 2. Axis-aligned edges (wall borders & opening jamb/head/sill edges)
    let mut best_v: Option<(SnapTarget, f64)> = None;
    let mut best_h: Option<(SnapTarget, f64)> = None;
    let mut consider_v_edge = |x: f64, y0: f64, y1: f64, anchor: DimAnchor| {
        if raw[1] >= y0 - threshold_mm && raw[1] <= y1 + threshold_mm {
            let d = (raw[0] - x).abs();
            if d <= threshold_mm && best_v.as_ref().is_none_or(|(_, bd)| d < *bd) {
                best_v = Some((
                    SnapTarget {
                        pt_mm: [x, raw[1].clamp(y0, y1)],
                        anchor,
                        edge_line_mm: Some(([x, y0], [x, y1])),
                        snapped: true,
                    },
                    d,
                ));
            }
        }
    };
    let mut consider_h_edge = |y: f64, x0: f64, x1: f64, anchor: DimAnchor| {
        if raw[0] >= x0 - threshold_mm && raw[0] <= x1 + threshold_mm {
            let d = (raw[1] - y).abs();
            if d <= threshold_mm && best_h.as_ref().is_none_or(|(_, bd)| d < *bd) {
                best_h = Some((
                    SnapTarget {
                        pt_mm: [raw[0].clamp(x0, x1), y],
                        anchor,
                        edge_line_mm: Some(([x0, y], [x1, y])),
                        snapped: true,
                    },
                    d,
                ));
            }
        }
    };

    consider_v_edge(
        0.0,
        0.0,
        wall.height,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Left,
        },
    );
    consider_v_edge(
        wall.width,
        0.0,
        wall.height,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Right,
        },
    );
    consider_h_edge(
        0.0,
        0.0,
        wall.width,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Bottom,
        },
    );
    consider_h_edge(
        wall.height,
        0.0,
        wall.width,
        DimAnchor::WallEdge {
            edge: WallEdgeKind::Top,
        },
    );

    for op in &wall.openings {
        let w = op.width(components);
        let h = op.height(components);
        let l = op.x_from_left;
        let r = l + w;
        let b = op.y_from_bottom;
        let t = b + h;
        let oid = &op.id;
        consider_v_edge(
            l,
            b,
            t,
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::Left,
            },
        );
        consider_v_edge(
            r,
            b,
            t,
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::Right,
            },
        );
        consider_h_edge(
            b,
            l,
            r,
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::Bottom,
            },
        );
        consider_h_edge(
            t,
            l,
            r,
            DimAnchor::OpeningEdge {
                opening_id: oid.clone(),
                edge: OpeningEdgeKind::Top,
            },
        );
    }

    match (best_v, best_h) {
        (Some((tv, dv)), Some((th, dh))) => return if dv <= dh { tv } else { th },
        (Some((tv, _)), None) => return tv,
        (None, Some((th, _))) => return th,
        (None, None) => {}
    }

    SnapTarget {
        pt_mm: raw,
        anchor: DimAnchor::Point {
            x: raw[0],
            y: raw[1],
        },
        edge_line_mm: None,
        snapped: false,
    }
}

fn dim_screen_geometry(
    xform: BlueprintXform,
    r0: [f64; 2],
    r1: [f64; 2],
    offset_mm: f64,
    text: &str,
) -> (Pos2, Pos2, Pos2, Pos2, Rect) {
    let p0 = xform.mm_to_screen(r0);
    let p1 = xform.mm_to_screen(r1);
    let dx = r1[0] - r0[0];
    let dy = r1[1] - r0[1];
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let nx = -dy / len;
    let ny = dx / len;
    let d0 = xform.mm_to_screen([r0[0] + nx * offset_mm, r0[1] + ny * offset_mm]);
    let d1 = xform.mm_to_screen([r1[0] + nx * offset_mm, r1[1] + ny * offset_mm]);
    let mid = d0.lerp(d1, 0.5);
    let pill = Rect::from_center_size(
        mid,
        Vec2::new((text.len() as f32 * 6.8 + 16.0).max(48.0), 19.0),
    );
    (p0, p1, d0, d1, pill)
}

fn format_resolved_dim_text(
    dim: &WallDimension,
    r0: [f64; 2],
    r1: [f64; 2],
    unit: DisplayUnit,
) -> String {
    let live_len = ((r1[0] - r0[0]).powi(2) + (r1[1] - r0[1]).powi(2)).sqrt();
    let val = unit.format_value(live_len);
    let trimmed = dim.label.trim();
    if trimmed.is_empty() {
        val
    } else {
        format!("{trimmed}: {val}")
    }
}

/// Hit-tests all rendered dimension pills at `pos` for right-click menus or selection.
pub fn hit_test_dimension_pill(
    xform: BlueprintXform,
    wall: &ProjectWall,
    components: &[ComponentDef],
    unit: DisplayUnit,
    pos: Pos2,
) -> Option<String> {
    for dim in wall.dimensions.iter().rev() {
        let (r0, r1) = wall.resolve_dimension_endpoints(dim, components);
        let text = format_resolved_dim_text(dim, r0, r1, unit);
        let (_, _, _, _, pill) = dim_screen_geometry(xform, r0, r1, dim.offset_mm, &text);
        if pill.expand(5.0).contains(pos) {
            return Some(dim.id.clone());
        }
    }
    None
}

/// Renders live-synced associative CAD dimensions, edge/corner snapping, double-click/inline numeric
/// driving length edits, and subtle live clearances for a single selected opening.
#[allow(clippy::too_many_arguments)]
pub fn interact_and_draw_dimensions(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall: &mut ProjectWall,
    components: &[ComponentDef],
    selected_ids: &[String],
    unit: DisplayUnit,
    state: &mut DimensionToolState,
    allow_edit: bool,
    canvas_hovered: bool,
) -> DimensionsOutput {
    let mut out = DimensionsOutput::default();
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_pressed = allow_edit && canvas_hovered && ui.input(|i| i.pointer.primary_pressed());
    let primary_down = allow_edit && ui.input(|i| i.pointer.primary_down());
    let primary_released = allow_edit && ui.input(|i| i.pointer.primary_released());
    let double_clicked = allow_edit
        && canvas_hovered
        && ui.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        });
    let shift_held = ui.input(|i| i.modifiers.shift);

    let snap_thresh_mm = (10.0 / xform.px_per_mm.max(0.2)) as f64;

    if primary_released {
        if state.dragging_dim_offset.take().is_some() {
            out.commit_label = Some("Move dimension line");
            out.consumed_pointer = true;
        }
        if state.dragging_endpoint.take().is_some() {
            wall.sync_anchored_dimensions(components);
            out.commit_label = Some("Re-anchor dimension endpoint");
            out.consumed_pointer = true;
        }
        if let Some((start_pt, start_anchor)) = state.draft_start.take()
            && let Some(pos) = pointer_pos
        {
            let mut end_target =
                snap_cad_target(xform.screen_to_mm(pos), wall, components, snap_thresh_mm);
            let orient = infer_orientation(
                &start_anchor,
                &end_target.anchor,
                start_pt,
                end_target.pt_mm,
                shift_held,
            );
            match orient {
                DimOrientation::Horizontal => end_target.pt_mm[1] = start_pt[1],
                DimOrientation::Vertical => end_target.pt_mm[0] = start_pt[0],
                DimOrientation::Auto | DimOrientation::Aligned => {}
            }
            let len = ((end_target.pt_mm[0] - start_pt[0]).powi(2)
                + (end_target.pt_mm[1] - start_pt[1]).powi(2))
            .sqrt();
            if len >= 2.0 {
                let mut dim = WallDimension::new_anchored(
                    start_anchor,
                    end_target.anchor,
                    orient,
                    14.0,
                    start_pt,
                    end_target.pt_mm,
                );
                let (r0, r1) = wall.resolve_dimension_endpoints(&dim, components);
                dim.x0 = r0[0];
                dim.y0 = r0[1];
                dim.x1 = r1[0];
                dim.y1 = r1[1];
                state.selected_dim_id = Some(dim.id.clone());
                wall.dimensions.push(dim);
                out.changed = true;
                out.commit_label = Some("Add linked CAD dimension");
            }
            out.consumed_pointer = true;
        }
    }

    // Active drag adjustments.
    if primary_down && let Some(pos) = pointer_pos {
        if let Some(ref dim_id) = state.dragging_dim_offset {
            if let Some(dim_idx) = wall.dimensions.iter().position(|d| &d.id == dim_id) {
                let (r0, r1) =
                    wall.resolve_dimension_endpoints(&wall.dimensions[dim_idx], components);
                let cur = xform.screen_to_mm(pos);
                let dx = r1[0] - r0[0];
                let dy = r1[1] - r0[1];
                let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                let nx = -dy / len;
                let ny = dx / len;
                let signed = (cur[0] - r0[0]) * nx + (cur[1] - r0[1]) * ny;
                wall.dimensions[dim_idx].offset_mm = signed.clamp(-140.0, 140.0);
                out.changed = true;
                out.consumed_pointer = true;
            }
        } else if let Some((ref dim_id, ep)) = state.dragging_endpoint {
            let mut target =
                snap_cad_target(xform.screen_to_mm(pos), wall, components, snap_thresh_mm);
            if let Some(dim_idx) = wall.dimensions.iter().position(|d| &d.id == dim_id) {
                let (cur_r0, cur_r1) =
                    wall.resolve_dimension_endpoints(&wall.dimensions[dim_idx], components);
                if shift_held {
                    let anchor_pt = if ep == 0 { cur_r1 } else { cur_r0 };
                    if (target.pt_mm[0] - anchor_pt[0]).abs()
                        >= (target.pt_mm[1] - anchor_pt[1]).abs()
                    {
                        target.pt_mm[1] = anchor_pt[1];
                    } else {
                        target.pt_mm[0] = anchor_pt[0];
                    }
                }
                let dim = &mut wall.dimensions[dim_idx];
                if ep == 0 {
                    dim.x0 = target.pt_mm[0];
                    dim.y0 = target.pt_mm[1];
                    dim.anchor_a = Some(target.anchor);
                } else {
                    dim.x1 = target.pt_mm[0];
                    dim.y1 = target.pt_mm[1];
                    dim.anchor_b = Some(target.anchor);
                }
                dim.orientation = infer_orientation(
                    dim.anchor_a.as_ref().unwrap_or(&DimAnchor::Point {
                        x: dim.x0,
                        y: dim.y0,
                    }),
                    dim.anchor_b.as_ref().unwrap_or(&DimAnchor::Point {
                        x: dim.x1,
                        y: dim.y1,
                    }),
                    [dim.x0, dim.y0],
                    [dim.x1, dim.y1],
                    shift_held,
                );
                out.changed = true;
                out.consumed_pointer = true;
            }
        }
    }

    // Subtle live clearances when one opening is selected.
    if selected_ids.len() == 1 && !state.active_draw_mode {
        draw_selected_clearance_guides(
            painter,
            pal,
            xform,
            wall,
            components,
            &selected_ids[0],
            unit,
        );
    }

    // Draw all persistent dimensions using live resolved endpoints.
    let mut pending_double_click_dim: Option<String> = None;
    let mut edit_anchor_screen: Option<(Pos2, bool)> = None;

    for dim in &wall.dimensions {
        let (r0, r1) = wall.resolve_dimension_endpoints(dim, components);
        let text = format_resolved_dim_text(dim, r0, r1, unit);
        let (p0, p1, d0, d1, pill_rect) = dim_screen_geometry(xform, r0, r1, dim.offset_mm, &text);
        let mid = pill_rect.center();

        let selected = state.selected_dim_id.as_deref() == Some(dim.id.as_str());
        let linked = dim.is_anchored();
        let line_col = if selected {
            pal.gold
        } else if linked {
            pal.accent
        } else {
            pal.text_secondary
        };

        let ext_stroke = Stroke::new(1.0, line_col.linear_multiply(0.55));
        painter.line_segment([p0, d0], ext_stroke);
        painter.line_segment([p1, d1], ext_stroke);

        painter.line_segment([d0, d1], Stroke::new(1.5, line_col));
        let dir = (d1 - d0).normalized();
        let perp = Vec2::new(-dir.y, dir.x);
        painter.line_segment(
            [d0 - perp * 4.5, d0 + perp * 4.5],
            Stroke::new(1.5, line_col),
        );
        painter.line_segment(
            [d1 - perp * 4.5, d1 + perp * 4.5],
            Stroke::new(1.5, line_col),
        );

        painter.rect_filled(pill_rect, 4.0, pal.hud_glass);
        painter.rect_stroke(
            pill_rect,
            4.0,
            Stroke::new(if selected { 1.4 } else { 1.0 }, line_col),
            StrokeKind::Inside,
        );
        if linked {
            painter.circle_filled(Pos2::new(pill_rect.left() + 6.0, mid.y), 2.3, line_col);
        }
        painter.text(
            if linked {
                Pos2::new(mid.x + 3.0, mid.y)
            } else {
                mid
            },
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(11.5),
            if selected { pal.gold } else { pal.text_primary },
        );

        if selected {
            if linked {
                painter.circle_filled(p0, 4.2, pal.gold);
                painter.circle_filled(p1, 4.2, pal.gold);
            } else {
                painter.circle_stroke(p0, 4.2, Stroke::new(1.6, pal.gold));
                painter.circle_stroke(p1, 4.2, Stroke::new(1.6, pal.gold));
            }
        }

        if state.editing_dim_id.as_deref() == Some(dim.id.as_str()) {
            edit_anchor_screen = Some((pill_rect.center(), linked));
        }

        if let Some(pos) = pointer_pos {
            if double_clicked && pill_rect.expand(5.0).contains(pos) {
                pending_double_click_dim = Some(dim.id.clone());
                state.dragging_dim_offset = None;
                out.consumed_pointer = true;
            } else if primary_pressed && !out.consumed_pointer {
                if p0.distance(pos) <= 9.0 {
                    state.selected_dim_id = Some(dim.id.clone());
                    state.dragging_endpoint = Some((dim.id.clone(), 0));
                    out.consumed_pointer = true;
                } else if p1.distance(pos) <= 9.0 {
                    state.selected_dim_id = Some(dim.id.clone());
                    state.dragging_endpoint = Some((dim.id.clone(), 1));
                    out.consumed_pointer = true;
                } else if pill_rect.expand(5.0).contains(pos) {
                    state.selected_dim_id = Some(dim.id.clone());
                    state.dragging_dim_offset = Some(dim.id.clone());
                    out.consumed_pointer = true;
                }
            }
        }
    }

    if let Some(id) = pending_double_click_dim {
        state.begin_edit_dim(wall, components, &id);
    }

    // Active dimension drafting when Annotate / Draw Dimension mode is enabled.
    if allow_edit
        && state.active_draw_mode
        && !out.consumed_pointer
        && let Some(pos) = pointer_pos
        && xform.canvas_rect.contains(pos)
    {
        let target = snap_cad_target(xform.screen_to_mm(pos), wall, components, snap_thresh_mm);
        let snapped_s = xform.mm_to_screen(target.pt_mm);
        if target.snapped {
            if let Some((e0, e1)) = target.edge_line_mm {
                painter.line_segment(
                    [xform.mm_to_screen(e0), xform.mm_to_screen(e1)],
                    Stroke::new(2.6, pal.gold.gamma_multiply(0.85)),
                );
            }
            painter.circle_filled(snapped_s, 4.5, pal.gold);
            painter.circle_stroke(snapped_s, 8.0, Stroke::new(1.5, pal.gold));
        }

        if primary_pressed && state.draft_start.is_none() {
            state.draft_start = Some((target.pt_mm, target.anchor.clone()));
            out.consumed_pointer = true;
        }

        if let Some((start_pt, ref start_anchor)) = state.draft_start {
            let mut end_pt = target.pt_mm;
            let orient =
                infer_orientation(start_anchor, &target.anchor, start_pt, end_pt, shift_held);
            match orient {
                DimOrientation::Horizontal => end_pt[1] = start_pt[1],
                DimOrientation::Vertical => end_pt[0] = start_pt[0],
                DimOrientation::Auto | DimOrientation::Aligned => {}
            }
            let s0 = xform.mm_to_screen(start_pt);
            let s1 = xform.mm_to_screen(end_pt);
            painter.line_segment([s0, s1], Stroke::new(2.0, pal.gold));
            let len =
                ((end_pt[0] - start_pt[0]).powi(2) + (end_pt[1] - start_pt[1]).powi(2)).sqrt();
            painter.text(
                s0.lerp(s1, 0.5) + Vec2::new(0.0, -12.0),
                Align2::CENTER_BOTTOM,
                unit.format_value(len),
                FontId::proportional(12.0),
                pal.gold,
            );
        }
    }

    // Inline Driving Dimension Numeric Editor Popup
    if let (Some(edit_id), Some((anchor_pos, linked))) =
        (state.editing_dim_id.clone(), edit_anchor_screen)
    {
        let bounds = ui.ctx().content_rect().shrink(12.0);
        let box_w = 230.0;
        let box_h = 84.0;
        let pos = Pos2::new(
            (anchor_pos.x - box_w * 0.5)
                .clamp(bounds.left(), (bounds.right() - box_w).max(bounds.left())),
            (anchor_pos.y - box_h - 14.0)
                .clamp(bounds.top(), (bounds.bottom() - box_h).max(bounds.top())),
        );
        let mut submit = false;
        let mut cancel = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));

        egui::Area::new(Id::unique("inline_dim_drive_popup"))
            .fixed_pos(pos)
            .order(Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(pal.surface_panel)
                    .stroke(Stroke::new(1.5, pal.accent))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(if linked {
                                "Drive Linked Length"
                            } else {
                                "Unlinked Note (anchor to wall/opening to drive)"
                            })
                            .small()
                            .strong()
                            .color(pal.text_primary),
                        );
                        ui.horizontal(|ui| {
                            let mut disp = unit.from_mm(state.edit_value_mm);
                            let dv = ui.add(
                                egui::DragValue::new(&mut disp)
                                    .speed(unit.drag_speed() * 0.5)
                                    .range(unit.from_mm(1.0)..=unit.from_mm(2500.0))
                                    .max_decimals(unit.decimals().max(2))
                                    .suffix(unit.suffix()),
                            );
                            if state.edit_focus_requested {
                                dv.request_focus();
                                state.edit_focus_requested = false;
                            }
                            if dv.changed() {
                                state.edit_value_mm = unit.to_mm(disp).max(0.5);
                            }
                            if dv.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                submit = true;
                            }
                            if ui.button("Apply").clicked() {
                                submit = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel = true;
                            }
                        });
                    });
            });

        if submit {
            out.drive_request = Some((edit_id, state.edit_value_mm));
            state.editing_dim_id = None;
        } else if cancel {
            state.editing_dim_id = None;
        }
    }

    out
}

fn draw_selected_clearance_guides(
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall: &ProjectWall,
    components: &[ComponentDef],
    opening_id: &str,
    unit: DisplayUnit,
) {
    let Some(op) = wall.openings.iter().find(|o| o.id == opening_id) else {
        return;
    };
    let w = op.width(components);
    let h = op.height(components);
    let l = op.x_from_left;
    let r = l + w;
    let b = op.y_from_bottom;
    let mid_x = l + w * 0.5;
    let mid_y = b + h * 0.5;
    let col = pal.text_muted.gamma_multiply(0.85);
    let guide = Stroke::new(1.0, col);

    if l > 1.5 {
        let a = xform.mm_to_screen([0.0, mid_y]);
        let c = xform.mm_to_screen([l, mid_y]);
        painter.line_segment([a, c], guide);
        painter.text(
            a.lerp(c, 0.5) + Vec2::new(0.0, -3.0),
            Align2::CENTER_BOTTOM,
            format!("L: {}", unit.format_value(l)),
            FontId::proportional(10.0),
            col,
        );
    }
    let right_gap = (wall.width - r).max(0.0);
    if right_gap > 1.5 {
        let a = xform.mm_to_screen([r, mid_y]);
        let c = xform.mm_to_screen([wall.width, mid_y]);
        painter.line_segment([a, c], guide);
        painter.text(
            a.lerp(c, 0.5) + Vec2::new(0.0, -3.0),
            Align2::CENTER_BOTTOM,
            format!("R: {}", unit.format_value(right_gap)),
            FontId::proportional(10.0),
            col,
        );
    }
    if b > 1.5 {
        let a = xform.mm_to_screen([mid_x, 0.0]);
        let c = xform.mm_to_screen([mid_x, b]);
        painter.line_segment([a, c], guide);
        painter.text(
            a.lerp(c, 0.5) + Vec2::new(4.0, 0.0),
            Align2::LEFT_CENTER,
            format!("Sill: {}", unit.format_value(b)),
            FontId::proportional(10.0),
            col,
        );
    }
}
