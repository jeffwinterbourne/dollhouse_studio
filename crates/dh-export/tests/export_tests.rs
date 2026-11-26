use dh_core::{
    DisplayUnit, Project, ScalePreset, WindowOpeningMode, WindowParams, WindowStyle,
};
use dh_export::{
    BedSizePreset, PageOrientation, PaperSize, PdfExportOptions, PdfInstanceMode,
    estimate_pdf_pages, pack_project_onto_build_plates, pack_wall_onto_build_plates,
    write_binary_stl_to_writer, write_piece_pdf_bytes, write_project_booklet_pdf_bytes,
    write_wall_pdf_bytes, write_wall_svg_to_string, write_window_svg_to_string,
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

    let mut proj = Project::new_multi_wall("SVG Test", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
    let comps = proj.components.clone();
    proj.walls[0].generate_auto_dimensions(&comps);
    let wall_svg = write_wall_svg_to_string(&proj.walls[0], &proj.components);
    assert!(wall_svg.contains("Parlor Left"));
    assert!(wall_svg.contains("Sill: 62.0 mm"));
}

#[test]
fn multi_page_standard_paper_pdf_tiles_walls_and_supports_unique_vs_all_instances() {
    let mut proj = Project::new_multi_wall("Georgian Manor", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
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
    let mut proj = Project::new_multi_wall("Whole House", true, ScalePreset::Scale1_12, DisplayUnit::Mm);
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
