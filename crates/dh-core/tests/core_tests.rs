use dh_core::{
    ComponentDef, DisplayUnit, FlushTopMode, InstanceOverrides, Project, ProjectKind, ScalePreset,
    UndoHistory, WindowParams, WindowStyle,
};
use std::collections::HashSet;

#[test]
fn display_unit_conversions_and_presets_are_exact() {
    assert!((DisplayUnit::Mm.from_mm(50.8) - 50.8).abs() < 1e-9);
    assert!((DisplayUnit::Cm.from_mm(50.0) - 5.0).abs() < 1e-9);
    assert!((DisplayUnit::Inches.from_mm(25.4) - 1.0).abs() < 1e-9);
    assert!((DisplayUnit::Inches.to_mm(2.0) - 50.8).abs() < 1e-9);

    assert_eq!(DisplayUnit::Mm.format_value(48.0), "48.0 mm");
    assert_eq!(DisplayUnit::Cm.format_value(48.0), "4.80 cm");
    assert_eq!(DisplayUnit::Inches.format_value(25.4), "1.000 in");

    let in_presets = DisplayUnit::Inches.calibration_presets();
    assert!(
        in_presets
            .iter()
            .any(|(lbl, mm)| *lbl == "1 in" && (*mm - 25.4).abs() < 1e-6)
    );
}

#[test]
fn component_instancing_across_walls_with_overrides_and_make_unique() {
    let mut proj =
        Project::new_multi_wall("Townhouse", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    assert_eq!(proj.manifest.kind, ProjectKind::MultiWall);
    assert_eq!(proj.components.len(), 1);
    let comp_id = proj.components[0].id.clone();

    // Starter facade already has 2 linked instances of Parlor Georgian Window
    assert_eq!(proj.component_usage(&comp_id), (2, 1));

    // Add a second thinner wall (6mm) and place an instance on it
    let wall2_id = proj.add_wall("Side Gable", 280.0, 240.0, 6.0, 0.40);
    let op3_id = proj
        .place_component_instance(&wall2_id, &comp_id, Some("Side Parlor"), 80.0, 62.0)
        .unwrap();
    assert_eq!(proj.component_usage(&comp_id), (3, 2));

    // Verify Wall 2 instance kept 6mm wall thickness while Wall 1 stayed 9mm
    let wall2 = proj.walls.iter().find(|w| w.id == wall2_id).unwrap();
    let op3 = wall2.openings.iter().find(|o| o.id == op3_id).unwrap();
    assert_eq!(op3.resolve_params(&proj.components).wall_thickness, 6.0);

    let wall1 = &proj.walls[0];
    let op1 = &wall1.openings[0];
    assert_eq!(op1.resolve_params(&proj.components).wall_thickness, 9.0);

    // Edit the shared component -> both walls update live while keeping per-instance wall thickness!
    proj.component_mut(&comp_id).unwrap().params.bars_x = 4;
    proj.component_mut(&comp_id).unwrap().params.opening_width = 66.0;
    assert_eq!(
        proj.walls[0].openings[0]
            .resolve_params(&proj.components)
            .bars_x,
        4
    );
    assert_eq!(
        proj.walls[1].openings[0]
            .resolve_params(&proj.components)
            .opening_width,
        66.0
    );
    assert_eq!(
        proj.walls[1].openings[0]
            .resolve_params(&proj.components)
            .wall_thickness,
        6.0
    );

    // Make Wall 2 instance unique -> further edits to ComponentDef no longer affect it
    assert!(proj.make_opening_unique(&wall2_id, &op3_id));
    assert_eq!(proj.component_usage(&comp_id), (2, 1));
    proj.component_mut(&comp_id).unwrap().params.bars_x = 2;
    assert_eq!(
        proj.walls[1].openings[0]
            .resolve_params(&proj.components)
            .bars_x,
        4
    );
    assert_eq!(
        proj.walls[0].openings[0]
            .resolve_params(&proj.components)
            .bars_x,
        2
    );
}

#[test]
fn promote_unique_opening_and_door_mirror_hinge_override() {
    let mut proj = Project::new_multi_wall("Manor", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let wall_id = proj.walls[0].id.clone();

    // Promote the front door (originally unique) into a component
    let door_id = proj.walls[0].openings[1].id.clone();
    let door_comp_id = proj
        .promote_opening_to_component(&wall_id, &door_id, Some("Main Door"))
        .unwrap();
    assert_eq!(proj.component_usage(&door_comp_id), (1, 1));

    // Mirror across wall center -> single door flips hinge_right via instance override
    let mut sel = HashSet::new();
    sel.insert(door_id);
    let comps = proj.components.clone();
    proj.walls[0].mirror_openings_horizontally(&sel, &comps);
    assert!(
        proj.walls[0].openings[1]
            .resolve_params(&proj.components)
            .hinge_right
    );
    // Component definition itself remains left-hinged
    assert!(!proj.component(&door_comp_id).unwrap().params.hinge_right);

    // Per-instance flush override check
    let mut ov = InstanceOverrides {
        flush_top_mode: Some(FlushTopMode::HorizontalHead),
        flush_bottom_sill: Some(true),
        ..Default::default()
    };
    let mut p = WindowParams::default();
    ov.apply_to(&mut p);
    assert_eq!(p.flush_top_mode, FlushTopMode::HorizontalHead);
    assert!(p.flush_bottom_sill);
    ov.clear_flush_overrides();
    assert!(!ov.has_flush_override());
}

#[test]
fn single_piece_promotes_cleanly_to_multi_wall_and_undo_works() {
    let mut params = WindowParams::default();
    WindowStyle::ArchedGeorgian.apply_style_defaults(&mut params);
    let mut proj = Project::new_single_piece(
        "Stair Landing",
        "Arched Window",
        params,
        ScalePreset::Scale1_12,
        DisplayUnit::Mm,
    );
    assert_eq!(proj.manifest.kind, ProjectKind::SinglePiece);
    assert_eq!(proj.pieces.len(), 1);
    assert!(proj.walls.is_empty());

    let mut history = UndoHistory::new(proj.clone());
    let oid = proj.promote_single_piece_to_multi_wall().unwrap();
    history.commit("Promote to Wall Project", proj.clone());

    assert_eq!(proj.manifest.kind, ProjectKind::MultiWall);
    assert_eq!(proj.walls.len(), 1);
    assert_eq!(proj.walls[0].openings[0].id, oid);
    assert!(proj.pieces.is_empty());

    let (lbl, prev) = history.undo(proj.clone()).unwrap();
    assert_eq!(lbl, "Promote to Wall Project");
    assert_eq!(prev.manifest.kind, ProjectKind::SinglePiece);
    assert_eq!(prev.pieces.len(), 1);
}

#[test]
fn apply_style_only_preserves_opening_dimensions_and_wall_specs() {
    let mut target = WindowParams {
        opening_width: 73.5,
        opening_height: 99.0,
        wall_thickness: 12.0,
        acetate_thickness: 2.0,
        flush_bottom_sill: true,
        ..Default::default()
    };

    let mut tpl = WindowParams::default();
    WindowStyle::PalladianVenetian.apply_style_defaults(&mut tpl);
    tpl.wall_thickness = 6.0;
    tpl.acetate_thickness = 0.4;

    target.apply_style_only_from(&tpl);
    assert_eq!(target.style, WindowStyle::PalladianVenetian);
    assert!((target.opening_width - 73.5).abs() < 1e-6);
    assert!((target.opening_height - 99.0).abs() < 1e-6);
    assert!((target.wall_thickness - 12.0).abs() < 1e-6);
    assert!((target.acetate_thickness - 2.0).abs() < 1e-6);
    assert!(target.flush_bottom_sill);

    let comp = ComponentDef::new("Test", tpl);
    assert!(!comp.id.is_empty());
}
