use dh_core::{
    ArchitraveStyle, ComponentDef, DisplayUnit, DoorHandleStyle, FlushTopMode, GrooveStyle,
    HeaderStyle, InstanceOverrides, Project, ProjectWall, ScalePreset, ShutterStyle, WallOpening,
    WindowOpeningMode, WindowParams, WindowStyle,
};
use dh_geometry::{
    BuilderRegistry, HingeFace, WallMeshCache, build_cone_knuckle_block, build_window,
    validate_generated_piece,
};

#[test]
fn all_styles_are_watertight_manifold_with_zero_overhangs() {
    let registry = BuilderRegistry::default();
    for style in WindowStyle::ALL {
        let mut params = WindowParams::default();
        style.apply_style_defaults(&mut params);
        let win = registry.build_window_or_door(&params);

        let ext_diag = win
            .exterior_half
            .verify_printable_manifold()
            .unwrap_or_else(|e| panic!("Exterior half failed for {:?}: {e}", style));
        assert_eq!(ext_diag.overhang_triangles, 0);
        assert!(ext_diag.min_z.abs() < 1e-4);

        let int_diag = win
            .interior_half
            .verify_printable_manifold()
            .unwrap_or_else(|e| panic!("Interior half failed for {:?}: {e}", style));
        assert_eq!(int_diag.overhang_triangles, 0);
        assert!(int_diag.min_z.abs() < 1e-4);

        let plate_diag = win
            .print_plate
            .verify_printable_manifold()
            .unwrap_or_else(|e| panic!("Print plate failed for {:?}: {e}", style));
        assert_eq!(plate_diag.overhang_triangles, 0);
        assert!(validate_generated_piece(&win).is_valid);
    }
}

#[test]
fn all_scale_presets_and_sill_variants_are_printable() {
    for preset in ScalePreset::ALL {
        for style in [
            WindowStyle::GeorgianGrid,
            WindowStyle::ArchedGeorgian,
            WindowStyle::GothicPointed,
            WindowStyle::CircularBullsEye,
            WindowStyle::CottageLattice,
        ] {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.apply_scale_preset(preset);
            params.sill_enabled = false;
            let win = build_window(&params);
            assert!(
                win.printable_ok,
                "Failed for {:?} at {:?}: ext={}, int={}",
                style, preset, win.ext_diag, win.int_diag
            );
        }
    }
}

#[test]
fn extreme_parameter_ranges_remain_watertight_and_overhang_free() {
    let cases = [
        (WindowStyle::GeorgianGrid, 22.0, 26.0, 3.2, 1, 1),
        (WindowStyle::GeorgianGrid, 110.0, 140.0, 20.0, 5, 6),
        (WindowStyle::ArchedGeorgian, 90.0, 50.0, 9.0, 3, 1),
        (WindowStyle::GothicPointed, 32.0, 120.0, 12.0, 1, 2),
        (WindowStyle::CircularBullsEye, 70.0, 48.0, 9.0, 0, 0),
        (WindowStyle::CottageLattice, 80.0, 100.0, 9.0, 0, 0),
    ];

    for (style, w, h, wall, bx, by) in cases {
        let mut params = WindowParams::default();
        style.apply_style_defaults(&mut params);
        params.opening_width = w;
        params.opening_height = h;
        params.wall_thickness = wall;
        params.bars_x = bx;
        params.bars_y = by;

        let win = build_window(&params);
        assert!(
            win.printable_ok,
            "Stress case failed for {:?} ({}x{}x{}): ext={}, int={}",
            style, w, h, wall, win.ext_diag, win.int_diag
        );
    }
}

#[test]
fn all_architrave_chamfer_styles_and_thick_5mm_acetate_are_printable() {
    for astyle in ArchitraveStyle::ALL {
        for style in WindowStyle::ALL {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.architrave_style = astyle;
            params.chamfer_size = 1.1;
            params.chamfer_interior = true;
            params.acetate_thickness = 5.0;
            params.wall_thickness = 9.0;

            let win = build_window(&params);
            assert!(
                win.printable_ok,
                "Failed for {:?} + {:?} with 5mm acetate: ext={}, int={}",
                style, astyle, win.ext_diag, win.int_diag
            );
        }
    }

    let thin = WindowParams {
        wall_thickness: 4.0,
        acetate_thickness: 5.0,
        architrave_style: ArchitraveStyle::ChamferedStepped,
        ..Default::default()
    };
    let win = build_window(&thin);
    assert!(win.printable_ok);
    assert!(win.specs.effective_wall_thickness >= 7.0);
    assert!(win.specs.index_engagement >= 0.4);
}

#[test]
fn thick_glazing_above_5mm_remains_watertight_and_overhang_free() {
    for wall_t in [9.0, 14.0] {
        for style in [
            WindowStyle::GeorgianGrid,
            WindowStyle::ArchedGeorgian,
            WindowStyle::DoorGlazedFrench,
        ] {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.wall_thickness = wall_t;
            params.acetate_thickness = 8.0;

            let win = build_window(&params);
            assert!(
                win.printable_ok,
                "Thick glazing 8mm failed for {:?} (wall_t={wall_t}): ext={}, int={}",
                style, win.ext_diag, win.int_diag
            );
            assert!((win.params.acetate_thickness - 8.0).abs() < 1e-6);
            assert!(win.specs.effective_wall_thickness >= 10.2);
        }
    }
}

#[test]
fn elaborate_bar_chamfers_and_v_grooves_are_watertight_and_overhang_free() {
    for groove in GrooveStyle::ALL {
        for style in WindowStyle::ALL {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.architrave_style = ArchitraveStyle::ChamferedStepped;
            params.chamfer_size = 0.85;
            params.chamfer_bars = true;
            params.bar_chamfer_size = 0.35;
            params.groove_style = groove;
            params.groove_width = 0.85;

            let win = build_window(&params);
            assert!(
                win.printable_ok,
                "Moulding test failed for {:?} + {:?}: ext={}, int={}",
                style, groove, win.ext_diag, win.int_diag
            );
        }
    }
}

#[test]
fn hinged_doors_double_doors_and_separate_handle_are_watertight_and_overhang_free() {
    for style in WindowStyle::ALL_DOORS {
        for &(double_doors, hinge_right) in &[(false, false), (false, true), (true, false)] {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.double_doors = double_doors;
            params.hinge_right = hinge_right;
            params.groove_style = GrooveStyle::DoubleVGroove;
            params.chamfer_bars = true;

            let win = build_window(&params);
            assert!(win.is_door);
            assert!(win.printable_ok);
            assert_eq!(win.door_leaves.len(), if double_doors { 2 } else { 1 });

            let handle_diag = win.door_handle.verify_printable_manifold().unwrap();
            assert_eq!(handle_diag.overhang_triangles, 0);
            assert!(handle_diag.min_z.abs() < 1e-4);
        }
    }
}

#[test]
fn project_wall_with_component_instances_builds_watertight_punched_wall() {
    let mut proj = Project::new_multi_wall("Test", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let comp_id = proj.components[0].id.clone();

    let mut arch_p = WindowParams::default();
    WindowStyle::ArchedGeorgian.apply_style_defaults(&mut arch_p);
    let mut round_p = WindowParams::default();
    WindowStyle::CircularBullsEye.apply_style_defaults(&mut round_p);

    proj.walls[0]
        .openings
        .push(WallOpening::new_unique("Upper Arch", 40.0, 145.0, arch_p));
    proj.walls[0]
        .openings
        .push(WallOpening::new_unique("Attic Eye", 280.0, 160.0, round_p));
    proj.walls[0].apply_common_settings_to_openings();

    let mut cache = WallMeshCache::default();
    let built1 = cache.build_wall(&proj.walls[0], &proj.components);
    let diag = built1.wall_mesh.verify_printable_manifold().unwrap();
    assert!(diag.is_watertight);
    assert_eq!(built1.placed_openings.len(), 5);
    for (_, _, _, win_gen) in &built1.placed_openings {
        assert!(win_gen.printable_ok);
    }

    // Mutating the shared ComponentDef invalidates and updates both linked instances on rebuild!
    proj.component_mut(&comp_id).unwrap().params.opening_width = 64.0;
    let built2 = cache.build_wall(&proj.walls[0], &proj.components);
    assert!((built2.placed_openings[0].3.params.opening_width - 64.0).abs() < 1e-6);
    assert!((built2.placed_openings[2].3.params.opening_width - 64.0).abs() < 1e-6);

    let wall = ProjectWall::new_default("Extra");
    let comp = ComponentDef::new("Extra Comp", WindowParams::default());
    let mut w = wall;
    w.openings.push(WallOpening::new_instance(
        "Inst",
        40.0,
        60.0,
        comp.id.clone(),
        InstanceOverrides::default(),
    ));
    let built_extra = cache.build_wall(&w, &[comp]);
    assert!(built_extra.wall_mesh.verify_printable_manifold().is_ok());
}

#[test]
fn exterior_addons_shutters_headers_and_flower_boxes_are_watertight_zero_overhang() {
    assert_eq!(WindowStyle::ALL.len(), 16);
    for astyle in [
        ArchitraveStyle::Chamfered,
        ArchitraveStyle::ChamferedStepped,
    ] {
        for shutter in ShutterStyle::ALL {
            for header in HeaderStyle::ALL {
                for flower_box in [false, true] {
                    for style in [
                        WindowStyle::GeorgianGrid,
                        WindowStyle::PalladianVenetian,
                        WindowStyle::ArchedGeorgian,
                        WindowStyle::GothicPointed,
                        WindowStyle::HalfRoundLunette,
                        WindowStyle::CircularBullsEye,
                        WindowStyle::DoorFanlightGeorgian,
                    ] {
                        let mut params = WindowParams::default();
                        style.apply_style_defaults(&mut params);
                        params.architrave_style = astyle;
                        params.shutter_style = shutter;
                        params.header_style = header;
                        params.flower_box_enabled = flower_box;
                        params.groove_style = GrooveStyle::SingleVGroove;

                        let win = build_window(&params);
                        assert!(
                            win.printable_ok,
                            "Addon test failed for {:?} + {:?} + {:?} + {:?} + fb={}: ext={}, int={}",
                            style, astyle, shutter, header, flower_box, win.ext_diag, win.int_diag
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn existing_wall_flush_sill_head_and_inset_modes_are_watertight_zero_overhang() {
    let mut rect_params = WindowParams::default();
    WindowStyle::GeorgianGrid.apply_style_defaults(&mut rect_params);
    rect_params.architrave_style = ArchitraveStyle::ChamferedStepped;
    rect_params.groove_style = GrooveStyle::SingleVGroove;
    rect_params.sill_enabled = true;
    rect_params.flower_box_enabled = true;
    rect_params.shutter_style = ShutterStyle::Louvred;
    rect_params.header_style = HeaderStyle::TriangularPediment;

    let mut sill_head = rect_params.clone();
    sill_head.flush_bottom_sill = true;
    sill_head.flush_top_mode = FlushTopMode::HorizontalHead;
    sill_head.flush_trim_setback = 2.5;
    let w_sh = build_window(&sill_head);
    assert!(w_sh.printable_ok, "Sill+Head failed: {}", w_sh.ext_diag);
    let (_, sh_max) = w_sh.exterior_half.bounds();
    assert!(sh_max[1] <= (sill_head.opening_height * 0.5) as f32 + 0.05);

    for style in [
        WindowStyle::ArchedGeorgian,
        WindowStyle::GothicPointed,
        WindowStyle::HalfRoundLunette,
        WindowStyle::CircularBullsEye,
        WindowStyle::DoorFanlightGeorgian,
        WindowStyle::DoorArchedHeritage,
    ] {
        for top_mode in FlushTopMode::ALL {
            for all_jambs in [false, true] {
                let mut p = WindowParams::default();
                style.apply_style_defaults(&mut p);
                p.architrave_style = ArchitraveStyle::ChamferedStepped;
                p.groove_style = GrooveStyle::DoubleVGroove;
                p.flush_top_mode = *top_mode;
                p.flush_bottom_sill = true;
                p.flush_left_jamb = all_jambs;
                p.flush_right_jamb = all_jambs;
                p.flush_trim_setback = if all_jambs { 0.0 } else { 2.0 };

                let win = build_window(&p);
                assert!(win.printable_ok);
            }
        }
    }
}

#[test]
fn door_handles_letterbox_and_cat_flap_are_watertight_zero_overhang() {
    for style in WindowStyle::ALL_DOORS {
        for &handle in DoorHandleStyle::ALL {
            for (letterbox, cat_flap, double_doors) in [
                (false, false, false),
                (true, true, false),
                (true, false, true),
                (false, true, true),
            ] {
                let mut params = WindowParams::default();
                style.apply_style_defaults(&mut params);
                params.door_handle_style = handle;
                params.letterbox_enabled = letterbox;
                params.cat_flap_enabled = cat_flap;
                params.double_doors = double_doors;

                let win = build_window(&params);
                assert!(win.printable_ok);
                if !win.door_handle.triangles.is_empty() {
                    let hd = win.door_handle.verify_printable_manifold().unwrap();
                    assert_eq!(hd.overhang_triangles, 0);
                }
            }
        }
    }
}

#[test]
fn opening_windows_print_in_place_cone_hinges_are_watertight_zero_overhang() {
    for face in [
        HingeFace::PosX,
        HingeFace::NegX,
        HingeFace::PosY,
        HingeFace::NegY,
    ] {
        for is_socket in [false, true] {
            let block = build_cone_knuckle_block(
                -2.0, 2.0, -2.0, 2.0, 0.0, 3.6, face, 1.4, 1.1, is_socket, 16,
            );
            let diag = block.verify_printable_manifold().unwrap();
            assert!(diag.is_watertight);
            assert_eq!(diag.overhang_triangles, 0);
        }
    }

    for style in [
        WindowStyle::GeorgianGrid,
        WindowStyle::SashSplit,
        WindowStyle::CasementSplit,
        WindowStyle::CottageLattice,
        WindowStyle::CraftsmanPrairie,
        WindowStyle::PictureFixed,
        WindowStyle::PalladianVenetian,
    ] {
        for mode in [
            WindowOpeningMode::CasementLeft,
            WindowOpeningMode::CasementRight,
            WindowOpeningMode::CasementDouble,
            WindowOpeningMode::AwningTop,
            WindowOpeningMode::HopperBottom,
        ] {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            params.opening_width = 76.0;
            params.opening_height = 92.0;
            params.window_opening_mode = mode;

            let win = build_window(&params);
            assert!(win.printable_ok);
            let combined = win.printable_exterior_assembly();
            let cd = combined.verify_printable_manifold().unwrap();
            assert_eq!(cd.overhang_triangles, 0);
        }
    }
}

#[test]
fn solid_door_panelling_is_on_exterior_face_and_interior_is_optional() {
    fn count_inner_zone_z_band(
        mesh: &dh_geometry::TriMesh,
        z_min: f32,
        z_max: f32,
        half_x: f32,
        y_min: f32,
        y_max: f32,
    ) -> usize {
        mesh.positions
            .iter()
            .filter(|p| {
                p[2] > z_min && p[2] < z_max && p[0].abs() < half_x && p[1] > y_min && p[1] < y_max
            })
            .count()
    }

    for style in [
        WindowStyle::DoorColonial4Panel,
        WindowStyle::DoorGeorgian6Panel,
        WindowStyle::DoorArchedHeritage,
    ] {
        // Default: exterior has raised-panel V-grooves; interior face is plain flat
        let mut p_ext_only = WindowParams::default();
        style.apply_style_defaults(&mut p_ext_only);
        p_ext_only.bars_x = 0;
        p_ext_only.bars_y = 0;
        p_ext_only.door_thickness = 4.0;
        p_ext_only.door_interior_panels = false;

        let win_ext = build_window(&p_ext_only);
        assert!(win_ext.printable_ok, "Failed ext-only for {:?}", style);
        let leaf_ext = &win_ext.door_leaves[0].mesh;
        let t = win_ext.door_leaves[0].leaf_thickness;
        let ext_groove_verts =
            count_inner_zone_z_band(leaf_ext, t - 1.0, t - 0.08, 27.0, -70.0, 44.0);
        let int_groove_verts = count_inner_zone_z_band(leaf_ext, 0.08, 0.56, 27.0, -70.0, 44.0);
        assert!(
            ext_groove_verts >= 6,
            "{:?} expected exterior grooves on top face, got {ext_groove_verts}",
            style
        );
        assert_eq!(
            int_groove_verts, 0,
            "{:?} expected flat interior face when door_interior_panels=false, got {int_groove_verts}",
            style
        );

        // Optional interior panels enabled: BOTH exterior and interior faces have grooves
        let mut p_both = p_ext_only;
        p_both.door_interior_panels = true;
        let win_both = build_window(&p_both);
        assert!(win_both.printable_ok, "Failed both-sides for {:?}", style);
        let leaf_both = &win_both.door_leaves[0].mesh;
        let ext_both = count_inner_zone_z_band(leaf_both, t - 1.0, t - 0.08, 27.0, -70.0, 44.0);
        let int_both = count_inner_zone_z_band(leaf_both, 0.08, 0.56, 27.0, -70.0, 44.0);
        assert!(
            ext_both >= 6,
            "{:?} lost exterior grooves when interior enabled",
            style
        );
        assert!(
            int_both >= 6,
            "{:?} missing interior grooves when door_interior_panels=true (got {int_both})",
            style
        );
    }

    // Glazed Fanlight & French doors: exterior half ALWAYS has kick panels; interior half ONLY when opted in
    for (style, max_panel_y) in [
        (WindowStyle::DoorFanlightGeorgian, 10.0f32),
        (WindowStyle::DoorGlazedFrench, -28.0f32),
    ] {
        let count_lower_panel = |mesh: &dh_geometry::TriMesh| -> usize {
            mesh.positions
                .iter()
                .filter(|p| {
                    p[2] > 0.08
                        && p[2] < 0.95
                        && p[0].abs() < 22.0
                        && p[1] > -75.0
                        && p[1] < max_panel_y
                })
                .count()
        };

        let mut p_fan = WindowParams::default();
        style.apply_style_defaults(&mut p_fan);
        p_fan.door_interior_panels = false;
        let w_fan_off = build_window(&p_fan);
        assert!(w_fan_off.printable_ok);
        let ext_half = w_fan_off.door_leaves[0].exterior_leaf.as_ref().unwrap();
        assert!(count_lower_panel(ext_half) >= 4);
        assert_eq!(count_lower_panel(&w_fan_off.door_leaves[0].mesh), 0);

        p_fan.door_interior_panels = true;
        let w_fan_on = build_window(&p_fan);
        assert!(w_fan_on.printable_ok);
        assert!(count_lower_panel(&w_fan_on.door_leaves[0].mesh) >= 4);
    }
}

#[test]
fn door_panelling_survives_letterbox_and_cat_flap_additions() {
    let count_z_band = |mesh: &dh_geometry::TriMesh, z_min: f32, z_max: f32, max_y: f32| -> usize {
        mesh.positions
            .iter()
            .filter(|p| {
                p[2] > z_min && p[2] < z_max && p[0].abs() < 22.0 && p[1] > -65.0 && p[1] < max_y
            })
            .count()
    };

    for style in [
        WindowStyle::DoorColonial4Panel,
        WindowStyle::DoorGeorgian6Panel,
        WindowStyle::DoorArchedHeritage,
        WindowStyle::DoorFanlightGeorgian,
    ] {
        for door_t in [2.6, 4.0] {
            let mut params = WindowParams::default();
            style.apply_style_defaults(&mut params);
            if style == WindowStyle::DoorArchedHeritage {
                params.bars_x = 0;
                params.bars_y = 0;
            }
            params.door_thickness = door_t;
            params.door_interior_panels = true;
            params.letterbox_enabled = true;
            params.letterbox_height_ratio = 0.62;
            params.cat_flap_enabled = true;

            let win = build_window(&params);
            assert!(
                win.printable_ok,
                "Printable check failed for {:?} (door_t={door_t}): ext={}, int={}",
                style, win.ext_diag, win.int_diag
            );

            let leaf = &win.door_leaves[0];
            if let Some(ext_half) = &leaf.exterior_leaf {
                assert!(
                    count_z_band(ext_half, 0.08, 0.95, 10.0) >= 4,
                    "{:?} ext_half lost panel grooves with letterbox+cat_flap",
                    style
                );
                assert!(
                    count_z_band(&leaf.mesh, 0.08, 0.95, 10.0) >= 4,
                    "{:?} int_half lost panel grooves with letterbox+cat_flap",
                    style
                );
            } else {
                let t = leaf.leaf_thickness;
                assert!(
                    count_z_band(&leaf.mesh, t - 1.0, t - 0.08, 65.0) >= 4,
                    "{:?} lost exterior panel grooves with letterbox+cat_flap (door_t={door_t})",
                    style
                );
                assert!(
                    count_z_band(&leaf.mesh, 0.08, 0.95, 65.0) >= 4,
                    "{:?} lost interior panel grooves with letterbox+cat_flap (door_t={door_t})",
                    style
                );
            }
        }
    }

    // Verify 6-panel exterior outline bounds remain invariant & symmetric when letterbox/cat-flap intersect grooves
    let mut base_p = WindowParams::default();
    WindowStyle::DoorGeorgian6Panel.apply_style_defaults(&mut base_p);
    base_p.door_interior_panels = true;
    let base_win = build_window(&base_p);

    for lb_ratio in [0.48, 0.62, 0.25] {
        let mut add_p = base_p.clone();
        add_p.letterbox_enabled = true;
        add_p.letterbox_height_ratio = lb_ratio;
        add_p.cat_flap_enabled = true;
        let add_win = build_window(&add_p);
        assert!(
            add_win.printable_ok,
            "Printable check failed for 6-panel lb_ratio={lb_ratio}"
        );

        let groove_xy_bounds = |w: &dh_geometry::GeneratedWindow| {
            let mesh = &w.door_leaves[0].mesh;
            let t = w.door_leaves[0].leaf_thickness;
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for p in &mesh.positions {
                if p[2] > t - 1.0 && p[2] < t - 0.08 {
                    min_x = min_x.min(p[0]);
                    max_x = max_x.max(p[0]);
                    min_y = min_y.min(p[1]);
                    max_y = max_y.max(p[1]);
                }
            }
            (min_x, max_x, min_y, max_y)
        };
        let (bx0, bx1, by0, by1) = groove_xy_bounds(&base_win);
        let (ax0, ax1, ay0, ay1) = groove_xy_bounds(&add_win);
        assert!(
            (bx0 + bx1).abs() < 0.05,
            "Panels must be centered symmetrically: {bx0}..{bx1}"
        );
        assert!(
            (bx0 - ax0).abs() < 0.05
                && (bx1 - ax1).abs() < 0.05
                && (by0 - ay0).abs() < 0.05
                && (by1 - ay1).abs() < 0.05,
            "6-panel bounds shifted when letterbox (ratio={lb_ratio}) + cat flap added: base=({bx0:.2},{bx1:.2},{by0:.2},{by1:.2}) vs add=({ax0:.2},{ax1:.2},{ay0:.2},{ay1:.2})"
        );
    }
}
