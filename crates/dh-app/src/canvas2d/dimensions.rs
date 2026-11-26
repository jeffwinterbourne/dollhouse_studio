use crate::canvas2d::photo_layer::BlueprintXform;
use crate::theme::StudioPalette;
use dh_core::{ComponentDef, DisplayUnit, ProjectWall, WallDimension};
use eframe::egui::{self, Align2, Color32, FontId, Rect, Stroke, Vec2};

#[derive(Debug, Clone, Default)]
pub struct DimensionToolState {
    pub active_draw_mode: bool,
    draft_start: Option<[f64; 2]>,
    dragging_dim_offset: Option<String>,
    dragging_endpoint: Option<(String, usize)>,
    pub selected_dim_id: Option<String>,
}

impl DimensionToolState {
    pub fn cancel_draft(&mut self) {
        self.draft_start = None;
    }

    pub fn is_busy(&self) -> bool {
        self.draft_start.is_some()
            || self.dragging_dim_offset.is_some()
            || self.dragging_endpoint.is_some()
    }
}

#[derive(Default)]
pub struct DimensionsOutput {
    pub changed: bool,
    pub commit_label: Option<&'static str>,
    pub consumed_pointer: bool,
}


/// Collects snapping candidates (wall corners/midpoints + all opening corners & centers) in mm.
fn collect_snap_points(wall: &ProjectWall, components: &[ComponentDef]) -> Vec<[f64; 2]> {
    let mut pts = Vec::with_capacity(8 + wall.openings.len() * 5);
    pts.push([0.0, 0.0]);
    pts.push([wall.width, 0.0]);
    pts.push([wall.width, wall.height]);
    pts.push([0.0, wall.height]);
    pts.push([wall.width * 0.5, 0.0]);
    pts.push([wall.width * 0.5, wall.height]);
    pts.push([0.0, wall.height * 0.5]);
    pts.push([wall.width, wall.height * 0.5]);

    for op in &wall.openings {
        let w = op.width(components);
        let h = op.height(components);
        let x0 = op.x_from_left;
        let y0 = op.y_from_bottom;
        pts.push([x0, y0]);
        pts.push([x0 + w, y0]);
        pts.push([x0 + w, y0 + h]);
        pts.push([x0, y0 + h]);
        pts.push([x0 + w * 0.5, y0 + h * 0.5]);
    }
    pts
}

fn snap_mm(raw: [f64; 2], snaps: &[[f64; 2]], threshold_mm: f64) -> ([f64; 2], bool) {
    let mut best: Option<([f64; 2], f64)> = None;
    for &s in snaps {
        let d = ((s[0] - raw[0]).powi(2) + (s[1] - raw[1]).powi(2)).sqrt();
        if d <= threshold_mm && best.as_ref().is_none_or(|(_, bd)| d < *bd) {
            best = Some((s, d));
        }
    }
    match best {
        Some((p, _)) => (p, true),
        None => (raw, false),
    }
}

/// Draws persistent `wall.dimensions` and lets users add, adjust offset/endpoints, or delete via mouse.
#[allow(clippy::too_many_arguments)]
pub fn interact_and_draw_dimensions(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall: &mut ProjectWall,
    components: &[ComponentDef],
    unit: DisplayUnit,
    state: &mut DimensionToolState,
    allow_edit: bool,
    canvas_hovered: bool,
) -> DimensionsOutput {
    let mut out = DimensionsOutput::default();
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_pressed = canvas_hovered && ui.input(|i| i.pointer.primary_pressed());
    let primary_down = ui.input(|i| i.pointer.primary_down());
    let primary_released = ui.input(|i| i.pointer.primary_released());
    let secondary_pressed = canvas_hovered && ui.input(|i| i.pointer.secondary_pressed());
    let shift_held = ui.input(|i| i.modifiers.shift);

    let snaps = collect_snap_points(wall, components);
    let snap_thresh_mm = (10.0 / xform.px_per_mm.max(0.2)) as f64;

    if primary_released {
        if state.dragging_dim_offset.take().is_some() {
            out.commit_label = Some("Move dimension line");
            out.consumed_pointer = true;
        }
        if state.dragging_endpoint.take().is_some() {
            out.commit_label = Some("Adjust dimension endpoints");
            out.consumed_pointer = true;
        }
        if let Some(start) = state.draft_start.take()
            && let Some(pos) = pointer_pos {
                let mut end = xform.screen_to_mm(pos);
                let (snapped, _) = snap_mm(end, &snaps, snap_thresh_mm);
                end = snapped;
                if shift_held {
                    if (end[0] - start[0]).abs() >= (end[1] - start[1]).abs() {
                        end[1] = start[1];
                    } else {
                        end[0] = start[0];
                    }
                }
                let len = ((end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2)).sqrt();
                if len >= 3.0 {
                    let dim = WallDimension::new(start[0], start[1], end[0], end[1], 12.0);
                    state.selected_dim_id = Some(dim.id.clone());
                    wall.dimensions.push(dim);
                    out.changed = true;
                    out.commit_label = Some("Add architectural dimension");
                }
                out.consumed_pointer = true;
            }
    }

    // Active drag adjustments.
    if allow_edit && primary_down
        && let Some(pos) = pointer_pos {
            if let Some(ref dim_id) = state.dragging_dim_offset {
                if let Some(dim) = wall.dimensions.iter_mut().find(|d| &d.id == dim_id) {
                    let cur = xform.screen_to_mm(pos);
                    let dx = dim.x1 - dim.x0;
                    let dy = dim.y1 - dim.y0;
                    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                    // Perpendicular unit normal in wall mm coords
                    let nx = -dy / len;
                    let ny = dx / len;
                    let signed = (cur[0] - dim.x0) * nx + (cur[1] - dim.y0) * ny;
                    dim.offset_mm = signed.clamp(-140.0, 140.0);
                    out.changed = true;
                    out.consumed_pointer = true;
                }
            } else if let Some((ref dim_id, ep)) = state.dragging_endpoint {
                let (mut mm, _) = snap_mm(xform.screen_to_mm(pos), &snaps, snap_thresh_mm);
                if let Some(dim) = wall.dimensions.iter_mut().find(|d| &d.id == dim_id) {
                    if shift_held {
                        let anchor = if ep == 0 {
                            [dim.x1, dim.y1]
                        } else {
                            [dim.x0, dim.y0]
                        };
                        if (mm[0] - anchor[0]).abs() >= (mm[1] - anchor[1]).abs() {
                            mm[1] = anchor[1];
                        } else {
                            mm[0] = anchor[0];
                        }
                    }
                    if ep == 0 {
                        dim.x0 = mm[0];
                        dim.y0 = mm[1];
                    } else {
                        dim.x1 = mm[0];
                        dim.y1 = mm[1];
                    }
                    out.changed = true;
                    out.consumed_pointer = true;
                }
            }
        }

    // Check hover & start dragging existing dimensions or creating a new one.
    let mut delete_id: Option<String> = None;

    for dim in &wall.dimensions {
        let p0 = xform.mm_to_screen([dim.x0, dim.y0]);
        let p1 = xform.mm_to_screen([dim.x1, dim.y1]);
        let dx = dim.x1 - dim.x0;
        let dy = dim.y1 - dim.y0;
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let nx = -dy / len;
        let ny = dx / len;

        let d0 = xform.mm_to_screen([dim.x0 + nx * dim.offset_mm, dim.y0 + ny * dim.offset_mm]);
        let d1 = xform.mm_to_screen([dim.x1 + nx * dim.offset_mm, dim.y1 + ny * dim.offset_mm]);
        let mid = d0.lerp(d1, 0.5);

        let selected = state.selected_dim_id.as_deref() == Some(dim.id.as_str());
        let line_col = if selected { pal.gold } else { pal.accent };

        // Extension lines.
        let ext_stroke = Stroke::new(1.0, line_col.linear_multiply(0.55));
        painter.line_segment([p0, d0], ext_stroke);
        painter.line_segment([p1, d1], ext_stroke);

        // Main dimension line & tick caps.
        painter.line_segment([d0, d1], Stroke::new(1.5, line_col));
        let dir = (d1 - d0).normalized();
        let perp = Vec2::new(-dir.y, dir.x);
        painter.line_segment([d0 - perp * 4.0, d0 + perp * 4.0], Stroke::new(1.5, line_col));
        painter.line_segment([d1 - perp * 4.0, d1 + perp * 4.0], Stroke::new(1.5, line_col));

        // Text label pill.
        let text = dim.format_text(unit);
        let pill_rect = Rect::from_center_size(mid, Vec2::new((text.len() as f32 * 6.8 + 14.0).max(46.0), 18.0));
        painter.rect_filled(pill_rect, 4.0, pal.hud_glass);
        painter.rect_stroke(
            pill_rect,
            4.0,
            Stroke::new(1.0, line_col),
            egui::StrokeKind::Inside,
        );
        painter.text(
            mid,
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(11.5),
            if selected { pal.gold } else { pal.text_primary },
        );

        if selected {
            painter.circle_filled(p0, 4.0, pal.gold);
            painter.circle_filled(p1, 4.0, pal.gold);
        }

        if allow_edit
            && let Some(pos) = pointer_pos {
                if secondary_pressed && pill_rect.expand(4.0).contains(pos) {
                    delete_id = Some(dim.id.clone());
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

    if let Some(del) = delete_id {
        wall.dimensions.retain(|d| d.id != del);
        if state.selected_dim_id.as_deref() == Some(del.as_str()) {
            state.selected_dim_id = None;
        }
        out.changed = true;
        out.commit_label = Some("Delete dimension");
        out.consumed_pointer = true;
    }

    // Active dimension drafting when Annotate mode is enabled.
    if allow_edit && state.active_draw_mode && !out.consumed_pointer {
        if let Some(pos) = pointer_pos
            && xform.canvas_rect.contains(pos) {
                let (snapped_mm, was_snapped) =
                    snap_mm(xform.screen_to_mm(pos), &snaps, snap_thresh_mm);
                let snapped_s = xform.mm_to_screen(snapped_mm);
                if was_snapped {
                    painter.circle_stroke(snapped_s, 7.0, Stroke::new(1.5, pal.gold));
                }

                if primary_pressed && state.draft_start.is_none() {
                    state.draft_start = Some(snapped_mm);
                    out.consumed_pointer = true;
                }

                if let Some(start) = state.draft_start {
                    let mut end = snapped_mm;
                    if shift_held {
                        if (end[0] - start[0]).abs() >= (end[1] - start[1]).abs() {
                            end[1] = start[1];
                        } else {
                            end[0] = start[0];
                        }
                    }
                    let s0 = xform.mm_to_screen(start);
                    let s1 = xform.mm_to_screen(end);
                    painter.line_segment([s0, s1], Stroke::new(2.0, pal.gold));
                    let len = ((end[0] - start[0]).powi(2) + (end[1] - start[1]).powi(2)).sqrt();
                    painter.text(
                        s0.lerp(s1, 0.5) + Vec2::new(0.0, -12.0),
                        Align2::CENTER_BOTTOM,
                        unit.format_value(len),
                        FontId::proportional(12.0),
                        pal.gold,
                    );
                }
            }
    } else if state.active_draw_mode && !allow_edit {
        let _ = Color32::TRANSPARENT;
    }

    out
}
