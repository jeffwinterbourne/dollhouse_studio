use dh_core::{
    ComponentDef, DisplayUnit, Project, ProjectKind, ProjectWall, WindowOpeningMode, WindowParams,
};
use dh_geometry::{
    Contour2D, GeneratedWindow, ProfileKind, build_window, clipped_trim_outer_contour,
    profile_contour, profile_kind,
};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

const PDF_ARC_SEGMENTS: usize = 48;
const MM_TO_PT: f64 = 72.0 / 25.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    A4,
    UsLetter,
    A3,
}

impl PaperSize {
    pub const ALL: [PaperSize; 3] = [PaperSize::A4, PaperSize::UsLetter, PaperSize::A3];

    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4 (210 × 297 mm)",
            PaperSize::UsLetter => "US Letter (8.5 × 11 in)",
            PaperSize::A3 => "A3 (297 × 420 mm)",
        }
    }

    pub fn portrait_mm(self) -> (f64, f64) {
        match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::UsLetter => (215.9, 279.4),
            PaperSize::A3 => (297.0, 420.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageOrientation {
    #[default]
    Auto,
    Portrait,
    Landscape,
}

impl PageOrientation {
    pub const ALL: [PageOrientation; 3] = [
        PageOrientation::Auto,
        PageOrientation::Portrait,
        PageOrientation::Landscape,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PageOrientation::Auto => "Auto (Best Fit)",
            PageOrientation::Portrait => "Portrait",
            PageOrientation::Landscape => "Landscape",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PdfInstanceMode {
    /// Default: print one copy per unique piece with a clear Qty badge (e.g. "Qty: x4").
    #[default]
    UniqueWithQty,
    /// Print every instance individually so each pane can be cut straight from the printout.
    AllInstances,
}

impl PdfInstanceMode {
    pub const ALL: [PdfInstanceMode; 2] = [
        PdfInstanceMode::UniqueWithQty,
        PdfInstanceMode::AllInstances,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PdfInstanceMode::UniqueWithQty => "Unique Parts + Qty Badge (Saves Paper)",
            PdfInstanceMode::AllInstances => "Every Instance Individually",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub paper: PaperSize,
    pub orientation: PageOrientation,
    pub margin_mm: f64,
    pub tile_overlap_mm: f64,
    pub instance_mode: PdfInstanceMode,
    pub display_unit: DisplayUnit,
    pub include_cover_bom: bool,
    pub include_wall_tiles: bool,
    pub include_cutout_and_acetate: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::A4,
            orientation: PageOrientation::Auto,
            margin_mm: 12.0,
            tile_overlap_mm: 12.0,
            instance_mode: PdfInstanceMode::UniqueWithQty,
            display_unit: DisplayUnit::Mm,
            include_cover_bom: true,
            include_wall_tiles: true,
            include_cutout_and_acetate: true,
        }
    }
}

impl PdfExportOptions {
    fn page_dims_mm(&self, prefer_landscape: bool) -> (f64, f64) {
        let (pw, ph) = self.paper.portrait_mm();
        let use_land = match self.orientation {
            PageOrientation::Auto => prefer_landscape,
            PageOrientation::Portrait => false,
            PageOrientation::Landscape => true,
        };
        if use_land { (ph, pw) } else { (pw, ph) }
    }
}

struct PdfPage {
    w_mm: f64,
    h_mm: f64,
    stream: String,
    title: String,
    subtitle: String,
    show_legend: bool,
    include_wall_and_dims: bool,
    has_acetate: bool,
}

impl PdfPage {
    fn new(
        w_mm: f64,
        h_mm: f64,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        show_legend: bool,
        include_wall_and_dims: bool,
        has_acetate: bool,
    ) -> Self {
        Self {
            w_mm,
            h_mm,
            stream: String::with_capacity(8192),
            title: title.into(),
            subtitle: subtitle.into(),
            show_legend,
            include_wall_and_dims,
            has_acetate,
        }
    }

    fn px(&self, x_mm: f64) -> f64 {
        x_mm * MM_TO_PT
    }

    fn py(&self, y_top_down_mm: f64) -> f64 {
        (self.h_mm - y_top_down_mm) * MM_TO_PT
    }

    fn set_stroke(&mut self, r: f64, g: f64, b: f64, width_mm: f64, dashed: bool) {
        let _ = writeln!(
            self.stream,
            "{r:.3} {g:.3} {b:.3} RG {:.3} w",
            width_mm * MM_TO_PT
        );
        if dashed {
            let d1 = 2.5 * MM_TO_PT;
            let d2 = 1.5 * MM_TO_PT;
            let _ = writeln!(self.stream, "[{d1:.2} {d2:.2}] 0 d");
        } else {
            self.stream.push_str("[] 0 d\n");
        }
    }

    fn set_fill(&mut self, r: f64, g: f64, b: f64) {
        let _ = writeln!(self.stream, "{r:.3} {g:.3} {b:.3} rg");
    }

    fn push_clip_rect(&mut self, x_mm: f64, y_mm: f64, w_mm: f64, h_mm: f64) {
        let rx = self.px(x_mm);
        let ry = self.py(y_mm + h_mm);
        let rw = w_mm * MM_TO_PT;
        let rh = h_mm * MM_TO_PT;
        let _ = writeln!(self.stream, "q {rx:.3} {ry:.3} {rw:.3} {rh:.3} re W n");
    }

    fn pop_clip(&mut self) {
        self.stream.push_str("Q\n");
    }

    fn fill_stroke_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        w_mm: f64,
        h_mm: f64,
        fill: Option<[f64; 3]>,
    ) {
        let rx = self.px(x_mm);
        let ry = self.py(y_mm + h_mm);
        let rw = w_mm * MM_TO_PT;
        let rh = h_mm * MM_TO_PT;
        if let Some([fr, fg, fb]) = fill {
            self.set_fill(fr, fg, fb);
            let _ = writeln!(self.stream, "{rx:.3} {ry:.3} {rw:.3} {rh:.3} re B");
        } else {
            let _ = writeln!(self.stream, "{rx:.3} {ry:.3} {rw:.3} {rh:.3} re S");
        }
    }

    fn draw_line(&mut self, x0_mm: f64, y0_mm: f64, x1_mm: f64, y1_mm: f64) {
        let _ = writeln!(
            self.stream,
            "{:.3} {:.3} m {:.3} {:.3} l S",
            self.px(x0_mm),
            self.py(y0_mm),
            self.px(x1_mm),
            self.py(y1_mm)
        );
    }

    fn draw_contour(
        &mut self,
        contour: &Contour2D,
        cx_mm: f64,
        cy_mm: f64,
        fill: Option<[f64; 3]>,
    ) {
        if contour.len() < 3 {
            return;
        }
        for (i, pt) in contour.iter().enumerate() {
            let sx = cx_mm + pt[0];
            let sy = cy_mm - pt[1];
            if i == 0 {
                let _ = writeln!(self.stream, "{:.3} {:.3} m", self.px(sx), self.py(sy));
            } else {
                let _ = writeln!(self.stream, "{:.3} {:.3} l", self.px(sx), self.py(sy));
            }
        }
        if let Some([fr, fg, fb]) = fill {
            self.set_fill(fr, fg, fb);
            self.stream.push_str("h B\n");
        } else {
            self.stream.push_str("h S\n");
        }
    }

    fn draw_circle_stroke(&mut self, cx_mm: f64, cy_mm: f64, r_mm: f64) {
        let segs = 20usize;
        for i in 0..segs {
            let a = (i as f64 / segs as f64) * std::f64::consts::TAU;
            let sx = cx_mm + r_mm * a.cos();
            let sy = cy_mm + r_mm * a.sin();
            if i == 0 {
                let _ = writeln!(self.stream, "{:.3} {:.3} m", self.px(sx), self.py(sy));
            } else {
                let _ = writeln!(self.stream, "{:.3} {:.3} l", self.px(sx), self.py(sy));
            }
        }
        self.stream.push_str("h S\n");
    }

    fn draw_crosshair(&mut self, cx_mm: f64, cy_mm: f64, r_mm: f64) {
        self.set_stroke(0.25, 0.30, 0.38, 0.25, false);
        self.draw_circle_stroke(cx_mm, cy_mm, r_mm);
        self.draw_line(cx_mm - r_mm * 1.4, cy_mm, cx_mm + r_mm * 1.4, cy_mm);
        self.draw_line(cx_mm, cy_mm - r_mm * 1.4, cx_mm, cy_mm + r_mm * 1.4);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &mut self,
        x_mm: f64,
        baseline_y_mm: f64,
        text: &str,
        size_mm: f64,
        bold: bool,
        centered: bool,
        rgb: [f64; 3],
    ) {
        let clean = pdf_sanitize_text(text);
        let size_pt = size_mm * MM_TO_PT;
        let x_pt = if centered {
            let approx_w_pt = clean.len() as f64 * size_pt * 0.50;
            self.px(x_mm) - approx_w_pt * 0.5
        } else {
            self.px(x_mm)
        };
        let y_pt = self.py(baseline_y_mm);
        let font = if bold { "F2" } else { "F1" };
        let _ = writeln!(
            self.stream,
            "BT /{font} {size_pt:.2} Tf {:.3} {:.3} {:.3} rg 1 0 0 1 {x_pt:.3} {y_pt:.3} Tm ({}) Tj ET",
            rgb[0],
            rgb[1],
            rgb[2],
            pdf_escape_str(&clean)
        );
    }

    fn stamp_chrome(
        &mut self,
        margin: f64,
        page_num: usize,
        total_pages: usize,
        unit: DisplayUnit,
    ) {
        let content_w = self.w_mm - 2.0 * margin;
        // Top header bar
        self.draw_text(
            margin,
            margin + 4.0,
            &self.title.clone(),
            4.1,
            true,
            false,
            [0.067, 0.094, 0.153],
        );
        self.draw_text(
            margin,
            margin + 8.6,
            &self.subtitle.clone(),
            2.8,
            false,
            false,
            [0.294, 0.333, 0.388],
        );

        let right_x = self.w_mm - margin - 52.0;
        self.draw_text(
            right_x,
            margin + 4.0,
            "1:1 TRUE SCALE - PRINT AT 100%",
            2.7,
            true,
            false,
            [0.72, 0.14, 0.14],
        );
        self.draw_text(
            right_x,
            margin + 8.6,
            &format!("Page {page_num} of {total_pages}"),
            2.8,
            false,
            false,
            [0.25, 0.30, 0.38],
        );

        self.set_stroke(0.82, 0.85, 0.90, 0.25, false);
        self.draw_line(margin, margin + 11.0, self.w_mm - margin, margin + 11.0);

        // Footer legend + calibration bar
        if self.show_legend {
            self.draw_legend(
                margin,
                self.h_mm - margin - 20.0,
                content_w,
                self.include_wall_and_dims,
                self.has_acetate,
            );
        }
        self.draw_calibration_bar(margin, self.h_mm - margin - 3.5, unit);
    }

    fn draw_legend(
        &mut self,
        x0: f64,
        y0: f64,
        box_w: f64,
        include_wall_and_dims: bool,
        has_acetate: bool,
    ) {
        let w = box_w.max(120.0);
        let h = 11.5;
        self.set_stroke(0.796, 0.835, 0.882, 0.25, false);
        self.fill_stroke_rect(x0, y0, w, h, Some([0.973, 0.980, 0.988]));
        self.draw_text(
            x0 + 2.5,
            y0 + 3.9,
            "Line Key:",
            2.8,
            true,
            false,
            [0.122, 0.161, 0.216],
        );

        let mut entries: Vec<([f64; 3], bool, &str)> = vec![
            ([0.851, 0.169, 0.169], false, "Wall Cutout"),
            ([0.533, 0.600, 0.667], true, "Architrave Trim"),
        ];
        if has_acetate {
            entries.push(([0.059, 0.431, 0.769], false, "Acetate Glazing"));
        }
        if include_wall_and_dims {
            entries.push(([0.122, 0.161, 0.216], false, "Wall Boundary"));
            entries.push(([0.059, 0.463, 0.431], false, "Dimensions"));
        }

        let cols = 3usize;
        let col_w = ((w - 20.0) / cols as f64).max(36.0);
        for (idx, (col, dashed, label)) in entries.iter().enumerate() {
            let row = idx / cols;
            let col_i = idx % cols;
            let ix = x0 + 19.0 + col_i as f64 * col_w;
            let iy = y0 + 3.3 + row as f64 * 4.5;
            self.set_stroke(col[0], col[1], col[2], 0.50, *dashed);
            self.draw_line(ix, iy, ix + 6.5, iy);
            self.draw_text(
                ix + 8.0,
                iy + 0.8,
                label,
                2.5,
                false,
                false,
                [0.25, 0.30, 0.36],
            );
        }
    }

    fn draw_calibration_bar(&mut self, x0: f64, y0: f64, unit: DisplayUnit) {
        let (bar_mm, label) = match unit {
            DisplayUnit::Inches => (
                50.8,
                "2.00 in (50.8 mm) Scale Calibration Bar - Print at 100% / Actual Size",
            ),
            _ => (
                50.0,
                "50.0 mm Scale Calibration Bar - Print at 100% / Actual Size",
            ),
        };
        self.set_stroke(0.067, 0.094, 0.153, 0.40, false);
        self.draw_line(x0, y0, x0 + bar_mm, y0);
        self.draw_line(x0, y0 - 1.8, x0, y0 + 1.8);
        self.draw_line(x0 + bar_mm, y0 - 1.8, x0 + bar_mm, y0 + 1.8);
        self.draw_text(
            x0 + bar_mm + 4.0,
            y0 + 0.9,
            label,
            2.6,
            false,
            false,
            [0.294, 0.333, 0.388],
        );
    }
}

struct PdfDocument {
    pages: Vec<PdfPage>,
    margin_mm: f64,
    unit: DisplayUnit,
}

impl PdfDocument {
    fn new(margin_mm: f64, unit: DisplayUnit) -> Self {
        Self {
            pages: Vec::new(),
            margin_mm,
            unit,
        }
    }

    fn into_pdf_bytes(mut self) -> Vec<u8> {
        let total = self.pages.len().max(1);
        for (idx, page) in self.pages.iter_mut().enumerate() {
            page.stamp_chrome(self.margin_mm, idx + 1, total, self.unit);
        }

        let mut pdf = Vec::with_capacity(self.pages.len() * 8192 + 1024);
        pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

        // Obj 1: Catalog, Obj 2: Pages, Obj 3: F1 Helvetica, Obj 4: F2 Helvetica-Bold
        let page_count = self.pages.len();
        let mut offsets = Vec::with_capacity(4 + page_count * 2);

        offsets.push(pdf.len());
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        let kids: Vec<String> = (0..page_count)
            .map(|i| format!("{} 0 R", 5 + i * 2))
            .collect();
        offsets.push(pdf.len());
        let pages_obj = format!(
            "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
            kids.join(" "),
            page_count
        );
        pdf.extend_from_slice(pages_obj.as_bytes());

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
        );

        offsets.push(pdf.len());
        pdf.extend_from_slice(
            b"4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n",
        );

        for (i, page) in self.pages.iter().enumerate() {
            let page_id = 5 + i * 2;
            let content_id = page_id + 1;
            let w_pt = page.w_mm * MM_TO_PT;
            let h_pt = page.h_mm * MM_TO_PT;

            offsets.push(pdf.len());
            let p_obj = format!(
                "{page_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w_pt:.4} {h_pt:.4}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {content_id} 0 R >>\nendobj\n"
            );
            pdf.extend_from_slice(p_obj.as_bytes());

            let stream_bytes = page.stream.as_bytes();
            offsets.push(pdf.len());
            let c_hdr = format!(
                "{content_id} 0 obj\n<< /Length {} >>\nstream\n",
                stream_bytes.len()
            );
            pdf.extend_from_slice(c_hdr.as_bytes());
            pdf.extend_from_slice(stream_bytes);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        }

        let xref_offset = pdf.len();
        let obj_count = offsets.len() + 1;
        pdf.extend_from_slice(format!("xref\n0 {obj_count}\n").as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for off in offsets {
            let line = format!("{off:010} 00000 n \n");
            pdf.extend_from_slice(line.as_bytes());
        }
        let trailer = format!(
            "trailer\n<< /Size {obj_count} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
        );
        pdf.extend_from_slice(trailer.as_bytes());
        pdf
    }
}

struct PackedTemplateCard {
    title: String,
    qty: usize,
    acetate_qty: usize,
    style_name: String,
    opening_w: f64,
    opening_h: f64,
    cutout_contour: Contour2D,
    trim_contour: Contour2D,
    acetate_contour: Option<(Contour2D, f64, f64)>,
    box_w: f64,
    box_h: f64,
    walls_used: Vec<String>,
}

fn canonical_template_key(p: &WindowParams) -> String {
    let mut c = p.clone();
    // Cutout and acetate templates do not depend on wall thickness
    c.wall_thickness = 0.0;
    serde_json::to_string(&c).unwrap_or_default()
}

fn build_template_card(
    title: String,
    qty: usize,
    walls_used: Vec<String>,
    built: &GeneratedWindow,
) -> PackedTemplateCard {
    let p = &built.params;
    let specs = &built.specs;
    let kind = profile_kind(p.style);
    let cutout_contour = profile_contour(
        kind,
        p.opening_width,
        p.opening_height,
        0.0,
        PDF_ARC_SEGMENTS,
    );
    let trim_contour = clipped_trim_outer_contour(
        kind,
        specs.sleeve_outer_width,
        specs.sleeve_outer_height,
        p.trim_width + p.wall_fit_clearance,
        p,
        PDF_ARC_SEGMENTS,
    );

    let has_acetate = !built.is_door || p.style.has_acetate_glazing();
    let sash_mult = if (built.is_door && p.double_doors)
        || p.effective_opening_mode() == WindowOpeningMode::CasementDouble
    {
        2
    } else {
        1
    };
    let acetate_contour = if has_acetate {
        let contour = if built.is_door || !built.window_sashes.is_empty() {
            profile_contour(
                ProfileKind::Rectangle,
                specs.acetate_cut_width,
                specs.acetate_cut_height,
                0.0,
                PDF_ARC_SEGMENTS,
            )
        } else {
            let ac_offset = -(specs.barrel_wall_thickness + p.acetate_clearance);
            profile_contour(
                kind,
                specs.sleeve_outer_width,
                specs.sleeve_outer_height,
                ac_offset,
                PDF_ARC_SEGMENTS,
            )
        };
        Some((contour, specs.acetate_cut_width, specs.acetate_cut_height))
    } else {
        None
    };

    let cut_pad_w = p.opening_width + 2.0 * (p.trim_width + 3.0);
    let cut_pad_h = p.opening_height + 2.0 * (p.trim_width + p.sill_depth + 3.0);
    let gap = 8.0;
    let ac_w = acetate_contour
        .as_ref()
        .map(|(_, w, _)| *w + 6.0)
        .unwrap_or(0.0);
    let ac_h = acetate_contour
        .as_ref()
        .map(|(_, _, h)| *h + 6.0)
        .unwrap_or(0.0);

    let inner_w = if acetate_contour.is_some() {
        cut_pad_w + gap + ac_w
    } else {
        cut_pad_w
    };
    let box_w = inner_w.max(46.0) + 6.0;
    let box_h = cut_pad_h.max(ac_h).max(28.0) + 18.0;

    PackedTemplateCard {
        title,
        qty,
        acetate_qty: if has_acetate { qty * sash_mult } else { 0 },
        style_name: p.style.name().to_string(),
        opening_w: p.opening_width,
        opening_h: p.opening_height,
        cutout_contour,
        trim_contour,
        acetate_contour,
        box_w,
        box_h,
        walls_used,
    }
}

fn collect_wall_cards(
    wall: &ProjectWall,
    components: &[ComponentDef],
    mode: PdfInstanceMode,
) -> Vec<PackedTemplateCard> {
    collect_multi_wall_cards(&[(wall, components)], mode)
}

fn collect_multi_wall_cards(
    walls: &[(&ProjectWall, &[ComponentDef])],
    mode: PdfInstanceMode,
) -> Vec<PackedTemplateCard> {
    match mode {
        PdfInstanceMode::AllInstances => {
            let mut out = Vec::new();
            for (wall, comps) in walls {
                for op in &wall.openings {
                    let params = op.resolve_params(comps);
                    let built = build_window(&params);
                    out.push(build_template_card(
                        op.name.clone(),
                        1,
                        vec![wall.name.clone()],
                        &built,
                    ));
                }
            }
            out
        }
        PdfInstanceMode::UniqueWithQty => {
            let mut order: Vec<String> = Vec::new();
            let mut groups: HashMap<String, (String, WindowParams, usize, Vec<String>)> =
                HashMap::new();
            for (wall, comps) in walls {
                for op in &wall.openings {
                    let params = op.resolve_params(comps);
                    let key = canonical_template_key(&params);
                    let base_name = op
                        .component_id()
                        .and_then(|cid| comps.iter().find(|c| c.id == cid))
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| op.name.clone());
                    groups
                        .entry(key.clone())
                        .and_modify(|(_, _, qty, wnames)| {
                            *qty += 1;
                            if !wnames.contains(&wall.name) {
                                wnames.push(wall.name.clone());
                            }
                        })
                        .or_insert_with(|| {
                            order.push(key);
                            (base_name, params, 1, vec![wall.name.clone()])
                        });
                }
            }

            order
                .into_iter()
                .filter_map(|k| groups.remove(&k))
                .map(|(title, params, qty, wnames)| {
                    let built = build_window(&params);
                    build_template_card(title, qty, wnames, &built)
                })
                .collect()
        }
    }
}

fn append_tiled_wall_pages(
    doc: &mut PdfDocument,
    wall: &ProjectWall,
    components: &[ComponentDef],
    opts: &PdfExportOptions,
) {
    let margin = opts.margin_mm.max(8.0);
    let pad_l = 12.0;
    let pad_r = 12.0;
    let pad_t = if wall.dimensions.is_empty() {
        10.0
    } else {
        20.0
    };
    let pad_b = 10.0;
    let total_draw_w = wall.width + pad_l + pad_r;
    let total_draw_h = wall.height + pad_t + pad_b;

    let prefer_land = total_draw_w >= total_draw_h;
    let (page_w, page_h) = opts.page_dims_mm(prefer_land);

    let header_h = 13.0;
    let footer_h = 24.0;
    let avail_w = (page_w - 2.0 * margin).max(60.0);
    let avail_h = (page_h - 2.0 * margin - header_h - footer_h).max(60.0);

    let fits_single = total_draw_w <= avail_w && total_draw_h <= avail_h;
    let (cols, rows, overlap) = if fits_single {
        (1usize, 1usize, 0.0)
    } else {
        let ov = opts.tile_overlap_mm.clamp(6.0, 24.0);
        let step_w = (avail_w - ov).max(30.0);
        let step_h = (avail_h - ov).max(30.0);
        let c = ((total_draw_w - ov) / step_w).ceil().max(1.0) as usize;
        let r = ((total_draw_h - ov) / step_h).ceil().max(1.0) as usize;
        (c, r, ov)
    };

    let step_w = (avail_w - overlap).max(30.0);
    let step_h = (avail_h - overlap).max(30.0);
    let unit = opts.display_unit;

    for r in 0..rows {
        for c in 0..cols {
            let tile_tag = if rows * cols > 1 {
                format!(" [Tile R{}-C{} of {}x{}]", r + 1, c + 1, rows, cols)
            } else {
                String::new()
            };
            let title = format!("{} - 1:1 Wall Elevation{tile_tag}", wall.name);
            let subtitle = format!(
                "Wall: {} x {} | Wall Thick: {} | Acetate: {}",
                unit.format_value(wall.width),
                unit.format_value(wall.height),
                unit.format_value(wall.wall_thickness),
                unit.format_value(wall.acetate_thickness)
            );
            let mut page = PdfPage::new(page_w, page_h, title, subtitle, true, true, false);

            let area_x0 = margin;
            let area_y0 = margin + header_h;

            // Compute world offset for this tile
            let (origin_x, origin_y) = if fits_single {
                (
                    area_x0 + (avail_w - wall.width) * 0.5,
                    area_y0 + (avail_h - wall.height) * 0.5,
                )
            } else {
                let slice_x = c as f64 * step_w;
                let slice_y = r as f64 * step_h;
                (area_x0 + pad_l - slice_x, area_y0 + pad_t - slice_y)
            };

            page.push_clip_rect(area_x0, area_y0, avail_w, avail_h);

            // Outer wall boundary
            page.set_stroke(0.122, 0.161, 0.216, 0.45, false);
            page.fill_stroke_rect(origin_x, origin_y, wall.width, wall.height, None);

            // Openings
            for op in &wall.openings {
                let params = op.resolve_params(components);
                let kind = profile_kind(params.style);
                let cx = origin_x + op.x_from_left + params.opening_width * 0.5;
                let cy = origin_y + wall.height - (op.y_from_bottom + params.opening_height * 0.5);

                let trim_c = clipped_trim_outer_contour(
                    kind,
                    params.opening_width,
                    params.opening_height,
                    params.trim_width,
                    &params,
                    PDF_ARC_SEGMENTS,
                );
                page.set_stroke(0.533, 0.600, 0.667, 0.25, true);
                page.draw_contour(&trim_c, cx, cy, None);

                let cut_c = profile_contour(
                    kind,
                    params.opening_width,
                    params.opening_height,
                    0.0,
                    PDF_ARC_SEGMENTS,
                );
                page.set_stroke(0.851, 0.169, 0.169, 0.35, false);
                page.draw_contour(&cut_c, cx, cy, Some([0.992, 0.945, 0.945]));
                page.draw_text(
                    cx,
                    cy - 1.5,
                    &op.name,
                    3.0,
                    true,
                    true,
                    [0.122, 0.161, 0.216],
                );
                page.draw_text(
                    cx,
                    cy + 3.8,
                    &unit.format_dims(params.opening_width, params.opening_height),
                    2.5,
                    false,
                    true,
                    [0.294, 0.333, 0.388],
                );
            }

            // Dimensions
            for dim in &wall.dimensions {
                let dx = dim.x1 - dim.x0;
                let dy = dim.y1 - dim.y0;
                let len = (dx * dx + dy * dy).sqrt();
                if len < 0.5 {
                    continue;
                }
                let nx = -dy / len;
                let ny = dx / len;
                let sx0 = origin_x + dim.x0;
                let sy0 = origin_y + wall.height - dim.y0;
                let sx1 = origin_x + dim.x1;
                let sy1 = origin_y + wall.height - dim.y1;
                let lx0 = origin_x + dim.x0 + nx * dim.offset_mm;
                let ly0 = origin_y + wall.height - (dim.y0 + ny * dim.offset_mm);
                let lx1 = origin_x + dim.x1 + nx * dim.offset_mm;
                let ly1 = origin_y + wall.height - (dim.y1 + ny * dim.offset_mm);

                page.set_stroke(0.059, 0.463, 0.431, 0.22, true);
                page.draw_line(sx0, sy0, lx0, ly0);
                page.draw_line(sx1, sy1, lx1, ly1);
                page.set_stroke(0.059, 0.463, 0.431, 0.32, false);
                page.draw_line(lx0, ly0, lx1, ly1);
                page.draw_text(
                    (lx0 + lx1) * 0.5,
                    (ly0 + ly1) * 0.5 - 1.5,
                    &dim.format_text(unit),
                    2.6,
                    true,
                    true,
                    [0.059, 0.463, 0.431],
                );
            }

            page.pop_clip();

            // Tile boundary, alignment crosshairs & mini tile key map for multi-sheet tiles
            if !fits_single {
                page.set_stroke(0.70, 0.74, 0.80, 0.20, true);
                page.fill_stroke_rect(area_x0, area_y0, avail_w, avail_h, None);
                page.draw_crosshair(area_x0 + 3.0, area_y0 + 3.0, 2.2);
                page.draw_crosshair(area_x0 + avail_w - 3.0, area_y0 + 3.0, 2.2);
                page.draw_crosshair(area_x0 + 3.0, area_y0 + avail_h - 3.0, 2.2);
                page.draw_crosshair(area_x0 + avail_w - 3.0, area_y0 + avail_h - 3.0, 2.2);

                // Mini tile grid indicator in top-right below subtitle
                let cell = 3.2;
                let map_x = page_w - margin - cols as f64 * cell - 2.0;
                let map_y = margin + 12.5;
                for mr in 0..rows {
                    for mc in 0..cols {
                        let fill = if mr == r && mc == c {
                            Some([0.96, 0.78, 0.42])
                        } else {
                            Some([0.94, 0.96, 0.98])
                        };
                        page.set_stroke(0.45, 0.52, 0.62, 0.2, false);
                        page.fill_stroke_rect(
                            map_x + mc as f64 * cell,
                            map_y + mr as f64 * cell,
                            cell,
                            cell,
                            fill,
                        );
                    }
                }
            }

            doc.pages.push(page);
        }
    }
}

fn append_packed_template_pages(
    doc: &mut PdfDocument,
    sheet_title: &str,
    cards: &[PackedTemplateCard],
    opts: &PdfExportOptions,
) {
    if cards.is_empty() {
        return;
    }

    let margin = opts.margin_mm.max(8.0);
    let max_card_w = cards.iter().map(|c| c.box_w).fold(0.0f64, f64::max);
    let max_card_h = cards.iter().map(|c| c.box_h).fold(0.0f64, f64::max);
    let prefer_land = max_card_w > max_card_h;
    let (page_w, page_h) = opts.page_dims_mm(prefer_land);

    let header_h = 13.0;
    let footer_h = 24.0;
    let avail_w = (page_w - 2.0 * margin).max(50.0);
    let avail_h = (page_h - 2.0 * margin - header_h - footer_h).max(50.0);
    let gap = 6.0;
    let any_acetate = cards.iter().any(|c| c.acetate_contour.is_some());

    let mode_note = match opts.instance_mode {
        PdfInstanceMode::UniqueWithQty => "Unique Parts + Qty",
        PdfInstanceMode::AllInstances => "All Instances",
    };

    let make_page = |sheet_idx: usize| -> PdfPage {
        PdfPage::new(
            page_w,
            page_h,
            format!("{sheet_title} - 1:1 Cutout & Acetate Sheet #{sheet_idx}"),
            format!("Layout: {mode_note}"),
            true,
            false,
            any_acetate,
        )
    };

    let mut page_num_local = 1usize;
    let mut page = make_page(page_num_local);
    let mut cursor_x = margin;
    let mut cursor_y = margin + header_h;
    let mut row_h = 0.0f64;

    for card in cards {
        if cursor_x > margin + 0.1 && cursor_x + card.box_w > page_w - margin {
            cursor_x = margin;
            cursor_y += row_h + gap;
            row_h = 0.0;
        }
        if cursor_y > margin + header_h + 0.1 && cursor_y + card.box_h > page_h - margin - footer_h
        {
            doc.pages.push(page);
            page_num_local += 1;
            page = make_page(page_num_local);
            cursor_x = margin;
            cursor_y = margin + header_h;
            row_h = 0.0;
        }

        draw_template_card(&mut page, cursor_x, cursor_y, card, opts.display_unit);
        cursor_x += card.box_w + gap;
        row_h = row_h.max(card.box_h);
    }

    let _ = avail_w;
    let _ = avail_h;
    doc.pages.push(page);
}

fn draw_template_card(
    page: &mut PdfPage,
    x0: f64,
    y0: f64,
    card: &PackedTemplateCard,
    unit: DisplayUnit,
) {
    page.set_stroke(0.84, 0.87, 0.92, 0.25, false);
    page.fill_stroke_rect(x0, y0, card.box_w, card.box_h, Some([0.992, 0.994, 0.997]));

    let header_txt = if card.qty > 1 {
        format!("{}  [Qty: x{}]", card.title, card.qty)
    } else {
        card.title.clone()
    };
    page.draw_text(
        x0 + 3.0,
        y0 + 4.5,
        &header_txt,
        2.9,
        true,
        false,
        [0.08, 0.12, 0.20],
    );
    page.draw_text(
        x0 + 3.0,
        y0 + 8.3,
        &format!(
            "{} | Cut: {}",
            card.style_name,
            unit.format_dims(card.opening_w, card.opening_h)
        ),
        2.3,
        false,
        false,
        [0.32, 0.36, 0.44],
    );

    let cut_area_w = if card.acetate_contour.is_some() {
        (card.box_w - 14.0) * 0.54
    } else {
        card.box_w - 6.0
    };
    let draw_top = y0 + 11.0;
    let draw_h = (card.box_h - 15.0).max(12.0);
    let cut_cx = x0 + 3.0 + cut_area_w * 0.5;
    let cut_cy = draw_top + draw_h * 0.5;

    page.set_stroke(0.533, 0.600, 0.667, 0.25, true);
    page.draw_contour(&card.trim_contour, cut_cx, cut_cy, None);
    page.set_stroke(0.851, 0.169, 0.169, 0.35, false);
    page.draw_contour(
        &card.cutout_contour,
        cut_cx,
        cut_cy,
        Some([0.992, 0.945, 0.945]),
    );

    if let Some((ref ac_c, aw, ah)) = card.acetate_contour {
        let ac_cx = x0 + 3.0 + cut_area_w + 8.0 + (card.box_w - cut_area_w - 14.0) * 0.5;
        let ac_cy = cut_cy;
        page.set_stroke(0.059, 0.431, 0.769, 0.35, false);
        page.draw_contour(ac_c, ac_cx, ac_cy, Some([0.941, 0.973, 0.992]));
        let ac_lbl = if card.acetate_qty > 1 {
            format!(
                "Acetate x{}: {}",
                card.acetate_qty,
                unit.format_dims(aw, ah)
            )
        } else {
            format!("Acetate: {}", unit.format_dims(aw, ah))
        };
        page.draw_text(
            ac_cx,
            y0 + card.box_h - 2.4,
            &ac_lbl,
            2.2,
            true,
            true,
            [0.059, 0.431, 0.769],
        );
    }
}

fn append_project_cover_bom_page(
    doc: &mut PdfDocument,
    project: &Project,
    cards: &[PackedTemplateCard],
    opts: &PdfExportOptions,
) {
    let margin = opts.margin_mm.max(10.0);
    let (page_w, page_h) = opts.page_dims_mm(false);
    let unit = opts.display_unit;
    let mut page = PdfPage::new(
        page_w,
        page_h,
        format!("{} - Project Parts Schedule & BOM", project.manifest.name),
        format!(
            "Kind: {} | Scale: {} | Display Unit: {}",
            project.manifest.kind.label(),
            project.manifest.scale_preset.label(),
            unit.label()
        ),
        false,
        false,
        false,
    );

    let mut y = margin + 18.0;
    let total_openings: usize = project.walls.iter().map(|w| w.openings.len()).sum();
    page.draw_text(
        margin,
        y,
        &format!(
            "Walls: {}   |   Total Openings: {}   |   Shared Components: {}   |   Standalone Pieces: {}",
            project.walls.len(),
            total_openings,
            project.components.len(),
            project.pieces.len()
        ),
        3.1,
        true,
        false,
        [0.12, 0.16, 0.22],
    );
    y += 8.0;

    if !project.walls.is_empty() {
        page.draw_text(
            margin,
            y,
            "Wall Schedule",
            3.4,
            true,
            false,
            [0.10, 0.20, 0.36],
        );
        y += 5.0;
        for w in &project.walls {
            page.draw_text(
                margin + 3.0,
                y,
                &format!(
                    "- {} ({} x {}, Thick: {}, Acetate: {}, Openings: {})",
                    w.name,
                    unit.format_value(w.width),
                    unit.format_value(w.height),
                    unit.format_value(w.wall_thickness),
                    unit.format_value(w.acetate_thickness),
                    w.openings.len()
                ),
                2.7,
                false,
                false,
                [0.24, 0.28, 0.35],
            );
            y += 4.6;
        }
        y += 5.0;
    }

    page.draw_text(
        margin,
        y,
        "Opening & Acetate Cutting Schedule (BOM)",
        3.4,
        true,
        false,
        [0.10, 0.20, 0.36],
    );
    y += 5.5;

    let col_x = [
        margin,
        margin + 44.0,
        margin + 84.0,
        margin + 120.0,
        margin + 154.0,
    ];
    page.set_stroke(0.75, 0.80, 0.88, 0.25, false);
    page.fill_stroke_rect(
        margin,
        y - 3.8,
        page_w - 2.0 * margin,
        5.5,
        Some([0.94, 0.96, 0.98]),
    );
    for (hdr, &x) in ["Item", "Style", "Cutout Size", "Acetate Cut", "Qty"]
        .iter()
        .zip(col_x.iter())
    {
        page.draw_text(x + 1.0, y, hdr, 2.7, true, false, [0.12, 0.16, 0.22]);
    }
    y += 5.8;

    for card in cards {
        if y > page_h - margin - 18.0 {
            break;
        }
        let ac_str = match &card.acetate_contour {
            Some((_, aw, ah)) => unit.format_dims(*aw, *ah),
            None => "Solid (None)".to_string(),
        };
        let qty_str = if card.acetate_qty > card.qty {
            format!("x{} (Ac x{})", card.qty, card.acetate_qty)
        } else {
            format!("x{}", card.qty)
        };
        page.draw_text(
            col_x[0] + 1.0,
            y,
            &card.title,
            2.5,
            true,
            false,
            [0.15, 0.20, 0.28],
        );
        page.draw_text(
            col_x[1] + 1.0,
            y,
            &card.style_name,
            2.5,
            false,
            false,
            [0.25, 0.30, 0.36],
        );
        page.draw_text(
            col_x[2] + 1.0,
            y,
            &unit.format_dims(card.opening_w, card.opening_h),
            2.5,
            false,
            false,
            [0.25, 0.30, 0.36],
        );
        page.draw_text(
            col_x[3] + 1.0,
            y,
            &ac_str,
            2.5,
            false,
            false,
            [0.06, 0.43, 0.77],
        );
        page.draw_text(
            col_x[4] + 1.0,
            y,
            &qty_str,
            2.5,
            true,
            false,
            [0.15, 0.20, 0.28],
        );
        if !card.walls_used.is_empty() {
            y += 3.8;
            page.draw_text(
                col_x[0] + 4.0,
                y,
                &format!("Walls: {}", card.walls_used.join(", ")),
                2.2,
                false,
                false,
                [0.45, 0.50, 0.58],
            );
        }
        y += 5.0;
    }

    doc.pages.push(page);
}

/// Generates a standard-paper 1:1 millimeter vector PDF for a single piece (A4 / US Letter / A3).
pub fn write_piece_pdf_bytes(
    window: &GeneratedWindow,
    title: &str,
    opts: &PdfExportOptions,
) -> Vec<u8> {
    let mut doc = PdfDocument::new(opts.margin_mm.max(8.0), opts.display_unit);
    let card = build_template_card(title.to_string(), 1, Vec::new(), window);
    append_packed_template_pages(&mut doc, title, &[card], opts);
    doc.into_pdf_bytes()
}

/// Generates a multi-page standard-paper 1:1 vector PDF for a single wall
/// (tiled 1:1 wall elevation + packed 1:1 cutout & acetate sheets).
pub fn write_wall_pdf_bytes(
    wall: &ProjectWall,
    components: &[ComponentDef],
    opts: &PdfExportOptions,
) -> Vec<u8> {
    let mut doc = PdfDocument::new(opts.margin_mm.max(8.0), opts.display_unit);
    if opts.include_wall_tiles {
        append_tiled_wall_pages(&mut doc, wall, components, opts);
    }
    if opts.include_cutout_and_acetate {
        let cards = collect_wall_cards(wall, components, opts.instance_mode);
        append_packed_template_pages(&mut doc, &wall.name, &cards, opts);
    }
    if doc.pages.is_empty() {
        append_tiled_wall_pages(&mut doc, wall, components, opts);
    }
    doc.into_pdf_bytes()
}

/// Generates a complete multi-page Project Booklet PDF across standard paper sizes
/// (Cover/BOM + tiled 1:1 elevations for all walls + auto-packed 1:1 cutout & acetate sheets).
pub fn write_project_booklet_pdf_bytes(project: &Project, opts: &PdfExportOptions) -> Vec<u8> {
    let mut doc = PdfDocument::new(opts.margin_mm.max(8.0), opts.display_unit);
    let cards: Vec<PackedTemplateCard> = match project.manifest.kind {
        ProjectKind::SinglePiece => project
            .pieces
            .iter()
            .map(|p| {
                let built = build_window(&p.params);
                build_template_card(p.name.clone(), 1, Vec::new(), &built)
            })
            .collect(),
        ProjectKind::MultiWall => {
            let wall_refs: Vec<(&ProjectWall, &[ComponentDef])> = project
                .walls
                .iter()
                .map(|w| (w, project.components.as_slice()))
                .collect();
            collect_multi_wall_cards(&wall_refs, opts.instance_mode)
        }
    };

    if opts.include_cover_bom {
        append_project_cover_bom_page(&mut doc, project, &cards, opts);
    }
    if opts.include_wall_tiles && project.manifest.kind == ProjectKind::MultiWall {
        for wall in &project.walls {
            append_tiled_wall_pages(&mut doc, wall, &project.components, opts);
        }
    }
    if opts.include_cutout_and_acetate {
        append_packed_template_pages(&mut doc, &project.manifest.name, &cards, opts);
    }
    if doc.pages.is_empty() {
        append_project_cover_bom_page(&mut doc, project, &cards, opts);
    }
    doc.into_pdf_bytes()
}

/// Estimates total PDF page count for live preview inside the Export dialog.
pub fn estimate_pdf_pages(
    project: &Project,
    active_wall_only: Option<&ProjectWall>,
    opts: &PdfExportOptions,
) -> usize {
    let mut doc = PdfDocument::new(opts.margin_mm.max(8.0), opts.display_unit);
    if let Some(wall) = active_wall_only {
        if opts.include_wall_tiles {
            append_tiled_wall_pages(&mut doc, wall, &project.components, opts);
        }
        if opts.include_cutout_and_acetate {
            let cards = collect_wall_cards(wall, &project.components, opts.instance_mode);
            append_packed_template_pages(&mut doc, &wall.name, &cards, opts);
        }
        return doc.pages.len().max(1);
    }
    let cards: Vec<PackedTemplateCard> = match project.manifest.kind {
        ProjectKind::SinglePiece => project
            .pieces
            .iter()
            .map(|p| {
                let built = build_window(&p.params);
                build_template_card(p.name.clone(), 1, Vec::new(), &built)
            })
            .collect(),
        ProjectKind::MultiWall => {
            let wall_refs: Vec<(&ProjectWall, &[ComponentDef])> = project
                .walls
                .iter()
                .map(|w| (w, project.components.as_slice()))
                .collect();
            collect_multi_wall_cards(&wall_refs, opts.instance_mode)
        }
    };
    if opts.include_cover_bom {
        doc.pages
            .push(PdfPage::new(210.0, 297.0, "", "", false, false, false));
    }
    if opts.include_wall_tiles && project.manifest.kind == ProjectKind::MultiWall {
        for wall in &project.walls {
            append_tiled_wall_pages(&mut doc, wall, &project.components, opts);
        }
    }
    if opts.include_cutout_and_acetate {
        append_packed_template_pages(&mut doc, &project.manifest.name, &cards, opts);
    }
    doc.pages.len().max(1)
}

pub fn export_piece_pdf(
    window: &GeneratedWindow,
    title: &str,
    opts: &PdfExportOptions,
    path: &Path,
) -> std::io::Result<()> {
    let bytes = write_piece_pdf_bytes(window, title, opts);
    fs::write(path, bytes)
}

pub fn export_wall_pdf(
    wall: &ProjectWall,
    components: &[ComponentDef],
    opts: &PdfExportOptions,
    path: &Path,
) -> std::io::Result<()> {
    let bytes = write_wall_pdf_bytes(wall, components, opts);
    fs::write(path, bytes)
}

pub fn export_project_booklet_pdf(
    project: &Project,
    opts: &PdfExportOptions,
    path: &Path,
) -> std::io::Result<()> {
    let bytes = write_project_booklet_pdf_bytes(project, opts);
    fs::write(path, bytes)
}

fn pdf_sanitize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '×' => out.push('x'),
            '—' | '–' => out.push('-'),
            '✔' => out.push('+'),
            c if c.is_ascii() => out.push(c),
            _ => out.push(' '),
        }
    }
    out
}

fn pdf_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            c => out.push(c),
        }
    }
    out
}
