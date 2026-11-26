use dh_core::{DisplayUnit, Project, ScalePreset};
use dh_geometry::WallMeshCache;
use dh_render::{
    CameraPreset, OrbitCamera, pick_wall_opening, ray_aabb_intersect, ray_wall_plane_point,
    viewcube::point_in_convex_quad,
};
use eframe::egui::{Pos2, Rect};
use glam::Vec3;

#[test]
fn orbit_camera_preset_animation_converges_cleanly() {
    let mut cam = OrbitCamera::default();
    cam.animate_to_preset(CameraPreset::Front);
    for _ in 0..30 {
        cam.step_animation(0.016);
    }
    let (fy, fp) = OrbitCamera::preset_angles(CameraPreset::Front);
    assert!((cam.yaw - fy).abs() < 0.01);
    assert!((cam.pitch - fp).abs() < 0.01);
}

#[test]
fn ray_picking_and_wall_plane_intersection_match_opening_coordinates() {
    let proj = Project::new_multi_wall("Test", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let mut cache = WallMeshCache::default();
    let built = cache.build_wall(&proj.walls[0], &proj.components);

    let mut cam = OrbitCamera::default();
    cam.reset_for_project_wall(built.wall.width as f32, built.wall.height as f32);
    cam.set_preset(CameraPreset::Front);

    let rect = Rect::from_min_size(Pos2::ZERO, eframe::egui::Vec2::new(800.0, 600.0));
    // Project the center of the front door (opening 1, centered near bottom-middle)
    let (door_id, cx, cy, _) = &built.placed_openings[1];
    let screen_pt = cam
        .project_point(rect, Vec3::new(*cx, 0.0, *cy), 4000.0)
        .unwrap();

    let hit = pick_wall_opening(&built, &cam, rect, screen_pt);
    assert_eq!(hit.as_deref(), Some(door_id.as_str()));

    let wall_pt =
        ray_wall_plane_point(built.wall.width, built.wall.height, &cam, rect, screen_pt).unwrap();
    let expected_wx = *cx as f64 + built.wall.width * 0.5;
    let expected_wy = *cy as f64 + built.wall.height * 0.5;
    assert!((wall_pt[0] - expected_wx).abs() < 1.0);
    assert!((wall_pt[1] - expected_wy).abs() < 1.0);
}

#[test]
fn ray_aabb_and_viewcube_quad_hit_tests() {
    let t = ray_aabb_intersect(
        Vec3::new(0.0, -50.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(-5.0, -5.0, -5.0),
        Vec3::new(5.0, 5.0, 5.0),
    );
    assert!(t.is_some_and(|d| (d - 45.0).abs() < 1e-4));

    let quad = [
        Pos2::new(10.0, 10.0),
        Pos2::new(30.0, 10.0),
        Pos2::new(30.0, 30.0),
        Pos2::new(10.0, 30.0),
    ];
    assert!(point_in_convex_quad(Pos2::new(20.0, 20.0), &quad));
    assert!(!point_in_convex_quad(Pos2::new(5.0, 20.0), &quad));
}
