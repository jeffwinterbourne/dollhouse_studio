use dh_app::app::DollhouseStudioApp;
use dh_app::wizards::PhotoCalibrationLine;
use dh_app::wizards::photo_wizard::compute_calibrated_size_mm;
use dh_core::{DisplayUnit, Project, ScalePreset, WallPhotoConfig, WindowParams, WindowStyle};
use eframe::egui;

fn run_frame(ctx: &egui::Context, app: &mut DollhouseStudioApp) {
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        app.draw_ui(ui);
    });
    out.textures_delta.clear();
}

#[test]
fn welcome_hub_renders_cleanly_in_light_and_dark_themes() {
    let ctx = egui::Context::default();
    let mut app = DollhouseStudioApp::new_headless();
    assert!(app.show_hub_overlay);

    for mode in [dh_project::ThemeMode::Light, dh_project::ThemeMode::Dark] {
        app.prefs.theme = mode;
        for _ in 0..4 {
            run_frame(&ctx, &mut app);
        }
    }

    app.show_hub_overlay = false;
    for _ in 0..4 {
        run_frame(&ctx, &mut app);
    }
    assert!(app.last_explorer_width > 200.0 && app.last_inspector_width > 250.0);
}

#[test]
fn onboarding_tour_renders_all_four_steps_cleanly() {
    let ctx = egui::Context::default();
    let proj = Project::new_multi_wall("Tour Test", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let mut app = DollhouseStudioApp::new_headless_with_project(proj, None);
    app.onboarding.open_tour();
    for step in 0..4 {
        app.onboarding.step = step;
        run_frame(&ctx, &mut app);
    }
    assert!(app.onboarding.open);
}

#[test]
fn multi_wall_and_single_piece_side_panels_remain_stable_over_20_frames() {
    let ctx = egui::Context::default();

    let proj =
        Project::new_multi_wall("Test Facade", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let mut app = DollhouseStudioApp::new_headless_with_project(proj, None);
    assert!(!app.show_hub_overlay);
    let mut exp_widths = Vec::new();
    let mut insp_widths = Vec::new();
    for i in 0..20 {
        app.canvas2d.show_architrave_outlines = i < 14;
        run_frame(&ctx, &mut app);
        exp_widths.push(app.last_explorer_width);
        insp_widths.push(app.last_inspector_width);
    }
    let exp_first = exp_widths[0];
    let insp_first = insp_widths[0];
    assert!(exp_first > 200.0 && insp_first > 250.0);
    assert!(
        exp_widths.iter().all(|w| (*w - exp_first).abs() < 1.0),
        "Explorer panel width drifted across frames: {exp_widths:?}"
    );
    assert!(
        insp_widths.iter().all(|w| (*w - insp_first).abs() < 1.0),
        "Inspector panel width drifted across frames: {insp_widths:?}"
    );

    let single = Project::new_single_piece(
        "Single Window",
        "Parlor",
        WindowParams::default(),
        ScalePreset::Scale1_12,
        DisplayUnit::Inches,
    );
    let mut app2 = DollhouseStudioApp::new_headless_with_project(single, None);
    for _ in 0..10 {
        run_frame(&ctx, &mut app2);
    }
}

#[test]
fn component_instancing_propagates_and_make_unique_unlinks_cleanly() {
    let mut proj = Project::new_multi_wall(
        "Component Sync Test",
        true,
        ScalePreset::Scale1_12,
        DisplayUnit::Mm,
    );
    let cid = proj.components[0].id.clone();
    let linked_ids: Vec<String> = proj.walls[0]
        .openings
        .iter()
        .filter(|o| o.component_id() == Some(cid.as_str()))
        .map(|o| o.id.clone())
        .collect();
    assert_eq!(linked_ids.len(), 2);

    proj.component_mut(&cid).unwrap().params.opening_width = 72.0;
    for op in &proj.walls[0].openings {
        if linked_ids.contains(&op.id) {
            assert!((op.resolve_params(&proj.components).opening_width - 72.0).abs() < 1e-6);
        }
    }

    let wid = proj.walls[0].id.clone();
    assert!(proj.make_opening_unique(&wid, &linked_ids[0]));
    proj.component_mut(&cid).unwrap().params.opening_width = 80.0;
    let first_w = proj.walls[0]
        .openings
        .iter()
        .find(|o| o.id == linked_ids[0])
        .unwrap()
        .resolve_params(&proj.components)
        .opening_width;
    let second_w = proj.walls[0]
        .openings
        .iter()
        .find(|o| o.id == linked_ids[1])
        .unwrap()
        .resolve_params(&proj.components)
        .opening_width;
    assert!((first_w - 72.0).abs() < 1e-6);
    assert!((second_w - 80.0).abs() < 1e-6);

    let _ = WindowStyle::GeorgianGrid;
}

#[test]
fn photo_two_point_calibration_math_scales_accurately() {
    let mut photo = dh_core::WallPhotoConfig {
        asset_path: Some("assets/photos/wall.jpg".into()),
        offset_x_mm: 20.0,
        offset_y_mm: 10.0,
        width_mm: 200.0,
        height_mm: 150.0,
        ..Default::default()
    };
    // Measured 89mm on photo represents a 178mm dollhouse door (2x scale) anchored at (0, 0).
    photo.apply_two_point_scale(89.0, 178.0, [0.0, 0.0]);
    assert!((photo.width_mm - 400.0).abs() < 1e-4);
    assert!((photo.height_mm - 300.0).abs() < 1e-4);
    assert!((photo.offset_x_mm - 40.0).abs() < 1e-4);
    assert!((photo.offset_y_mm - 20.0).abs() < 1e-4);
}

#[test]
fn photo_wizard_multi_line_and_independent_hv_calibration_accuracy() {
    let cfg = WallPhotoConfig {
        asset_path: Some("assets/photos/wall.jpg".into()),
        pixel_size: [1600, 1200],
        rotation_deg: 90.0,
        ..Default::default()
    };
    assert!((cfg.processed_aspect_ratio() - 0.75).abs() < 1e-4);

    let lines = vec![
        PhotoCalibrationLine {
            id: 1,
            label: "Door H".into(),
            uv_a: [0.5, 0.2],
            uv_b: [0.5, 0.9], // 0.7 of photo height
            target_mm: 176.4, // non-round real length
        },
        PhotoCalibrationLine {
            id: 2,
            label: "Door W".into(),
            uv_a: [0.3, 0.5],
            uv_b: [0.7, 0.5], // 0.4 of photo width
            target_mm: 83.25, // non-round real length with slightly different H/V stretch
        },
    ];

    let (w, h, residuals) = compute_calibrated_size_mm(&cfg, 380.0, 240.0, &lines, true);
    assert!((h * 0.7 - 176.4).abs() < 0.05);
    assert!((w * 0.4 - 83.25).abs() < 0.05);
    assert!((residuals[0].1 - 176.4).abs() < 0.05);
    assert!((residuals[1].1 - 83.25).abs() < 0.05);
}
