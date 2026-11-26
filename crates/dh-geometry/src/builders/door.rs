use crate::builders::window::{
    build_exterior_addons, build_wall_cutaway_mesh, clip_exterior_trim_for_flush_wall,
};
use crate::extrude::{
    MouldingConfig, Slab, TriMesh, VGroove, extrude_slabs, extrude_slabs_with_moulding,
};
use crate::polygon2d::{
    Contour2D, ProfileKind, circle_contour, difference_shapes, intersection_shapes, profile_band,
    profile_contour, rect_bounds, rect_centered, ring_shape, segment_bar, shape_from_contour,
    union_contours, union_shapes,
};
use dh_core::{DoorHandleStyle, WindowParams, WindowSpecs, WindowStyle};
use std::f32::consts::{FRAC_PI_2, PI as PI32};
use std::f64::consts::PI;

const ARC_SEGMENTS: usize = 36;

/// Metadata & mesh for a single hinged door leaf.
#[derive(Debug, Clone)]
pub struct DoorLeafAsset {
    /// Primary door leaf mesh (printed interior-face down at Z=0, includes octagonal/beveled pivot pins).
    pub mesh: TriMesh,
    /// Optional exterior cladding half for glazed doors sandwiching cut-to-size acetate.
    pub exterior_leaf: Option<TriMesh>,
    /// Optional acetate pane for glazed doors.
    pub leaf_acetate: Option<TriMesh>,
    /// Hinge pivot X in leaf-local coordinates (for 3D swing animation).
    pub pivot_x: f32,
    /// Hinge pivot Z in leaf-local coordinates (measured from interior face Z=0).
    pub pivot_z: f32,
    /// X offset of this leaf inside the assembled frame.
    pub assembled_x_offset: f32,
    /// Swing direction (+1.0 for left-hinged opening into room, -1.0 for right-hinged).
    pub swing_dir: f32,
    /// Handle mount `(x, y)` in leaf-local coordinates.
    pub handle_xy: [f32; 2],
    /// Thickness of the assembled leaf (mm).
    pub leaf_thickness: f32,
    /// Pre-positioned brass hardware (handle, escutcheon plates, letterbox plates) in assembled leaf coords.
    pub hardware_preview: TriMesh,
    /// Pre-positioned frame-colored door accessories (cat flap bezel & flap) in assembled leaf coords.
    pub accessory_preview: TriMesh,
}

/// Outputs produced by `build_door_assembly`.
pub struct DoorBuildOutput {
    pub exterior_frame: TriMesh,
    pub interior_frame: TriMesh,
    pub wall_cutaway: TriMesh,
    pub door_leaves: Vec<DoorLeafAsset>,
    pub door_handle: TriMesh,
    /// Separate logically grouped printable hardware parts (e.g. handles, letterbox plates, cat flap).
    pub hardware_parts: Vec<(String, TriMesh)>,
    pub acetate_cut_w: f64,
    pub acetate_cut_h: f64,
}

pub fn build_door_assembly(
    params: &WindowParams,
    specs: &WindowSpecs,
    ext_moulding: &MouldingConfig,
    int_moulding: &MouldingConfig,
) -> DoorBuildOutput {
    let kind = if params.style == WindowStyle::DoorArchedHeritage {
        ProfileKind::RoundArch
    } else {
        ProfileKind::Rectangle
    };

    let w = specs.sleeve_outer_width;
    let h = specs.sleeve_outer_height;
    let barrel_w = specs.barrel_wall_thickness;
    let clr = params.hinge_clearance;
    let door_t = params.door_thickness;

    // 1. Clear door opening inside the frame
    let inner_w = (w - 2.0 * params.frame_width).max(12.0);
    let inner_h = (h - 2.0 * params.frame_width).max(20.0);

    // Door stop ledge narrows the opening slightly on the exterior side so the closed door seats cleanly.
    let stop_lip = 1.4;

    // Pin dimensions (in XY on print bed, Z=0..2.2mm)
    let pin_w = 2.2;
    let pin_len = 2.4;
    let pin_inset_from_edge = 0.25;
    let pin_socket_w = pin_w + 2.0 * clr;
    let pin_socket_len = pin_len + clr + 0.35;

    // Z layout of the frame (measured across `effective_wall_thickness`):
    // From interior face (room side) going inward into the wall:
    //   0.0 .. 0.65: solid cover over hinge pin pocket (keeps room face clean at top/bottom)
    //   0.65 .. 0.65 + door_t + 0.35: door leaf + pivot pin space
    //   remaining to exterior face: door-stop ledge (stop_inner)
    let int_pin_cover = 0.65;
    let ext_total = params.trim_thickness + specs.effective_wall_thickness - 0.4;
    let int_total =
        params.interior_trim_thickness + (specs.effective_wall_thickness * 0.48).max(1.8);
    let door_recess_from_ext =
        (params.trim_thickness + specs.effective_wall_thickness - int_pin_cover - door_t - 0.25)
            .max(params.trim_thickness + 0.9);
    let ext_pin_floor =
        (params.trim_thickness + specs.effective_wall_thickness - int_pin_cover - 2.4)
            .max(door_recess_from_ext);

    // Determine hinge X positions for single or double doors
    let left_hinge_x0 = -inner_w * 0.5 + clr + pin_inset_from_edge;
    let right_hinge_x1 = inner_w * 0.5 - clr - pin_inset_from_edge;

    let mut hinge_x_ranges: Vec<(f64, f64)> = Vec::new();
    if params.double_doors {
        hinge_x_ranges.push((left_hinge_x0 - clr, left_hinge_x0 + pin_w + clr));
        hinge_x_ranges.push((right_hinge_x1 - pin_w - clr, right_hinge_x1 + clr));
    } else if params.hinge_right {
        hinge_x_ranges.push((right_hinge_x1 - pin_w - clr, right_hinge_x1 + clr));
    } else {
        hinge_x_ranges.push((left_hinge_x0 - clr, left_hinge_x0 + pin_w + clr));
    }

    // Y position of top/bottom hinge pin sockets
    let (y_pin_bot_top, y_pin_top_bot) = if kind == ProfileKind::RoundArch {
        let ry_base = (w * 0.5).min(h * 0.72);
        let y_spring = h * 0.5 - ry_base;
        (-inner_h * 0.5 + clr, y_spring - 2.2)
    } else {
        (-inner_h * 0.5 + clr, inner_h * 0.5 - clr)
    };

    let mut socket_notches: Vec<Contour2D> = Vec::new();
    for &(hx0, hx1) in &hinge_x_ranges {
        // Bottom socket notch extending down into threshold/jamb
        socket_notches.push(rect_bounds(
            hx0,
            y_pin_bot_top - pin_socket_len,
            hx1,
            y_pin_bot_top + 0.4,
        ));
        // Top socket notch extending up into lintel/spring shoulder
        socket_notches.push(rect_bounds(
            hx0,
            y_pin_top_bot - 0.4,
            hx1,
            y_pin_top_bot + pin_socket_len,
        ));
    }
    let _ = pin_socket_w;
    let socket_shapes = union_contours(&socket_notches);

    // 2. Build Exterior Frame Half (printed outside-down at Z = 0)
    let ext_trim_band = profile_band(
        kind,
        w,
        h,
        -(params.frame_width + stop_lip),
        params.trim_width + params.wall_fit_clearance,
        ARC_SEGMENTS,
    );
    let ext_trim_with_sill = if params.sill_enabled && !params.flush_bottom_sill {
        let sill_w = params.opening_width + 2.0 * (params.trim_width + params.sill_overhang);
        let y_top = -params.opening_height * 0.5;
        let y_bot = y_top - params.trim_width - params.sill_depth;
        let sill_rect = shape_from_contour(rect_bounds(-sill_w * 0.5, y_bot, sill_w * 0.5, y_top));
        union_shapes(&ext_trim_band, &sill_rect)
    } else {
        ext_trim_band
    };
    let ext_addons = build_exterior_addons(
        kind,
        w,
        h,
        params.trim_width,
        specs.effective_ext_chamfer,
        params,
        0.0,
        None,
    );
    let ext_stop_band = profile_band(
        kind,
        w,
        h,
        -(params.frame_width + stop_lip),
        0.0,
        ARC_SEGMENTS,
    );
    let ext_trim_with_addons = clip_exterior_trim_for_flush_wall(
        &union_shapes(&ext_trim_with_sill, &ext_addons),
        &ext_stop_band,
        kind,
        w,
        h,
        params,
    );
    let ext_pocket_band = profile_band(kind, w, h, -params.frame_width, 0.0, ARC_SEGMENTS);
    let ext_barrel_band = profile_band(kind, w, h, -barrel_w, 0.0, ARC_SEGMENTS);
    let ext_pocket_notched = difference_shapes(&ext_pocket_band, &socket_shapes);
    let ext_barrel_notched = difference_shapes(&ext_barrel_band, &socket_shapes);

    let mut ext_slabs = Vec::new();
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
            -(params.frame_width + stop_lip),
            (params.trim_width - step_inset).max(0.8) + params.wall_fit_clearance,
            ARC_SEGMENTS,
        );
        let stepped_trim = if params.sill_enabled && !params.flush_bottom_sill {
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
            params,
            step_inset,
            None,
        );
        let stepped_trim_with_addons = clip_exterior_trim_for_flush_wall(
            &union_shapes(&stepped_trim, &stepped_addons),
            &ext_stop_band,
            kind,
            w,
            h,
            params,
        );
        ext_slabs.push(Slab {
            z0: 0.0,
            z1: tier1_z,
            region: ext_trim_with_addons,
        });
        ext_slabs.push(Slab {
            z0: tier1_z,
            z1: params.trim_thickness,
            region: stepped_trim_with_addons,
        });
    } else {
        ext_slabs.push(Slab {
            z0: 0.0,
            z1: params.trim_thickness,
            region: ext_trim_with_addons,
        });
    }

    if door_recess_from_ext > params.trim_thickness + 0.05 {
        ext_slabs.push(Slab {
            z0: params.trim_thickness,
            z1: door_recess_from_ext,
            region: ext_stop_band,
        });
    }
    if ext_pin_floor > door_recess_from_ext + 0.05 {
        ext_slabs.push(Slab {
            z0: door_recess_from_ext,
            z1: ext_pin_floor,
            region: ext_pocket_band,
        });
    }
    // Outer barrel with notched hinge sockets for remaining depth
    let barrel_z0 = ext_pin_floor;
    let barrel_z1 = ext_total.max(barrel_z0 + 1.0);
    if barrel_z0 > door_recess_from_ext + 0.05 {
        ext_slabs.push(Slab {
            z0: barrel_z0,
            z1: (barrel_z0 + 1.2).min(barrel_z1),
            region: ext_pocket_notched,
        });
    }
    ext_slabs.push(Slab {
        z0: (barrel_z0 + 1.2).min(barrel_z1),
        z1: barrel_z1,
        region: ext_barrel_notched,
    });

    let exterior_frame = extrude_slabs_with_moulding(&ext_slabs, ext_moulding);

    let mut int_socket_notches: Vec<Contour2D> = Vec::new();
    // Interior half is flipped 180 deg around Y in assembly (x_assembled = -x_bed),
    // so mirror the hinge socket X range on the interior print bed for single doors.
    let int_hinge_x_ranges: Vec<(f64, f64)> = if params.double_doors {
        hinge_x_ranges.clone()
    } else if params.hinge_right {
        vec![(left_hinge_x0 - clr, left_hinge_x0 + pin_w + clr)]
    } else {
        vec![(right_hinge_x1 - pin_w - clr, right_hinge_x1 + clr)]
    };

    for &(hx0, hx1) in &int_hinge_x_ranges {
        int_socket_notches.push(rect_bounds(
            hx0,
            y_pin_bot_top - pin_socket_len,
            hx1,
            y_pin_bot_top + 0.4,
        ));
        int_socket_notches.push(rect_bounds(
            hx0,
            y_pin_top_bot - 0.4,
            hx1,
            y_pin_top_bot + pin_socket_len,
        ));
    }
    let int_socket_shapes = union_contours(&int_socket_notches);

    // 3. Build Interior Frame Half (printed interior-face down at Z = 0)
    let int_spigot_outer_offset = -(barrel_w + params.index_clearance);
    let int_trim_band = profile_band(
        kind,
        w,
        h,
        -params.frame_width,
        params.interior_trim_width + params.wall_fit_clearance,
        ARC_SEGMENTS,
    );
    let int_trim_notched = difference_shapes(&int_trim_band, &int_socket_shapes);
    let int_spigot_band = profile_band(
        kind,
        w,
        h,
        -params.frame_width,
        int_spigot_outer_offset,
        ARC_SEGMENTS,
    );
    let int_spigot_notched = difference_shapes(&int_spigot_band, &int_socket_shapes);

    let mut int_slabs = Vec::new();
    // Solid cover [0..int_pin_cover] over hinge sockets
    int_slabs.push(Slab {
        z0: 0.0,
        z1: int_pin_cover.min(params.interior_trim_thickness - 0.1),
        region: int_trim_band,
    });
    if params.architrave_style.has_step()
        && params.chamfer_interior
        && params.interior_trim_thickness >= 1.4
        && params.interior_trim_width >= 2.2
    {
        let tier1_z = (params.interior_trim_thickness * 0.58)
            .max(specs.effective_int_chamfer * 1.15 + 0.25)
            .max(int_pin_cover + 0.2);
        let step_inset = (params.interior_trim_width * 0.36).clamp(0.5, 2.2);
        let stepped_int_band = profile_band(
            kind,
            w,
            h,
            -params.frame_width,
            (params.interior_trim_width - step_inset).max(0.8) + params.wall_fit_clearance,
            ARC_SEGMENTS,
        );
        let stepped_int_notched = difference_shapes(&stepped_int_band, &int_socket_shapes);
        if tier1_z < params.interior_trim_thickness - 0.1 {
            int_slabs.push(Slab {
                z0: int_pin_cover.min(params.interior_trim_thickness - 0.1),
                z1: tier1_z,
                region: int_trim_notched,
            });
            int_slabs.push(Slab {
                z0: tier1_z,
                z1: params.interior_trim_thickness,
                region: stepped_int_notched,
            });
        } else {
            int_slabs.push(Slab {
                z0: int_pin_cover.min(params.interior_trim_thickness - 0.1),
                z1: params.interior_trim_thickness,
                region: int_trim_notched,
            });
        }
    } else {
        int_slabs.push(Slab {
            z0: int_pin_cover.min(params.interior_trim_thickness - 0.1),
            z1: params.interior_trim_thickness,
            region: int_trim_notched,
        });
    }
    int_slabs.push(Slab {
        z0: params.interior_trim_thickness,
        z1: int_total.max(params.interior_trim_thickness + 1.0),
        region: int_spigot_notched,
    });

    let interior_frame = extrude_slabs_with_moulding(&int_slabs, int_moulding);

    // 4. Build Wall Cutaway (including pre-existing flush bosses on -Z)
    let wall_cutaway = build_wall_cutaway_mesh(kind, params, specs);

    // 5. Build Hinged Door Leaves (1 for single, 2 for double French doors)
    let mut door_leaves = Vec::new();
    let mut ac_w_out = 0.0;
    let mut ac_h_out = 0.0;
    let mut letterbox_w_for_print: Option<f64> = None;
    let mut cat_flap_for_print: Option<(f64, f64)> = None;

    if params.double_doors {
        let leaf_w = (inner_w - 3.0 * clr) * 0.5;
        let left_leaf = build_single_leaf(
            params,
            kind,
            w,
            h,
            leaf_w,
            inner_h - 2.0 * clr,
            false,
            true,
            true,
            y_pin_bot_top,
            y_pin_top_bot,
            pin_w,
            pin_len,
            pin_inset_from_edge,
            -inner_w * 0.5 + clr + leaf_w * 0.5,
            &mut ac_w_out,
            &mut ac_h_out,
            &mut letterbox_w_for_print,
            &mut cat_flap_for_print,
        );
        let right_leaf = build_single_leaf(
            params,
            kind,
            w,
            h,
            leaf_w,
            inner_h - 2.0 * clr,
            true,
            true,
            false,
            y_pin_bot_top,
            y_pin_top_bot,
            pin_w,
            pin_len,
            pin_inset_from_edge,
            inner_w * 0.5 - clr - leaf_w * 0.5,
            &mut ac_w_out,
            &mut ac_h_out,
            &mut None,
            &mut None,
        );
        door_leaves.push(left_leaf);
        door_leaves.push(right_leaf);
    } else {
        let leaf_w = inner_w - 2.0 * clr;
        let leaf = build_single_leaf(
            params,
            kind,
            w,
            h,
            leaf_w,
            inner_h - 2.0 * clr,
            params.hinge_right,
            false,
            true,
            y_pin_bot_top,
            y_pin_top_bot,
            pin_w,
            pin_len,
            pin_inset_from_edge,
            0.0,
            &mut ac_w_out,
            &mut ac_h_out,
            &mut letterbox_w_for_print,
            &mut cat_flap_for_print,
        );
        door_leaves.push(leaf);
    }

    // 6. Build Separately Printable Door Handle + Escutcheon Backplate + Letterbox / Cat-Flap Set (flat on Z=0)
    let hardware_parts = build_printable_hardware_parts(
        params.door_handle_style,
        if params.double_doors { 4 } else { 2 },
        letterbox_w_for_print,
        cat_flap_for_print,
    );
    let door_handle = build_printable_hardware_set(
        params.door_handle_style,
        if params.double_doors { 4 } else { 2 },
        letterbox_w_for_print,
        cat_flap_for_print,
    );

    DoorBuildOutput {
        exterior_frame,
        interior_frame,
        wall_cutaway,
        door_leaves,
        door_handle,
        hardware_parts,
        acetate_cut_w: ac_w_out,
        acetate_cut_h: ac_h_out,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_single_leaf(
    params: &WindowParams,
    kind: ProfileKind,
    frame_w: f64,
    frame_h: f64,
    leaf_w: f64,
    leaf_h: f64,
    assembled_hinge_right: bool,
    is_double_leaf: bool,
    allow_door_features: bool,
    y_pin_bot: f64,
    y_pin_top: f64,
    pin_w: f64,
    pin_len: f64,
    pin_inset: f64,
    assembled_x_offset: f64,
    ac_w_out: &mut f64,
    ac_h_out: &mut f64,
    letterbox_w_out: &mut Option<f64>,
    cat_flap_out: &mut Option<(f64, f64)>,
) -> DoorLeafAsset {
    let door_t = params.door_thickness;
    // Printed interior-down and flipped 180 deg around Y to assemble:
    let bed_hinge_right = !assembled_hinge_right;

    // Outer contour of the door leaf body on the print bed (centered at leaf_x = 0, y = 0)
    let leaf_body_contour: Contour2D = if kind == ProfileKind::RoundArch {
        let full_arch = profile_contour(
            ProfileKind::RoundArch,
            frame_w,
            frame_h,
            -(params.frame_width + params.hinge_clearance),
            ARC_SEGMENTS,
        );
        if is_double_leaf {
            let half_clip = if assembled_hinge_right {
                shape_from_contour(rect_bounds(
                    params.hinge_clearance * 0.5,
                    -frame_h,
                    frame_w,
                    frame_h,
                ))
            } else {
                shape_from_contour(rect_bounds(
                    -frame_w,
                    -frame_h,
                    -params.hinge_clearance * 0.5,
                    frame_h,
                ))
            };
            let inter = intersection_shapes(&shape_from_contour(full_arch), &half_clip);
            inter
                .into_iter()
                .next()
                .and_then(|s| s.into_iter().next())
                .map(|c| {
                    // Shift to leaf center, then mirror X for interior-down print bed (reverse to preserve CCW)
                    let mut mirrored: Contour2D = c
                        .into_iter()
                        .map(|p| [-(p[0] - assembled_x_offset), p[1]])
                        .collect();
                    mirrored.reverse();
                    mirrored
                })
                .unwrap_or_else(|| rect_centered(0.0, 0.0, leaf_w, leaf_h))
        } else {
            full_arch
        }
    } else {
        rect_centered(0.0, 0.0, leaf_w, leaf_h)
    };

    // Pin X coordinates in bed-local space
    let (px0, px1) = if bed_hinge_right {
        (leaf_w * 0.5 - pin_inset - pin_w, leaf_w * 0.5 - pin_inset)
    } else {
        (-leaf_w * 0.5 + pin_inset, -leaf_w * 0.5 + pin_inset + pin_w)
    };
    let pivot_x = ((px0 + px1) * 0.5) as f32;

    // Top and bottom pivot pin tabs
    let bot_pin = rect_bounds(px0, y_pin_bot - pin_len, px1, y_pin_bot + 1.2);
    let top_pin = rect_bounds(px0, y_pin_top - 1.2, px1, y_pin_top + pin_len);
    let body_with_pins = union_shapes(
        &shape_from_contour(leaf_body_contour.clone()),
        &union_contours(&[bot_pin, top_pin]),
    );

    // Handle through-hole at the latch side (`2.1 x 2.1 mm`), plus optional letterbox & cat flap cutouts
    let handle_x = if bed_hinge_right {
        -leaf_w * 0.5 + (leaf_w * 0.14).clamp(4.0, 8.0)
    } else {
        leaf_w * 0.5 - (leaf_w * 0.14).clamp(4.0, 8.0)
    };
    let handle_y = -leaf_h * 0.06;

    // Check glazing height limits first so letterbox & cat flap stay clear of glass pockets
    let is_glazed = matches!(
        params.style,
        WindowStyle::DoorGlazedFrench | WindowStyle::DoorFanlightGeorgian
    ) || (params.style == WindowStyle::DoorArchedHeritage
        && params.bars_x + params.bars_y > 0);
    let half_w_inset = (leaf_w * 0.16).clamp(4.0, 10.0);
    let (glass_y_bot, glass_y_top) = if params.style == WindowStyle::DoorFanlightGeorgian {
        let top = leaf_h * 0.5 - half_w_inset.max(5.5);
        let gw_est = (leaf_w - 2.0 * half_w_inset).max(6.0);
        let bot = (top - gw_est * 0.58).max(leaf_h * 0.10);
        (bot, top)
    } else {
        let bot = -leaf_h * 0.5 + leaf_h * params.lower_panel_ratio + 3.0;
        let top = if kind == ProfileKind::RoundArch {
            y_pin_top - 1.5
        } else {
            leaf_h * 0.5 - half_w_inset
        };
        (bot, top)
    };

    let mut cut_contours: Vec<Contour2D> = Vec::new();
    if params.door_handle_style != DoorHandleStyle::None {
        if params.door_handle_style == DoorHandleStyle::PullBar {
            cut_contours.push(rect_centered(handle_x, handle_y - 5.2, 2.1, 2.1));
            cut_contours.push(rect_centered(handle_x, handle_y + 5.2, 2.1, 2.1));
        } else {
            cut_contours.push(rect_centered(handle_x, handle_y, 2.1, 2.1));
        }
    }

    let solid_top_limit = if is_glazed {
        glass_y_bot - 4.2
    } else if kind == ProfileKind::RoundArch {
        y_pin_top - 4.0
    } else {
        leaf_h * 0.5 - 7.0
    };

    let mut cat_flap_placement: Option<(f64, f64, f64)> = None; // (cy, cf_w, cf_h)
    if allow_door_features && params.cat_flap_enabled {
        let cf_y_min = -leaf_h * 0.5 + 3.8;
        let max_cf_w = (leaf_w - 8.5).max(0.0);
        let max_cf_h = (solid_top_limit - cf_y_min).max(0.0);
        let cf_w = params.cat_flap_width.min(max_cf_w);
        let cf_h = params.cat_flap_height.min(max_cf_h);
        if cf_w >= 7.5 && cf_h >= 8.0 {
            let cf_cy = cf_y_min + cf_h * 0.5;
            cut_contours.push(rect_centered(0.0, cf_cy, cf_w, cf_h));
            cat_flap_placement = Some((cf_cy, cf_w, cf_h));
            *cat_flap_out = Some((cf_w, cf_h));
        }
    }

    let mut letterbox_placement: Option<(f64, f64, f64)> = None; // (lb_y, lb_w, lb_h)
    if allow_door_features && params.letterbox_enabled {
        let lb_h = 3.2;
        let max_w_edge = (leaf_w - 10.0).min((handle_x.abs() - 4.0).max(0.0) * 2.0);
        let lb_w = (leaf_w * 0.36).clamp(9.0, 22.0).min(max_w_edge);
        let lb_y_min = match cat_flap_placement {
            Some((cf_cy, _, cf_h)) => cf_cy + cf_h * 0.5 + lb_h * 0.5 + 4.0,
            None => -leaf_h * 0.5 + 8.0,
        };
        let lb_y_max = solid_top_limit - lb_h * 0.5 - 2.0;
        if lb_w >= 8.0 && lb_y_max >= lb_y_min {
            let raw_y = -leaf_h * 0.5 + leaf_h * params.letterbox_height_ratio;
            let lb_y = raw_y.clamp(lb_y_min, lb_y_max);
            cut_contours.push(rect_centered(0.0, lb_y, lb_w, lb_h));
            letterbox_placement = Some((lb_y, lb_w, lb_h));
            *letterbox_w_out = Some(lb_w);
        }
    }

    let handle_hole = union_contours(&cut_contours);
    // Note `ext_face` mirrors X (`-x`), so handle cutouts are mirrored while x=0 features stay centered
    let mut ext_cut_contours: Vec<Contour2D> = Vec::new();
    if params.door_handle_style != DoorHandleStyle::None {
        if params.door_handle_style == DoorHandleStyle::PullBar {
            ext_cut_contours.push(rect_centered(-handle_x, handle_y - 5.2, 2.1, 2.1));
            ext_cut_contours.push(rect_centered(-handle_x, handle_y + 5.2, 2.1, 2.1));
        } else {
            ext_cut_contours.push(rect_centered(-handle_x, handle_y, 2.1, 2.1));
        }
    }
    if let Some((cf_cy, cf_w, cf_h)) = cat_flap_placement {
        ext_cut_contours.push(rect_centered(0.0, cf_cy, cf_w, cf_h));
    }
    if let Some((lb_y, lb_w, lb_h)) = letterbox_placement {
        ext_cut_contours.push(rect_centered(0.0, lb_y, lb_w, lb_h));
    }
    let ext_cut_shapes = union_contours(&ext_cut_contours);

    let assembled_hx = -handle_x;

    // Actual through-hole cutouts in bed-local coordinates (NOT outer hardware surrounds) so
    // fixed decorative panels never move or split, and only grooves crossing a hole are clipped
    // cleanly underneath the hardware flange.
    let gw = params.groove_width.clamp(0.55, 1.4);
    let mut hole_cutouts: Vec<[f64; 4]> = Vec::new();
    if let Some((lb_y, lb_w, lb_h)) = letterbox_placement {
        hole_cutouts.push([
            -lb_w * 0.5,
            lb_y - lb_h * 0.5,
            lb_w * 0.5,
            lb_y + lb_h * 0.5,
        ]);
    }
    if let Some((cf_cy, cf_w, cf_h)) = cat_flap_placement {
        hole_cutouts.push([
            -cf_w * 0.5,
            cf_cy - cf_h * 0.5,
            cf_w * 0.5,
            cf_cy + cf_h * 0.5,
        ]);
    }
    if params.door_handle_style != DoorHandleStyle::None {
        if params.door_handle_style == DoorHandleStyle::PullBar {
            hole_cutouts.push([
                handle_x - 1.05,
                handle_y - 5.2 - 1.05,
                handle_x + 1.05,
                handle_y - 5.2 + 1.05,
            ]);
            hole_cutouts.push([
                handle_x - 1.05,
                handle_y + 5.2 - 1.05,
                handle_x + 1.05,
                handle_y + 5.2 + 1.05,
            ]);
        } else {
            hole_cutouts.push([
                handle_x - 1.05,
                handle_y - 1.05,
                handle_x + 1.05,
                handle_y + 1.05,
            ]);
        }
    }

    let stile = (leaf_w * 0.165).clamp(5.0, 11.2);
    let panel_x0 = -leaf_w * 0.5 + stile;
    let panel_x1 = leaf_w * 0.5 - stile;
    let panel_w = (panel_x1 - panel_x0).max(4.0);
    let panel_cx = (panel_x0 + panel_x1) * 0.5;
    let two_cols = panel_w >= 20.0;

    if is_glazed {
        // Two-half glazed door leaf sandwiching cut-to-size acetate!
        let glass_w = (leaf_w - 2.0 * half_w_inset).max(6.0);
        let glass_h = (glass_y_top - glass_y_bot).max(8.0);
        let glass_cy = (glass_y_bot + glass_y_top) * 0.5;

        let (glass_shape, bar_shapes) = if params.style == WindowStyle::DoorFanlightGeorgian {
            let mut fan_contour =
                profile_contour(ProfileKind::HalfRoundArch, glass_w, glass_h, 0.0, 28);
            for p in &mut fan_contour {
                p[1] += glass_cy;
            }
            let mut bars = Vec::new();
            let max_r = (glass_w * 0.5).max(glass_h) + 3.0;
            let spokes = params.arch_spokes.max(2);
            for i in 1..=spokes {
                let a = PI * (i as f64 / (spokes + 1) as f64);
                bars.push(segment_bar(
                    [0.0, glass_y_bot - 0.3],
                    [max_r * a.cos(), glass_y_bot + max_r * a.sin()],
                    params.bar_width,
                ));
            }
            let mut bar_u = union_contours(&bars);
            if params.bars_x > 0 && (glass_w * 0.5).min(glass_h) > 6.0 {
                let ring_r = (glass_w * 0.5).min(glass_h) * 0.40;
                let ring = ring_shape(
                    0.0,
                    glass_y_bot,
                    (ring_r - params.bar_width * 0.5).max(0.7),
                    ring_r + params.bar_width * 0.5,
                    28,
                );
                bar_u = union_shapes(&bar_u, &vec![ring]);
            }
            (shape_from_contour(fan_contour), bar_u)
        } else {
            let glass_rect = rect_centered(0.0, glass_cy, glass_w, glass_h);
            let mut bars = Vec::new();
            for c in 1..=params.bars_x {
                let x = -glass_w * 0.5 + glass_w * (c as f64 / (params.bars_x + 1) as f64);
                bars.push(rect_centered(x, glass_cy, params.bar_width, glass_h + 2.0));
            }
            for r in 1..=params.bars_y {
                let y = glass_y_bot + glass_h * (r as f64 / (params.bars_y + 1) as f64);
                bars.push(rect_centered(0.0, y, glass_w + 2.0, params.bar_width));
            }
            (shape_from_contour(glass_rect), union_contours(&bars))
        };
        let glass_with_bars = difference_shapes(&glass_shape, &bar_shapes);

        let mut base_panel_rects: Vec<[f64; 4]> = Vec::new();
        let fanlight_gw = gw.clamp(0.55, 1.15);
        let fanlight_gd = fanlight_gw * 0.58;
        if params.style == WindowStyle::DoorFanlightGeorgian || glass_y_bot > -leaf_h * 0.5 + 24.0 {
            let lower_span_top = glass_y_bot - 5.0;
            let lower_span_bot = -leaf_h * 0.5 + 7.5;
            let lower_span = lower_span_top - lower_span_bot;
            if panel_w >= fanlight_gw + 3.0 && lower_span >= fanlight_gw + 6.0 {
                let row_centers: Vec<(f64, f64)> = if lower_span >= fanlight_gw * 2.0 + 18.0 {
                    let row_gap = (lower_span * 0.12).clamp(3.5, 7.0);
                    let row_h = (lower_span - row_gap) * 0.5;
                    vec![
                        (lower_span_top - row_h * 0.5, row_h),
                        (lower_span_bot + row_h * 0.5, row_h),
                    ]
                } else {
                    vec![((lower_span_bot + lower_span_top) * 0.5, lower_span)]
                };
                if two_cols {
                    let cgap = (panel_w * 0.14).clamp(3.0, 6.0);
                    let cw = (panel_w - cgap) * 0.5;
                    let cxl = panel_cx - cgap * 0.5 - cw * 0.5;
                    let cxr = panel_cx + cgap * 0.5 + cw * 0.5;
                    for (cy, row_h) in row_centers {
                        base_panel_rects.push([
                            cxl - cw * 0.5,
                            cy - row_h * 0.5,
                            cxl + cw * 0.5,
                            cy + row_h * 0.5,
                        ]);
                        base_panel_rects.push([
                            cxr - cw * 0.5,
                            cy - row_h * 0.5,
                            cxr + cw * 0.5,
                            cy + row_h * 0.5,
                        ]);
                    }
                } else {
                    for (cy, row_h) in row_centers {
                        base_panel_rects.push([
                            panel_cx - panel_w * 0.5,
                            cy - row_h * 0.5,
                            panel_cx + panel_w * 0.5,
                            cy + row_h * 0.5,
                        ]);
                    }
                }
            }
        }

        let (adapted_grooves, adapted_open_grooves) = clip_panel_grooves_around_cutouts(
            &base_panel_rects,
            &hole_cutouts,
            fanlight_gw,
            fanlight_gd,
        );

        // Acetate pocket around glazing opening
        let ac_margin = 1.8;
        let ac_w = (glass_w + 2.0 * ac_margin).min(leaf_w - 2.6);
        let ac_h = (glass_h + 2.0 * ac_margin).min(leaf_h - 4.0);
        *ac_w_out = ac_w - 0.3;
        *ac_h_out = ac_h - 0.3;

        let leaf_ac_region = shape_from_contour(rect_centered(0.0, glass_cy, *ac_w_out, *ac_h_out));
        let leaf_acetate = extrude_slabs(&[Slab {
            z0: 0.0,
            z1: params.acetate_thickness,
            region: leaf_ac_region,
        }]);

        // Interior door half (carries the octagonal/beveled pivot pins + outer barrel around acetate)
        let int_half_t = ((door_t - params.acetate_thickness) * 0.5).max(1.2);
        let total_leaf_t = int_half_t * 2.0 + params.acetate_thickness;
        let int_face_region = difference_shapes(
            &difference_shapes(&body_with_pins, &glass_with_bars),
            &handle_hole,
        );
        let body_no_pins_face = difference_shapes(
            &difference_shapes(
                &shape_from_contour(leaf_body_contour.clone()),
                &glass_with_bars,
            ),
            &handle_hole,
        );
        let ac_pocket_cut = shape_from_contour(rect_centered(0.0, glass_cy, ac_w, ac_h));
        let int_rim_region = difference_shapes(
            &difference_shapes(
                &shape_from_contour(leaf_body_contour.clone()),
                &ac_pocket_cut,
            ),
            &handle_hole,
        );

        let mut int_slabs = Vec::new();
        int_slabs.push(Slab {
            z0: 0.0,
            z1: 1.55,
            region: int_face_region.clone(),
        });
        if int_half_t > 1.6 {
            int_slabs.push(Slab {
                z0: 1.55,
                z1: int_half_t,
                region: body_no_pins_face.clone(),
            });
        }
        int_slabs.push(Slab {
            z0: int_half_t.max(1.55),
            z1: int_half_t.max(1.55) + params.acetate_thickness + 0.6,
            region: int_rim_region,
        });

        let ext_grooves: Vec<VGroove> = adapted_grooves
            .iter()
            .map(|g| {
                let mut c: Contour2D = g.centerline.iter().map(|p| [-p[0], p[1]]).collect();
                c.reverse();
                VGroove {
                    centerline: c,
                    width: g.width,
                    depth: g.depth,
                }
            })
            .collect();
        let ext_open_grooves: Vec<VGroove> = adapted_open_grooves
            .iter()
            .map(|g| VGroove {
                centerline: g.centerline.iter().map(|p| [-p[0], p[1]]).collect(),
                width: g.width,
                depth: g.depth,
            })
            .collect();
        let (int_grooves, int_open_grooves) = if params.door_interior_panels {
            (adapted_grooves, adapted_open_grooves)
        } else {
            (Vec::new(), Vec::new())
        };
        let pin_bevel = Some((0.50, 0.58));
        let int_mesh = extrude_slabs_with_moulding(
            &int_slabs,
            &MouldingConfig {
                outer_chamfer: pin_bevel,
                hole_chamfer: if params.chamfer_bars {
                    Some((
                        params.bar_chamfer_size.min(params.bar_width * 0.30),
                        params.bar_chamfer_size.min(params.bar_width * 0.30) * 1.15,
                    ))
                } else {
                    None
                },
                grooves: int_grooves,
                top_grooves: Vec::new(),
                open_grooves: int_open_grooves,
                top_open_grooves: Vec::new(),
            },
        );

        // Exterior door half (printed exterior-down, NOT flipped around Y in assembly, so uses `ext_cut_shapes`)
        let ext_leaf_contour: Contour2D = leaf_body_contour
            .iter()
            .rev()
            .map(|p| [-p[0], p[1]])
            .collect();
        let ext_face_region = difference_shapes(
            &difference_shapes(&shape_from_contour(ext_leaf_contour), &glass_with_bars),
            &ext_cut_shapes,
        );
        let ext_spigot_outer =
            shape_from_contour(rect_centered(0.0, glass_cy, ac_w - 0.25, ac_h - 0.25));
        let ext_spigot_region = difference_shapes(&ext_spigot_outer, &glass_with_bars);
        let ext_slabs = vec![
            Slab {
                z0: 0.0,
                z1: int_half_t,
                region: ext_face_region,
            },
            Slab {
                z0: int_half_t,
                z1: int_half_t + 0.55,
                region: ext_spigot_region,
            },
        ];
        let ext_mesh = extrude_slabs_with_moulding(
            &ext_slabs,
            &MouldingConfig {
                outer_chamfer: Some((0.35, 0.40)),
                hole_chamfer: if params.chamfer_bars {
                    Some((
                        params.bar_chamfer_size.min(params.bar_width * 0.30),
                        params.bar_chamfer_size.min(params.bar_width * 0.30) * 1.15,
                    ))
                } else {
                    None
                },
                grooves: ext_grooves,
                top_grooves: Vec::new(),
                open_grooves: ext_open_grooves,
                top_open_grooves: Vec::new(),
            },
        );

        let (hardware_preview, accessory_preview) = build_assembled_leaf_hardware(
            params.door_handle_style,
            total_leaf_t,
            assembled_hx,
            handle_y,
            assembled_hinge_right,
            letterbox_placement,
            cat_flap_placement,
        );

        DoorLeafAsset {
            mesh: int_mesh,
            exterior_leaf: Some(ext_mesh),
            leaf_acetate: Some(leaf_acetate),
            // After flipping 180 deg around Y in assembly, x becomes -pivot_x:
            pivot_x: -pivot_x,
            pivot_z: 1.1,
            assembled_x_offset: assembled_x_offset as f32,
            swing_dir: if assembled_hinge_right { -1.0 } else { 1.0 },
            handle_xy: [assembled_hx as f32, handle_y as f32],
            leaf_thickness: total_leaf_t as f32,
            hardware_preview,
            accessory_preview,
        }
    } else {
        // Solid panelled door leaf:
        // Printed interior-face down at Z=0 (`flip_y` in assembly maps Z=0 -> INTERIOR, Z=solid_t -> EXTERIOR).
        // Exterior raised-panel V-grooves are cut downward into Z=solid_t (`top_grooves` + `top_open_grooves`), while
        // optional interior panelling (`door_interior_panels`) is cut upward into Z=0 (`grooves` + `open_grooves`).
        let solid_t = door_t.max(2.6);
        let top_gd = (gw * 0.58).min((solid_t - 1.55 - 0.12).max(0.20));
        let bot_gd = (gw * 0.58).min(0.85);

        let mut base_panel_rects: Vec<[f64; 4]> = Vec::new();
        match params.style {
            WindowStyle::DoorGeorgian6Panel => {
                // Classic symmetric Georgian 6-panel proportions with balanced top/lock/kick rails
                let rows = [
                    (leaf_h * 0.34, leaf_h * 0.14),
                    (leaf_h * 0.12, leaf_h * 0.18),
                    (-leaf_h * 0.23, leaf_h * 0.30),
                ];
                if two_cols {
                    let col_gap = (panel_w * 0.14).clamp(3.0, 6.0);
                    let col_w = ((panel_w - col_gap) * 0.5).max(3.5);
                    let cx_l = panel_cx - col_gap * 0.5 - col_w * 0.5;
                    let cx_r = panel_cx + col_gap * 0.5 + col_w * 0.5;
                    for (cy, rh) in rows {
                        if rh > 4.0 {
                            base_panel_rects.push([
                                cx_l - col_w * 0.5,
                                cy - rh * 0.5,
                                cx_l + col_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                            base_panel_rects.push([
                                cx_r - col_w * 0.5,
                                cy - rh * 0.5,
                                cx_r + col_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                        }
                    }
                } else {
                    for (cy, rh) in rows {
                        if rh > 4.0 && panel_w > 4.0 {
                            base_panel_rects.push([
                                panel_cx - panel_w * 0.5,
                                cy - rh * 0.5,
                                panel_cx + panel_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                        }
                    }
                }
            }
            WindowStyle::DoorArchedHeritage => {
                // Keep upper panel strictly inside the rectangular trunk below the arch spring line.
                let arch_safe_top = if kind == ProfileKind::RoundArch {
                    (y_pin_top - 1.5).min(leaf_h * 0.36)
                } else {
                    leaf_h * 0.36
                };
                let bot_y0 = -leaf_h * 0.40;
                let bot_h = (leaf_h * 0.28).max(8.0);
                let bot_y1 = (bot_y0 + bot_h).min(arch_safe_top - 12.0);
                let top_y0 = (bot_y1 + 6.0).max(-leaf_h * 0.06);
                let top_y1 = arch_safe_top;
                if panel_w > 4.0 {
                    if top_y1 - top_y0 >= 6.0 {
                        base_panel_rects.push([
                            panel_cx - panel_w * 0.5,
                            top_y0,
                            panel_cx + panel_w * 0.5,
                            top_y1,
                        ]);
                    }
                    if bot_y1 - bot_y0 >= 6.0 {
                        base_panel_rects.push([
                            panel_cx - panel_w * 0.5,
                            bot_y0,
                            panel_cx + panel_w * 0.5,
                            bot_y1,
                        ]);
                    }
                }
            }
            _ => {
                // Balanced symmetric 4-Panel Colonial Door
                let upper_h = (leaf_h * 0.34).max(6.0);
                let lower_h = (leaf_h * 0.34).max(6.0);
                if two_cols {
                    let col_gap = (panel_w * 0.14).clamp(3.0, 6.0);
                    let col_w = ((panel_w - col_gap) * 0.5).max(3.5);
                    let cx_l = panel_cx - col_gap * 0.5 - col_w * 0.5;
                    let cx_r = panel_cx + col_gap * 0.5 + col_w * 0.5;
                    for (cy, rh) in [(leaf_h * 0.22, upper_h), (-leaf_h * 0.22, lower_h)] {
                        if rh > 4.0 {
                            base_panel_rects.push([
                                cx_l - col_w * 0.5,
                                cy - rh * 0.5,
                                cx_l + col_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                            base_panel_rects.push([
                                cx_r - col_w * 0.5,
                                cy - rh * 0.5,
                                cx_r + col_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                        }
                    }
                } else if panel_w > 4.0 {
                    for (cy, rh) in [(leaf_h * 0.22, upper_h), (-leaf_h * 0.22, lower_h)] {
                        if rh > 4.0 {
                            base_panel_rects.push([
                                panel_cx - panel_w * 0.5,
                                cy - rh * 0.5,
                                panel_cx + panel_w * 0.5,
                                cy + rh * 0.5,
                            ]);
                        }
                    }
                }
            }
        }

        let (ext_top_grooves, ext_top_open_grooves) =
            clip_panel_grooves_around_cutouts(&base_panel_rects, &hole_cutouts, gw, top_gd);
        let (int_bot_grooves, int_bot_open_grooves) = if params.door_interior_panels {
            clip_panel_grooves_around_cutouts(&base_panel_rects, &hole_cutouts, gw, bot_gd)
        } else {
            (Vec::new(), Vec::new())
        };

        let base_region = difference_shapes(&body_with_pins, &handle_hole);
        let body_no_pins = difference_shapes(&shape_from_contour(leaf_body_contour), &handle_hole);
        let slabs = vec![
            Slab {
                z0: 0.0,
                z1: 1.55,
                region: base_region,
            },
            Slab {
                z0: 1.55,
                z1: solid_t,
                region: body_no_pins,
            },
        ];
        let mesh = extrude_slabs_with_moulding(
            &slabs,
            &MouldingConfig {
                outer_chamfer: Some((0.50, 0.58)),
                hole_chamfer: None,
                grooves: int_bot_grooves,
                top_grooves: ext_top_grooves,
                open_grooves: int_bot_open_grooves,
                top_open_grooves: ext_top_open_grooves,
            },
        );

        let (hardware_preview, accessory_preview) = build_assembled_leaf_hardware(
            params.door_handle_style,
            solid_t,
            assembled_hx,
            handle_y,
            assembled_hinge_right,
            letterbox_placement,
            cat_flap_placement,
        );

        DoorLeafAsset {
            mesh,
            exterior_leaf: None,
            leaf_acetate: None,
            pivot_x: -pivot_x,
            pivot_z: 1.1,
            assembled_x_offset: assembled_x_offset as f32,
            swing_dir: if assembled_hinge_right { -1.0 } else { 1.0 },
            handle_xy: [assembled_hx as f32, handle_y as f32],
            leaf_thickness: solid_t as f32,
            hardware_preview,
            accessory_preview,
        }
    }
}

/// Preserves fixed door panel rectangles without moving, resizing, or splitting them.
/// Whenever a cutout hole crosses a panel's perimeter groove, only the intersecting groove segment
/// is clipped back slightly beyond the hole wall (leaving its ends hidden beneath the hardware surround),
/// emitting continuous chained open-polyline V-grooves for the surviving outline.
fn clip_panel_grooves_around_cutouts(
    panels: &[[f64; 4]],
    cutouts: &[[f64; 4]],
    gw: f64,
    gd: f64,
) -> (Vec<VGroove>, Vec<VGroove>) {
    let mut closed_grooves = Vec::new();
    let mut open_grooves = Vec::new();
    let trans_clr = gw * 0.5 + 0.35;
    let long_clr = gw + 0.35;
    let min_poly_len = (gw * 1.8).max(1.5);

    for &p in panels {
        let [x0, y0, x1, y1] = p;
        let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
        let any_hit = cutouts.iter().any(|&c| {
            let [cx0, cy0, cx1, cy1] = [
                c[0] - long_clr,
                c[1] - long_clr,
                c[2] + long_clr,
                c[3] + long_clr,
            ];
            let bot_hit = (y0 > cy0 && y0 < cy1) && (x1 > cx0 && x0 < cx1);
            let top_hit = (y1 > cy0 && y1 < cy1) && (x1 > cx0 && x0 < cx1);
            let left_hit = (x0 > cx0 && x0 < cx1) && (y1 > cy0 && y0 < cy1);
            let right_hit = (x1 > cx0 && x1 < cx1) && (y1 > cy0 && y0 < cy1);
            bot_hit || top_hit || left_hit || right_hit
        });

        if !any_hit {
            closed_grooves.push(VGroove {
                centerline: rect_bounds(x0, y0, x1, y1),
                width: gw,
                depth: gd,
            });
            continue;
        }

        // Clip each of the 4 CCW perimeter edges against all cutouts and emit chained polylines.
        let mut chains: Vec<Vec<[f64; 2]>> = Vec::new();
        for edge_idx in 0..4 {
            let a = corners[edge_idx];
            let b = corners[(edge_idx + 1) % 4];
            let is_horiz = edge_idx % 2 == 0;
            let len = if is_horiz {
                (b[0] - a[0]).abs()
            } else {
                (b[1] - a[1]).abs()
            };
            if len < 1e-4 {
                continue;
            }
            let mut intervals: Vec<(f64, f64)> = vec![(0.0, 1.0)];
            for &c in cutouts {
                let (perp_hit, lo, hi) = if is_horiz {
                    let y = a[1];
                    let hit = y > c[1] - trans_clr && y < c[3] + trans_clr;
                    if (b[0] - a[0]) > 0.0 {
                        (
                            hit,
                            ((c[0] - long_clr) - a[0]) / (b[0] - a[0]),
                            ((c[2] + long_clr) - a[0]) / (b[0] - a[0]),
                        )
                    } else {
                        (
                            hit,
                            ((c[2] + long_clr) - a[0]) / (b[0] - a[0]),
                            ((c[0] - long_clr) - a[0]) / (b[0] - a[0]),
                        )
                    }
                } else {
                    let x = a[0];
                    let hit = x > c[0] - trans_clr && x < c[2] + trans_clr;
                    if (b[1] - a[1]) > 0.0 {
                        (
                            hit,
                            ((c[1] - long_clr) - a[1]) / (b[1] - a[1]),
                            ((c[3] + long_clr) - a[1]) / (b[1] - a[1]),
                        )
                    } else {
                        (
                            hit,
                            ((c[3] + long_clr) - a[1]) / (b[1] - a[1]),
                            ((c[1] - long_clr) - a[1]) / (b[1] - a[1]),
                        )
                    }
                };
                if !perp_hit {
                    continue;
                }
                let cut0 = lo.min(hi).clamp(0.0, 1.0);
                let cut1 = lo.max(hi).clamp(0.0, 1.0);
                if cut1 <= cut0 + 1e-6 {
                    continue;
                }
                let mut next_iv = Vec::with_capacity(intervals.len());
                for (s, e) in intervals {
                    if cut1 <= s + 1e-6 || cut0 >= e - 1e-6 {
                        next_iv.push((s, e));
                    } else {
                        if cut0 > s + 1e-6 {
                            next_iv.push((s, cut0));
                        }
                        if cut1 < e - 1e-6 {
                            next_iv.push((cut1, e));
                        }
                    }
                }
                intervals = next_iv;
            }

            for (s, e) in intervals {
                if (e - s) * len < 0.25 {
                    continue;
                }
                let p0 = [a[0] + (b[0] - a[0]) * s, a[1] + (b[1] - a[1]) * s];
                let p1 = [a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e];
                if let Some(last_chain) = chains.last_mut()
                    && let Some(&tail) = last_chain.last()
                    && (tail[0] - p0[0]).hypot(tail[1] - p0[1]) < 1e-4
                {
                    last_chain.push(p1);
                    continue;
                }
                chains.push(vec![p0, p1]);
            }
        }

        // Merge wrap-around corner [x0, y0] if last chain ends at first chain's start
        if chains.len() >= 2 {
            let tail = *chains.last().unwrap().last().unwrap();
            let head = chains[0][0];
            if (tail[0] - head[0]).hypot(tail[1] - head[1]) < 1e-4 {
                let first = chains.remove(0);
                let last = chains.last_mut().unwrap();
                last.extend_from_slice(&first[1..]);
            }
        }

        for chain in chains {
            let mut total_len = 0.0;
            for w in chain.windows(2) {
                total_len += (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
            }
            if total_len >= min_poly_len && chain.len() >= 2 {
                open_grooves.push(VGroove {
                    centerline: chain,
                    width: gw,
                    depth: gd,
                });
            }
        }
    }

    (closed_grooves, open_grooves)
}

/// Builds logically grouped printable door hardware parts (`door_handles`, `letterbox_plates`,
/// `cat_flap`) flat on `Z = 0` with zero unprintable overhangs.
pub fn build_printable_hardware_parts(
    style: DoorHandleStyle,
    handle_count: usize,
    letterbox_w: Option<f64>,
    cat_flap_dims: Option<(f64, f64)>,
) -> Vec<(String, TriMesh)> {
    let mut out = Vec::new();
    if style != DoorHandleStyle::None {
        let single = build_single_handle_and_plate_for_style(style);
        let copies = vec![single; handle_count];
        let arranged = arrange_meshes_along_x(&copies, 2.5);
        if !arranged.triangles.is_empty() {
            out.push(("door_handles".to_string(), arranged));
        }
    }
    if let Some(lb_w) = letterbox_w {
        let plate = build_printable_letterbox_plate(lb_w, 3.2);
        let arranged = arrange_meshes_along_x(&[plate.clone(), plate], 2.5);
        out.push(("letterbox_plates".to_string(), arranged));
    }
    if let Some((cf_w, cf_h)) = cat_flap_dims {
        let bezel = build_printable_cat_flap_bezel(cf_w, cf_h);
        let flap = build_printable_cat_flap_door(cf_w, cf_h);
        let arranged = arrange_meshes_along_x(&[bezel.clone(), bezel, flap], 2.5);
        out.push(("cat_flap".to_string(), arranged));
    }
    out
}

/// Builds printable door hardware (handles + backplates + letterbox plates + cat flap bezels/flap)
/// flat on `Z = 0` with zero unprintable overhangs.
pub fn build_printable_hardware_set(
    style: DoorHandleStyle,
    handle_count: usize,
    letterbox_w: Option<f64>,
    cat_flap_dims: Option<(f64, f64)>,
) -> TriMesh {
    let grouped = build_printable_hardware_parts(style, handle_count, letterbox_w, cat_flap_dims);
    let meshes: Vec<TriMesh> = grouped.into_iter().map(|(_, m)| m).collect();
    arrange_meshes_along_x(&meshes, 3.0)
}

/// Backward-compatible helper returning one Modern Lever handle + backplate flat on `Z = 0`.
pub fn build_single_handle_and_plate() -> TriMesh {
    build_single_handle_and_plate_for_style(DoorHandleStyle::ModernLever)
}

fn arrange_meshes_along_x(meshes: &[TriMesh], gap: f32) -> TriMesh {
    if meshes.is_empty() {
        return TriMesh::default();
    }
    let mut widths = Vec::with_capacity(meshes.len());
    let mut bounds = Vec::with_capacity(meshes.len());
    let mut total_w = 0.0f32;
    for m in meshes {
        let (bmin, bmax) = m.bounds();
        let w = (bmax[0] - bmin[0]).max(0.2);
        widths.push(w);
        bounds.push((bmin, bmax));
        total_w += w;
    }
    if meshes.len() > 1 {
        total_w += gap * (meshes.len() as f32 - 1.0);
    }
    let mut out = TriMesh::default();
    let mut cursor = -total_w * 0.5;
    for (i, m) in meshes.iter().enumerate() {
        let (bmin, bmax) = bounds[i];
        let cx = (bmin[0] + bmax[0]) * 0.5;
        let cy = (bmin[1] + bmax[1]) * 0.5;
        let mut copy = m.clone();
        copy.translate(cursor + widths[i] * 0.5 - cx, -cy, -bmin[2]);
        out.append(&copy);
        cursor += widths[i] + gap;
    }
    out
}

fn build_single_handle_and_plate_for_style(style: DoorHandleStyle) -> TriMesh {
    match style {
        DoorHandleStyle::ModernLever => {
            let peg = rect_bounds(-0.85, -4.5, 0.85, 0.2);
            let lever = rect_bounds(-1.1, 0.0, 8.2, 1.65);
            let tip = rect_bounds(7.3, -0.45, 8.6, 1.65);
            let lever_mesh = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 1.65,
                    region: union_contours(&[peg, lever, tip]),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.20, 0.24)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            let plate_outer = shape_from_contour(rect_centered(-5.8, 0.0, 3.8, 11.4));
            let plate_hole = shape_from_contour(rect_centered(-5.8, 0.0, 2.0, 2.0));
            let mut out = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 0.85,
                    region: difference_shapes(&plate_outer, &plate_hole),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.24, 0.28)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            out.append(&lever_mesh);
            out
        }
        DoorHandleStyle::VictorianScroll => {
            let peg = rect_bounds(-0.85, -4.5, 0.85, 0.2);
            let collar = rect_bounds(-1.3, 0.0, 1.4, 2.0);
            let arch_a = rect_bounds(0.6, 0.4, 5.2, 1.9);
            let arch_b = rect_bounds(4.4, -0.1, 8.2, 1.45);
            let scroll_end = circle_contour(8.0, 0.25, 1.3, 20);
            let lever_mesh = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 1.7,
                    region: union_contours(&[peg, collar, arch_a, arch_b, scroll_end]),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.20, 0.24)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            let plate_outer = union_contours(&[
                rect_centered(-6.0, 0.0, 4.2, 9.6),
                circle_contour(-6.0, 4.8, 2.1, 20),
                circle_contour(-6.0, -4.8, 2.1, 20),
            ]);
            let plate_hole = shape_from_contour(rect_centered(-6.0, 0.0, 2.0, 2.0));
            let mut out = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 0.9,
                    region: difference_shapes(&plate_outer, &plate_hole),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.22, 0.26)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            out.append(&lever_mesh);
            out
        }
        DoorHandleStyle::RoundKnob => {
            // Face-down turned brass knob (0 overhangs!) + circular chamfered rose
            let head = shape_from_contour(circle_contour(0.0, 0.0, 2.6, 24));
            let neck = shape_from_contour(circle_contour(0.0, 0.0, 1.45, 20));
            let spindle = shape_from_contour(rect_centered(0.0, 0.0, 1.7, 1.7));
            let knob_mesh = extrude_slabs_with_moulding(
                &[
                    Slab {
                        z0: 0.0,
                        z1: 1.4,
                        region: head,
                    },
                    Slab {
                        z0: 1.4,
                        z1: 2.5,
                        region: neck,
                    },
                    Slab {
                        z0: 2.5,
                        z1: 5.8,
                        region: spindle,
                    },
                ],
                &MouldingConfig {
                    outer_chamfer: Some((0.36, 0.42)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            let rose_outer = shape_from_contour(circle_contour(-6.6, 0.0, 3.3, 24));
            let rose_hole = shape_from_contour(rect_centered(-6.6, 0.0, 2.0, 2.0));
            let mut out = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 0.9,
                    region: difference_shapes(&rose_outer, &rose_hole),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.22, 0.26)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            out.append(&knob_mesh);
            out
        }
        DoorHandleStyle::RingLatch => {
            let ring = vec![ring_shape(0.0, -3.5, 2.2, 3.8, 28)];
            let lug = union_contours(&[
                rect_bounds(-1.1, -0.2, 1.1, 1.5),
                rect_bounds(-0.85, 1.2, 0.85, 5.2),
            ]);
            let ring_mesh = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 1.5,
                    region: union_shapes(&ring, &lug),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.20, 0.24)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            let kite = vec![[-6.8, -4.8], [-4.2, 0.0], [-6.8, 4.8], [-9.4, 0.0]];
            let hole = shape_from_contour(rect_centered(-6.8, 0.0, 2.0, 2.0));
            let mut out = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 0.9,
                    region: difference_shapes(&shape_from_contour(kite), &hole),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.22, 0.26)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            out.append(&ring_mesh);
            out
        }
        DoorHandleStyle::PullBar => {
            let bar = rect_bounds(2.4, -7.2, 4.3, 7.2);
            let st_top = rect_bounds(-0.2, 4.2, 4.2, 5.9);
            let st_bot = rect_bounds(-0.2, -5.9, 4.2, -4.2);
            let peg_top = rect_bounds(-3.6, 4.25, 0.1, 5.85);
            let peg_bot = rect_bounds(-3.6, -5.85, 0.1, -4.25);
            let pull_mesh = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 1.8,
                    region: union_contours(&[bar, st_top, st_bot, peg_top, peg_bot]),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.22, 0.26)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            let strip = shape_from_contour(rect_centered(-6.2, 0.0, 3.4, 17.5));
            let holes = union_contours(&[
                rect_centered(-6.2, -5.05, 2.0, 2.0),
                rect_centered(-6.2, 5.05, 2.0, 2.0),
            ]);
            let mut out = extrude_slabs_with_moulding(
                &[Slab {
                    z0: 0.0,
                    z1: 0.85,
                    region: difference_shapes(&strip, &holes),
                }],
                &MouldingConfig {
                    outer_chamfer: Some((0.22, 0.26)),
                    hole_chamfer: None,
                    grooves: Vec::new(),
                    top_grooves: Vec::new(),
                    open_grooves: Vec::new(),
                    top_open_grooves: Vec::new(),
                },
            );
            out.append(&pull_mesh);
            out
        }
        DoorHandleStyle::None => TriMesh::default(),
    }
}

fn build_printable_letterbox_plate(lb_w: f64, lb_h: f64) -> TriMesh {
    let outer = shape_from_contour(rect_centered(0.0, 0.0, lb_w + 4.4, lb_h + 3.6));
    let slot = shape_from_contour(rect_centered(0.0, 0.0, lb_w * 0.92, (lb_h * 0.65).max(1.5)));
    extrude_slabs_with_moulding(
        &[Slab {
            z0: 0.0,
            z1: 0.95,
            region: difference_shapes(&outer, &slot),
        }],
        &MouldingConfig {
            outer_chamfer: Some((0.22, 0.26)),
            hole_chamfer: None,
            grooves: Vec::new(),
            top_grooves: Vec::new(),
            open_grooves: Vec::new(),
            top_open_grooves: Vec::new(),
        },
    )
}

fn build_printable_cat_flap_bezel(cf_w: f64, cf_h: f64) -> TriMesh {
    let outer = shape_from_contour(rect_centered(0.0, 0.0, cf_w + 3.6, cf_h + 3.6));
    let hole = shape_from_contour(rect_centered(0.0, 0.0, cf_w, cf_h));
    extrude_slabs_with_moulding(
        &[Slab {
            z0: 0.0,
            z1: 1.05,
            region: difference_shapes(&outer, &hole),
        }],
        &MouldingConfig {
            outer_chamfer: Some((0.24, 0.28)),
            hole_chamfer: None,
            grooves: Vec::new(),
            top_grooves: Vec::new(),
            open_grooves: Vec::new(),
            top_open_grooves: Vec::new(),
        },
    )
}

fn build_printable_cat_flap_door(cf_w: f64, cf_h: f64) -> TriMesh {
    let flap = shape_from_contour(rect_centered(
        0.0,
        0.0,
        (cf_w - 0.6).max(4.0),
        (cf_h - 0.6).max(4.0),
    ));
    extrude_slabs_with_moulding(
        &[Slab {
            z0: 0.0,
            z1: 1.2,
            region: flap,
        }],
        &MouldingConfig {
            outer_chamfer: Some((0.20, 0.24)),
            hole_chamfer: None,
            grooves: Vec::new(),
            top_grooves: Vec::new(),
            open_grooves: Vec::new(),
            top_open_grooves: Vec::new(),
        },
    )
}

fn build_assembled_leaf_hardware(
    style: DoorHandleStyle,
    leaf_t: f64,
    hx: f64,
    hy: f64,
    hinge_right: bool,
    letterbox_placement: Option<(f64, f64, f64)>,
    cat_flap_placement: Option<(f64, f64, f64)>,
) -> (TriMesh, TriMesh) {
    let mut hw = TriMesh::default();
    let mut acc = TriMesh::default();

    // Helper: places a z-extruded face plate flush on interior (+Z at z=0) & exterior (-Z at z=-leaf_t)
    let add_paired_plate = |target: &mut TriMesh,
                            region: &crate::polygon2d::Shapes2D,
                            thick: f64,
                            cx: f64,
                            cy: f64| {
        let mut m_ext = extrude_slabs_with_moulding(
            &[Slab {
                z0: 0.0,
                z1: thick,
                region: region.clone(),
            }],
            &MouldingConfig {
                outer_chamfer: Some((0.20, 0.24)),
                hole_chamfer: None,
                grooves: Vec::new(),
                top_grooves: Vec::new(),
                open_grooves: Vec::new(),
                top_open_grooves: Vec::new(),
            },
        );
        m_ext.translate(cx as f32, cy as f32, -(leaf_t + thick) as f32);
        target.append(&m_ext);

        let mut m_int = extrude_slabs_with_moulding(
            &[Slab {
                z0: 0.0,
                z1: thick,
                region: region.clone(),
            }],
            &MouldingConfig {
                outer_chamfer: Some((0.20, 0.24)),
                hole_chamfer: None,
                grooves: Vec::new(),
                top_grooves: Vec::new(),
                open_grooves: Vec::new(),
                top_open_grooves: Vec::new(),
            },
        );
        m_int.rotate_y_around(PI32, 0.0, 0.0);
        m_int.translate(cx as f32, cy as f32, thick as f32);
        target.append(&m_int);
    };

    // Helper: takes a side-printed lever profile in XY (peg at y <= 0, grip along +X at y >= 0,
    // extruded z in [0, w]) and mounts it centered on z_base pointing inward toward hinge side
    let add_paired_lever = |target: &mut TriMesh, lever_xy: &crate::polygon2d::Shapes2D, w: f64| {
        // Interior mount (+Z outward, peg into -Z)
        let dir_int = if hinge_right { 1.0f32 } else { -1.0f32 };
        let mut int_m = extrude_slabs_with_moulding(
            &[Slab {
                z0: 0.0,
                z1: w,
                region: lever_xy.clone(),
            }],
            &MouldingConfig {
                outer_chamfer: Some((0.18, 0.22)),
                hole_chamfer: None,
                grooves: Vec::new(),
                top_grooves: Vec::new(),
                open_grooves: Vec::new(),
                top_open_grooves: Vec::new(),
            },
        );
        int_m.translate(0.0, 0.0, -(w as f32) * 0.5);
        int_m.rotate_x(FRAC_PI_2);
        if dir_int < 0.0 {
            for p in &mut int_m.positions {
                p[0] = -p[0];
                p[1] = -p[1];
            }
        }
        int_m.translate(hx as f32, hy as f32, 0.8);
        target.append(&int_m);

        // Exterior mount (-Z outward): flip to opposite X then rotate_y(PI) -> points along `dir_int`
        let mut ext_m = extrude_slabs_with_moulding(
            &[Slab {
                z0: 0.0,
                z1: w,
                region: lever_xy.clone(),
            }],
            &MouldingConfig {
                outer_chamfer: Some((0.18, 0.22)),
                hole_chamfer: None,
                grooves: Vec::new(),
                top_grooves: Vec::new(),
                open_grooves: Vec::new(),
                top_open_grooves: Vec::new(),
            },
        );
        ext_m.translate(0.0, 0.0, -(w as f32) * 0.5);
        ext_m.rotate_x(FRAC_PI_2);
        if dir_int > 0.0 {
            for p in &mut ext_m.positions {
                p[0] = -p[0];
                p[1] = -p[1];
            }
        }
        ext_m.rotate_y_around(PI32, 0.0, 0.0);
        ext_m.translate(hx as f32, hy as f32, -(leaf_t as f32 + 0.8));
        target.append(&ext_m);
    };

    match style {
        DoorHandleStyle::ModernLever => {
            add_paired_plate(
                &mut hw,
                &shape_from_contour(rect_centered(0.0, 0.0, 3.8, 11.4)),
                0.85,
                hx,
                hy,
            );
            let lever_xy = union_contours(&[
                rect_bounds(-0.85, -2.0, 0.85, 0.4),
                rect_bounds(-1.0, 0.2, 8.2, 1.75),
                rect_bounds(7.3, -0.35, 8.6, 1.75),
            ]);
            add_paired_lever(&mut hw, &lever_xy, 1.6);
        }
        DoorHandleStyle::VictorianScroll => {
            let plate = union_contours(&[
                rect_centered(0.0, 0.0, 4.2, 9.6),
                circle_contour(0.0, 4.8, 2.1, 20),
                circle_contour(0.0, -4.8, 2.1, 20),
            ]);
            add_paired_plate(&mut hw, &plate, 0.9, hx, hy);
            let scroll_xy = union_contours(&[
                rect_bounds(-0.85, -2.0, 0.85, 0.4),
                rect_bounds(-1.2, 0.1, 1.4, 1.9),
                rect_bounds(0.6, 0.5, 5.2, 1.9),
                rect_bounds(4.4, 0.0, 8.2, 1.45),
                circle_contour(8.0, 0.35, 1.25, 18),
            ]);
            add_paired_lever(&mut hw, &scroll_xy, 1.65);
        }
        DoorHandleStyle::RoundKnob => {
            add_paired_plate(
                &mut hw,
                &shape_from_contour(circle_contour(0.0, 0.0, 3.3, 24)),
                0.9,
                hx,
                hy,
            );
            let head = shape_from_contour(circle_contour(0.0, 0.0, 2.6, 24));
            let neck = shape_from_contour(circle_contour(0.0, 0.0, 1.45, 20));
            let build_knob = || {
                extrude_slabs_with_moulding(
                    &[
                        Slab {
                            z0: 0.0,
                            z1: 1.5,
                            region: head.clone(),
                        },
                        Slab {
                            z0: 1.5,
                            z1: 2.8,
                            region: neck.clone(),
                        },
                    ],
                    &MouldingConfig {
                        outer_chamfer: Some((0.36, 0.42)),
                        hole_chamfer: None,
                        grooves: Vec::new(),
                        top_grooves: Vec::new(),
                        open_grooves: Vec::new(),
                        top_open_grooves: Vec::new(),
                    },
                )
            };
            let mut k_ext = build_knob();
            k_ext.translate(hx as f32, hy as f32, -(leaf_t + 3.4) as f32);
            hw.append(&k_ext);
            let mut k_int = build_knob();
            k_int.rotate_y_around(PI32, 0.0, 0.0);
            k_int.translate(hx as f32, hy as f32, 3.4);
            hw.append(&k_int);
        }
        DoorHandleStyle::RingLatch => {
            let kite = vec![[0.0, -4.8], [2.6, 0.0], [0.0, 4.8], [-2.6, 0.0]];
            add_paired_plate(&mut hw, &shape_from_contour(kite), 0.9, hx, hy);
            let ring = union_shapes(
                &vec![ring_shape(0.0, -3.5, 2.2, 3.8, 28)],
                &shape_from_contour(rect_bounds(-1.0, -0.3, 1.0, 1.3)),
            );
            let mut r_ext = extrude_slabs(&[Slab {
                z0: 0.0,
                z1: 1.4,
                region: ring.clone(),
            }]);
            r_ext.translate(hx as f32, hy as f32, -(leaf_t + 2.2) as f32);
            hw.append(&r_ext);
            let mut r_int = extrude_slabs(&[Slab {
                z0: 0.0,
                z1: 1.4,
                region: ring,
            }]);
            r_int.translate(hx as f32, hy as f32, 0.8);
            hw.append(&r_int);
        }
        DoorHandleStyle::PullBar => {
            add_paired_plate(
                &mut hw,
                &shape_from_contour(rect_centered(0.0, 0.0, 3.4, 17.5)),
                0.85,
                hx,
                hy,
            );
            let bar = shape_from_contour(rect_centered(0.0, 0.0, 2.0, 14.5));
            let s_top = shape_from_contour(rect_centered(0.0, 5.05, 1.8, 1.8));
            let s_bot = shape_from_contour(rect_centered(0.0, -5.05, 1.8, 1.8));
            let build_pull = || {
                let mut m = extrude_slabs(&[Slab {
                    z0: 0.0,
                    z1: 1.8,
                    region: bar.clone(),
                }]);
                let st = extrude_slabs(&[Slab {
                    z0: 1.8,
                    z1: 4.0,
                    region: union_shapes(&s_top, &s_bot),
                }]);
                m.append(&st);
                m
            };
            let mut p_ext = build_pull();
            p_ext.translate(hx as f32, hy as f32, -(leaf_t + 4.4) as f32);
            hw.append(&p_ext);
            let mut p_int = build_pull();
            p_int.rotate_y_around(PI32, 0.0, 0.0);
            p_int.translate(hx as f32, hy as f32, 4.4);
            hw.append(&p_int);
        }
        DoorHandleStyle::None => {}
    }

    if let Some((lb_y, lb_w, lb_h)) = letterbox_placement {
        let lb_outer = shape_from_contour(rect_centered(0.0, 0.0, lb_w + 4.4, lb_h + 3.6));
        let lb_slit =
            shape_from_contour(rect_centered(0.0, 0.0, lb_w * 0.92, (lb_h * 0.65).max(1.5)));
        add_paired_plate(
            &mut hw,
            &difference_shapes(&lb_outer, &lb_slit),
            0.95,
            0.0,
            lb_y,
        );
        // Brass inner letter flap on exterior
        let mut flap = extrude_slabs(&[Slab {
            z0: 0.0,
            z1: 0.6,
            region: shape_from_contour(rect_centered(
                0.0,
                0.0,
                lb_w * 0.88,
                (lb_h * 0.55).max(1.2),
            )),
        }]);
        flap.translate(0.0, lb_y as f32, -(leaf_t + 0.65) as f32);
        hw.append(&flap);
    }

    if let Some((cf_cy, cf_w, cf_h)) = cat_flap_placement {
        let cf_outer = shape_from_contour(rect_centered(0.0, 0.0, cf_w + 3.6, cf_h + 3.6));
        let cf_hole = shape_from_contour(rect_centered(0.0, 0.0, cf_w, cf_h));
        add_paired_plate(
            &mut acc,
            &difference_shapes(&cf_outer, &cf_hole),
            1.05,
            0.0,
            cf_cy,
        );
        let mut inner_flap = extrude_slabs(&[Slab {
            z0: 0.0,
            z1: 1.2,
            region: shape_from_contour(rect_centered(
                0.0,
                0.0,
                (cf_w - 0.8).max(4.0),
                (cf_h - 0.8).max(4.0),
            )),
        }]);
        inner_flap.translate(0.0, cf_cy as f32, -(leaf_t * 0.5 + 0.6) as f32);
        acc.append(&inner_flap);
    }

    (hw, acc)
}
