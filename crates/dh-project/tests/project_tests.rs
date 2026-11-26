use dh_core::{
    DisplayUnit, PieceTemplate, Project, ProjectKind, ScalePreset, WallPhotoConfig, WindowParams,
    WindowStyle,
};
use dh_project::{
    ProjectStore, delete_personal_template, export_template_file, import_template_file,
    import_wall_photo, list_personal_templates, process_photo_rgba, resolve_asset_path,
    save_personal_template, starter_templates,
};
use std::fs;
use std::path::PathBuf;

fn temp_test_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "dh_studio_test_{}_{}_{}",
        tag,
        std::process::id(),
        dh_core::now_epoch()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn single_piece_and_multi_wall_folder_projects_roundtrip_and_prune() {
    let root = temp_test_dir("roundtrip");

    // 1. SinglePiece project
    let single_dir = root.join("single_window");
    let mut sp_params = WindowParams::default();
    WindowStyle::ArchedGeorgian.apply_style_defaults(&mut sp_params);
    let mut single_proj = Project::new_single_piece(
        "Attic Window",
        "Attic Arch",
        sp_params,
        ScalePreset::Scale1_12,
        DisplayUnit::Inches,
    );
    ProjectStore::create_in_dir(&single_dir, &mut single_proj).unwrap();
    assert!(ProjectStore::is_project_dir(&single_dir));

    let loaded_single = ProjectStore::open_dir(&single_dir).unwrap();
    assert!(!loaded_single.has_crash_recovery);
    assert_eq!(loaded_single.project.manifest.kind, ProjectKind::SinglePiece);
    assert_eq!(loaded_single.project.pieces.len(), 1);
    assert_eq!(
        loaded_single.project.manifest.display_unit,
        DisplayUnit::Inches
    );

    // 2. MultiWall project with components, wall, project templates & orphan cleanup
    let house_dir = root.join("georgian_house");
    let mut house = Project::new_multi_wall(
        "Georgian Townhouse",
        true,
        ScalePreset::Scale1_12,
        DisplayUnit::Mm,
    );
    let w2 = house.add_wall("Side Wall", 260.0, 240.0, 6.0, 0.40);
    let cid = house.components[0].id.clone();
    house
        .place_component_instance(&w2, &cid, Some("Side Parlor"), 50.0, 62.0)
        .unwrap();
    house.project_templates.push(PieceTemplate::from_params(
        "House Parlor",
        "Saved inside project",
        vec!["parlor".into()],
        WindowParams::default(),
    ));
    ProjectStore::create_in_dir(&house_dir, &mut house).unwrap();

    // Remove Side Wall and verify its file is pruned on next save
    house.walls.retain(|w| w.id != w2);
    ProjectStore::save(&house_dir, &mut house).unwrap();

    let wall_files_on_disk: Vec<_> = fs::read_dir(house_dir.join("walls"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(wall_files_on_disk.len(), 1);

    let loaded_house = ProjectStore::open_dir(&house_dir).unwrap();
    assert_eq!(loaded_house.project.walls.len(), 1);
    assert_eq!(loaded_house.project.components.len(), 1);
    assert_eq!(loaded_house.project.project_templates.len(), 1);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn crash_recovery_autosave_detects_loads_and_clears_on_save() {
    let dir = temp_test_dir("autosave");
    let mut proj =
        Project::new_multi_wall("Recovery House", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    ProjectStore::create_in_dir(&dir, &mut proj).unwrap();

    // Simulate unsaved edit flushed to autosave
    proj.manifest.name = "Recovered Name".to_string();
    ProjectStore::write_autosave(&dir, &proj).unwrap();

    let loaded = ProjectStore::open_dir(&dir).unwrap();
    assert!(loaded.has_crash_recovery);
    assert_eq!(loaded.project.manifest.name, "Recovery House");

    let (_ts, recovered) = ProjectStore::load_autosave(&dir).unwrap();
    assert_eq!(recovered.manifest.name, "Recovered Name");

    // Explicit save clears autosave
    let mut rec_mut = recovered;
    ProjectStore::save(&dir, &mut rec_mut).unwrap();
    let reloaded = ProjectStore::open_dir(&dir).unwrap();
    assert!(!reloaded.has_crash_recovery);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn asset_copy_uses_relative_paths_and_blocks_traversal() {
    let dir = temp_test_dir("assets");
    let src_dir = dir.join("external");
    let proj_dir = dir.join("proj");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&proj_dir).unwrap();

    // Write a tiny valid 2x2 PNG using the `image` crate
    let img_path = src_dir.join("My Facade.png");
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 180, 150, 255]));
    img.save(&img_path).unwrap();

    let rel = import_wall_photo(&proj_dir, &img_path).unwrap();
    assert!(rel.starts_with("assets/photos/"));
    assert!(resolve_asset_path(&proj_dir, &rel).unwrap().is_file());

    // Importing same filename again yields collision-safe suffix
    let rel2 = import_wall_photo(&proj_dir, &img_path).unwrap();
    assert_ne!(rel, rel2);
    assert!(resolve_asset_path(&proj_dir, &rel2).unwrap().is_file());

    // Traversal must be rejected
    assert!(resolve_asset_path(&proj_dir, "../secret.png").is_err());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn personal_templates_crud_and_share_file_roundtrip() {
    let dir = temp_test_dir("templates");
    let starters = starter_templates();
    assert!(starters.len() >= 4);

    for tpl in &starters {
        save_personal_template(&dir, tpl).unwrap();
    }
    let listed = list_personal_templates(&dir);
    assert_eq!(listed.len(), starters.len());

    let share_path = dir.join("shared.dhtemplate.json");
    export_template_file(&listed[0], &share_path).unwrap();
    let imported = import_template_file(&share_path).unwrap();
    assert_eq!(imported.name, listed[0].name);
    assert_ne!(imported.id, listed[0].id);

    delete_personal_template(&dir, &listed[0].id).unwrap();
    assert_eq!(list_personal_templates(&dir).len(), starters.len() - 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wall_photo_crop_rotate_homography_and_calibration_work_accurately() {
    let sw = 120u32;
    let sh = 80u32;
    let mut rgba = vec![0u8; sw as usize * sh as usize * 4];
    for y in 0..sh {
        for x in 0..sw {
            let idx = (y as usize * sw as usize + x as usize) * 4;
            rgba[idx] = ((x * 255) / sw.max(1)) as u8;
            rgba[idx + 1] = ((y * 255) / sh.max(1)) as u8;
            rgba[idx + 2] = 180;
            rgba[idx + 3] = 255;
        }
    }

    let mut cfg = WallPhotoConfig {
        asset_path: Some("assets/photos/facade.png".to_string()),
        pixel_size: [1600, 1200],
        rotation_deg: 90.0,
        ..Default::default()
    };
    let (rw, rh, r_buf) = process_photo_rgba(sw, sh, &rgba, &cfg, 1400);
    assert_eq!(rw, sh);
    assert_eq!(rh, sw);
    assert_eq!(r_buf.len(), rw as usize * rh as usize * 4);

    let h0 = cfg.transform_hash();
    cfg.perspective_quad = Some([[0.15, 0.10], [0.85, 0.15], [0.90, 0.90], [0.10, 0.85]]);
    assert_ne!(cfg.transform_hash(), h0);

    let (qw, qh, q_buf) = process_photo_rgba(sw, sh, &rgba, &cfg, 480);
    assert!(qw > 16 && qh > 16);
    assert_eq!(q_buf.len(), qw as usize * qh as usize * 4);

    cfg.reset_edits();
    assert!(!cfg.has_edits());
    cfg.fit_to_wall(380.0, 240.0);
    assert!((cfg.height_mm - 240.0).abs() < 1e-6);
    assert!((cfg.width_mm - 320.0).abs() < 1e-6);

    let anchor = [90.0, 50.0];
    cfg.apply_two_point_scale(50.0, 100.0, anchor);
    assert!((cfg.width_mm - 640.0).abs() < 1e-6);
    assert!((cfg.height_mm - 480.0).abs() < 1e-6);
}
