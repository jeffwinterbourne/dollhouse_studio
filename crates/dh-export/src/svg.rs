use dh_core::{ComponentDef, ProjectWall, WindowOpeningMode};
use dh_geometry::{
    Contour2D, GeneratedWindow, ProfileKind, build_window, clipped_trim_outer_contour,
    profile_contour, profile_kind,
};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

const SVG_ARC_SEGMENTS: usize = 48;

/// Writes a 1:1 millimeter SVG cutting template for a single window or hinged door.
pub fn write_window_svg_to_string(window: &GeneratedWindow, title: &str) -> String {
    let p = &window.params;
    let specs = &window.specs;
    let kind = profile_kind(p.style);

    let cutout_contour =
        profile_contour(kind, p.opening_width, p.opening_height, 0.0, SVG_ARC_SEGMENTS);
    let trim_contour = clipped_trim_outer_contour(
        kind,
        specs.sleeve_outer_width,
        specs.sleeve_outer_height,
        p.trim_width + p.wall_fit_clearance,
        p,
        SVG_ARC_SEGMENTS,
    );

    let has_acetate = !window.is_door || p.style.has_acetate_glazing();
    let ac_contour_opt = if has_acetate {
        if window.is_door || !window.window_sashes.is_empty() {
            Some(profile_contour(
                ProfileKind::Rectangle,
                specs.acetate_cut_width,
                specs.acetate_cut_height,
                0.0,
                SVG_ARC_SEGMENTS,
            ))
        } else {
            let ac_offset = -(specs.barrel_wall_thickness + p.acetate_clearance);
            Some(profile_contour(
                kind,
                specs.sleeve_outer_width,
                specs.sleeve_outer_height,
                ac_offset,
                SVG_ARC_SEGMENTS,
            ))
        }
    } else {
        None
    };

    let margin = 16.0;
    let gap = 22.0;
    let header_h = 18.0;
    let footer_h = 44.0;

    let trim_w = p.opening_width + 2.0 * p.trim_width + 10.0;
    let trim_h = p.opening_height + 2.0 * (p.trim_width + p.sill_depth) + 12.0;
    let ac_box_w = if has_acetate {
        specs.acetate_cut_width + 16.0
    } else {
        0.0
    };

    let content_w = (trim_w + if has_acetate { gap + ac_box_w } else { 0.0 }).max(145.0);
    let content_h = trim_h.max(specs.acetate_cut_height + 16.0).max(50.0);
    let total_w = margin * 2.0 + content_w;
    let total_h = margin * 2.0 + header_h + content_h + footer_h;

    let cut_cx = margin + trim_w * 0.5;
    let cut_cy = margin + header_h + content_h * 0.5;

    let mut out = String::with_capacity(4096);
    svg_header(&mut out, total_w, total_h);

    let _ = writeln!(
        out,
        r##"  <text x="{margin:.2}" y="{:.2}" class="title">{} — 1:1 Cutting Template ({})</text>"##,
        margin + 5.0,
        escape_xml(title),
        escape_xml(p.style.name())
    );

    let trim_path = contour_to_svg_path(&trim_contour, cut_cx, cut_cy);
    let _ = writeln!(
        out,
        r##"  <path d="{trim_path}" fill="none" stroke="#8899aa" stroke-width="0.25" stroke-dasharray="2.5,1.5" />"##
    );

    let cut_path = contour_to_svg_path(&cutout_contour, cut_cx, cut_cy);
    let _ = writeln!(
        out,
        r##"  <path d="{cut_path}" fill="rgba(220,60,60,0.06)" stroke="#d92b2b" stroke-width="0.35" />"##
    );
    let _ = writeln!(
        out,
        r##"  <text x="{cut_cx:.2}" y="{:.2}" text-anchor="middle" class="label">Wall Cutout: {:.1} × {:.1} mm</text>"##,
        cut_cy + trim_h * 0.5 - 1.0,
        p.opening_width,
        p.opening_height
    );

    if let Some(ac_contour) = ac_contour_opt {
        let ac_cx = margin + trim_w + gap + ac_box_w * 0.5;
        let ac_cy = cut_cy;
        let ac_path = contour_to_svg_path(&ac_contour, ac_cx, ac_cy);
        let qty_note = if (window.is_door && p.double_doors)
            || p.effective_opening_mode() == WindowOpeningMode::CasementDouble
        {
            " (Cut 2x)"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            r##"  <path d="{ac_path}" fill="rgba(45,145,220,0.08)" stroke="#0f6ec4" stroke-width="0.35" />"##
        );
        let _ = writeln!(
            out,
            r##"  <text x="{ac_cx:.2}" y="{:.2}" text-anchor="middle" class="label">Acetate{}: {:.1} × {:.1} mm</text>"##,
            ac_cy + specs.acetate_cut_height * 0.5 + 6.0,
            qty_note,
            specs.acetate_cut_width,
            specs.acetate_cut_height
        );
    }

    draw_line_legend(&mut out, margin, total_h - margin - 24.0, content_w, false, has_acetate);
    draw_calibration_bar(&mut out, margin, total_h - margin - 5.0);
    out.push_str("</svg>\n");
    out
}

/// Writes a 1:1 millimeter SVG elevation & acetate cutting sheet for an entire `ProjectWall`.
pub fn write_wall_svg_to_string(wall: &ProjectWall, components: &[ComponentDef]) -> String {
    let margin = 18.0;
    let header_h = if wall.dimensions.is_empty() { 18.0 } else { 26.0 };
    let footer_h = 44.0;

    let mut glazed_items: Vec<(String, Contour2D, f64, f64, bool)> = Vec::new();
    let mut max_ac_h = 0.0f64;
    let mut sum_ac_w = 0.0f64;

    for op in &wall.openings {
        let params = op.resolve_params(components);
        let built = build_window(&params);
        if !built.is_door || built.params.style.has_acetate_glazing() {
            let ac_contour = if built.is_door || !built.window_sashes.is_empty() {
                profile_contour(
                    ProfileKind::Rectangle,
                    built.specs.acetate_cut_width,
                    built.specs.acetate_cut_height,
                    0.0,
                    SVG_ARC_SEGMENTS,
                )
            } else {
                let ac_offset =
                    -(built.specs.barrel_wall_thickness + built.params.acetate_clearance);
                profile_contour(
                    profile_kind(built.params.style),
                    built.specs.sleeve_outer_width,
                    built.specs.sleeve_outer_height,
                    ac_offset,
                    SVG_ARC_SEGMENTS,
                )
            };
            max_ac_h = max_ac_h.max(built.specs.acetate_cut_height);
            sum_ac_w += built.specs.acetate_cut_width + 14.0;
            let double_ac = (built.is_door && built.params.double_doors)
                || built.params.effective_opening_mode() == WindowOpeningMode::CasementDouble;
            glazed_items.push((
                op.name.clone(),
                ac_contour,
                built.specs.acetate_cut_width,
                built.specs.acetate_cut_height,
                double_ac,
            ));
        }
    }

    let ac_section_h = if glazed_items.is_empty() {
        0.0
    } else {
        max_ac_h + 28.0
    };
    let content_w = wall.width.max(sum_ac_w).max(165.0);
    let total_w = margin * 2.0 + content_w;
    let total_h = margin * 2.0 + header_h + wall.height + ac_section_h + footer_h;

    let wall_x0 = margin;
    let wall_y0 = margin + header_h;

    let mut out = String::with_capacity(8192);
    svg_header(&mut out, total_w, total_h);

    let _ = writeln!(
        out,
        r##"  <text x="{margin:.2}" y="{:.2}" class="title">{} — 1:1 Wall Cutout &amp; Acetate Sheet ({:.0} × {:.0} × {:.1} mm)</text>"##,
        margin + 5.0,
        escape_xml(&wall.name),
        wall.width,
        wall.height,
        wall.wall_thickness
    );

    let _ = writeln!(
        out,
        r##"  <rect x="{wall_x0:.2}" y="{wall_y0:.2}" width="{:.2}" height="{:.2}" fill="none" stroke="#1f2937" stroke-width="0.45" />"##,
        wall.width, wall.height
    );

    for op in &wall.openings {
        let params = op.resolve_params(components);
        let kind = profile_kind(params.style);
        let cx = wall_x0 + op.x_from_left + params.opening_width * 0.5;
        let cy = wall_y0 + wall.height - (op.y_from_bottom + params.opening_height * 0.5);

        let trim_c = clipped_trim_outer_contour(
            kind,
            params.opening_width,
            params.opening_height,
            params.trim_width,
            &params,
            SVG_ARC_SEGMENTS,
        );
        let trim_d = contour_to_svg_path(&trim_c, cx, cy);
        let _ = writeln!(
            out,
            r##"  <path d="{trim_d}" fill="none" stroke="#8899aa" stroke-width="0.22" stroke-dasharray="2.5,1.5" />"##
        );

        let cut_c = profile_contour(
            kind,
            params.opening_width,
            params.opening_height,
            0.0,
            SVG_ARC_SEGMENTS,
        );
        let cut_d = contour_to_svg_path(&cut_c, cx, cy);
        let _ = writeln!(
            out,
            r##"  <path d="{cut_d}" fill="rgba(220,60,60,0.06)" stroke="#d92b2b" stroke-width="0.35" />"##
        );
        let _ = writeln!(
            out,
            r##"  <text x="{cx:.2}" y="{:.2}" text-anchor="middle" class="label">{}</text>"##,
            cy - 1.5,
            escape_xml(&op.name)
        );
        let _ = writeln!(
            out,
            r##"  <text x="{cx:.2}" y="{:.2}" text-anchor="middle" class="sub">{:.1}×{:.1} @ ({:.1},{:.1})</text>"##,
            cy + 4.0,
            params.opening_width,
            params.opening_height,
            op.x_from_left,
            op.y_from_bottom
        );
    }

    for dim in &wall.dimensions {
        let dx = dim.x1 - dim.x0;
        let dy = dim.y1 - dim.y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.5 {
            continue;
        }
        let nx = -dy / len;
        let ny = dx / len;
        let sx0 = wall_x0 + dim.x0;
        let sy0 = wall_y0 + wall.height - dim.y0;
        let sx1 = wall_x0 + dim.x1;
        let sy1 = wall_y0 + wall.height - dim.y1;
        let lx0 = wall_x0 + dim.x0 + nx * dim.offset_mm;
        let ly0 = wall_y0 + wall.height - (dim.y0 + ny * dim.offset_mm);
        let lx1 = wall_x0 + dim.x1 + nx * dim.offset_mm;
        let ly1 = wall_y0 + wall.height - (dim.y1 + ny * dim.offset_mm);

        let _ = writeln!(
            out,
            r##"  <line x1="{sx0:.2}" y1="{sy0:.2}" x2="{lx0:.2}" y2="{ly0:.2}" stroke="#0f766e" stroke-width="0.2" stroke-dasharray="1.2,1.0" />"##
        );
        let _ = writeln!(
            out,
            r##"  <line x1="{sx1:.2}" y1="{sy1:.2}" x2="{lx1:.2}" y2="{ly1:.2}" stroke="#0f766e" stroke-width="0.2" stroke-dasharray="1.2,1.0" />"##
        );
        let _ = writeln!(
            out,
            r##"  <line x1="{lx0:.2}" y1="{ly0:.2}" x2="{lx1:.2}" y2="{ly1:.2}" stroke="#0f766e" stroke-width="0.3" />"##
        );
        let mx = (lx0 + lx1) * 0.5;
        let my = (ly0 + ly1) * 0.5 - 1.5;
        let _ = writeln!(
            out,
            r##"  <text x="{mx:.2}" y="{my:.2}" text-anchor="middle" class="dim">{}</text>"##,
            escape_xml(&dim.display_text())
        );
    }

    if !glazed_items.is_empty() {
        let strip_top = wall_y0 + wall.height + 14.0;
        let _ = writeln!(
            out,
            r##"  <text x="{margin:.2}" y="{strip_top:.2}" class="title">1:1 Acetate Glazing Templates (thickness: {:.2} mm)</text>"##,
            wall.acetate_thickness
        );

        let mut cursor_x = margin;
        let center_y = strip_top + 8.0 + max_ac_h * 0.5;
        for (name, contour, aw, ah, double_qty) in &glazed_items {
            let cx = cursor_x + aw * 0.5;
            let ac_d = contour_to_svg_path(contour, cx, center_y);
            let _ = writeln!(
                out,
                r##"  <path d="{ac_d}" fill="rgba(45,145,220,0.08)" stroke="#0f6ec4" stroke-width="0.35" />"##
            );
            let qty = if *double_qty { " (2x)" } else { "" };
            let _ = writeln!(
                out,
                r##"  <text x="{cx:.2}" y="{:.2}" text-anchor="middle" class="sub">{}{qty}: {:.1}×{:.1}</text>"##,
                center_y + ah * 0.5 + 5.5,
                escape_xml(name),
                aw,
                ah
            );
            cursor_x += aw + 14.0;
        }
    }

    draw_line_legend(
        &mut out,
        margin,
        total_h - margin - 24.0,
        content_w,
        true,
        !glazed_items.is_empty(),
    );
    draw_calibration_bar(&mut out, margin, total_h - margin - 5.0);
    out.push_str("</svg>\n");
    out
}

pub fn export_window_svg(
    window: &GeneratedWindow,
    title: &str,
    path: &Path,
) -> std::io::Result<()> {
    let svg = write_window_svg_to_string(window, title);
    fs::write(path, svg)
}

pub fn export_wall_svg(
    wall: &ProjectWall,
    components: &[ComponentDef],
    path: &Path,
) -> std::io::Result<()> {
    let svg = write_wall_svg_to_string(wall, components);
    fs::write(path, svg)
}

fn svg_header(out: &mut String, total_w: f64, total_h: f64) {
    let _ = writeln!(
        out,
        r##"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{total_w:.2}mm" height="{total_h:.2}mm" viewBox="0 0 {total_w:.2} {total_h:.2}">
  <style>
    .title {{ font-family: Arial, Helvetica, sans-serif; font-size: 4.2px; font-weight: bold; fill: #111827; }}
    .label {{ font-family: Arial, Helvetica, sans-serif; font-size: 3.3px; font-weight: bold; fill: #1f2937; }}
    .sub {{ font-family: Arial, Helvetica, sans-serif; font-size: 2.8px; fill: #4b5563; }}
    .dim {{ font-family: Arial, Helvetica, sans-serif; font-size: 2.7px; font-weight: bold; fill: #0f766e; }}
  </style>
  <rect x="0" y="0" width="{total_w:.2}" height="{total_h:.2}" fill="#ffffff" />"##
    );
}

fn draw_line_legend(
    out: &mut String,
    x0: f64,
    y0: f64,
    box_w: f64,
    include_wall_and_dims: bool,
    has_acetate: bool,
) {
    let w = box_w.max(140.0);
    let h = 13.0;
    let _ = writeln!(
        out,
        r##"  <rect x="{x0:.2}" y="{y0:.2}" width="{w:.2}" height="{h:.2}" rx="1.5" fill="#f8fafc" stroke="#cbd5e1" stroke-width="0.25" />"##
    );
    let _ = writeln!(
        out,
        r##"  <text x="{:.2}" y="{:.2}" class="label">Line Key:</text>"##,
        x0 + 2.5,
        y0 + 4.3
    );

    let mut entries: Vec<(&str, bool, &str)> = vec![
        ("#d92b2b", false, "Wall Cutout (Cut opening)"),
        ("#8899aa", true, "Architrave Trim (Reference only)"),
    ];
    if has_acetate {
        entries.push(("#0f6ec4", false, "Acetate Glazing (Cut pane)"));
    }
    if include_wall_and_dims {
        entries.push(("#1f2937", false, "Wall Outer Boundary"));
        entries.push(("#0f766e", false, "Dimensions (mm)"));
    }

    let cols = 3usize;
    let col_w = ((w - 22.0) / cols as f64).max(40.0);
    for (idx, (color, dashed, label)) in entries.iter().enumerate() {
        let row = idx / cols;
        let col = idx % cols;
        let ix = x0 + 21.0 + col as f64 * col_w;
        let iy = y0 + 3.6 + row as f64 * 4.8;
        let dash_attr = if *dashed {
            r#" stroke-dasharray="2.5,1.5""#
        } else {
            ""
        };
        let _ = writeln!(
            out,
            r##"  <line x1="{ix:.2}" y1="{iy:.2}" x2="{:.2}" y2="{iy:.2}" stroke="{color}" stroke-width="0.55"{dash_attr} />"##,
            ix + 7.5
        );
        let _ = writeln!(
            out,
            r##"  <text x="{:.2}" y="{:.2}" class="sub">{label}</text>"##,
            ix + 9.2,
            iy + 0.9
        );
    }
}

fn draw_calibration_bar(out: &mut String, x0: f64, y0: f64) {
    let _ = writeln!(
        out,
        r##"  <line x1="{x0:.2}" y1="{y0:.2}" x2="{:.2}" y2="{y0:.2}" stroke="#111827" stroke-width="0.4" />
  <line x1="{x0:.2}" y1="{:.2}" x2="{x0:.2}" y2="{:.2}" stroke="#111827" stroke-width="0.4" />
  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#111827" stroke-width="0.4" />
  <text x="{:.2}" y="{:.2}" class="sub">50.0 mm Scale Calibration Bar (Print at 100% / Actual Size)</text>"##,
        x0 + 50.0,
        y0 - 2.0,
        y0 + 2.0,
        x0 + 50.0,
        y0 - 2.0,
        x0 + 50.0,
        y0 + 2.0,
        x0 + 54.0,
        y0 + 1.0
    );
}

fn contour_to_svg_path(contour: &Contour2D, cx: f64, cy: f64) -> String {
    if contour.is_empty() {
        return String::new();
    }
    let mut d = String::with_capacity(contour.len() * 18);
    for (idx, p) in contour.iter().enumerate() {
        let sx = cx + p[0];
        let sy = cy - p[1];
        if idx == 0 {
            let _ = write!(d, "M {sx:.3} {sy:.3}");
        } else {
            let _ = write!(d, " L {sx:.3} {sy:.3}");
        }
    }
    d.push_str(" Z");
    d
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
