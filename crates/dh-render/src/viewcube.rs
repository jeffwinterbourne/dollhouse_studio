use crate::camera::{CameraPreset, OrbitCamera};
use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use glam::{Mat3, Vec3};

struct CubeFace {
    label: &'static str,
    preset: CameraPreset,
    normal: Vec3,
    verts: [Vec3; 4],
}

const CUBE_FACES: [CubeFace; 6] = [
    CubeFace {
        label: "FRONT",
        preset: CameraPreset::Front,
        normal: Vec3::new(0.0, -1.0, 0.0),
        verts: [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ],
    },
    CubeFace {
        label: "BACK",
        preset: CameraPreset::Back,
        normal: Vec3::new(0.0, 1.0, 0.0),
        verts: [
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ],
    },
    CubeFace {
        label: "RIGHT",
        preset: CameraPreset::Right,
        normal: Vec3::new(1.0, 0.0, 0.0),
        verts: [
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
        ],
    },
    CubeFace {
        label: "LEFT",
        preset: CameraPreset::Left,
        normal: Vec3::new(-1.0, 0.0, 0.0),
        verts: [
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ],
    },
    CubeFace {
        label: "TOP",
        preset: CameraPreset::Top,
        normal: Vec3::new(0.0, 0.0, 1.0),
        verts: [
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ],
    },
    CubeFace {
        label: "BOT",
        preset: CameraPreset::Bottom,
        normal: Vec3::new(0.0, 0.0, -1.0),
        verts: [
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, -1.0),
        ],
    },
];

pub fn point_in_convex_quad(p: Pos2, q: &[Pos2; 4]) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let a = q[i];
        let b = q[(i + 1) % 4];
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        if cross.abs() < 1e-4 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if sign != cross.signum() {
            return false;
        }
    }
    true
}

/// Draws the interactive CAD ViewCube overlay in the top-right corner of `viewport_rect`.
/// Returns `true` if the user clicked or dragged on the ViewCube.
pub fn draw_viewcube(
    ui: &mut egui::Ui,
    viewport_rect: Rect,
    camera: &mut OrbitCamera,
    dark_mode: bool,
) -> bool {
    if viewport_rect.width() < 150.0 || viewport_rect.height() < 150.0 {
        return false;
    }

    let box_size = 82.0;
    let cube_rect = Rect::from_min_size(
        Pos2::new(viewport_rect.right() - box_size - 12.0, viewport_rect.top() + 12.0),
        Vec2::new(box_size, box_size),
    );
    let iso_btn_rect = Rect::from_min_size(
        Pos2::new(cube_rect.left() - 2.0, cube_rect.top() - 2.0),
        Vec2::new(20.0, 20.0),
    );

    let resp = ui.interact(
        cube_rect.expand(4.0),
        ui.scope_id().with("cad_viewcube"),
        Sense::click_and_drag(),
    );
    let iso_resp = ui.interact(
        iso_btn_rect,
        ui.scope_id().with("cad_viewcube_iso"),
        Sense::click(),
    );

    if iso_resp.clicked() {
        camera.animate_to_preset(CameraPreset::Isometric);
    }

    if resp.dragged_by(egui::PointerButton::Primary) {
        let delta = resp.drag_delta();
        camera.yaw -= delta.x * 0.012;
        camera.pitch = (camera.pitch + delta.y * 0.012).clamp(-1.50, 1.50);
    }

    let rot = Mat3::from_mat4(camera.view_matrix());
    let center = cube_rect.center() + Vec2::new(0.0, 4.0);
    let scale = 19.5f32;

    let proj_pt = |v: Vec3| -> Pos2 {
        let rv = rot * v;
        Pos2::new(center.x + rv.x * scale, center.y - rv.y * scale)
    };

    struct DrawFace {
        depth: f32,
        quad: [Pos2; 4],
        center: Pos2,
        label: &'static str,
        preset: CameraPreset,
    }

    let mut visible_faces = Vec::new();
    for face in &CUBE_FACES {
        let rn = rot * face.normal;
        if rn.z > 0.03 {
            let quad = [
                proj_pt(face.verts[0]),
                proj_pt(face.verts[1]),
                proj_pt(face.verts[2]),
                proj_pt(face.verts[3]),
            ];
            let face_center = Pos2::new(
                (quad[0].x + quad[1].x + quad[2].x + quad[3].x) * 0.25,
                (quad[0].y + quad[1].y + quad[2].y + quad[3].y) * 0.25,
            );
            visible_faces.push(DrawFace {
                depth: rn.z,
                quad,
                center: face_center,
                label: face.label,
                preset: face.preset,
            });
        }
    }
    visible_faces.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));

    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    let hovered_idx = hover_pos.and_then(|hp| {
        visible_faces
            .iter()
            .rposition(|f| point_in_convex_quad(hp, &f.quad))
    });

    if resp.clicked()
        && let Some(click_pos) = resp.interact_pointer_pos()
        && let Some(idx) = visible_faces
            .iter()
            .rposition(|f| point_in_convex_quad(click_pos, &f.quad))
    {
        camera.animate_to_preset(visible_faces[idx].preset);
    }

    let painter = ui.painter_at(viewport_rect);
    let pill_bg = if dark_mode {
        Color32::from_rgba_unmultiplied(20, 24, 33, 185)
    } else {
        Color32::from_rgba_unmultiplied(245, 248, 252, 205)
    };
    let pill_stroke = if dark_mode {
        Color32::from_rgb(66, 76, 98)
    } else {
        Color32::from_rgb(196, 205, 218)
    };
    painter.rect(
        cube_rect.expand(4.0),
        CornerRadius::same(8),
        pill_bg,
        Stroke::new(1.0, pill_stroke),
        egui::StrokeKind::Inside,
    );

    for (idx, f) in visible_faces.iter().enumerate() {
        let is_hov = hovered_idx == Some(idx);
        let shade = (f.depth * 0.25 + 0.72).clamp(0.65, 1.0);
        let fill = if is_hov {
            Color32::from_rgb(245, 198, 108)
        } else if dark_mode {
            Color32::from_rgb(
                (58.0 * shade) as u8,
                (70.0 * shade) as u8,
                (94.0 * shade) as u8,
            )
        } else {
            Color32::from_rgb(
                (226.0 * shade) as u8,
                (234.0 * shade) as u8,
                (244.0 * shade) as u8,
            )
        };
        let border = if dark_mode {
            Color32::from_rgb(120, 140, 175)
        } else {
            Color32::from_rgb(130, 145, 168)
        };
        let text_col = if is_hov {
            Color32::from_rgb(18, 22, 30)
        } else if dark_mode {
            Color32::from_rgb(236, 242, 250)
        } else {
            Color32::from_rgb(28, 36, 52)
        };

        painter.add(egui::Shape::convex_polygon(
            f.quad.to_vec(),
            fill,
            Stroke::new(1.1, border),
        ));
        if f.depth > 0.30 {
            painter.text(
                f.center,
                egui::Align2::CENTER_CENTER,
                f.label,
                FontId::proportional(8.8),
                text_col,
            );
        }
    }

    // Small Iso home corner pill
    let iso_hov = iso_resp.hovered();
    painter.rect(
        iso_btn_rect,
        CornerRadius::same(4),
        if iso_hov {
            Color32::from_rgb(245, 198, 108)
        } else if dark_mode {
            Color32::from_rgb(44, 52, 70)
        } else {
            Color32::from_rgb(225, 232, 242)
        },
        Stroke::new(1.0, pill_stroke),
        egui::StrokeKind::Inside,
    );
    painter.text(
        iso_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Iso",
        FontId::proportional(9.5),
        if iso_hov {
            Color32::from_rgb(18, 22, 30)
        } else if dark_mode {
            Color32::WHITE
        } else {
            Color32::from_rgb(30, 38, 52)
        },
    );

    resp.hovered() || resp.dragged() || iso_resp.hovered() || iso_resp.clicked()
}
