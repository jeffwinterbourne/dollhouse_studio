use crate::camera::OrbitCamera;
use dh_geometry::BuiltProjectWall;
use eframe::egui;
use glam::Vec3;

/// Ray-AABB intersection returning distance `t >= 0` along normalized `dir`.
pub fn ray_aabb_intersect(orig: Vec3, dir: Vec3, bmin: Vec3, bmax: Vec3) -> Option<f32> {
    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    for k in 0..3 {
        if dir[k].abs() < 1e-7 {
            if orig[k] < bmin[k] || orig[k] > bmax[k] {
                return None;
            }
        } else {
            let inv = 1.0 / dir[k];
            let mut t1 = (bmin[k] - orig[k]) * inv;
            let mut t2 = (bmax[k] - orig[k]) * inv;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    Some(tmin)
}

/// Raycasts onto openings in `built_wall` and returns the closest hit opening ID.
pub fn pick_wall_opening(
    built_wall: &BuiltProjectWall,
    camera: &OrbitCamera,
    rect: egui::Rect,
    pos: egui::Pos2,
) -> Option<String> {
    let (ray_orig, ray_dir) = camera.screen_ray(rect, pos, 4000.0)?;
    let wall_t = built_wall.wall.wall_thickness as f32;
    let half_y = wall_t * 0.5 + 8.0;

    let mut best_t = f32::INFINITY;
    let mut best_id: Option<String> = None;

    for (id, cx, cy, win_gen) in &built_wall.placed_openings {
        let hw = win_gen.params.opening_width as f32 * 0.5 + win_gen.params.trim_width as f32;
        let hh = win_gen.params.opening_height as f32 * 0.5 + win_gen.params.trim_width as f32;
        let bmin = Vec3::new(*cx - hw, -half_y, *cy - hh);
        let bmax = Vec3::new(*cx + hw, half_y, *cy + hh);
        if let Some(t) = ray_aabb_intersect(ray_orig, ray_dir, bmin, bmax)
            && t < best_t
        {
            best_t = t;
            best_id = Some(id.clone());
        }
    }
    best_id
}

/// Intersects the camera ray through `pos` with the upright wall mid-plane (`Y = 0` in world)
/// and returns wall coordinates `[x_from_left_mm, y_from_bottom_mm]`.
pub fn ray_wall_plane_point(
    wall_w: f64,
    wall_h: f64,
    camera: &OrbitCamera,
    rect: egui::Rect,
    pos: egui::Pos2,
) -> Option<[f64; 2]> {
    let (ray_orig, ray_dir) = camera.screen_ray(rect, pos, 4000.0)?;
    if ray_dir.y.abs() < 1e-5 {
        return None;
    }
    let t = -ray_orig.y / ray_dir.y;
    if t <= 0.0 {
        return None;
    }
    let hit = ray_orig + ray_dir * t;
    Some([
        hit.x as f64 + wall_w * 0.5,
        hit.z as f64 + wall_h * 0.5,
    ])
}
