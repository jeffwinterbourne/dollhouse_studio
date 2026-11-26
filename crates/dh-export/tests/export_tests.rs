use dh_core::{DisplayUnit, Project, ScalePreset, WindowOpeningMode, WindowParams, WindowStyle};
use dh_export::{
    BedSizePreset, PageOrientation, PaperSize, PdfExportOptions, PdfInstanceMode,
    collect_piece_individual_stls, estimate_pdf_pages, pack_piece_onto_build_plates,
    pack_project_onto_build_plates, pack_wall_onto_build_plates, write_binary_stl_to_writer,
    write_piece_pdf_bytes, write_project_booklet_pdf_bytes, write_wall_pdf_bytes,
    write_wall_svg_to_string, write_window_svg_to_string,
};
use dh_geometry::build_window;

#[test]
fn binary_stl_export_has_exact_byte_length_and_clean_header() {
    let params = WindowParams::default();
    let win = build_window(&params);
    let mut buf = Vec::new();
    write_binary_stl_to_writer(&win.print_plate, &mut buf, "Parlor Window").unwrap();

    let expected_len = 80 + 4 + win.print_plate.triangle_count() * 50;
    assert_eq!(buf.len(), expected_len);
    let hdr = String::from_utf8_lossy(&buf[..40]);
    assert!(hdr.contains("Dollhouse Studio STL"));
}

#[test]
fn build_plate_packer_resolves_components_across_wall_and_project() {
    let mut proj = Project::new_multi_wall("Manor", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let cid = proj.components[0].id.clone();
    let w2 = proj.add_wall("Side", 240.0, 240.0, 6.0, 0.40);
    proj.place_component_instance(&w2, &cid, Some("Side Parlor"), 80.0, 62.0)
        .unwrap();

    let (bw, bh) = BedSizePreset::Standard220.dimensions_mm();
    let wall_plates = pack_wall_onto_build_plates(&proj.walls[0], &proj.components, bw, bh, 8.0);
    assert!(!wall_plates.is_empty());
    for plate in &wall_plates {
        let diag = plate.verify_printable_manifold().unwrap();
        assert_eq!(diag.overhang_triangles, 0);
        assert!(diag.min_z.abs() < 1e-4);
    }

    let proj_plates = pack_project_onto_build_plates(&proj, bw, bh, 8.0);
    assert!(proj_plates.len() >= wall_plates.len());
    for plate in &proj_plates {
        let diag = plate.verify_printable_manifold().unwrap();
        assert_eq!(diag.overhang_triangles, 0);
    }
}

#[test]
fn svg_1to1_cutting_templates_and_wall_sheets_are_valid() {
    let mut params = WindowParams::default();
    WindowStyle::GeorgianGrid.apply_style_defaults(&mut params);
    params.window_opening_mode = WindowOpeningMode::CasementDouble;
    let win = build_window(&params);

    let svg = write_window_svg_to_string(&win, "Double & <Cut>");
    assert!(svg.contains("<svg "));
    assert!(svg.contains("Double &amp; &lt;Cut&gt;"));
    assert!(svg.contains("(Cut 2x)"));
    assert!(svg.contains("50.0 mm Scale Calibration Bar"));

    let mut proj =
        Project::new_multi_wall("SVG Test", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let comps = proj.components.clone();
    proj.walls[0].generate_auto_dimensions(&comps);
    let wall_svg = write_wall_svg_to_string(&proj.walls[0], &proj.components);
    assert!(wall_svg.contains("Parlor Left"));
    assert!(wall_svg.contains("Sill: 62.0 mm"));
}

#[test]
fn multi_page_standard_paper_pdf_tiles_walls_and_supports_unique_vs_all_instances() {
    let mut proj = Project::new_multi_wall(
        "Georgian Manor",
        true,
        ScalePreset::Scale1_12,
        DisplayUnit::Mm,
    );
    let comps = proj.components.clone();
    proj.walls[0].generate_auto_dimensions(&comps);

    // Default A4 Portrait/Auto: 380x240mm wall must tile across multiple A4 pages!
    let opts_unique = PdfExportOptions {
        paper: PaperSize::A4,
        orientation: PageOrientation::Portrait,
        instance_mode: PdfInstanceMode::UniqueWithQty,
        ..Default::default()
    };
    let bytes_unique = write_wall_pdf_bytes(&proj.walls[0], &proj.components, &opts_unique);
    let txt_unique = String::from_utf8_lossy(&bytes_unique);
    assert!(bytes_unique.starts_with(b"%PDF-1.4"));
    assert!(txt_unique.contains("/MediaBox [0 0 595.2756 841.8898]"));
    assert!(txt_unique.contains("Tile R1-C1"));
    assert!(txt_unique.contains("Tile R1-C2"));
    assert!(txt_unique.contains("[Qty: x2]"));
    assert!(txt_unique.contains("%%EOF"));

    let est_wall = estimate_pdf_pages(&proj, Some(&proj.walls[0]), &opts_unique);
    assert!(txt_unique.contains(&format!("/Count {est_wall}")));

    // AllInstances prints all 3 openings individually without Qty: x2 merge
    let opts_all = PdfExportOptions {
        paper: PaperSize::UsLetter,
        orientation: PageOrientation::Landscape,
        instance_mode: PdfInstanceMode::AllInstances,
        display_unit: DisplayUnit::Inches,
        ..Default::default()
    };
    let bytes_all = write_wall_pdf_bytes(&proj.walls[0], &proj.components, &opts_all);
    let txt_all = String::from_utf8_lossy(&bytes_all);
    assert!(txt_all.contains("/MediaBox [0 0 792.0000 612.0000]"));
    assert!(!txt_all.contains("[Qty: x2]"));
    assert!(txt_all.contains("Parlor Left"));
    assert!(txt_all.contains("Parlor Right"));
    assert!(txt_all.contains("2.00 in \\(50.8 mm\\) Scale Calibration Bar"));
}

#[test]
fn project_booklet_and_single_piece_standard_pdf_generate_cleanly() {
    let mut proj =
        Project::new_multi_wall("Whole House", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let cid = proj.components[0].id.clone();
    let w2 = proj.add_wall("Garden Wall", 260.0, 220.0, 6.0, 0.40);
    proj.place_component_instance(&w2, &cid, Some("Garden Parlor"), 60.0, 62.0)
        .unwrap();

    let opts = PdfExportOptions::default();
    let booklet = write_project_booklet_pdf_bytes(&proj, &opts);
    let btxt = String::from_utf8_lossy(&booklet);
    assert!(booklet.starts_with(b"%PDF-1.4"));
    assert!(btxt.contains("Project Parts Schedule & BOM"));
    assert!(btxt.contains("Front Facade"));
    assert!(btxt.contains("Garden Wall"));
    // 3 instances of Parlor Georgian Window across 2 walls should aggregate to x3
    assert!(btxt.contains("x3"));

    let est = estimate_pdf_pages(&proj, None, &opts);
    assert!(btxt.contains(&format!("/Count {est}")));
    assert!(est >= 4);

    let win = build_window(&WindowParams::default());
    let single_pdf = write_piece_pdf_bytes(&win, "Single Test", &opts);
    assert!(single_pdf.starts_with(b"%PDF-1.4"));
    assert!(String::from_utf8_lossy(&single_pdf).contains("Single Test"));
}

#[test]
fn bed_aware_piece_plate_packing_respects_bed_dimensions_and_splits_when_needed() {
    let mut params = WindowParams::default();
    WindowStyle::DoorGeorgian6Panel.apply_style_defaults(&mut params);
    params.letterbox_enabled = true;
    params.cat_flap_enabled = true;
    let door = build_window(&params);

    for (preset, expect_split) in [
        (BedSizePreset::Bambu256, false),
        (BedSizePreset::Standard220, true),
    ] {
        let (bw, bh) = preset.dimensions_mm();
        let report = pack_piece_onto_build_plates(&door, bw, bh, 6.0);
        assert!(!report.plates.is_empty());
        assert!(
            report.oversized_parts.is_empty(),
            "{:?} had oversized parts: {:?}",
            preset,
            report.oversized_parts
        );
        if expect_split {
            assert!(
                report.plates.len() >= 2,
                "Expected 220x220 bed to split door across >=2 plates"
            );
        }
        for (idx, plate) in report.plates.iter().enumerate() {
            let (bmin, bmax) = plate.bounds();
            let span_x = bmax[0] - bmin[0];
            let span_y = bmax[1] - bmin[1];
            assert!(
                span_x <= bw + 0.01 && span_y <= bh + 0.01,
                "{:?} plate {idx} ({span_x:.1}x{span_y:.1}) exceeds {bw}x{bh}",
                preset
            );
            let diag = plate.verify_printable_manifold().unwrap();
            assert_eq!(diag.overhang_triangles, 0);
            assert!(diag.min_z.abs() < 1e-4);
        }
    }

    // Standard single window must pack onto Plate 1 of Mini 180x180 bed
    let win = build_window(&WindowParams::default());
    let (mw, mh) = BedSizePreset::Mini180.dimensions_mm();
    let win_report = pack_piece_onto_build_plates(&win, mw, mh, 6.0);
    assert_eq!(win_report.plates.len(), 1);
    let (wmin, wmax) = win_report.plates[0].bounds();
    assert!(wmax[0] - wmin[0] <= mw && wmax[1] - wmin[1] <= mh);
}

#[test]
fn individual_per_part_stl_export_outputs_distinct_watertight_parts() {
    let mut params = WindowParams::default();
    WindowStyle::DoorGeorgian6Panel.apply_style_defaults(&mut params);
    params.letterbox_enabled = true;
    params.cat_flap_enabled = true;
    let door = build_window(&params);

    let parts = collect_piece_individual_stls("Front Door", &door);
    let names: Vec<&str> = parts.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        parts.len(),
        6,
        "Expected 6 separate STL parts, got {:?}",
        names
    );
    assert!(names.contains(&"front_door_exterior_frame"));
    assert!(names.contains(&"front_door_interior_frame"));
    assert!(names.contains(&"front_door_door_leaf"));
    assert!(names.contains(&"front_door_door_handles"));
    assert!(names.contains(&"front_door_letterbox_plates"));
    assert!(names.contains(&"front_door_cat_flap"));

    for (name, mesh) in &parts {
        let diag = mesh
            .verify_printable_manifold()
            .unwrap_or_else(|e| panic!("Individual STL part {name} failed: {e}"));
        assert_eq!(diag.overhang_triangles, 0);
        assert!(diag.min_z.abs() < 1e-4);
        let (bmin, bmax) = mesh.bounds();
        assert!(((bmin[0] + bmax[0]) * 0.5).abs() < 1e-3);
        assert!(((bmin[1] + bmax[1]) * 0.5).abs() < 1e-3);
    }

    // Glazed Double French Fanlight door should produce separate interior & exterior leaves
    let mut double_fan = WindowParams::default();
    WindowStyle::DoorFanlightGeorgian.apply_style_defaults(&mut double_fan);
    double_fan.double_doors = true;
    let built_dbl = build_window(&double_fan);
    let dbl_parts = collect_piece_individual_stls("Patio", &built_dbl);
    let dbl_names: Vec<&str> = dbl_parts.iter().map(|(n, _)| n.as_str()).collect();
    assert!(dbl_names.contains(&"patio_door_leaf_left"));
    assert!(dbl_names.contains(&"patio_door_leaf_left_exterior"));
    assert!(dbl_names.contains(&"patio_door_leaf_right"));
    assert!(dbl_names.contains(&"patio_door_leaf_right_exterior"));
}
