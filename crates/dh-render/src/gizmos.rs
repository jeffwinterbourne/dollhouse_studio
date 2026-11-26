use crate::camera::OrbitCamera;
use crate::picking::ray_wall_plane_point;
use dh_geometry::BuiltProjectWall;
use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Vec2};
use glam::Vec3;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum Viewport3DEvent {
    PickedOpening {
        id: Option<String>,
        toggle: bool,
    },
    DoubleClickedOpening(String),
    MoveOpening {
        id: String,
        x_from_left: f64,
        y_from_bottom: f64,
    },
    ResizeOpening {
        id: String,
        x_from_left: f64,
        y_from_bottom: f64,
        width: f64,
        height: f64,
    },
    InteractionFinished(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    Move,
    North,
    South,
    East,
    West,
}

#[derive(Debug, Clone)]
pub enum ActiveGizmoDrag {
    Move {
        id: String,
        start_wall_pt: [f64; 2],
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
    },
    Resize {
        id: String,
        handle: GizmoHandle,
        start_wall_pt: [f64; 2],
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
    },
}

#[derive(Debug, Default)]
pub struct GizmoState {
    pub active_drag: Option<ActiveGizmoDrag>,
    pub hovered_handle: Option<GizmoHandle>,
}

/// Draws interactive 3D move & N/S/E/W resize handles on the selected opening in Wall 3D.
/// Returns `true` when pointer drag/click is captured by a gizmo handle (suppressing camera orbit).
#[allow(clippy::too_many_arguments)]
pub fn handle_and_draw_wall_gizmos(
    ui: &mut egui::Ui,
    rect: Rect,
    response: &egui::Response,
    camera: &OrbitCamera,
    built_wall: &BuiltProjectWall,
    selected_ids: &HashSet<String>,
    gizmo: &mut GizmoState,
    events: &mut Vec<Viewport3DEvent>,
) -> bool {
    if selected_ids.len() != 1 {
        gizmo.active_drag = None;
        gizmo.hovered_handle = None;
        return false;
    }
    let Some(sel_id) = selected_ids.iter().next() else {
        return false;
    };
    let Some(op) = built_wall.wall.openings.iter().find(|o| &o.id == sel_id) else {
        return false;
    };
    let Some((_, cx, cy, win_gen)) = built_wall
        .placed_openings
        .iter()
        .find(|(id, _, _, _)| id == sel_id)
    else {
        return false;
    };

    let wall_w = built_wall.wall.width;
    let wall_h = built_wall.wall.height;
    let wall_t = built_wall.wall.wall_thickness as f32;
    let front_y = -(wall_t * 0.5 + win_gen.params.trim_thickness as f32 + 1.5);
    let ow = win_gen.params.opening_width as f32;
    let oh = win_gen.params.opening_height as f32;
    let hw = ow * 0.5;
    let hh = oh * 0.5;

    let Some(c_pos) = camera.project_point(rect, Vec3::new(*cx, front_y, *cy), 4000.0) else {
        return false;
    };
    let corners_3d = [
        Vec3::new(*cx - hw, front_y, *cy - hh),
        Vec3::new(*cx + hw, front_y, *cy - hh),
        Vec3::new(*cx + hw, front_y, *cy + hh),
        Vec3::new(*cx - hw, front_y, *cy + hh),
    ];
    let mut corners_2d = Vec::with_capacity(4);
    for c in corners_3d {
        let Some(p) = camera.project_point(rect, c, 4000.0) else {
            return false;
        };
        corners_2d.push(p);
    }

    let handles = [
        (
            GizmoHandle::Move,
            c_pos,
            !op.lock_position,
        ),
        (
            GizmoHandle::North,
            camera
                .project_point(rect, Vec3::new(*cx, front_y, *cy + hh), 4000.0)
                .unwrap_or(c_pos),
            !op.lock_height && !op.lock_position,
        ),
        (
            GizmoHandle::South,
            camera
                .project_point(rect, Vec3::new(*cx, front_y, *cy - hh), 4000.0)
                .unwrap_or(c_pos),
            !op.lock_height && !op.lock_position,
        ),
        (
            GizmoHandle::East,
            camera
                .project_point(rect, Vec3::new(*cx + hw, front_y, *cy), 4000.0)
                .unwrap_or(c_pos),
            !op.lock_width && !op.lock_position,
        ),
        (
            GizmoHandle::West,
            camera
                .project_point(rect, Vec3::new(*cx - hw, front_y, *cy), 4000.0)
                .unwrap_or(c_pos),
            !op.lock_width && !op.lock_position,
        ),
    ];

    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    gizmo.hovered_handle = hover_pos.and_then(|hp| {
        handles
            .iter()
            .filter(|(_, _, enabled)| *enabled)
            .find(|(_, p, _)| p.distance(hp) <= 10.5)
            .map(|(h, _, _)| *h)
    });

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
        && let Some((h, _, _)) = handles
            .iter()
            .filter(|(_, _, enabled)| *enabled)
            .find(|(_, p, _)| p.distance(pos) <= 11.5)
        && let Some(wp) = ray_wall_plane_point(wall_w, wall_h, camera, rect, pos)
    {
        gizmo.active_drag = Some(match *h {
            GizmoHandle::Move => ActiveGizmoDrag::Move {
                id: sel_id.clone(),
                start_wall_pt: wp,
                orig_x: op.x_from_left,
                orig_y: op.y_from_bottom,
                orig_w: ow as f64,
                orig_h: oh as f64,
            },
            other => ActiveGizmoDrag::Resize {
                id: sel_id.clone(),
                handle: other,
                start_wall_pt: wp,
                orig_x: op.x_from_left,
                orig_y: op.y_from_bottom,
                orig_w: ow as f64,
                orig_h: oh as f64,
            },
        });
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(ref drag) = gizmo.active_drag
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        && let Some(wp) = ray_wall_plane_point(wall_w, wall_h, camera, rect, pos)
    {
        match drag {
            ActiveGizmoDrag::Move {
                id,
                start_wall_pt,
                orig_x,
                orig_y,
                orig_w,
                orig_h,
            } => {
                let dx = wp[0] - start_wall_pt[0];
                let dy = wp[1] - start_wall_pt[1];
                let mut nx = (orig_x + dx).round();
                let mut ny = (orig_y + dy).round();
                if ((nx + orig_w * 0.5) - wall_w * 0.5).abs() < 2.5 {
                    nx = wall_w * 0.5 - orig_w * 0.5;
                }
                if win_gen.is_door && ny.abs() < 4.0 {
                    ny = 0.0;
                }
                nx = nx.clamp(0.0, (wall_w - orig_w).max(0.0));
                ny = ny.clamp(0.0, (wall_h - orig_h).max(0.0));
                events.push(Viewport3DEvent::MoveOpening {
                    id: id.clone(),
                    x_from_left: nx,
                    y_from_bottom: ny,
                });
            }
            ActiveGizmoDrag::Resize {
                id,
                handle,
                start_wall_pt,
                orig_x,
                orig_y,
                orig_w,
                orig_h,
            } => {
                let dx = wp[0] - start_wall_pt[0];
                let dy = wp[1] - start_wall_pt[1];
                let mut nx = *orig_x;
                let mut ny = *orig_y;
                let mut nw = *orig_w;
                let mut nh = *orig_h;
                match handle {
                    GizmoHandle::East => {
                        nw = (*orig_w + dx).round().clamp(16.0, wall_w - *orig_x);
                    }
                    GizmoHandle::West => {
                        let right = *orig_x + *orig_w;
                        nx = (*orig_x + dx).round().clamp(0.0, right - 16.0);
                        nw = (right - nx).clamp(16.0, 260.0);
                    }
                    GizmoHandle::North => {
                        nh = (*orig_h + dy).round().clamp(16.0, wall_h - *orig_y);
                    }
                    GizmoHandle::South => {
                        let top = *orig_y + *orig_h;
                        ny = (*orig_y + dy).round().clamp(0.0, top - 16.0);
                        nh = (top - ny).clamp(16.0, 260.0);
                    }
                    GizmoHandle::Move => {}
                }
                events.push(Viewport3DEvent::ResizeOpening {
                    id: id.clone(),
                    x_from_left: nx,
                    y_from_bottom: ny,
                    width: nw.clamp(14.0, 260.0),
                    height: nh.clamp(14.0, 260.0),
                });
            }
        }
    }

    if !ui.input(|i| i.pointer.primary_down()) && gizmo.active_drag.is_some() {
        let label = match gizmo.active_drag.take() {
            Some(ActiveGizmoDrag::Move { .. }) => "Move opening in 3D",
            Some(ActiveGizmoDrag::Resize { .. }) => "Resize opening in 3D",
            None => "Edit in 3D",
        };
        events.push(Viewport3DEvent::InteractionFinished(label));
    }

    // Paint 3D bounding rectangle + gizmo handles
    let painter = ui.painter_at(rect);
    let gold = Color32::from_rgb(245, 198, 108);
    let accent = Color32::from_rgb(88, 190, 250);
    painter.add(egui::Shape::closed_line(
        corners_2d,
        Stroke::new(1.5, gold),
    ));

    for (h, p, enabled) in handles {
        if !enabled {
            continue;
        }
        let is_active = gizmo.hovered_handle == Some(h)
            || matches!(
                gizmo.active_drag,
                Some(ActiveGizmoDrag::Move { .. }) if h == GizmoHandle::Move
            )
            || matches!(
                gizmo.active_drag,
                Some(ActiveGizmoDrag::Resize { handle, .. }) if handle == h
            );
        let radius = if h == GizmoHandle::Move { 6.5 } else { 5.2 };
        let fill = if is_active {
            Color32::WHITE
        } else if h == GizmoHandle::Move {
            gold
        } else {
            accent
        };
        painter.circle_filled(p, radius, fill);
        painter.circle_stroke(p, radius, Stroke::new(1.4, Color32::from_rgb(16, 20, 28)));
    }

    if gizmo.active_drag.is_some() {
        let badge = Rect::from_min_size(
            Pos2::new(c_pos.x - 52.0, c_pos.y - 38.0),
            Vec2::new(104.0, 20.0),
        );
        painter.rect_filled(
            badge,
            CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(16, 20, 28, 230),
        );
        painter.text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            format!("{ow:.1} × {oh:.1} mm"),
            FontId::proportional(11.0),
            gold,
        );
    }

    gizmo.active_drag.is_some() || gizmo.hovered_handle.is_some()
}
