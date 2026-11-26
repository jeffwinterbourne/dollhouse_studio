use crate::canvas2d::photo_layer::BlueprintXform;
use crate::theme::StudioPalette;
use dh_core::{
    ComponentDef, DisplayUnit, FlushTopMode, ProjectWall, ShutterStyle, WallOpening, WindowParams,
    WindowStyle,
};
use dh_geometry::{clipped_trim_outer_contour, profile_kind};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub enum OpeningsInteraction {
    SelectSingle(String),
    ToggleSelect(String),
    SetSelection(Vec<String>),
    ClearSelection,
    DoubleClickOpen3D(String),
    MoveOpeningsAbsolute(Vec<(String, f64, f64)>),
    ResizeOpening {
        opening_id: String,
        x_from_left: f64,
        y_from_bottom: f64,
        width: f64,
        height: f64,
    },
    ResizeWall {
        width: f64,
        height: f64,
    },
    FinishedDrag(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct SnapGuide {
    pub vertical: bool,
    pub coord_mm: f64,
}

#[derive(Debug, Clone)]
enum OpeningDragMode {
    MoveOpenings {
        start_mm: [f64; 2],
        origins: Vec<(String, f64, f64)>,
    },
    ResizeOpening {
        id: String,
        handle: usize,
        start_mm: [f64; 2],
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
        lock_w: bool,
        lock_h: bool,
    },
    ResizeWallEdge {
        /// `0`: Right edge, `1`: Top edge, `2`: Top-Right corner
        edge: usize,
        start_mm: [f64; 2],
        orig_w: f64,
        orig_h: f64,
    },
    Marquee {
        start_screen: Pos2,
        current_screen: Pos2,
    },
}

#[derive(Default)]
pub struct OpeningsInteractionState {
    drag: Option<OpeningDragMode>,
    pub active_guides: Vec<SnapGuide>,
    pub hovered_opening_id: Option<String>,
}

impl OpeningsInteractionState {
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
}

/// Paints the wall elevation outline, architectural opening symbols, architrave tolerance envelopes,
/// flush indicators, smart guides, and interactive mouse handles.
#[allow(clippy::too_many_arguments)]
pub fn interact_and_draw_openings(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall: &ProjectWall,
    components: &[ComponentDef],
    selected_ids: &[String],
    unit: DisplayUnit,
    show_architrave_outlines: bool,
    state: &mut OpeningsInteractionState,
    allow_edit: bool,
    canvas_hovered: bool,
) -> Vec<OpeningsInteraction> {
    let mut actions = Vec::new();
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_pressed = canvas_hovered && ui.input(|i| i.pointer.primary_pressed());
    let primary_down = ui.input(|i| i.pointer.primary_down());
    let primary_released = ui.input(|i| i.pointer.primary_released());
    let double_clicked = canvas_hovered
        && ui.input(|i| {
            i.pointer
                .button_double_clicked(egui::PointerButton::Primary)
        });
    let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
    let shift_held = ui.input(|i| i.modifiers.shift);

    let wall_rect = Rect::from_min_max(
        xform.mm_to_screen([0.0, wall.height]),
        xform.mm_to_screen([wall.width, 0.0]),
    );

    // Wall dimension badges & draggable outer edge handles (no typing required to resize wall!).
    let right_edge_mid = Pos2::new(wall_rect.right(), wall_rect.center().y);
    let top_edge_mid = Pos2::new(wall_rect.center().x, wall_rect.top());
    let top_right_corner = wall_rect.right_top();

    // Handle pointer release first.
    if primary_released && let Some(drag) = state.drag.take() {
        state.active_guides.clear();
        match drag {
            OpeningDragMode::MoveOpenings { .. } => {
                actions.push(OpeningsInteraction::FinishedDrag("Move opening(s)"));
            }
            OpeningDragMode::ResizeOpening { .. } => {
                actions.push(OpeningsInteraction::FinishedDrag("Resize opening"));
            }
            OpeningDragMode::ResizeWallEdge { .. } => {
                actions.push(OpeningsInteraction::FinishedDrag("Resize wall"));
            }
            OpeningDragMode::Marquee {
                start_screen,
                current_screen,
            } => {
                let marq = Rect::from_two_pos(start_screen, current_screen);
                if marq.width() > 6.0 && marq.height() > 6.0 {
                    let picked: Vec<String> = wall
                        .openings
                        .iter()
                        .filter(|op| {
                            let r = opening_screen_rect(xform, op, components);
                            r.intersects(marq)
                        })
                        .map(|op| op.id.clone())
                        .collect();
                    actions.push(OpeningsInteraction::SetSelection(picked));
                }
            }
        }
    }

    // Update hover state.
    state.hovered_opening_id = None;
    if let Some(pos) = pointer_pos
        && xform.canvas_rect.contains(pos)
    {
        for op in wall.openings.iter().rev() {
            let r = opening_screen_rect(xform, op, components);
            if r.expand(3.0).contains(pos) {
                state.hovered_opening_id = Some(op.id.clone());
                break;
            }
        }
    }

    // Double-click jumps directly to Piece 3D.
    if allow_edit
        && double_clicked
        && let Some(ref hid) = state.hovered_opening_id
    {
        actions.push(OpeningsInteraction::DoubleClickOpen3D(hid.clone()));
        return actions;
    }

    // Start drags on primary press.
    if allow_edit
        && primary_pressed
        && let Some(pos) = pointer_pos
        && xform.canvas_rect.contains(pos)
    {
        let mut started = false;

        // 1) Check single selected opening 8 resize handles first.
        if selected_ids.len() == 1
            && let Some(op) = wall.openings.iter().find(|o| o.id == selected_ids[0])
        {
            let p = op.resolve_params(components);
            let r = opening_screen_rect(xform, op, components);
            let handles = rect_handles(r);
            if let Some((idx, _)) = handles
                .iter()
                .enumerate()
                .find(|(_, pt)| pt.distance(pos) <= 9.0)
            {
                state.drag = Some(OpeningDragMode::ResizeOpening {
                    id: op.id.clone(),
                    handle: idx,
                    start_mm: xform.screen_to_mm(pos),
                    orig_x: op.x_from_left,
                    orig_y: op.y_from_bottom,
                    orig_w: p.opening_width,
                    orig_h: p.opening_height,
                    lock_w: op.lock_width,
                    lock_h: op.lock_height,
                });
                started = true;
            }
        }

        // 2) Check wall outer edge/corner handles (`Top-Right`, `Right Edge`, `Top Edge`).
        if !started {
            let hit_wall_edge = if top_right_corner.distance(pos) <= 10.0 {
                Some(2)
            } else if (pos.x - wall_rect.right()).abs() <= 7.0
                && pos.y >= wall_rect.top()
                && pos.y <= wall_rect.bottom()
            {
                Some(0)
            } else if (pos.y - wall_rect.top()).abs() <= 7.0
                && pos.x >= wall_rect.left()
                && pos.x <= wall_rect.right()
            {
                Some(1)
            } else {
                None
            };
            if let Some(edge) = hit_wall_edge {
                state.drag = Some(OpeningDragMode::ResizeWallEdge {
                    edge,
                    start_mm: xform.screen_to_mm(pos),
                    orig_w: wall.width,
                    orig_h: wall.height,
                });
                started = true;
            }
        }

        // 3) Hit test openings for selection + move drag.
        if !started && let Some(hid) = state.hovered_opening_id.clone() {
            if ctrl_held {
                actions.push(OpeningsInteraction::ToggleSelect(hid));
            } else {
                let already = selected_ids.contains(&hid);
                if !already {
                    actions.push(OpeningsInteraction::SelectSingle(hid.clone()));
                }
                let moving_ids: Vec<String> = if already {
                    selected_ids.to_vec()
                } else {
                    vec![hid]
                };
                let origins: Vec<(String, f64, f64)> = wall
                    .openings
                    .iter()
                    .filter(|o| moving_ids.contains(&o.id) && !o.lock_position)
                    .map(|o| (o.id.clone(), o.x_from_left, o.y_from_bottom))
                    .collect();
                if !origins.is_empty() {
                    state.drag = Some(OpeningDragMode::MoveOpenings {
                        start_mm: xform.screen_to_mm(pos),
                        origins,
                    });
                }
            }
            started = true;
        }

        // 4) Empty canvas click starts marquee selection box.
        if !started {
            if !ctrl_held {
                actions.push(OpeningsInteraction::ClearSelection);
            }
            state.drag = Some(OpeningDragMode::Marquee {
                start_screen: pos,
                current_screen: pos,
            });
        }
    }

    // Active drag motion.
    if allow_edit
        && primary_down
        && let (Some(pos), Some(drag)) = (pointer_pos, state.drag.as_mut())
    {
        let cur_mm = xform.screen_to_mm(pos);
        match drag {
            OpeningDragMode::MoveOpenings { start_mm, origins } => {
                let mut dx = cur_mm[0] - start_mm[0];
                let mut dy = cur_mm[1] - start_mm[1];
                if shift_held {
                    if dx.abs() >= dy.abs() {
                        dy = 0.0;
                    } else {
                        dx = 0.0;
                    }
                }

                // Snap primary dragged item against wall edges/center + sibling openings.
                let mut guides = Vec::new();
                if let Some((lead_id, ox, oy)) = origins.first()
                    && let Some(lead) = wall.openings.iter().find(|o| &o.id == lead_id)
                {
                    let lw = lead.width(components);
                    let lh = lead.height(components);
                    let moving_set: HashSet<&str> =
                        origins.iter().map(|(id, _, _)| id.as_str()).collect();
                    let cand_x = ox + dx;
                    let cand_y = oy + dy;
                    let snap_x = snap_axis_x(cand_x, lw, wall, components, &moving_set, 3.5);
                    if let Some((snapped_dx, gx)) = snap_x {
                        dx += snapped_dx;
                        guides.push(SnapGuide {
                            vertical: true,
                            coord_mm: gx,
                        });
                    }
                    let snap_y = snap_axis_y(cand_y, lh, wall, components, &moving_set, 3.5);
                    if let Some((snapped_dy, gy)) = snap_y {
                        dy += snapped_dy;
                        guides.push(SnapGuide {
                            vertical: false,
                            coord_mm: gy,
                        });
                    }
                }

                state.active_guides = guides;
                let moved: Vec<(String, f64, f64)> = origins
                    .iter()
                    .map(|(id, ox, oy)| (id.clone(), ox + dx, oy + dy))
                    .collect();
                if !moved.is_empty() {
                    actions.push(OpeningsInteraction::MoveOpeningsAbsolute(moved));
                }
            }
            OpeningDragMode::ResizeOpening {
                id,
                handle,
                start_mm,
                orig_x,
                orig_y,
                orig_w,
                orig_h,
                lock_w,
                lock_h,
            } => {
                let dx = cur_mm[0] - start_mm[0];
                let dy = cur_mm[1] - start_mm[1];
                let mut left = *orig_x;
                let mut bottom = *orig_y;
                let mut right = *orig_x + *orig_w;
                let mut top = *orig_y + *orig_h;

                if !*lock_w {
                    match *handle {
                        0 | 6 | 7 => left = (*orig_x + dx).clamp(0.0, right - 18.0),
                        2..=4 => right = (*orig_x + *orig_w + dx).clamp(left + 18.0, wall.width),
                        _ => {}
                    }
                }
                if !*lock_h {
                    match *handle {
                        0..=2 => top = (*orig_y + *orig_h + dy).clamp(bottom + 18.0, wall.height),
                        4..=6 => bottom = (*orig_y + dy).clamp(0.0, top - 18.0),
                        _ => {}
                    }
                }
                actions.push(OpeningsInteraction::ResizeOpening {
                    opening_id: id.clone(),
                    x_from_left: left,
                    y_from_bottom: bottom,
                    width: (right - left).max(18.0),
                    height: (top - bottom).max(18.0),
                });
            }
            OpeningDragMode::ResizeWallEdge {
                edge,
                start_mm,
                orig_w,
                orig_h,
            } => {
                let dx = cur_mm[0] - start_mm[0];
                let dy = cur_mm[1] - start_mm[1];
                let mut nw = *orig_w;
                let mut nh = *orig_h;
                if matches!(*edge, 0 | 2) {
                    nw = (*orig_w + dx).clamp(60.0, 2000.0);
                }
                if matches!(*edge, 1 | 2) {
                    nh = (*orig_h + dy).clamp(60.0, 1500.0);
                }
                if !shift_held {
                    nw = (nw / 2.0).round() * 2.0;
                    nh = (nh / 2.0).round() * 2.0;
                }
                actions.push(OpeningsInteraction::ResizeWall {
                    width: nw,
                    height: nh,
                });
            }
            OpeningDragMode::Marquee { current_screen, .. } => {
                *current_screen = pos;
            }
        }
    }

    // Draw snap guides.
    for g in &state.active_guides {
        if g.vertical {
            let sx = xform.mm_to_screen([g.coord_mm, 0.0]).x;
            painter.line_segment(
                [
                    Pos2::new(sx, wall_rect.top() - 24.0),
                    Pos2::new(sx, wall_rect.bottom() + 24.0),
                ],
                Stroke::new(1.2, pal.gold),
            );
        } else {
            let sy = xform.mm_to_screen([0.0, g.coord_mm]).y;
            painter.line_segment(
                [
                    Pos2::new(wall_rect.left() - 24.0, sy),
                    Pos2::new(wall_rect.right() + 24.0, sy),
                ],
                Stroke::new(1.2, pal.gold),
            );
        }
    }

    // Optional Architrave / Exterior Trim Tolerance Outlines & Clash Alerts.
    if show_architrave_outlines {
        paint_architrave_tolerance_outlines(painter, xform, wall, components);
    }

    // Draw every opening cutout & architectural styling.
    for op in &wall.openings {
        let params = op.resolve_params(components);
        let r = opening_screen_rect(xform, op, components);
        let selected = selected_ids.contains(&op.id);
        let hovered = state.hovered_opening_id.as_deref() == Some(op.id.as_str());
        paint_opening_2d(
            painter, pal, op, &params, components, r, unit, selected, hovered,
        );
    }

    // Paint draggable wall resize edge knobs on wall border (subtle unless hovered).
    if allow_edit {
        for (idx, pt) in [
            (0usize, right_edge_mid),
            (1, top_edge_mid),
            (2, top_right_corner),
        ] {
            let hot = pointer_pos.is_some_and(|p| p.distance(pt) <= 10.0)
                || matches!(state.drag, Some(OpeningDragMode::ResizeWallEdge { edge, .. }) if edge == idx);
            let r = if hot { 5.5 } else { 4.0 };
            painter.rect_filled(
                Rect::from_center_size(pt, Vec2::splat(r * 2.0)),
                2.0,
                if hot { pal.gold } else { pal.wall_stroke },
            );
        }
    }

    // Paint marquee rectangle if active.
    if let Some(OpeningDragMode::Marquee {
        start_screen,
        current_screen,
    }) = state.drag
    {
        let mr = Rect::from_two_pos(start_screen, current_screen);
        painter.rect_filled(mr, 2.0, pal.accent.linear_multiply(0.15));
        painter.rect_stroke(
            mr,
            2.0,
            Stroke::new(1.5, pal.accent),
            egui::StrokeKind::Inside,
        );
    }

    actions
}

#[derive(Clone, Copy)]
struct TrimEnvelopeMm {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl TrimEnvelopeMm {
    fn include_pt(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }
}

struct PreparedTrimEnvelope {
    contour_mm: Vec<[f64; 2]>,
    sill_mm: Option<[f64; 4]>,
    shutters_mm: Vec<[f64; 4]>,
    bbox: TrimEnvelopeMm,
    overhang: bool,
    clash: bool,
}

fn paint_architrave_tolerance_outlines(
    painter: &egui::Painter,
    xform: BlueprintXform,
    wall: &ProjectWall,
    components: &[ComponentDef],
) {
    let mut prepared: Vec<PreparedTrimEnvelope> = Vec::with_capacity(wall.openings.len());
    for op in &wall.openings {
        let params = op.resolve_params(components);
        let w = params.opening_width;
        let h = params.opening_height;
        let cx = op.x_from_left + w * 0.5;
        let cy = op.y_from_bottom + h * 0.5;
        let outset = params.trim_width + params.wall_fit_clearance;
        let contour =
            clipped_trim_outer_contour(profile_kind(params.style), w, h, outset, &params, 32);

        let mut bbox = TrimEnvelopeMm {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        let mut contour_mm = Vec::with_capacity(contour.len());
        for pt in contour {
            let wx = cx + pt[0];
            let wy = cy + pt[1];
            bbox.include_pt(wx, wy);
            contour_mm.push([wx, wy]);
        }

        let sill_mm = if params.sill_enabled
            && (params.style.supports_sill() || params.style.is_door())
            && !params.flush_bottom_sill
        {
            let half_s = w * 0.5 + params.trim_width + params.sill_overhang;
            let sx0 = cx
                - if params.flush_left_jamb {
                    w * 0.5 + params.wall_fit_clearance
                } else {
                    half_s
                };
            let sx1 = cx
                + if params.flush_right_jamb {
                    w * 0.5 + params.wall_fit_clearance
                } else {
                    half_s
                };
            let sy0 = op.y_from_bottom - params.trim_width - params.sill_depth;
            let sy1 = op.y_from_bottom;
            bbox.include_pt(sx0, sy0);
            bbox.include_pt(sx1, sy1);
            Some([sx0, sy0, sx1, sy1])
        } else {
            None
        };

        let mut shutters_mm = Vec::new();
        if params.shutter_style != ShutterStyle::None && !params.style.is_door() {
            let sh_w = (w * params.shutter_width_ratio).clamp(6.0, 45.0);
            let sy0 = op.y_from_bottom;
            let sy1 = op.y_from_bottom + h;
            if !params.flush_left_jamb {
                let lx1 = op.x_from_left - outset;
                let lx0 = lx1 - sh_w;
                bbox.include_pt(lx0, sy0);
                bbox.include_pt(lx1, sy1);
                shutters_mm.push([lx0, sy0, lx1, sy1]);
            }
            if !params.flush_right_jamb {
                let rx0 = op.x_from_left + w + outset;
                let rx1 = rx0 + sh_w;
                bbox.include_pt(rx0, sy0);
                bbox.include_pt(rx1, sy1);
                shutters_mm.push([rx0, sy0, rx1, sy1]);
            }
        }

        let overhang = bbox.min_x < -0.15
            || bbox.min_y < -0.15
            || bbox.max_x > wall.width + 0.15
            || bbox.max_y > wall.height + 0.15;

        prepared.push(PreparedTrimEnvelope {
            contour_mm,
            sill_mm,
            shutters_mm,
            bbox,
            overhang,
            clash: false,
        });
    }

    let mut overlap_zones: Vec<[f64; 4]> = Vec::new();
    for i in 0..prepared.len() {
        for j in (i + 1)..prepared.len() {
            let a = prepared[i].bbox;
            let b = prepared[j].bbox;
            let ox0 = a.min_x.max(b.min_x);
            let ox1 = a.max_x.min(b.max_x);
            let oy0 = a.min_y.max(b.min_y);
            let oy1 = a.max_y.min(b.max_y);
            if ox1 - ox0 > 0.15 && oy1 - oy0 > 0.15 {
                prepared[i].clash = true;
                prepared[j].clash = true;
                overlap_zones.push([ox0, oy0, ox1, oy1]);
            }
        }
    }

    let clean_col = Color32::from_rgb(72, 188, 198);
    let warn_col = Color32::from_rgb(245, 165, 45);
    let clash_col = Color32::from_rgb(235, 72, 72);

    for env in &prepared {
        let col = if env.clash {
            clash_col
        } else if env.overhang {
            warn_col
        } else {
            clean_col
        };
        let halo = col.gamma_multiply(0.18);
        let stroke = Stroke::new(1.5, col.gamma_multiply(0.92));

        if env.contour_mm.len() >= 3 {
            let pts: Vec<Pos2> = env
                .contour_mm
                .iter()
                .map(|p| xform.mm_to_screen(*p))
                .collect();
            painter.add(Shape::closed_line(pts.clone(), Stroke::new(4.5, halo)));
            painter.add(Shape::closed_line(pts, stroke));
        }

        if let Some([x0, y0, x1, y1]) = env.sill_mm {
            let sr = Rect::from_min_max(xform.mm_to_screen([x0, y1]), xform.mm_to_screen([x1, y0]));
            painter.rect_filled(sr, 1.0, halo);
            painter.rect_stroke(sr, 1.0, stroke, StrokeKind::Outside);
        }

        for [x0, y0, x1, y1] in &env.shutters_mm {
            let sr = Rect::from_min_max(
                xform.mm_to_screen([*x0, *y1]),
                xform.mm_to_screen([*x1, *y0]),
            );
            painter.rect_filled(sr, 1.0, halo);
            painter.rect_stroke(sr, 1.0, stroke, StrokeKind::Outside);
        }

        if env.overhang && !env.clash {
            let top_mid =
                xform.mm_to_screen([(env.bbox.min_x + env.bbox.max_x) * 0.5, env.bbox.max_y]);
            painter.text(
                top_mid + Vec2::new(0.0, -3.0),
                Align2::CENTER_BOTTOM,
                "Trim overhang",
                FontId::proportional(10.0),
                warn_col,
            );
        }
    }

    for [ox0, oy0, ox1, oy1] in overlap_zones {
        let cr = Rect::from_min_max(
            xform.mm_to_screen([ox0, oy1]),
            xform.mm_to_screen([ox1, oy0]),
        );
        painter.rect_filled(cr, 1.0, Color32::from_rgba_unmultiplied(235, 72, 72, 70));
        painter.rect_stroke(cr, 1.0, Stroke::new(1.5, clash_col), StrokeKind::Outside);
        painter.text(
            cr.center(),
            Align2::CENTER_CENTER,
            "Trim Clash",
            FontId::proportional(10.0),
            clash_col,
        );
    }
}

pub fn opening_screen_rect(
    xform: BlueprintXform,
    op: &WallOpening,
    components: &[ComponentDef],
) -> Rect {
    let w = op.width(components);
    let h = op.height(components);
    Rect::from_min_max(
        xform.mm_to_screen([op.x_from_left, op.y_from_bottom + h]),
        xform.mm_to_screen([op.x_from_left + w, op.y_from_bottom]),
    )
}

fn rect_handles(r: Rect) -> [Pos2; 8] {
    [
        r.left_top(),
        r.center_top(),
        r.right_top(),
        r.right_center(),
        r.right_bottom(),
        r.center_bottom(),
        r.left_bottom(),
        r.left_center(),
    ]
}

fn snap_axis_x(
    cand_x: f64,
    w: f64,
    wall: &ProjectWall,
    components: &[ComponentDef],
    moving_set: &HashSet<&str>,
    thresh: f64,
) -> Option<(f64, f64)> {
    let mut targets = vec![0.0, wall.width * 0.5, wall.width];
    for op in &wall.openings {
        if !moving_set.contains(op.id.as_str()) {
            let ow = op.width(components);
            targets.push(op.x_from_left);
            targets.push(op.x_from_left + ow * 0.5);
            targets.push(op.x_from_left + ow);
        }
    }
    let edges = [cand_x, cand_x + w * 0.5, cand_x + w];
    let mut best: Option<(f64, f64, f64)> = None;
    for e in edges {
        for &t in &targets {
            let diff = t - e;
            if diff.abs() <= thresh && best.is_none_or(|(bd, _, _)| diff.abs() < bd.abs()) {
                best = Some((diff, t, diff.abs()));
            }
        }
    }
    best.map(|(d, gx, _)| (d, gx))
}

fn snap_axis_y(
    cand_y: f64,
    h: f64,
    wall: &ProjectWall,
    components: &[ComponentDef],
    moving_set: &HashSet<&str>,
    thresh: f64,
) -> Option<(f64, f64)> {
    let mut targets = vec![0.0, wall.height * 0.5, wall.height];
    for op in &wall.openings {
        if !moving_set.contains(op.id.as_str()) {
            let oh = op.height(components);
            targets.push(op.y_from_bottom);
            targets.push(op.y_from_bottom + oh * 0.5);
            targets.push(op.y_from_bottom + oh);
        }
    }
    let edges = [cand_y, cand_y + h * 0.5, cand_y + h];
    let mut best: Option<(f64, f64, f64)> = None;
    for e in edges {
        for &t in &targets {
            let diff = t - e;
            if diff.abs() <= thresh && best.is_none_or(|(bd, _, _)| diff.abs() < bd.abs()) {
                best = Some((diff, t, diff.abs()));
            }
        }
    }
    best.map(|(d, gy, _)| (d, gy))
}

#[allow(clippy::too_many_arguments)]
fn paint_opening_2d(
    painter: &egui::Painter,
    pal: &StudioPalette,
    op: &WallOpening,
    params: &WindowParams,
    components: &[ComponentDef],
    r: Rect,
    unit: DisplayUnit,
    selected: bool,
    hovered: bool,
) {
    let comp_linked = op.is_component();
    let frame_col = if selected {
        pal.gold
    } else if hovered {
        pal.accent
    } else if comp_linked {
        Color32::from_rgb(80, 175, 240)
    } else {
        pal.text_primary
    };

    let fill_col = if selected {
        pal.gold.linear_multiply(0.16)
    } else if comp_linked {
        Color32::from_rgba_unmultiplied(64, 155, 225, 32)
    } else {
        pal.surface_base.linear_multiply(0.45)
    };

    // Draw shaped outer silhouette & interior detail matching each WindowStyle.
    let stroke = Stroke::new(if selected { 2.2 } else { 1.6 }, frame_col);
    let inner = r.shrink(3.5);
    let bar = Stroke::new(1.1, frame_col.linear_multiply(0.75));

    match params.style {
        WindowStyle::CircularBullsEye => {
            let c = r.center();
            let rad = r.width().min(r.height()) * 0.5;
            painter.circle_filled(c, rad, fill_col);
            painter.circle_stroke(c, rad, stroke);
            painter.circle_stroke(c, (rad - 3.5).max(2.0), bar);
            let n = params.arch_spokes.max(2) as usize;
            for i in 0..n {
                let a = (i as f32 / n as f32) * 2.0 * PI;
                painter.line_segment(
                    [c, c + Vec2::new(a.cos(), a.sin()) * (rad - 3.5).max(2.0)],
                    bar,
                );
            }
        }
        WindowStyle::HalfRoundLunette => {
            let pts = arched_top_poly(r, r.height() * 0.98, 22);
            painter.add(Shape::convex_polygon(pts.clone(), fill_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, stroke));
            let c = Pos2::new(r.center().x, r.bottom());
            let rad = (r.width() * 0.45).min(r.height() * 0.9);
            let n = params.arch_spokes.max(2) as usize;
            for i in 1..n {
                let a = PI + (i as f32 / n as f32) * PI;
                painter.line_segment([c, c + Vec2::new(a.cos(), a.sin()) * rad], bar);
            }
        }
        WindowStyle::ArchedGeorgian
        | WindowStyle::DoorArchedHeritage
        | WindowStyle::DoorFanlightGeorgian => {
            let pts = arched_top_poly(r, (r.width() * 0.48).min(r.height() * 0.45), 18);
            painter.add(Shape::convex_polygon(pts.clone(), fill_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, stroke));
            draw_interior_bars(painter, inner, params, bar);
        }
        WindowStyle::GothicPointed => {
            let pts = gothic_top_poly(r, (r.height() * 0.40).min(r.width() * 0.65), 16);
            painter.add(Shape::convex_polygon(pts.clone(), fill_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, stroke));
            draw_interior_bars(painter, inner, params, bar);
        }
        _ => {
            painter.rect_filled(r, 1.5, fill_col);
            painter.rect_stroke(r, 1.5, stroke, egui::StrokeKind::Outside);
            draw_interior_bars(painter, inner, params, bar);
        }
    }

    // Flush-edge amber highlights so clipped trim edges are unmistakable on the 2D wall!
    let flush_stroke = Stroke::new(3.0, Color32::from_rgb(245, 165, 45));
    if params.flush_top_mode != FlushTopMode::None {
        painter.line_segment([r.left_top(), r.right_top()], flush_stroke);
    }
    if params.flush_bottom_sill {
        painter.line_segment([r.left_bottom(), r.right_bottom()], flush_stroke);
    }
    if params.flush_left_jamb {
        painter.line_segment([r.left_top(), r.left_bottom()], flush_stroke);
    }
    if params.flush_right_jamb {
        painter.line_segment([r.right_top(), r.right_bottom()], flush_stroke);
    }

    // Top name tag + Component link badge.
    let badge = if let Some(cid) = op.component_id() {
        let cname = components
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.name.as_str())
            .unwrap_or("Component");
        format!("{} [{}]", op.name, cname)
    } else {
        op.name.clone()
    };
    painter.text(
        Pos2::new(r.center().x, r.top() - 4.0),
        Align2::CENTER_BOTTOM,
        badge,
        FontId::proportional(11.0),
        frame_col,
    );

    if r.width() > 38.0 && r.height() > 22.0 {
        let dims = unit.format_dims(params.opening_width, params.opening_height);
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            dims,
            FontId::proportional(10.5),
            pal.text_secondary,
        );
    }

    // Resize handles when selected.
    if selected {
        for pt in rect_handles(r) {
            painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(7.0)), 1.5, pal.gold);
            painter.rect_stroke(
                Rect::from_center_size(pt, Vec2::splat(7.0)),
                1.5,
                Stroke::new(1.0, pal.accent_text),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn draw_interior_bars(painter: &egui::Painter, inner: Rect, params: &WindowParams, bar: Stroke) {
    if inner.width() < 8.0 || inner.height() < 8.0 {
        return;
    }
    if params.style.is_door() {
        let hx = if params.hinge_right {
            inner.left() + 3.5
        } else {
            inner.right() - 3.5
        };
        painter.circle_filled(Pos2::new(hx, inner.center().y), 2.2, bar.color);
        let hinge_x = if params.hinge_right {
            inner.right()
        } else {
            inner.left()
        };
        let knob_x = if params.hinge_right {
            inner.left()
        } else {
            inner.right()
        };
        painter.line_segment(
            [
                Pos2::new(hinge_x, inner.top()),
                Pos2::new(knob_x, inner.center().y),
            ],
            bar,
        );
        painter.line_segment(
            [
                Pos2::new(hinge_x, inner.bottom()),
                Pos2::new(knob_x, inner.center().y),
            ],
            bar,
        );
        return;
    }

    if params.style.uses_grid_bars() {
        let cols = (params.bars_x + 1).clamp(1, 8) as usize;
        let rows = (params.bars_y + 1).clamp(1, 8) as usize;
        for c in 1..cols {
            let x = inner.left() + inner.width() * (c as f32 / cols as f32);
            painter.line_segment(
                [Pos2::new(x, inner.top()), Pos2::new(x, inner.bottom())],
                bar,
            );
        }
        for r in 1..rows {
            let y = inner.top() + inner.height() * (r as f32 / rows as f32);
            painter.line_segment(
                [Pos2::new(inner.left(), y), Pos2::new(inner.right(), y)],
                bar,
            );
        }
    }
}

fn arched_top_poly(r: Rect, arch_h: f32, segs: usize) -> Vec<Pos2> {
    let mut pts = Vec::with_capacity(segs + 4);
    pts.push(r.left_bottom());
    pts.push(r.right_bottom());
    let spring_y = r.top() + arch_h;
    let cx = r.center().x;
    let rx = r.width() * 0.5;
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let a = t * PI;
        pts.push(Pos2::new(cx + rx * a.cos(), spring_y - arch_h * a.sin()));
    }
    pts
}

fn gothic_top_poly(r: Rect, arch_h: f32, segs: usize) -> Vec<Pos2> {
    let mut pts = Vec::with_capacity(segs * 2 + 3);
    pts.push(r.left_bottom());
    pts.push(r.right_bottom());
    let spring_y = r.top() + arch_h;
    let right_spring = Pos2::new(r.right(), spring_y);
    let apex = Pos2::new(r.center().x, r.top());
    let left_spring = Pos2::new(r.left(), spring_y);
    for i in 0..=segs {
        let t = i as f32 / segs as f32;
        let curve = t.powf(0.8);
        pts.push(right_spring.lerp(apex, curve) - Vec2::new(0.0, arch_h * 0.14 * (t * PI).sin()));
    }
    for i in 1..=segs {
        let t = i as f32 / segs as f32;
        let curve = 1.0 - (1.0 - t).powf(0.8);
        pts.push(apex.lerp(left_spring, curve) - Vec2::new(0.0, arch_h * 0.14 * (t * PI).sin()));
    }
    pts
}
