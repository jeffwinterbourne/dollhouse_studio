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
