use crate::builders::door::{DoorLeafAsset, build_door_assembly};
use crate::extrude::{
    HingeFace, MouldingConfig, Slab, TriMesh, VGroove, build_cone_knuckle_block, extrude_slabs,
    extrude_slabs_with_moulding,
};
use crate::polygon2d::{
    Contour2D, ProfileKind, Shapes2D, circle_contour, difference_shapes, intersection_shapes,
    profile_band, profile_contour, rect_bounds, rect_centered, ring_shape, segment_bar,
    shape_from_contour, union_contours, union_shapes,
};
use dh_core::{
    FlushTopMode, GrooveStyle, HeaderStyle, ShutterStyle, WindowOpeningMode, WindowParams,
    WindowSpecs, WindowStyle,
};
use std::f64::consts::PI;

const ARC_SEGMENTS: usize = 36;

/// Hinged print-in-place window sash (for casement, double casement, awning, or hopper modes).
#[derive(Debug, Clone)]
pub struct WindowSashAsset {
    /// Watertight zero-overhang sash mesh positioned in-place at `Z = 0` inside `exterior_half`.
    pub mesh: TriMesh,
    /// Cut-to-size rectangular acetate pane for this sash (`z in [0, acetate_thickness]`).
    pub sash_acetate: TriMesh,
    /// Sash center in window XY coordinates (`(0, 0)` for single sashes).
    pub center_xy: [f32; 2],
    /// Cone-hinge pivot point `[x, y, z]` in exterior-half coordinates.
    pub pivot_origin: [f32; 3],
    /// Unit rotation axis (`[0, 1, 0]` for casements, `[1, 0, 0]` for awning/hopper).
    pub pivot_axis: [f32; 3],
    /// Rotation sign (`+1.0` or `-1.0`) so positive `door_open_deg` swings the sash cleanly outward.
    pub swing_dir: f32,
}

/// Generated 3D assets for a window or hinged door configuration.
#[derive(Debug, Clone)]
pub struct GeneratedWindow {
    pub params: WindowParams,
    pub specs: WindowSpecs,
    /// True when this design is a hinged door + frame + handle.
    pub is_door: bool,
    /// Exterior half (or exterior door frame half) printed outside-down (`z = 0` on bed).
    pub exterior_half: TriMesh,
    /// Interior half (or interior door frame half) printed outside-down (`z = 0` on bed).
    pub interior_half: TriMesh,
    /// Cut-to-size acetate sheet mesh (centered at XY = `(0, 0)`, `z in [0, acetate_thickness]`).
    pub acetate_sheet: TriMesh,
    /// Translucent dollhouse wall section with cutout for "Assembled in Wall" preview.
    pub wall_cutaway: TriMesh,
    /// Hinged door leaves (empty for fixed windows).
    pub door_leaves: Vec<DoorLeafAsset>,
    /// Hinged window sashes printed in place inside `exterior_half` (empty for fixed windows & doors).
    pub window_sashes: Vec<WindowSashAsset>,
    /// Separately printable door handle + escutcheon + letterbox/cat-flap set (empty for windows).
    pub door_handle: TriMesh,
    /// Combined print plate mesh (all printable parts arranged side-by-side at `z = 0`).
    pub print_plate: TriMesh,
    /// Exterior half diagnostic summary.
    pub ext_diag: String,
    /// Interior half diagnostic summary.
    pub int_diag: String,
    /// True if all printable parts passed strict watertight & zero-overhang checks.
    pub printable_ok: bool,
}

impl GeneratedWindow {
    /// Returns the exterior half combined with any print-in-place opening window sashes so
    /// exporting STL or batch-packing prints the complete print-in-place frame + hinged sash assembly.
    pub fn printable_exterior_assembly(&self) -> TriMesh {
        if self.window_sashes.is_empty() {
            return self.exterior_half.clone();
        }
        let mut out = self.exterior_half.clone();
        for sash in &self.window_sashes {
            out.append(&sash.mesh);
        }
        out
    }
}

pub fn profile_kind(style: WindowStyle) -> ProfileKind {
    match style {
        WindowStyle::ArchedGeorgian | WindowStyle::DoorArchedHeritage => ProfileKind::RoundArch,
        WindowStyle::HalfRoundLunette => ProfileKind::HalfRoundArch,
        WindowStyle::GothicPointed => ProfileKind::GothicArch,
        WindowStyle::CircularBullsEye => ProfileKind::CircleOrEllipse,
        _ => ProfileKind::Rectangle,
    }
}

/// Returns `(clip_x0, clip_x1, clip_y0, clip_y1)` in centered opening coordinates for clipping
/// exterior trim and add-ons around pre-existing flush wall features.
/// Unclipped bounds extend to `±pad`. Returns `None` if flush settings remove all exterior trim
/// (e.g., `FlushTopMode::FullContour` on a circular opening).
pub(crate) fn flush_clip_bounds(
    kind: ProfileKind,
    w: f64,
    h: f64,
    params: &WindowParams,
) -> Option<(f64, f64, f64, f64)> {
    let pad = params.trim_width
        + params.wall_fit_clearance
        + params.sill_overhang
        + params.sill_depth
        + 120.0;
    let setback = if params.flush_trim_setback < 0.05 {
        0.0
    } else {
        params.flush_trim_setback
    };

    let x0 = if params.flush_left_jamb {
        -w * 0.5 + setback
    } else {
        -pad
    };
    let x1 = if params.flush_right_jamb {
        w * 0.5 - setback
    } else {
        pad
    };
    let y0 = if params.flush_bottom_sill {
        -h * 0.5 + setback
    } else {
        -pad
    };
    let y1 = match params.flush_top_mode {
        FlushTopMode::None => pad,
        FlushTopMode::HorizontalHead => h * 0.5 - setback,
        FlushTopMode::FullContour => match kind {
            ProfileKind::Rectangle => h * 0.5 - setback,
            ProfileKind::RoundArch => {
                let ry = (w * 0.5).min(h * 0.72);
                (h * 0.5 - ry) - setback
            }
            ProfileKind::GothicArch => {
                let h_arch = (w * 0.72).min(h * 0.76).max(w * 0.52);
                (h * 0.5 - h_arch) - setback
            }
            ProfileKind::HalfRoundArch => -h * 0.5 - setback,
            ProfileKind::CircleOrEllipse => return None,
        },
    };

    if x1 <= x0 + 0.05 || y1 <= y0 + 0.05 {
        return None;
    }
    Some((x0, x1, y0, y1))
}

/// Clips an exterior trim + add-ons 2D region against pre-existing flush wall features and
/// unions it back with `frame_band` so the sleeve/stop frame inside the wall hole stays
/// intact and single-component.
pub(crate) fn clip_exterior_trim_for_flush_wall(
    trim_with_addons: &Shapes2D,
    frame_band: &Shapes2D,
    kind: ProfileKind,
    w: f64,
    h: f64,
    params: &WindowParams,
) -> Shapes2D {
    if !params.has_any_flush_edge() {
        return trim_with_addons.clone();
    }
    let Some((x0, x1, y0, y1)) = flush_clip_bounds(kind, w, h, params) else {
        return frame_band.clone();
    };
    let clip_box = shape_from_contour(rect_bounds(x0, y0, x1, y1));
    let clipped = intersection_shapes(trim_with_addons, &clip_box);
    union_shapes(&clipped, frame_band)
}

/// Returns the outer 2D reference contour (centered on `(0, 0)`) of an architrave of width
/// `trim_outset` around `(w, h)`, clipped against any active pre-existing flush wall features.
pub fn clipped_trim_outer_contour(
    kind: ProfileKind,
    w: f64,
    h: f64,
    trim_outset: f64,
    params: &WindowParams,
    segments: usize,
) -> Contour2D {
    let base_outer = profile_contour(kind, w, h, trim_outset, segments);
    if !params.has_any_flush_edge() {
        return base_outer;
    }
    let Some((x0, x1, y0, y1)) = flush_clip_bounds(kind, w, h, params) else {
        return profile_contour(kind, w, h, 0.0, segments);
    };
    let outer_shape = shape_from_contour(base_outer);
    let core_shape = shape_from_contour(profile_contour(kind, w, h, 0.0, segments));
    let clip_box = shape_from_contour(rect_bounds(x0, y0, x1, y1));
    let combined = union_shapes(&intersection_shapes(&outer_shape, &clip_box), &core_shape);
    combined
        .into_iter()
        .filter_map(|mut s| (!s.is_empty()).then(|| s.remove(0)))
        .max_by(|a, b| a.len().cmp(&b.len()))
        .unwrap_or_else(|| profile_contour(kind, w, h, 0.0, segments))
}

/// Builds the preview wall cutaway mesh (`z in [0, effective_wall_thickness]`) plus optional
/// proud pre-existing flush wall bosses (`z in [-boss_depth, 0]`) when any flush edge is active.
pub(crate) fn build_wall_cutaway_mesh(
    kind: ProfileKind,
    params: &WindowParams,
    specs: &WindowSpecs,
) -> TriMesh {
    let wall_outer_w = params.opening_width + 2.0 * (params.trim_width + 16.0);
    let wall_outer_h = params.opening_height + 2.0 * (params.trim_width + 14.0);
    let wall_outer = shape_from_contour(rect_centered(0.0, 0.0, wall_outer_w, wall_outer_h));
    let wall_cutout = shape_from_contour(profile_contour(
        kind,
        params.opening_width,
        params.opening_height,
        0.0,
        ARC_SEGMENTS,
    ));
    let wall_region = difference_shapes(&wall_outer, &wall_cutout);
    let mut wall_cutaway = extrude_slabs(&[Slab {
        z0: 0.0,
        z1: specs.effective_wall_thickness,
        region: wall_region,
    }]);

    if params.has_any_flush_edge() {
        let w0 = params.opening_width;
        let h0 = params.opening_height;
        let boss_band = (params.trim_width * 0.95 + 2.5).clamp(4.0, 12.0);
        let ear = params.flush_trim_setback.clamp(2.5, 8.0);
        let boss_depth = params.trim_thickness.clamp(1.5, 4.5);
        let mut boss_shapes: Shapes2D = Vec::new();

        if params.flush_bottom_sill {
            let sill_shape = shape_from_contour(rect_bounds(
                -w0 * 0.5 - ear,
                -h0 * 0.5 - boss_band,
                w0 * 0.5 + ear,
                -h0 * 0.5,
            ));
            boss_shapes = union_shapes(&boss_shapes, &sill_shape);
        }
        match params.flush_top_mode {
            FlushTopMode::None => {}
            FlushTopMode::HorizontalHead => {
                let head_shape = shape_from_contour(rect_bounds(
                    -w0 * 0.5 - ear,
                    h0 * 0.5,
                    w0 * 0.5 + ear,
                    h0 * 0.5 + boss_band,
                ));
                boss_shapes = union_shapes(&boss_shapes, &head_shape);
            }
            FlushTopMode::FullContour => {
                let ring = profile_band(kind, w0, h0, 0.0, boss_band, ARC_SEGMENTS);
                let top_band = match arch_spring_top_y(kind, w0, h0) {
                    Some(spring_y) => {
                        let mask = shape_from_contour(rect_bounds(
                            -wall_outer_w * 0.5,
                            spring_y,
                            wall_outer_w * 0.5,
                            wall_outer_h * 0.5,
                        ));
                        intersection_shapes(&ring, &mask)
                    }
                    None if kind == ProfileKind::HalfRoundArch => {
                        let mask = shape_from_contour(rect_bounds(
                            -wall_outer_w * 0.5,
                            -h0 * 0.5,
                            wall_outer_w * 0.5,
                            wall_outer_h * 0.5,
                        ));
                        intersection_shapes(&ring, &mask)
                    }
                    None => ring,
                };
                boss_shapes = union_shapes(&boss_shapes, &top_band);
            }
        }
        if params.flush_left_jamb {
            let lj = shape_from_contour(rect_bounds(
                -w0 * 0.5 - boss_band,
                -h0 * 0.5 - ear * 0.5,
                -w0 * 0.5,
                h0 * 0.5 + ear * 0.5,
            ));
            boss_shapes = union_shapes(&boss_shapes, &lj);
        }
        if params.flush_right_jamb {
            let rj = shape_from_contour(rect_bounds(
                w0 * 0.5,
                -h0 * 0.5 - ear * 0.5,
                w0 * 0.5 + boss_band,
                h0 * 0.5 + ear * 0.5,
            ));
            boss_shapes = union_shapes(&boss_shapes, &rj);
        }

        let clipped_boss = difference_shapes(
            &intersection_shapes(&boss_shapes, &wall_outer),
            &wall_cutout,
        );
        if !clipped_boss.is_empty() {
            let boss_mesh = extrude_slabs(&[Slab {
                z0: -boss_depth,
                z1: 0.0,
                region: clipped_boss,
            }]);
            wall_cutaway.append(&boss_mesh);
        }
    }

    wall_cutaway
}

fn arch_spring_top_y(kind: ProfileKind, w: f64, h: f64) -> Option<f64> {
    match kind {
        ProfileKind::Rectangle => Some(h * 0.5),
        ProfileKind::RoundArch => Some(h * 0.5 - (w * 0.5).min(h * 0.72)),
        ProfileKind::GothicArch => Some(h * 0.5 - (w * 0.72).min(h * 0.76).max(w * 0.52)),
        ProfileKind::HalfRoundArch | ProfileKind::CircleOrEllipse => None,
    }
}

fn build_architrave_grooves(
    kind: ProfileKind,
    w: f64,
    h: f64,
    trim_w: f64,
    chamfer_inset: f64,
    params: &WindowParams,
    is_exterior: bool,
) -> Vec<VGroove> {
    if params.groove_style == GrooveStyle::None {
        return Vec::new();
    }
    let gw = params.groove_width.clamp(0.4, 1.8);
    let gd = gw * 0.58;
    let usable_start = gw * 0.7;
    let usable_end = trim_w - chamfer_inset - gw * 0.7;
    if usable_end <= usable_start + 0.3 {
        return Vec::new();
    }

    if is_exterior && params.has_any_flush_edge() {
        let Some((cx0, cx1, cy0, cy1)) = flush_clip_bounds(kind, w, h, params) else {
            return Vec::new();
        };
        let end_clr = chamfer_inset + gw * 0.75 + 0.5;
        let fit = params.wall_fit_clearance;
        let band_t = (gw + 0.75).max(1.2);
        let req_span = band_t + gw + 0.4;
        let avail_t = usable_end - usable_start;
        if avail_t < req_span {
            return Vec::new();
        }
        let mid_off = (usable_start + usable_end) * 0.5 + fit * 0.5;
        let band_offsets: Vec<f64> = match params.groove_style {
            GrooveStyle::None => Vec::new(),
            GrooveStyle::SingleVGroove => vec![mid_off],
            GrooveStyle::DoubleVGroove => {
                if avail_t >= req_span * 2.0 + gw + 0.8 {
                    let q1 = usable_start + avail_t * 0.26 + fit * 0.5;
                    let q2 = usable_start + avail_t * 0.74 + fit * 0.5;
                    vec![q1, q2]
                } else {
                    vec![mid_off]
                }
            }
        };

        let mut grooves = Vec::new();
        // Straight side pilasters
        if let Some(jamb_top) = arch_spring_top_y(kind, w, h) {
            let py0 = (-h * 0.5).max(cy0) + end_clr;
            let py1 = jamb_top.min(cy1) - end_clr;
            if py1 - py0 >= gw + 1.2 {
                for &off in &band_offsets {
                    if !params.flush_left_jamb {
                        let cx = -(w * 0.5 + off);
                        grooves.push(VGroove {
                            centerline: rect_centered(cx, (py0 + py1) * 0.5, band_t, py1 - py0),
                            width: gw,
                            depth: gd,
                        });
                    }
                    if !params.flush_right_jamb {
                        let cx = w * 0.5 + off;
                        grooves.push(VGroove {
                            centerline: rect_centered(cx, (py0 + py1) * 0.5, band_t, py1 - py0),
                            width: gw,
                            depth: gd,
                        });
                    }
                }
            }
        }

        // Unclipped horizontal top band (rectangular frames)
        if kind == ProfileKind::Rectangle && params.flush_top_mode == FlushTopMode::None {
            let px0 = (-w * 0.5).max(cx0) + end_clr;
            let px1 = (w * 0.5).min(cx1) - end_clr;
            if px1 - px0 >= gw + 1.2 {
                for &off in &band_offsets {
                    let cy = h * 0.5 + off;
                    grooves.push(VGroove {
                        centerline: rect_centered((px0 + px1) * 0.5, cy, px1 - px0, band_t),
                        width: gw,
                        depth: gd,
                    });
                }
            }
        }

        // Unclipped horizontal bottom band (non-circular frames when sill is off)
        if !params.flush_bottom_sill
            && kind != ProfileKind::CircleOrEllipse
            && !(params.sill_enabled && params.style.supports_sill())
        {
            let px0 = (-w * 0.5).max(cx0) + end_clr;
            let px1 = (w * 0.5).min(cx1) - end_clr;
            if px1 - px0 >= gw + 1.2 {
                for &off in &band_offsets {
                    let cy = -(h * 0.5 + off);
                    grooves.push(VGroove {
                        centerline: rect_centered((px0 + px1) * 0.5, cy, px1 - px0, band_t),
                        width: gw,
                        depth: gd,
                    });
                }
            }
        }

        return grooves;
    }

    let offsets: Vec<f64> = match params.groove_style {
        GrooveStyle::None => Vec::new(),
        GrooveStyle::SingleVGroove => vec![(usable_start + usable_end) * 0.5],
        GrooveStyle::DoubleVGroove => {
            if usable_end - usable_start >= gw * 2.2 {
                vec![
                    usable_start + (usable_end - usable_start) * 0.25,
                    usable_start + (usable_end - usable_start) * 0.75,
                ]
            } else {
                vec![(usable_start + usable_end) * 0.5]
            }
        }
    };

    offsets
        .into_iter()
        .map(|off| VGroove {
            centerline: profile_contour(
                kind,
                w,
                h,
                off + params.wall_fit_clearance * 0.5,
                ARC_SEGMENTS,
            ),
            width: gw,
            depth: gd,
        })
        .collect()
}

/// Builds overhang-free exterior architectural add-ons (shutters, headers/pediments, flower boxes)
/// plus optional front-face V-grooves so they union cleanly into the base exterior slab.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_exterior_addons(
    kind: ProfileKind,
    w: f64,
    h: f64,
    trim_w: f64,
    chamfer_inset: f64,
    params: &WindowParams,
    step_inset: f64,
    ext_grooves: Option<&mut Vec<VGroove>>,
) -> Shapes2D {
    let mut contours: Vec<Contour2D> = Vec::new();
    let fit_clr = params.wall_fit_clearance;
    let x_trim = w * 0.5 + trim_w + fit_clr;
    let y_trim_top = h * 0.5 + trim_w + fit_clr;
    let sill_active =
        params.sill_enabled && params.style.supports_sill() && !params.flush_bottom_sill;
    let clip_bounds = if params.has_any_flush_edge() {
        flush_clip_bounds(kind, w, h, params)
    } else {
        Some((-1e6, 1e6, -1e6, 1e6))
    };

    // 1. Exterior Shutters (for styles with vertical jambs, omitting sides marked flush)
    if !params.style.is_door()
        && params.shutter_style != ShutterStyle::None
        && (!params.flush_left_jamb || !params.flush_right_jamb)
        && let (Some(y_sh_top_raw), Some((_, _, cy0, cy1))) =
            (arch_spring_top_y(kind, w, h), clip_bounds)
    {
        let y_sh_bot_base = if sill_active {
            -params.opening_height * 0.5
        } else {
            -h * 0.5
        };
        let y_sh_bot = y_sh_bot_base.max(cy0);
        let y_sh_top = (y_sh_top_raw - step_inset * 0.25).min(cy1);
        let sh_w = (params.opening_width * params.shutter_width_ratio - step_inset * 0.45)
            .clamp(6.0, 45.0);
        if y_sh_top > y_sh_bot + 10.0 && sh_w >= 6.0 {
            // Overlap 0.45mm inward into the architrave so union stays single-component
            let mut side_centers = Vec::new();
            if !params.flush_left_jamb {
                contours.push(rect_bounds(
                    -x_trim - sh_w,
                    y_sh_bot,
                    -x_trim + 0.45,
                    y_sh_top,
                ));
                side_centers.push(-x_trim - sh_w * 0.5);
            }
            if !params.flush_right_jamb {
                contours.push(rect_bounds(
                    x_trim - 0.45,
                    y_sh_bot,
                    x_trim + sh_w,
                    y_sh_top,
                ));
                side_centers.push(x_trim + sh_w * 0.5);
            }

            if let Some(grooves) = ext_grooves {
                let gw = params.groove_width.clamp(0.45, 1.0);
                let gd = gw * 0.58;
                let mx = (chamfer_inset + gw * 0.75 + 1.0).max(2.0);
                let my = (chamfer_inset + gw * 0.75 + 1.2).max(2.4);
                let inner_pw = sh_w - 2.0 * mx;
                let inner_ph = (y_sh_top - y_sh_bot) - 2.0 * my;

                if inner_pw >= gw + 1.2 && inner_ph >= gw + 1.6 {
                    for side_cx in side_centers {
                        let cy_mid = (y_sh_bot + y_sh_top) * 0.5;
                        match params.shutter_style {
                            ShutterStyle::None => {}
                            ShutterStyle::Panelled => {
                                let gap = (gw + 1.4).max(2.2);
                                if inner_ph >= (gw + 1.2) * 2.0 + gap {
                                    let ph = (inner_ph - gap) * 0.5;
                                    let upper_cy = cy_mid + gap * 0.5 + ph * 0.5;
                                    let lower_cy = cy_mid - gap * 0.5 - ph * 0.5;
                                    grooves.push(VGroove {
                                        centerline: rect_centered(side_cx, upper_cy, inner_pw, ph),
                                        width: gw,
                                        depth: gd,
                                    });
                                    grooves.push(VGroove {
                                        centerline: rect_centered(side_cx, lower_cy, inner_pw, ph),
                                        width: gw,
                                        depth: gd,
                                    });
                                } else {
                                    grooves.push(VGroove {
                                        centerline: rect_centered(
                                            side_cx, cy_mid, inner_pw, inner_ph,
                                        ),
                                        width: gw,
                                        depth: gd,
                                    });
                                }
                            }
                            ShutterStyle::Louvred => {
                                let min_slot_h = gw + 1.1;
                                let min_gap = gw + 0.9;
                                let max_n = ((inner_ph + min_gap) / (min_slot_h + min_gap)).floor()
                                    as usize;
                                let n = max_n.clamp(1, 6);
                                if n >= 2 {
                                    let total_gaps = (n - 1) as f64 * min_gap;
                                    let slot_h = (inner_ph - total_gaps) / n as f64;
                                    if slot_h >= min_slot_h {
                                        let y_start = y_sh_bot + my;
                                        for k in 0..n {
                                            let sy = y_start
                                                + k as f64 * (slot_h + min_gap)
                                                + slot_h * 0.5;
                                            grooves.push(VGroove {
                                                centerline: rect_centered(
                                                    side_cx, sy, inner_pw, slot_h,
                                                ),
                                                width: gw,
                                                depth: gd,
                                            });
                                        }
                                    }
                                } else {
                                    grooves.push(VGroove {
                                        centerline: rect_centered(
                                            side_cx, cy_mid, inner_pw, inner_ph,
                                        ),
                                        width: gw,
                                        depth: gd,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Classical Headers / Pediments (rectangular frames, suppressed when top is flush)
    if kind == ProfileKind::Rectangle
        && params.header_style != HeaderStyle::None
        && params.flush_top_mode == FlushTopMode::None
    {
        let lintel_overhang = (2.5 - step_inset * 0.4).max(0.8);
        let lintel_half_w = x_trim + lintel_overhang;
        let lintel_h = ((trim_w * 0.65).max(2.6) - step_inset * 0.35).max(1.4);
        let y0 = y_trim_top - 0.5;
        let y_lintel_top = y_trim_top + lintel_h;

        match params.header_style {
            HeaderStyle::None => {}
            HeaderStyle::FlatLintel => {
                contours.push(rect_bounds(-lintel_half_w, y0, lintel_half_w, y_lintel_top));
            }
            HeaderStyle::KeystoneArch => {
                contours.push(rect_bounds(-lintel_half_w, y0, lintel_half_w, y_lintel_top));
                let ks_half_w = (trim_w * 0.78 - step_inset * 0.3).max(1.8);
                let ks_top = y_trim_top + ((trim_w * 1.15).max(4.2) - step_inset * 0.35).max(2.2);
                contours.push(rect_bounds(-ks_half_w, y0, ks_half_w, ks_top));
            }
            HeaderStyle::TriangularPediment => {
                let cornice_top = y_trim_top + (1.6 - step_inset * 0.3).max(0.8);
                contours.push(rect_bounds(-lintel_half_w, y0, lintel_half_w, cornice_top));
                let ped_h = ((w * 0.22).clamp(6.0, 24.0) - step_inset * 0.4).max(4.0);
                let ped_y0 = cornice_top - 0.4;
                contours.push(vec![
                    [-lintel_half_w, ped_y0],
                    [lintel_half_w, ped_y0],
                    [0.0, cornice_top + ped_h],
                ]);
            }
        }
    }

    // 3. Sill Flower / Planter Box
    if !params.style.is_door() && sill_active && params.flower_box_enabled {
        let y_sill_bot = -params.opening_height * 0.5 - trim_w - params.sill_depth;
        let fb_w = (params.opening_width + 2.0 * (trim_w * 0.55) - step_inset * 0.8).max(10.0);
        let fb_h = ((params.opening_height * 0.14).clamp(6.0, 16.0) - step_inset * 0.4).max(4.0);
        contours.push(rect_bounds(
            -fb_w * 0.5,
            y_sill_bot - fb_h,
            fb_w * 0.5,
            y_sill_bot + 0.5,
        ));
    }

    union_contours(&contours)
}

/// Builds the complete 3D meshes for the given window or hinged door parameters.
pub fn build_window(raw_params: &WindowParams) -> GeneratedWindow {
    let params = raw_params.sanitized();
    let mut specs = params.compute_specs();
    let kind = profile_kind(params.style);

    let w = specs.sleeve_outer_width;
    let h = specs.sleeve_outer_height;
    let barrel_w = specs.barrel_wall_thickness;

    let ext_outer_chamfer = if specs.effective_ext_chamfer >= 0.15 {
        Some((
            specs.effective_ext_chamfer,
            specs.effective_ext_chamfer * 1.15,
        ))
    } else {
        None
    };
    let int_outer_chamfer = if specs.effective_int_chamfer >= 0.15 {
        Some((
            specs.effective_int_chamfer,
            specs.effective_int_chamfer * 1.15,
        ))
    } else {
        None
    };

    let bar_c = if params.chamfer_bars {
        let max_bc = (params.bar_width * 0.30).min(0.55);
        let c = params.bar_chamfer_size.min(max_bc);
        if c >= 0.10 { Some((c, c * 1.15)) } else { None }
    } else {
        None
    };

    let mut ext_grooves = build_architrave_grooves(
        kind,
        w,
        h,
        params.trim_width,
        specs.effective_ext_chamfer,
        &params,
        true,
    );
    let int_grooves = if params.chamfer_interior {
        build_architrave_grooves(
            kind,
            w,
            h,
            params.interior_trim_width,
            specs.effective_int_chamfer,
            &params,
            false,
        )
    } else {
        Vec::new()
    };

    let ext_addons = build_exterior_addons(
        kind,
        w,
        h,
        params.trim_width,
        specs.effective_ext_chamfer,
        &params,
        0.0,
        Some(&mut ext_grooves),
    );

    let ext_moulding = MouldingConfig {
        outer_chamfer: ext_outer_chamfer,
        hole_chamfer: bar_c,
        grooves: ext_grooves,
    };
    let int_moulding = MouldingConfig {
        outer_chamfer: int_outer_chamfer,
        hole_chamfer: bar_c,
        grooves: int_grooves,
    };

    if params.style.is_door() {
        let door_out = build_door_assembly(&params, &specs, &ext_moulding, &int_moulding);
        if door_out.acetate_cut_w > 0.5 {
            specs.acetate_cut_width = door_out.acetate_cut_w;
            specs.acetate_cut_height = door_out.acetate_cut_h;
            specs.acetate_is_rectangular = true;
        }

        // Arrange all printable parts side-by-side on Z=0 for the combined print plate
        let mut parts: Vec<TriMesh> = Vec::new();
        parts.push(door_out.exterior_frame.clone());
        parts.push(door_out.interior_frame.clone());
        for leaf in &door_out.door_leaves {
            parts.push(leaf.mesh.clone());
            if let Some(ext_leaf) = &leaf.exterior_leaf {
                parts.push(ext_leaf.clone());
            }
        }
        if !door_out.door_handle.triangles.is_empty() {
            parts.push(door_out.door_handle.clone());
        }

        let print_plate = pack_meshes_on_bed(&parts, params.plate_spacing as f32);

        let (ext_diag, ext_ok) = match door_out.exterior_frame.verify_printable_manifold() {
            Ok(d) => (
                format!(
                    "{} tris | watertight | {} overhangs",
                    d.triangle_count, d.overhang_triangles
                ),
                d.is_watertight && d.overhang_triangles == 0,
            ),
            Err(e) => (e, false),
        };
        let (int_diag, int_ok) = match door_out.interior_frame.verify_printable_manifold() {
            Ok(d) => (
                format!(
                    "{} tris | watertight | {} overhangs",
                    d.triangle_count, d.overhang_triangles
                ),
                d.is_watertight && d.overhang_triangles == 0,
            ),
            Err(e) => (e, false),
        };
        let mut leaf_err: Option<String> = None;
        for (idx, leaf) in door_out.door_leaves.iter().enumerate() {
            if let Err(e) = leaf.mesh.verify_printable_manifold() {
                leaf_err = Some(format!("Door leaf {idx}: {e}"));
                break;
            }
            if let Some(ext_l) = &leaf.exterior_leaf
                && let Err(e) = ext_l.verify_printable_manifold()
            {
                leaf_err = Some(format!("Door ext leaf {idx}: {e}"));
                break;
            }
        }
        if let Some(err_msg) = leaf_err {
            return GeneratedWindow {
                params,
                specs,
                is_door: true,
                exterior_half: door_out.exterior_frame,
                interior_half: door_out.interior_frame,
                acetate_sheet: TriMesh::default(),
                wall_cutaway: door_out.wall_cutaway,
                door_leaves: door_out.door_leaves,
                window_sashes: Vec::new(),
                door_handle: door_out.door_handle,
                print_plate,
                ext_diag,
                int_diag: err_msg,
                printable_ok: false,
            };
        }
        let handle_ok = door_out.door_handle.triangles.is_empty()
            || door_out
                .door_handle
                .verify_printable_manifold()
                .is_ok_and(|d| d.is_watertight && d.overhang_triangles == 0);

        return GeneratedWindow {
            params,
            specs,
            is_door: true,
            exterior_half: door_out.exterior_frame,
            interior_half: door_out.interior_frame,
            acetate_sheet: TriMesh::default(),
            wall_cutaway: door_out.wall_cutaway,
            door_leaves: door_out.door_leaves,
            window_sashes: Vec::new(),
            door_handle: door_out.door_handle,
            print_plate,
            ext_diag,
            int_diag,
            printable_ok: ext_ok && int_ok && handle_ok,
        };
    }

    // Full frame from glass edge to sleeve outer edge (also used as core region for flush clipping)
    let ext_frame_band = profile_band(kind, w, h, -params.frame_width, 0.0, ARC_SEGMENTS);

    // 1. Exterior architrave + optional bottom sill + v2 exterior add-ons (at z = 0..trim_thickness)
    let ext_trim_band = profile_band(
        kind,
        w,
        h,
        -params.frame_width,
        params.trim_width + params.wall_fit_clearance,
        ARC_SEGMENTS,
    );
    let ext_trim_with_sill =
        if params.sill_enabled && params.style.supports_sill() && !params.flush_bottom_sill {
            let sill_w = params.opening_width + 2.0 * (params.trim_width + params.sill_overhang);
            let y_top = -params.opening_height * 0.5;
            let y_bot = y_top - params.trim_width - params.sill_depth;
            let sill_rect =
                shape_from_contour(rect_bounds(-sill_w * 0.5, y_bot, sill_w * 0.5, y_top));
            union_shapes(&ext_trim_band, &sill_rect)
        } else {
            ext_trim_band
        };
    let ext_trim_with_addons = clip_exterior_trim_for_flush_wall(
        &union_shapes(&ext_trim_with_sill, &ext_addons),
        &ext_frame_band,
        kind,
        w,
        h,
        &params,
    );

    // 2. Interior architrave (at z = 0..interior_trim_thickness)
    let int_spigot_outer_offset = -(barrel_w + params.index_clearance);
    let int_trim_band = profile_band(
        kind,
        w,
        h,
        -params.frame_width,
        params.interior_trim_width + params.wall_fit_clearance,
        ARC_SEGMENTS,
    );

    // 3. Structural frame bands (outer barrel & interior spigot)
    let ext_barrel_band = profile_band(kind, w, h, -barrel_w, 0.0, ARC_SEGMENTS);
    // Interior telescopic spigot band (fits inside `ext_barrel_band` with `index_clearance`)
    let int_spigot_band = profile_band(
        kind,
        w,
        h,
        -params.frame_width,
        int_spigot_outer_offset,
        ARC_SEGMENTS,
    );

    // 4. Glazing bars / mullions / lattice / tracery clipped to the daylight glass opening
    let opening_mode = params.effective_opening_mode();
    let open_w = w - 2.0 * params.frame_width;
    let open_h = h - 2.0 * params.frame_width;
    let use_opening_sash = match opening_mode {
        WindowOpeningMode::Fixed => false,
        WindowOpeningMode::CasementDouble => open_w >= 22.0 && open_h >= 12.0,
        _ => open_w >= 12.0 && open_h >= 12.0,
    };

    let bars_clipped = if use_opening_sash {
        // Opening windows carry glazing bars inside their hinged print-in-place sash(es), not in the fixed outer frame.
        Vec::new()
    } else {
        let glass_region = shape_from_contour(profile_contour(
            kind,
            w,
            h,
            -params.frame_width,
            ARC_SEGMENTS,
        ));
        let bars_raw = build_style_bars(&params, &specs, &glass_region);
        // Clip bars slightly into the frame (+0.2mm overlap) so union is seamless, and clip outer to glass+0.2mm
        let bar_clip = shape_from_contour(profile_contour(
            kind,
            w,
            h,
            (-params.frame_width + 0.35).min(-0.2),
            ARC_SEGMENTS,
        ));
        intersection_shapes(&bars_raw, &bar_clip)
    };

    // Exterior slabs (supporting flat, 41-deg chamfered, stepped, or combined architrave)
    let ext_base_with_bars = union_shapes(&ext_trim_with_addons, &bars_clipped);
    let ext_sleeve_with_bars = union_shapes(&ext_frame_band, &bars_clipped);
    let mut ext_slabs: Vec<Slab> = Vec::new();
    if params.architrave_style.has_step()
        && params.trim_thickness >= 1.5
        && params.trim_width >= 2.2
    {
        let tier1_z = (params.trim_thickness * 0.58).max(specs.effective_ext_chamfer * 1.15 + 0.25);
        let step_inset = (params.trim_width * 0.36).clamp(0.5, 2.2);
        let stepped_band = profile_band(
            kind,
            w,
            h,
            -params.frame_width,
            (params.trim_width - step_inset).max(0.8) + params.wall_fit_clearance,
            ARC_SEGMENTS,
        );
        let stepped_trim =
            if params.sill_enabled && params.style.supports_sill() && !params.flush_bottom_sill {
                let sw = params.opening_width
                    + 2.0 * ((params.trim_width - step_inset).max(0.8) + params.sill_overhang);
                let y_top = -params.opening_height * 0.5;
                let y_bot = y_top
                    - (params.trim_width - step_inset).max(0.8)
                    - (params.sill_depth - step_inset * 0.5).max(0.4);
                let srect = shape_from_contour(rect_bounds(-sw * 0.5, y_bot, sw * 0.5, y_top));
                union_shapes(&stepped_band, &srect)
            } else {
                stepped_band
            };
        let stepped_addons = build_exterior_addons(
            kind,
            w,
            h,
            (params.trim_width - step_inset).max(0.8),
            specs.effective_ext_chamfer,
            &params,
            step_inset,
            None,
        );
        let stepped_trim_with_addons = clip_exterior_trim_for_flush_wall(
            &union_shapes(&stepped_trim, &stepped_addons),
            &ext_frame_band,
            kind,
            w,
            h,
            &params,
        );
        let ext_step_with_bars = union_shapes(&stepped_trim_with_addons, &bars_clipped);
        ext_slabs.push(Slab {
            z0: 0.0,
            z1: tier1_z,
            region: ext_base_with_bars,
        });
        ext_slabs.push(Slab {
            z0: tier1_z,
            z1: params.trim_thickness,
            region: ext_step_with_bars,
        });
    } else {
        ext_slabs.push(Slab {
            z0: 0.0,
            z1: params.trim_thickness,
            region: ext_base_with_bars,
        });
    }
    ext_slabs.push(Slab {
        z0: params.trim_thickness,
        z1: specs.ext_seat_z,
        region: ext_sleeve_with_bars,
    });
    ext_slabs.push(Slab {
        z0: specs.ext_seat_z,
        z1: specs.ext_total_height,
        region: ext_barrel_band,
    });

    let mut exterior_half = extrude_slabs_with_moulding(&ext_slabs, &ext_moulding);

    // Interior slabs (optional matching chamfer/step/grooves + bar chamfers)
    let int_base_with_bars = union_shapes(&int_trim_band, &bars_clipped);
    let int_spigot_with_bars = union_shapes(&int_spigot_band, &bars_clipped);
    let mut int_slabs: Vec<Slab> = Vec::new();
    if params.architrave_style.has_step()
        && params.chamfer_interior
        && params.interior_trim_thickness >= 1.4
        && params.interior_trim_width >= 2.2
    {
        let tier1_z =
            (params.interior_trim_thickness * 0.58).max(specs.effective_int_chamfer * 1.15 + 0.25);
        let step_inset = (params.interior_trim_width * 0.36).clamp(0.5, 2.2);
        let stepped_int_band = profile_band(
            kind,
            w,
            h,
            -params.frame_width,
            (params.interior_trim_width - step_inset).max(0.8) + params.wall_fit_clearance,
            ARC_SEGMENTS,
        );
        let int_step_with_bars = union_shapes(&stepped_int_band, &bars_clipped);
        int_slabs.push(Slab {
            z0: 0.0,
            z1: tier1_z,
            region: int_base_with_bars,
        });
        int_slabs.push(Slab {
            z0: tier1_z,
            z1: params.interior_trim_thickness,
            region: int_step_with_bars,
        });
    } else {
        int_slabs.push(Slab {
            z0: 0.0,
            z1: params.interior_trim_thickness,
            region: int_base_with_bars,
        });
    }
    int_slabs.push(Slab {
        z0: params.interior_trim_thickness,
        z1: specs.int_total_height,
        region: int_spigot_with_bars,
    });

    let interior_half = extrude_slabs_with_moulding(&int_slabs, &int_moulding);

    // 5. Cut-to-size acetate sheet & optional print-in-place cone-hinged sashes
    let mut window_sashes = Vec::new();
    let acetate_sheet = if use_opening_sash {
        let (frame_knuckles, sashes, ac_w, ac_h) =
            build_opening_sashes(&params, &specs, open_w, open_h);
        for k in frame_knuckles {
            exterior_half.append(&k);
        }
        specs.acetate_cut_width = ac_w;
        specs.acetate_cut_height = ac_h;
        specs.acetate_is_rectangular = true;
        window_sashes = sashes;
        TriMesh::default()
    } else {
        let ac_offset = -(barrel_w + params.acetate_clearance);
        let ac_region = shape_from_contour(profile_contour(kind, w, h, ac_offset, ARC_SEGMENTS));
        extrude_slabs(&[Slab {
            z0: 0.0,
            z1: params.acetate_thickness,
            region: ac_region,
        }])
    };

    // 6. Dollhouse wall cutaway mesh for in-wall preview (including pre-existing flush bosses on -Z)
    let wall_cutaway = build_wall_cutaway_mesh(kind, &params, &specs);

    let mut printable_ext = exterior_half.clone();
    for sash in &window_sashes {
        printable_ext.append(&sash.mesh);
    }
    let print_plate = pack_meshes_on_bed(
        &[printable_ext, interior_half.clone()],
        params.plate_spacing as f32,
    );

    // 8. Verify watertight manifoldness & zero overhangs
    let (ext_diag, mut ext_ok) = match exterior_half.verify_printable_manifold() {
        Ok(d) => (
            format!(
                "{} tris | watertight | {} overhangs",
                d.triangle_count, d.overhang_triangles
            ),
            d.is_watertight && d.overhang_triangles == 0,
        ),
        Err(e) => (e, false),
    };
    for (s_idx, sash) in window_sashes.iter().enumerate() {
        match sash.mesh.verify_printable_manifold() {
            Ok(d) if d.is_watertight && d.overhang_triangles == 0 => {}
            Ok(d) => {
                ext_ok = false;
                let _ = (s_idx, d.overhang_triangles);
            }
            Err(_) => {
                ext_ok = false;
            }
        }
    }

    let (int_diag, int_ok) = match interior_half.verify_printable_manifold() {
        Ok(d) => (
            format!(
                "{} tris | watertight | {} overhangs",
                d.triangle_count, d.overhang_triangles
            ),
            d.is_watertight && d.overhang_triangles == 0,
        ),
        Err(e) => (e, false),
    };

    GeneratedWindow {
        params,
        specs,
        is_door: false,
        exterior_half,
        interior_half,
        acetate_sheet,
        wall_cutaway,
        door_leaves: Vec::new(),
        window_sashes,
        door_handle: TriMesh::default(),
        print_plate,
        ext_diag,
        int_diag,
        printable_ok: ext_ok && int_ok,
    }
}

fn pack_meshes_on_bed(meshes: &[TriMesh], spacing: f32) -> TriMesh {
    let mut widths = Vec::with_capacity(meshes.len());
    let mut bounds_list = Vec::with_capacity(meshes.len());
    let mut total_w = 0.0f32;
    for m in meshes {
        let (bmin, bmax) = m.bounds();
        let w = (bmax[0] - bmin[0]).max(0.1);
        widths.push(w);
        bounds_list.push((bmin, bmax));
        total_w += w;
    }
    if meshes.len() > 1 {
        total_w += spacing * (meshes.len() as f32 - 1.0);
    }

    let mut out = TriMesh::default();
    let mut cursor_x = -total_w * 0.5;
    for (idx, m) in meshes.iter().enumerate() {
        let (bmin, bmax) = bounds_list[idx];
        let cx = (bmin[0] + bmax[0]) * 0.5;
        let cy = (bmin[1] + bmax[1]) * 0.5;
        let w = widths[idx];
        let mut copy = m.clone();
        copy.translate(cursor_x + w * 0.5 - cx, -cy, -bmin[2]);
        out.append(&copy);
        cursor_x += w + spacing;
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SashHingeEdge {
    Left,
    Right,
    Top,
    Bottom,
}

/// Builds print-in-place cone-in-socket frame knuckles (`Vec<TriMesh>`) and hinged window
/// sash assets (`Vec<WindowSashAsset>`) for the active opening mode.
#[allow(clippy::type_complexity)]
fn build_opening_sashes(
    params: &WindowParams,
    specs: &WindowSpecs,
    open_w: f64,
    open_h: f64,
) -> (Vec<TriMesh>, Vec<WindowSashAsset>, f64, f64) {
    let mode = params.effective_opening_mode();
    let clr = params.hinge_clearance.clamp(0.22, 0.65);

    let (specs_list, double_leaf): (Vec<(f64, f64, f64, f64, SashHingeEdge)>, bool) = match mode {
        WindowOpeningMode::CasementLeft => (
            vec![(
                -open_w * 0.5 + clr,
                open_w * 0.5 - clr,
                -open_h * 0.5 + clr,
                open_h * 0.5 - clr,
                SashHingeEdge::Left,
            )],
            false,
        ),
        WindowOpeningMode::CasementRight => (
            vec![(
                -open_w * 0.5 + clr,
                open_w * 0.5 - clr,
                -open_h * 0.5 + clr,
                open_h * 0.5 - clr,
                SashHingeEdge::Right,
            )],
            false,
        ),
        WindowOpeningMode::CasementDouble => (
            vec![
                (
                    -open_w * 0.5 + clr,
                    -clr * 0.5,
                    -open_h * 0.5 + clr,
                    open_h * 0.5 - clr,
                    SashHingeEdge::Left,
                ),
                (
                    clr * 0.5,
                    open_w * 0.5 - clr,
                    -open_h * 0.5 + clr,
                    open_h * 0.5 - clr,
                    SashHingeEdge::Right,
                ),
            ],
            true,
        ),
        WindowOpeningMode::AwningTop => (
            vec![(
                -open_w * 0.5 + clr,
                open_w * 0.5 - clr,
                -open_h * 0.5 + clr,
                open_h * 0.5 - clr,
                SashHingeEdge::Top,
            )],
            false,
        ),
        WindowOpeningMode::HopperBottom => (
            vec![(
                -open_w * 0.5 + clr,
                open_w * 0.5 - clr,
                -open_h * 0.5 + clr,
                open_h * 0.5 - clr,
                SashHingeEdge::Bottom,
            )],
            false,
        ),
        WindowOpeningMode::Fixed => return (Vec::new(), Vec::new(), 0.0, 0.0),
    };

    let mut frame_knuckles = Vec::new();
    let mut sashes = Vec::new();
    let mut ac_w_out = 0.0;
    let mut ac_h_out = 0.0;

    for (sx0, sx1, sy0, sy1, edge) in specs_list {
        let (k0, k1, sash, aw, ah) = build_one_cone_hinged_sash(
            params,
            specs,
            open_w,
            open_h,
            sx0,
            sx1,
            sy0,
            sy1,
            edge,
            double_leaf,
        );
        frame_knuckles.push(k0);
        frame_knuckles.push(k1);
        ac_w_out = aw;
        ac_h_out = ah;
        sashes.push(sash);
    }

    (frame_knuckles, sashes, ac_w_out, ac_h_out)
}

#[allow(clippy::too_many_arguments)]
fn build_one_cone_hinged_sash(
    params: &WindowParams,
    specs: &WindowSpecs,
    open_w: f64,
    open_h: f64,
    sx0: f64,
    sx1: f64,
    sy0: f64,
    sy1: f64,
    hinge_edge: SashHingeEdge,
    double_leaf: bool,
) -> (TriMesh, TriMesh, WindowSashAsset, f64, f64) {
    let sash_t = (params.trim_thickness + 1.2).clamp(3.6, specs.ext_seat_z.clamp(3.6, 4.6));
    let clr = params.hinge_clearance.clamp(0.22, 0.65);
    let cone_r = (sash_t * 0.5 - 0.36).clamp(1.15, 1.65);
    let cone_len = (cone_r * 0.80).clamp(0.95, 1.32);
    let socket_r = (cone_r + clr * 0.60).min(sash_t * 0.5 - 0.16);
    let socket_len = (socket_r * 0.80).max(cone_len + 0.05);
    let axis_gap = (clr * 1.15).min(cone_len * 0.48);

    let k_thick = sash_t;
    let lug_protrude = 1.6;
    let sock_block_len = socket_len + 1.4;
    let frame_embed = 0.65;
    let sock_embed = 0.30;

    let other_stile = (params.frame_width * 0.62).clamp(2.4, 3.8);
    let hinge_stile = (k_thick + 0.55).max(other_stile);

    let mut cut_boxes: Vec<Contour2D> = Vec::new();
    let (
        frame_k0,
        frame_k1,
        sock_k0,
        sock_k1,
        gx0,
        gx1,
        gy0,
        gy1,
        pivot_origin,
        pivot_axis,
        swing_dir,
    ) = match hinge_edge {
        SashHingeEdge::Left | SashHingeEdge::Right => {
            let is_left = hinge_edge == SashHingeEdge::Left;
            let (kx0, kx1, gx0, gx1, swing_dir) = if is_left {
                (
                    sx0,
                    sx0 + k_thick,
                    sx0 + hinge_stile,
                    sx1 - other_stile,
                    1.0f32,
                )
            } else {
                (
                    sx1 - k_thick,
                    sx1,
                    sx0 + other_stile,
                    sx1 - hinge_stile,
                    -1.0f32,
                )
            };
            let cx_notch0 = if is_left { kx0 - 0.2 } else { kx0 - clr };
            let cx_notch1 = if is_left { kx1 + clr } else { kx1 + 0.2 };

            let fy_bot0 = -open_h * 0.5 - frame_embed;
            let fy_bot1 = -open_h * 0.5 + lug_protrude;
            let fy_top0 = open_h * 0.5 - lug_protrude;
            let fy_top1 = open_h * 0.5 + frame_embed;

            let fk0 = build_cone_knuckle_block(
                kx0,
                kx1,
                fy_bot0,
                fy_bot1,
                0.0,
                sash_t,
                HingeFace::PosY,
                cone_r,
                cone_len,
                false,
                16,
            );
            let fk1 = build_cone_knuckle_block(
                kx0,
                kx1,
                fy_top0,
                fy_top1,
                0.0,
                sash_t,
                HingeFace::NegY,
                cone_r,
                cone_len,
                false,
                16,
            );

            let sb0_y0 = fy_bot1 + axis_gap;
            let sb0_y1 = sb0_y0 + sock_block_len;
            let st1_y1 = fy_top0 - axis_gap;
            let st1_y0 = st1_y1 - sock_block_len;

            let sk0 = build_cone_knuckle_block(
                kx0,
                kx1,
                sb0_y0,
                sb0_y1,
                0.0,
                sash_t,
                HingeFace::NegY,
                socket_r,
                socket_len,
                true,
                16,
            );
            let sk1 = build_cone_knuckle_block(
                kx0,
                kx1,
                st1_y0,
                st1_y1,
                0.0,
                sash_t,
                HingeFace::PosY,
                socket_r,
                socket_len,
                true,
                16,
            );

            // Corner relief notches + cavity clearances leaving `sock_embed` overlap at inner end
            cut_boxes.push(rect_bounds(cx_notch0, sy0 - 0.2, cx_notch1, sb0_y0 + 0.01));
            cut_boxes.push(rect_bounds(cx_notch0, st1_y1 - 0.01, cx_notch1, sy1 + 0.2));
            cut_boxes.push(rect_bounds(
                cx_notch0,
                sb0_y0 - 0.01,
                cx_notch1,
                (sb0_y1 - sock_embed).max(sb0_y0 + 0.2),
            ));
            cut_boxes.push(rect_bounds(
                cx_notch0,
                (st1_y0 + sock_embed).min(st1_y1 - 0.2),
                cx_notch1,
                st1_y1 + 0.01,
            ));

            (
                fk0,
                fk1,
                sk0,
                sk1,
                gx0,
                gx1,
                sy0 + other_stile,
                sy1 - other_stile,
                [
                    ((kx0 + kx1) * 0.5) as f32,
                    ((sy0 + sy1) * 0.5) as f32,
                    (sash_t * 0.5) as f32,
                ],
                [0.0, 1.0, 0.0],
                swing_dir,
            )
        }
        SashHingeEdge::Top | SashHingeEdge::Bottom => {
            let is_top = hinge_edge == SashHingeEdge::Top;
            let (ky0, ky1, gy0, gy1, swing_dir) = if is_top {
                (
                    sy1 - k_thick,
                    sy1,
                    sy0 + other_stile,
                    sy1 - hinge_stile,
                    1.0f32,
                )
            } else {
                (
                    sy0,
                    sy0 + k_thick,
                    sy0 + hinge_stile,
                    sy1 - other_stile,
                    -1.0f32,
                )
            };
            let cy_notch0 = if is_top { ky0 - clr } else { ky0 - 0.2 };
            let cy_notch1 = if is_top { ky1 + 0.2 } else { ky1 + clr };

            let fx_l0 = -open_w * 0.5 - frame_embed;
            let fx_l1 = -open_w * 0.5 + lug_protrude;
            let fx_r0 = open_w * 0.5 - lug_protrude;
            let fx_r1 = open_w * 0.5 + frame_embed;

            let fk0 = build_cone_knuckle_block(
                fx_l0,
                fx_l1,
                ky0,
                ky1,
                0.0,
                sash_t,
                HingeFace::PosX,
                cone_r,
                cone_len,
                false,
                16,
            );
            let fk1 = build_cone_knuckle_block(
                fx_r0,
                fx_r1,
                ky0,
                ky1,
                0.0,
                sash_t,
                HingeFace::NegX,
                cone_r,
                cone_len,
                false,
                16,
            );

            let sl0_x0 = fx_l1 + axis_gap;
            let sl0_x1 = sl0_x0 + sock_block_len;
            let sr1_x1 = fx_r0 - axis_gap;
            let sr1_x0 = sr1_x1 - sock_block_len;

            let sk0 = build_cone_knuckle_block(
                sl0_x0,
                sl0_x1,
                ky0,
                ky1,
                0.0,
                sash_t,
                HingeFace::NegX,
                socket_r,
                socket_len,
                true,
                16,
            );
            let sk1 = build_cone_knuckle_block(
                sr1_x0,
                sr1_x1,
                ky0,
                ky1,
                0.0,
                sash_t,
                HingeFace::PosX,
                socket_r,
                socket_len,
                true,
                16,
            );

            cut_boxes.push(rect_bounds(sx0 - 0.2, cy_notch0, sl0_x0 + 0.01, cy_notch1));
            cut_boxes.push(rect_bounds(sr1_x1 - 0.01, cy_notch0, sx1 + 0.2, cy_notch1));
            cut_boxes.push(rect_bounds(
                sl0_x0 - 0.01,
                cy_notch0,
                (sl0_x1 - sock_embed).max(sl0_x0 + 0.2),
                cy_notch1,
            ));
            cut_boxes.push(rect_bounds(
                (sr1_x0 + sock_embed).min(sr1_x1 - 0.2),
                cy_notch0,
                sr1_x1 + 0.01,
                cy_notch1,
            ));

            (
                fk0,
                fk1,
                sk0,
                sk1,
                sx0 + other_stile,
                sx1 - other_stile,
                gy0,
                gy1,
                [
                    ((sx0 + sx1) * 0.5) as f32,
                    ((ky0 + ky1) * 0.5) as f32,
                    (sash_t * 0.5) as f32,
                ],
                [1.0, 0.0, 0.0],
                swing_dir,
            )
        }
    };

    let gx_lo = gx0.min(gx1 - 3.0);
    let gx_hi = gx1.max(gx0 + 3.0);
    let gy_lo = gy0.min(gy1 - 3.0);
    let gy_hi = gy1.max(gy0 + 3.0);
    let gw = (gx_hi - gx_lo).max(3.0);
    let gh = (gy_hi - gy_lo).max(3.0);
    let g_cx = (gx_lo + gx_hi) * 0.5;
    let g_cy = (gy_lo + gy_hi) * 0.5;

    let sash_outer = shape_from_contour(rect_bounds(sx0, sy0, sx1, sy1));
    let glass_hole = shape_from_contour(rect_bounds(gx_lo, gy_lo, gx_hi, gy_hi));
    let sash_frame_raw = difference_shapes(&sash_outer, &glass_hole);

    // Style-matched glazing bars inside this sash's daylight opening
    let mut sp = params.clone();
    if double_leaf && sp.bars_x > 1 {
        sp.bars_x = (sp.bars_x / 2).max(1);
    }
    sp.frame_width = 3.2;
    let mut ss = specs.clone();
    ss.sleeve_outer_width = gw + 6.4;
    ss.sleeve_outer_height = gh + 6.4;
    let mut bars_raw = build_style_bars(&sp, &ss, &glass_hole);
    for shape in &mut bars_raw {
        for contour in shape {
            for pt in contour {
                pt[0] += g_cx;
                pt[1] += g_cy;
            }
        }
    }
    let bar_clip = shape_from_contour(rect_bounds(
        gx_lo - 0.35,
        gy_lo - 0.35,
        gx_hi + 0.35,
        gy_hi + 0.35,
    ));
    let sash_bars = intersection_shapes(&bars_raw, &bar_clip);

    let notch_shapes = union_contours(&cut_boxes);
    let sash_front_region =
        difference_shapes(&union_shapes(&sash_frame_raw, &sash_bars), &notch_shapes);

    // Rear acetate pocket (strictly inset inside the solid sash rim, away from cone sockets)
    let hinge_rebate = (hinge_stile - k_thick - 0.32).max(0.45);
    let std_rebate = (other_stile * 0.38).clamp(0.65, 1.3);
    let (ax0, ax1, ay0, ay1) = match hinge_edge {
        SashHingeEdge::Left => (
            gx_lo - hinge_rebate,
            gx_hi + std_rebate,
            gy_lo - std_rebate,
            gy_hi + std_rebate,
        ),
        SashHingeEdge::Right => (
            gx_lo - std_rebate,
            gx_hi + hinge_rebate,
            gy_lo - std_rebate,
            gy_hi + std_rebate,
        ),
        SashHingeEdge::Top => (
            gx_lo - std_rebate,
            gx_hi + std_rebate,
            gy_lo - std_rebate,
            gy_hi + hinge_rebate,
        ),
        SashHingeEdge::Bottom => (
            gx_lo - std_rebate,
            gx_hi + std_rebate,
            gy_lo - hinge_rebate,
            gy_hi + std_rebate,
        ),
    };
    let ac_pocket_region = shape_from_contour(rect_bounds(ax0, ay0, ax1, ay1));
    let sash_rear_region = difference_shapes(
        &difference_shapes(&sash_outer, &ac_pocket_region),
        &notch_shapes,
    );

    let ac_depth = (params.acetate_thickness + 0.45).clamp(0.7, (sash_t - 1.3).max(0.7));
    let front_z1 = (sash_t - ac_depth).max(1.2);
    let mut sash_slabs = vec![Slab {
        z0: 0.0,
        z1: front_z1,
        region: sash_front_region,
    }];
    if sash_t > front_z1 + 0.25 {
        sash_slabs.push(Slab {
            z0: front_z1,
            z1: sash_t,
            region: sash_rear_region,
        });
    }

    let bar_c = if params.chamfer_bars {
        let c = params
            .bar_chamfer_size
            .min((params.bar_width * 0.28).min(0.40));
        (c >= 0.10).then_some((c, c * 1.15))
    } else {
        None
    };
    let mut sash_mesh = extrude_slabs_with_moulding(
        &sash_slabs,
        &MouldingConfig {
            outer_chamfer: Some((0.28, 0.34)),
            hole_chamfer: bar_c,
            grooves: Vec::new(),
        },
    );
    sash_mesh.append(&sock_k0);
    sash_mesh.append(&sock_k1);

    let ac_w = ((ax1 - ax0) - 2.0 * params.acetate_clearance).max(2.0);
    let ac_h = ((ay1 - ay0) - 2.0 * params.acetate_clearance).max(2.0);
    let sash_center_x = (sx0 + sx1) * 0.5;
    let sash_center_y = (sy0 + sy1) * 0.5;
    let ac_cx = (ax0 + ax1) * 0.5;
    let ac_cy = (ay0 + ay1) * 0.5;
    let sash_ac_region = shape_from_contour(rect_centered(ac_cx, ac_cy, ac_w, ac_h));
    let sash_acetate = extrude_slabs(&[Slab {
        z0: 0.0,
        z1: params.acetate_thickness,
        region: sash_ac_region,
    }]);

    let asset = WindowSashAsset {
        mesh: sash_mesh,
        sash_acetate,
        center_xy: [sash_center_x as f32, sash_center_y as f32],
        pivot_origin,
        pivot_axis,
        swing_dir,
    };

    (frame_k0, frame_k1, asset, ac_w, ac_h)
}

fn build_style_bars(params: &WindowParams, specs: &WindowSpecs, _glass: &Shapes2D) -> Shapes2D {
    let w = specs.sleeve_outer_width;
    let h = specs.sleeve_outer_height;
    let gw = (w - 2.0 * params.frame_width).max(2.0);
    let gh = (h - 2.0 * params.frame_width).max(2.0);
    let bw = params.bar_width;
    let mw = params.meeting_rail_width;

    let mut contours: Vec<Contour2D> = Vec::new();

    match params.style {
        WindowStyle::PictureFixed
        | WindowStyle::DoorColonial4Panel
        | WindowStyle::DoorGeorgian6Panel
        | WindowStyle::DoorGlazedFrench
        | WindowStyle::DoorArchedHeritage
        | WindowStyle::DoorFanlightGeorgian => {}

        WindowStyle::PalladianVenetian => {
            let mullion_x = gw * 0.22;
            let y_transom = -gh * 0.5 + gh * params.split_ratio;
            // Two main bay mullions
            contours.push(rect_centered(-mullion_x, 0.0, mw, gh + 4.0));
            contours.push(rect_centered(mullion_x, 0.0, mw, gh + 4.0));
            // Horizontal transom bar
            contours.push(rect_centered(0.0, y_transom, gw + 4.0, mw));

            // Optional horizontal bars below transom
            let lower_h = y_transom - (-gh * 0.5);
            if lower_h > 4.0 && params.bars_y > 0 {
                for i in 1..=params.bars_y {
                    let y = -gh * 0.5 + lower_h * (i as f64 / (params.bars_y + 1) as f64);
                    contours.push(rect_centered(0.0, y, gw + 4.0, bw));
                }
            }
            // Extra center mullion in lower bay if bars_x >= 3
            if params.bars_x >= 3 && lower_h > 4.0 {
                contours.push(rect_bounds(-bw * 0.5, -gh * 0.5 - 2.0, bw * 0.5, y_transom));
            }

            let base_bars = union_contours(&contours);
            let r_mid = mullion_x;
            if r_mid > mw + 1.0 {
                let ring = ring_shape(
                    0.0,
                    y_transom,
                    (r_mid - mw * 0.5).max(0.8),
                    r_mid + mw * 0.5,
                    32,
                );
                let arch_mask =
                    shape_from_contour(rect_bounds(-gw, y_transom - mw * 0.5, gw, gh + 10.0));
                let clipped_ring = intersection_shapes(&vec![ring], &arch_mask);
                return union_shapes(&base_bars, &clipped_ring);
            }
            return base_bars;
        }

        WindowStyle::HalfRoundLunette => {
            let base_y = -gh * 0.5;
            let max_r = (gw * 0.5).max(gh) + 4.0;
            let spokes = params.arch_spokes.max(2);
            for i in 1..=spokes {
                let a = PI * (i as f64 / (spokes + 1) as f64);
                contours.push(segment_bar(
                    [0.0, base_y - 0.3],
                    [max_r * a.cos(), base_y + max_r * a.sin()],
                    bw,
                ));
            }
            let base_bars = union_contours(&contours);
            if params.bars_x > 0 && (gw * 0.5).min(gh) > 7.0 {
                let ring_r = (gw * 0.5).min(gh) * 0.42;
                let ring = ring_shape(
                    0.0,
                    base_y,
                    (ring_r - bw * 0.5).max(0.8),
                    ring_r + bw * 0.5,
                    36,
                );
                return union_shapes(&base_bars, &vec![ring]);
            }
            return base_bars;
        }

        WindowStyle::GeorgianGrid => {
            add_even_vertical_bars(&mut contours, gw, gh, params.bars_x, bw);
            add_even_horizontal_bars(&mut contours, gw, gh, params.bars_y, bw);
        }

        WindowStyle::SashSplit => {
            let y_split = -gh * 0.5 + gh * params.split_ratio;
            contours.push(rect_centered(0.0, y_split, gw + 2.0, mw));

            add_even_vertical_bars(&mut contours, gw, gh, params.bars_x, bw);

            // Upper sash horizontal bars
            let upper_h = (gh * 0.5) - (y_split + mw * 0.5);
            if upper_h > 4.0 && params.bars_y > 0 {
                for i in 1..=params.bars_y {
                    let y =
                        (y_split + mw * 0.5) + upper_h * (i as f64 / (params.bars_y + 1) as f64);
                    contours.push(rect_centered(0.0, y, gw + 2.0, bw));
                }
            }
            // Lower sash horizontal bars (use same count)
            let lower_h = (y_split - mw * 0.5) - (-gh * 0.5);
            if lower_h > 4.0 && params.bars_y > 0 {
                for i in 1..=params.bars_y {
                    let y = (-gh * 0.5) + lower_h * (i as f64 / (params.bars_y + 1) as f64);
                    contours.push(rect_centered(0.0, y, gw + 2.0, bw));
                }
            }
        }

        WindowStyle::CasementSplit => {
            // Sturdy vertical mullions (`bars_x` dividers)
            add_even_vertical_bars(&mut contours, gw, gh, params.bars_x.max(1), mw);
            // Optional transom rail near top when bars_y > 0
            if params.bars_y > 0 {
                let y_transom = -gh * 0.5 + gh * params.split_ratio;
                contours.push(rect_centered(0.0, y_transom, gw + 2.0, mw));
                // Additional fine horizontal bars below transom if bars_y > 1
                let lower_h = (y_transom - mw * 0.5) - (-gh * 0.5);
                for i in 1..params.bars_y {
                    let y = (-gh * 0.5) + lower_h * (i as f64 / params.bars_y as f64);
                    contours.push(rect_centered(0.0, y, gw + 2.0, bw));
                }
            }
        }

        WindowStyle::CottageLattice => {
            // Diagonal +45 and -45 degree lattice bars spaced by `diamond_pitch`
            let pitch = params.diamond_pitch.max(5.0);
            let diag_step = pitch * std::f64::consts::SQRT_2;
            let span = gw + gh + 20.0;
            let half_n = (span / diag_step).ceil() as i32;
            let reach = gw.max(gh) + 10.0;

            for k in -half_n..=half_n {
                let c = k as f64 * diag_step;
                // y = x + c
                contours.push(segment_bar([-reach, -reach + c], [reach, reach + c], bw));
                // y = -x + c
                contours.push(segment_bar([-reach, reach + c], [reach, -reach + c], bw));
            }
        }

        WindowStyle::ArchedGeorgian => {
            let rx_base = w * 0.5;
            let ry_base = (w * 0.5).min(h * 0.72);
            let y_spring = h * 0.5 - ry_base;
            let glass_r = (rx_base - params.frame_width).max(2.0);

            // Spring-line transom bar + lower grid
            if y_spring > -gh * 0.5 + 4.0 && y_spring < gh * 0.5 - 3.0 {
                contours.push(rect_centered(0.0, y_spring, gw + 2.0, bw));
            }

            // Vertical bars stop at spring line if radial spokes are active, or run all the way up
            let v_top = if params.arch_spokes > 0 {
                y_spring
            } else {
                gh * 0.5 + 2.0
            };
            if params.bars_x > 0 && v_top > -gh * 0.5 + 2.0 {
                for i in 1..=params.bars_x {
                    let x = -gw * 0.5 + gw * (i as f64 / (params.bars_x + 1) as f64);
                    contours.push(rect_bounds(
                        x - bw * 0.5,
                        -gh * 0.5 - 2.0,
                        x + bw * 0.5,
                        v_top,
                    ));
                }
            }

            // Horizontal bars in lower rectangular zone
            let lower_h = y_spring - (-gh * 0.5);
            if lower_h > 4.0 && params.bars_y > 0 {
                for i in 1..=params.bars_y {
                    let y = (-gh * 0.5) + lower_h * (i as f64 / (params.bars_y + 1) as f64);
                    contours.push(rect_centered(0.0, y, gw + 2.0, bw));
                }
            }

            // Radial fanlight spokes in the arch + small inner hub ring
            if params.arch_spokes > 0 {
                let spoke_len = glass_r + 3.0;
                for i in 1..=params.arch_spokes {
                    let a = PI * (i as f64 / (params.arch_spokes + 1) as f64);
                    contours.push(segment_bar(
                        [0.0, y_spring],
                        [spoke_len * a.cos(), y_spring + spoke_len * a.sin()],
                        bw,
                    ));
                }
                // Decorative half-hub ring at spring center
                let hub_r = (glass_r * 0.28).clamp(bw * 1.5, 10.0);
                let ring = ring_shape(
                    0.0,
                    y_spring,
                    (hub_r - bw * 0.5).max(0.5),
                    hub_r + bw * 0.5,
                    28,
                );
                let base_bars = union_contours(&contours);
                let hub_shapes = vec![ring];
                let arch_mask =
                    shape_from_contour(rect_bounds(-gw, y_spring - bw * 0.5, gw, gh + 10.0));
                let clipped_hub = intersection_shapes(&hub_shapes, &arch_mask);
                return union_shapes(&base_bars, &clipped_hub);
            }
        }

        WindowStyle::GothicPointed => {
            let half_w_base = w * 0.5;
            let h_arch = (w * 0.72).min(h * 0.76).max(w * 0.52);
            let y_spring = h * 0.5 - h_arch;

            // Vertical mullions
            add_even_vertical_bars(&mut contours, gw, gh + 6.0, params.bars_x, bw);

            // Spring-line bar + lower horizontal bars
            if params.bars_y > 0 {
                let lower_h = y_spring - (-gh * 0.5);
                if lower_h > 4.0 {
                    for i in 1..=params.bars_y {
                        let y = (-gh * 0.5) + lower_h * (i as f64 / params.bars_y as f64);
                        contours.push(rect_centered(0.0, y, gw + 2.0, bw));
                    }
                }
            }

            // Gothic Y-tracery arches rising from spring line toward lancet flanks
            let glass_half = (half_w_base - params.frame_width).max(2.0);
            let y_apex = gh * 0.5;
            if glass_half > 5.0 {
                let mid_y = (y_spring + y_apex) * 0.52;
                contours.push(segment_bar(
                    [0.0, y_spring],
                    [-glass_half * 0.65, mid_y + 3.0],
                    bw,
                ));
                contours.push(segment_bar(
                    [0.0, y_spring],
                    [glass_half * 0.65, mid_y + 3.0],
                    bw,
                ));
            }
        }

        WindowStyle::CircularBullsEye => {
            let rx = (w * 0.5 - params.frame_width).max(2.0);
            let ry = (h * 0.5 - params.frame_width).max(2.0);
            let max_r = rx.max(ry) + 3.0;

            let spoke_count = params.arch_spokes.max(2);
            for i in 0..spoke_count {
                let a = PI * (i as f64 / spoke_count as f64);
                let dx = max_r * a.cos();
                let dy = max_r * a.sin();
                contours.push(segment_bar([-dx, -dy], [dx, dy], bw));
            }

            let base_bars = union_contours(&contours);
            // If bars_x > 0, add a concentric inner ring!
            if params.bars_x > 0 && rx.min(ry) > 7.0 {
                let ring_r = rx.min(ry) * 0.42;
                let ring = ring_shape(
                    0.0,
                    0.0,
                    (ring_r - bw * 0.5).max(0.8),
                    ring_r + bw * 0.5,
                    40,
                );
                return union_shapes(&base_bars, &vec![ring]);
            }
            return base_bars;
        }

        WindowStyle::CraftsmanPrairie => {
            // High transom rail
            let y_transom = -gh * 0.5 + gh * params.split_ratio;
            contours.push(rect_centered(0.0, y_transom, gw + 2.0, mw));

            // Vertical prairie bars in upper transom (and optionally full height if bars_x >= 1)
            let cols = params.bars_x.max(2);
            let inset = (gw * 0.16).clamp(4.0, gw * 0.28);
            // Left and right vertical prairie stiles running full height
            let x_l = -gw * 0.5 + inset;
            let x_r = gw * 0.5 - inset;
            contours.push(rect_bounds(
                x_l - bw * 0.5,
                -gh * 0.5 - 2.0,
                x_l + bw * 0.5,
                gh * 0.5 + 2.0,
            ));
            contours.push(rect_bounds(
                x_r - bw * 0.5,
                -gh * 0.5 - 2.0,
                x_r + bw * 0.5,
                gh * 0.5 + 2.0,
            ));

            // Additional upper-sash muntins if cols > 2
            if cols > 2 {
                let inner_span = x_r - x_l;
                let extra = cols - 2;
                for i in 1..=extra {
                    let x = x_l + inner_span * (i as f64 / (extra + 1) as f64);
                    contours.push(rect_bounds(
                        x - bw * 0.5,
                        y_transom,
                        x + bw * 0.5,
                        gh * 0.5 + 2.0,
                    ));
                }
            }
            // Classic corner prairie squares in the upper corners
            let upper_h = gh * 0.5 - y_transom;
            if upper_h > 6.0 {
                let y_sub = y_transom + upper_h * 0.48;
                contours.push(rect_bounds(
                    -gw * 0.5 - 2.0,
                    y_sub - bw * 0.5,
                    x_l,
                    y_sub + bw * 0.5,
                ));
                contours.push(rect_bounds(
                    x_r,
                    y_sub - bw * 0.5,
                    gw * 0.5 + 2.0,
                    y_sub + bw * 0.5,
                ));
            }
        }
    }

    let _ = circle_contour; // keep clean import usage if needed
    union_contours(&contours)
}

fn add_even_vertical_bars(contours: &mut Vec<Contour2D>, gw: f64, gh: f64, count: u32, bw: f64) {
    if count == 0 {
        return;
    }
    for i in 1..=count {
        let x = -gw * 0.5 + gw * (i as f64 / (count + 1) as f64);
        contours.push(rect_centered(x, 0.0, bw, gh + 4.0));
    }
}

fn add_even_horizontal_bars(contours: &mut Vec<Contour2D>, gw: f64, gh: f64, count: u32, bw: f64) {
    if count == 0 {
        return;
    }
    for i in 1..=count {
        let y = -gh * 0.5 + gh * (i as f64 / (count + 1) as f64);
        contours.push(rect_centered(0.0, y, gw + 4.0, bw));
    }
}
