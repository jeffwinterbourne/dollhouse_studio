use crate::polygon2d::{
    Contour2D, Pt2, Shape2D, Shapes2D, difference_shapes, intersection_shapes, miter_inset_contour,
    miter_offset_contour,
};
use glam::Vec3;
use i_triangle::float::triangulatable::Triangulatable;
use std::collections::{HashMap, HashSet};

/// A horizontal Z-slab of 2D solid geometry `[z0, z1]`.
#[derive(Debug, Clone)]
pub struct Slab {
    pub z0: f64,
    pub z1: f64,
    pub region: Shapes2D,
}

/// A closed 41-degree V-groove channel cut upward from `z = 0` into the front face.
#[derive(Debug, Clone)]
pub struct VGroove {
    /// Closed CCW centerline loop of the groove apex at `z = depth`.
    pub centerline: Contour2D,
    /// Groove opening width on the `z = 0` face (mm).
    pub width: f64,
    /// Groove apex depth in `+Z` (mm).
    pub depth: f64,
}

/// Decorative front-face (`z = 0`) moulding operations applied to the base slab.
#[derive(Debug, Clone, Default)]
pub struct MouldingConfig {
    /// `(inset_w, bevel_h)` for the outer architrave/door perimeter.
    pub outer_chamfer: Option<(f64, f64)>,
    /// `(outset_w, bevel_h)` for inner glass pane holes (chamfering glazing bars & pane edges).
    pub hole_chamfer: Option<(f64, f64)>,
    /// Closed V-grooves cut into the `z = 0` face.
    pub grooves: Vec<VGroove>,
}

/// Indexed watertight triangle mesh in millimeters (`Z` up).
#[derive(Debug, Clone, Default)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

impl TriMesh {
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Returns axis-aligned bounding box `(min, max)`.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        if self.positions.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for p in &self.positions {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        (min, max)
    }

    /// Translates all vertices in place.
    pub fn translate(&mut self, dx: f32, dy: f32, dz: f32) {
        for p in &mut self.positions {
            p[0] += dx;
            p[1] += dy;
            p[2] += dz;
        }
    }

    /// Rotates around the X axis by `angle_rad` around the origin.
    pub fn rotate_x(&mut self, angle_rad: f32) {
        let (s, c) = angle_rad.sin_cos();
        for p in &mut self.positions {
            let y = p[1];
            let z = p[2];
            p[1] = y * c - z * s;
            p[2] = y * s + z * c;
        }
    }

    /// Rotates around a vertical axis parallel to Y passing through `(pivot_x, pivot_z)`.
    pub fn rotate_y_around(&mut self, angle_rad: f32, pivot_x: f32, pivot_z: f32) {
        let (s, c) = angle_rad.sin_cos();
        for p in &mut self.positions {
            let dx = p[0] - pivot_x;
            let dz = p[2] - pivot_z;
            p[0] = pivot_x + dx * c + dz * s;
            p[2] = pivot_z - dx * s + dz * c;
        }
    }

    /// Appends another mesh into `self`.
    pub fn append(&mut self, other: &TriMesh) {
        let offset = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        for tri in &other.triangles {
            self.triangles
                .push([tri[0] + offset, tri[1] + offset, tri[2] + offset]);
        }
    }

    /// Checks whether the mesh is a closed, oriented 2-manifold with zero unprintable overhangs above `z = 0`.
    pub fn verify_printable_manifold(&self) -> Result<MeshDiagnostics, String> {
        if self.triangles.is_empty() {
            return Err("Mesh has no triangles".to_string());
        }

        let mut directed_edges: HashMap<(u32, u32), usize> = HashMap::new();
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        let mut overhang_triangles = 0usize;
        let mut degenerate_triangles = 0usize;
        let mut signed_volume_6 = 0.0f64;

        for tri in &self.triangles {
            let [i0, i1, i2] = *tri;
            if i0 == i1 || i1 == i2 || i2 == i0 {
                degenerate_triangles += 1;
                continue;
            }
            for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
                *directed_edges.entry((a, b)).or_insert(0) += 1;
            }

            let p0 = Vec3::from(self.positions[i0 as usize]);
            let p1 = Vec3::from(self.positions[i1 as usize]);
            let p2 = Vec3::from(self.positions[i2 as usize]);

            min_z = min_z.min(p0.z).min(p1.z).min(p2.z);
            max_z = max_z.max(p0.z).max(p1.z).max(p2.z);

            let cross = (p1 - p0).cross(p2 - p0);
            let area2 = cross.length();
            if area2 < 1e-7 {
                degenerate_triangles += 1;
                continue;
            }
            let n = cross / area2;

            // Signed volume * 6
            let d0 = [p0.x as f64, p0.y as f64, p0.z as f64];
            let d1 = [p1.x as f64, p1.y as f64, p1.z as f64];
            let d2 = [p2.x as f64, p2.y as f64, p2.z as f64];
            signed_volume_6 += d0[0] * (d1[1] * d2[2] - d1[2] * d2[1])
                - d0[1] * (d1[0] * d2[2] - d1[2] * d2[0])
                + d0[2] * (d1[0] * d2[1] - d1[1] * d2[0]);

            // Any downward-facing triangle (steep overhang > 45 deg) whose vertices are not on the print bed z = 0
            if n.z < -0.71 {
                let on_bed = p0.z.abs() < 1e-3 && p1.z.abs() < 1e-3 && p2.z.abs() < 1e-3;
                if !on_bed {
                    overhang_triangles += 1;
                }
            }
        }

        let mut boundary_edges = 0usize;
        let mut non_manifold_edges = 0usize;
        for (&(a, b), &count) in &directed_edges {
            if count != 1 {
                non_manifold_edges += 1;
            }
            let rev = directed_edges.get(&(b, a)).copied().unwrap_or(0);
            if rev == 0 {
                boundary_edges += 1;
            } else if rev != 1 {
                non_manifold_edges += 1;
            }
        }

        let is_watertight =
            boundary_edges == 0 && non_manifold_edges == 0 && degenerate_triangles == 0;
        let volume_mm3 = (signed_volume_6 / 6.0) as f32;

        if !is_watertight {
            let mut sample = Vec::new();
            for &(a, b) in directed_edges.keys() {
                if !directed_edges.contains_key(&(b, a)) && sample.len() < 6 {
                    sample.push(format!(
                        "{:?} -> {:?}",
                        self.positions[a as usize], self.positions[b as usize]
                    ));
                }
            }
            return Err(format!(
                "Non-manifold mesh: boundary_edges={boundary_edges}, non_manifold={non_manifold_edges}, degenerate={degenerate_triangles}, sample={:?}",
                sample
            ));
        }
        if volume_mm3 <= 0.0 {
            return Err(format!("Inverted mesh volume: {volume_mm3}"));
        }

        Ok(MeshDiagnostics {
            triangle_count: self.triangles.len(),
            is_watertight,
            overhang_triangles,
            volume_mm3,
            min_z,
            max_z,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MeshDiagnostics {
    pub triangle_count: usize,
    pub is_watertight: bool,
    pub overhang_triangles: usize,
    pub volume_mm3: f32,
    pub min_z: f32,
    pub max_z: f32,
}

struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    vert_map: HashMap<(i64, i64, i64), u32>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            triangles: Vec::new(),
            vert_map: HashMap::new(),
        }
    }

    fn vertex(&mut self, x: f64, y: f64, z: f64) -> u32 {
        // 1e-5 mm (10 nanometer) quantization avoids floating-point seam splits while preserving fine curves.
        let key = (
            (x * 100_000.0).round() as i64,
            (y * 100_000.0).round() as i64,
            (z * 100_000.0).round() as i64,
        );
        if let Some(&idx) = self.vert_map.get(&key) {
            return idx;
        }
        let idx = self.positions.len() as u32;
        self.positions.push([x as f32, y as f32, z as f32]);
        self.vert_map.insert(key, idx);
        idx
    }

    fn push_tri(&mut self, a: u32, b: u32, c: u32) {
        if a == b || b == c || c == a {
            return;
        }
        self.triangles.push([a, b, c]);
    }

    fn finish(self) -> TriMesh {
        TriMesh {
            positions: self.positions,
            triangles: self.triangles,
        }
    }
}

/// Extrudes a stack of Z-slabs into a single watertight, zero-overhang 2-manifold `TriMesh`.
/// Enforces `region[i] ⊆ region[i - 1]` for all `i > 0`.
pub fn extrude_slabs(raw_slabs: &[Slab]) -> TriMesh {
    extrude_slabs_with_base_chamfer(raw_slabs, None)
}

/// Extrudes a stack of Z-slabs with an optional 41-degree chamfered bevel on the base outer contour.
/// `base_chamfer = Some((inset_width, bevel_height))` insets the outer contour at `z0` and flares
/// outward to full width at `z0 + bevel_height`.
pub fn extrude_slabs_with_base_chamfer(
    raw_slabs: &[Slab],
    base_chamfer: Option<(f64, f64)>,
) -> TriMesh {
    extrude_slabs_with_moulding(
        raw_slabs,
        &MouldingConfig {
            outer_chamfer: base_chamfer,
            hole_chamfer: None,
            grooves: Vec::new(),
        },
    )
}

/// Extrudes a stack of Z-slabs with front-face chamfers (outer + bar/pane holes) and V-grooves.
pub fn extrude_slabs_with_moulding(raw_slabs: &[Slab], moulding: &MouldingConfig) -> TriMesh {
    if raw_slabs.is_empty() {
        return TriMesh::default();
    }

    // 1. Enforce monotone containment `R_i ⊆ R_{i-1}` so overhangs are impossible.
    let mut slabs: Vec<Slab> = Vec::with_capacity(raw_slabs.len());
    for (idx, s) in raw_slabs.iter().enumerate() {
        if s.z1 <= s.z0 + 1e-6 {
            continue;
        }
        let mut region = if idx == 0 || slabs.is_empty() {
            s.region.clone()
        } else {
            intersection_shapes(&s.region, &slabs.last().unwrap().region)
        };
        normalize_shapes_winding(&mut region);
        if !region.is_empty() {
            slabs.push(Slab {
                z0: s.z0,
                z1: s.z1,
                region,
            });
        }
    }
    if slabs.is_empty() {
        return TriMesh::default();
    }

    // 2. Precompute horizontal step-cap regions `step[k] = R_{k-1} \ R_k` for interfaces `k = 1..slabs.len()`.
    let mut steps: Vec<Shapes2D> = Vec::with_capacity(slabs.len().saturating_sub(1));
    for k in 1..slabs.len() {
        let mut diff = difference_shapes(&slabs[k - 1].region, &slabs[k].region);
        normalize_shapes_winding(&mut diff);
        steps.push(diff);
    }

    // 3. Collect all 2D vertices across slabs and steps, then subdivide any edges containing
    //    other vertices strictly along them (eliminating all T-junctions at Z interfaces).
    let mut all_pts: Vec<Pt2> = Vec::new();
    let mut seen: HashSet<(i64, i64)> = HashSet::new();
    let mut add_pt = |p: Pt2| {
        let key = (
            (p[0] * 100_000.0).round() as i64,
            (p[1] * 100_000.0).round() as i64,
        );
        if seen.insert(key) {
            all_pts.push(p);
        }
    };
    for s in &slabs {
        for shape in &s.region {
            for contour in shape {
                for &p in contour {
                    add_pt(p);
                }
            }
        }
    }
    for step in &steps {
        for shape in step {
            for contour in shape {
                for &p in contour {
                    add_pt(p);
                }
            }
        }
    }

    for s in &mut slabs {
        subdivide_shapes(&mut s.region, &all_pts);
    }
    for step in &mut steps {
        subdivide_shapes(step, &all_pts);
    }

    // 4. Emit bottom cap, optional base moulding (outer chamfer, hole/bar chamfers, V-grooves),
    //    vertical walls, step caps, and top cap into MeshBuilder.
    let mut mb = MeshBuilder::new();

    let has_moulding = moulding.outer_chamfer.is_some()
        || moulding.hole_chamfer.is_some()
        || !moulding.grooves.is_empty();

    let moulding_applied = if has_moulding && slabs[0].region.len() == 1 {
        let z0 = slabs[0].z0;
        let z1 = slabs[0].z1;
        let slab_h = z1 - z0;
        let outer_top = &slabs[0].region[0][0];
        let holes_top = &slabs[0].region[0][1..];

        let (outer_bot, outer_bevel_z) = if let Some((inset_w, bevel_h)) = moulding.outer_chamfer {
            if bevel_h > 0.05 && bevel_h < slab_h - 0.05 {
                if let Some(ob) = miter_inset_contour(outer_top, inset_w) {
                    (ob, Some(z0 + bevel_h))
                } else {
                    (outer_top.clone(), None)
                }
            } else {
                (outer_top.clone(), None)
            }
        } else {
            (outer_top.clone(), None)
        };

        let mut holes_bot: Vec<(Contour2D, Option<f64>)> = Vec::with_capacity(holes_top.len());
        let outer_bot_shape = vec![vec![outer_bot.clone()]];
        let mut accepted_holes_ccw: Shapes2D = Vec::new();
        for h_top in holes_top {
            if let Some((outset_w, bevel_h)) = moulding.hole_chamfer
                && bevel_h > 0.05
                && bevel_h < slab_h - 0.05
                && let Some(hb) = miter_offset_contour(h_top, -outset_w)
            {
                let mut hb_ccw = hb.clone();
                hb_ccw.reverse();
                let hb_shape = vec![vec![hb_ccw.clone()]];
                let inside_outer = difference_shapes(&hb_shape, &outer_bot_shape).is_empty();
                let clear_of_others =
                    intersection_shapes(&hb_shape, &accepted_holes_ccw).is_empty();
                if inside_outer && clear_of_others {
                    accepted_holes_ccw.push(vec![hb_ccw]);
                    holes_bot.push((hb, Some(z0 + bevel_h)));
                    continue;
                }
            }
            let mut ht_ccw = h_top.clone();
            ht_ccw.reverse();
            accepted_holes_ccw.push(vec![ht_ccw]);
            holes_bot.push((h_top.clone(), None));
        }

        let mut bot_shape = Vec::with_capacity(1 + holes_bot.len());
        bot_shape.push(outer_bot.clone());
        for (hb, _) in &holes_bot {
            bot_shape.push(hb.clone());
        }
        let mut bot_shapes: Shapes2D = vec![bot_shape];

        // Prepare valid V-grooves that stay inside the base solid
        struct BuiltGroove {
            centerline: Contour2D,
            outer: Contour2D,
            inner: Contour2D,
            z_apex: f64,
        }
        let mut built_grooves: Vec<BuiltGroove> = Vec::new();
        let mut groove_bands: Shapes2D = Vec::new();
        for g in &moulding.grooves {
            if g.depth <= 0.05 || g.depth >= slab_h - 0.05 || g.width <= 0.15 {
                continue;
            }
            if let (Some(go), Some(gi)) = (
                miter_offset_contour(&g.centerline, -g.width * 0.5),
                miter_offset_contour(&g.centerline, g.width * 0.5),
            ) {
                let mut rev_gi = gi.clone();
                rev_gi.reverse();
                let band = vec![go.clone(), rev_gi];
                let band_shapes = vec![band.clone()];
                let fits_inside = difference_shapes(&band_shapes, &bot_shapes).is_empty();
                let clear_of_prev = intersection_shapes(&band_shapes, &groove_bands).is_empty();
                // Only add groove if the entire band sits strictly inside `bot_shapes` and doesn't touch holes or other grooves
                if fits_inside && clear_of_prev {
                    groove_bands.push(band);
                    built_grooves.push(BuiltGroove {
                        centerline: g.centerline.clone(),
                        outer: go,
                        inner: gi,
                        z_apex: z0 + g.depth,
                    });
                }
            }
        }

        if !groove_bands.is_empty() {
            bot_shapes = difference_shapes(&bot_shapes, &groove_bands);
            normalize_shapes_winding(&mut bot_shapes);
            let mut gpts: Vec<Pt2> = Vec::new();
            for p in &outer_bot {
                gpts.push(*p);
            }
            for (hb, _) in &holes_bot {
                for p in hb {
                    gpts.push(*p);
                }
            }
            for bg in &built_grooves {
                gpts.extend_from_slice(&bg.outer);
                gpts.extend_from_slice(&bg.inner);
            }
            subdivide_shapes(&mut bot_shapes, &gpts);
        }

        // 1. Bottom cap at z0
        emit_horizontal_cap(&mut mb, &bot_shapes, z0, false);

        // 2. Outer wall (chamfered + vertical)
        if let Some(zb) = outer_bevel_z {
            emit_tapered_wall(&mut mb, &outer_bot, outer_top, z0, zb);
            emit_contour_wall(&mut mb, outer_top, zb, z1);
        } else {
            emit_contour_wall(&mut mb, outer_top, z0, z1);
        }

        // 3. Inner hole walls (chamfered + vertical)
        for (k, h_top) in holes_top.iter().enumerate() {
            let (ref h_bot, h_zb) = holes_bot[k];
            if let Some(zb) = h_zb {
                emit_tapered_wall(&mut mb, h_bot, h_top, z0, zb);
                emit_contour_wall(&mut mb, h_top, zb, z1);
            } else {
                emit_contour_wall(&mut mb, h_top, z0, z1);
            }
        }

        // 4. V-groove sloped walls meeting at z_apex
        for bg in &built_grooves {
            let mut rev_outer = bg.outer.clone();
            rev_outer.reverse();
            let mut rev_mid = bg.centerline.clone();
            rev_mid.reverse();
            emit_tapered_wall(&mut mb, &rev_outer, &rev_mid, z0, bg.z_apex);
            emit_tapered_wall(&mut mb, &bg.inner, &bg.centerline, z0, bg.z_apex);
        }

        true
    } else {
        false
    };

    if !moulding_applied {
        emit_horizontal_cap(&mut mb, &slabs[0].region, slabs[0].z0, false);
        emit_side_walls(&mut mb, &slabs[0].region, slabs[0].z0, slabs[0].z1);
    }

    // Side walls for remaining slabs
    for s in slabs.iter().skip(1) {
        emit_side_walls(&mut mb, &s.region, s.z0, s.z1);
    }

    // Step caps at z_k (normal +Z)
    for (idx, step) in steps.iter().enumerate() {
        let z = slabs[idx + 1].z0;
        emit_horizontal_cap(&mut mb, step, z, true);
    }

    // Top cap at z_top (normal +Z)
    let last = slabs.last().unwrap();
    emit_horizontal_cap(&mut mb, &last.region, last.z1, true);

    mb.finish()
}

fn contour_signed_area(c: &[Pt2]) -> f64 {
    if c.len() < 3 {
        return 0.0;
    }
    let mut area2 = 0.0;
    for i in 0..c.len() {
        let a = c[i];
        let b = c[(i + 1) % c.len()];
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    area2 * 0.5
}

fn normalize_shapes_winding(shapes: &mut Shapes2D) {
    shapes.retain(|s| !s.is_empty() && s[0].len() >= 3);
    for shape in shapes.iter_mut() {
        shape.retain(|c| c.len() >= 3 && contour_signed_area(c).abs() > 1e-8);
        if shape.is_empty() {
            continue;
        }
        if contour_signed_area(&shape[0]) < 0.0 {
            shape[0].reverse();
        }
        for hole in shape.iter_mut().skip(1) {
            if contour_signed_area(hole) > 0.0 {
                hole.reverse();
            }
        }
    }
    shapes.retain(|s| !s.is_empty());
}

fn subdivide_shapes(shapes: &mut Shapes2D, pts: &[Pt2]) {
    for shape in shapes.iter_mut() {
        for contour in shape.iter_mut() {
            subdivide_contour(contour, pts);
        }
    }
}

fn subdivide_contour(contour: &mut Contour2D, pts: &[Pt2]) {
    if contour.len() < 2 {
        return;
    }
    let n = contour.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        out.push(a);

        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len2 = dx * dx + dy * dy;
        if len2 < 1e-12 {
            continue;
        }
        let len = len2.sqrt();
        let min_x = a[0].min(b[0]) - 1e-6;
        let max_x = a[0].max(b[0]) + 1e-6;
        let min_y = a[1].min(b[1]) - 1e-6;
        let max_y = a[1].max(b[1]) + 1e-6;

        let mut splits: Vec<(f64, Pt2)> = Vec::new();
        for &p in pts {
            if p[0] < min_x || p[0] > max_x || p[1] < min_y || p[1] > max_y {
                continue;
            }
            let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2;
            if t <= 1e-7 || t >= 1.0 - 1e-7 {
                continue;
            }
            let cross = ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs();
            let perp_dist = cross / len;
            if perp_dist < 1e-7 {
                splits.push((t, p));
            }
        }
        if !splits.is_empty() {
            splits.sort_by(|u, v| u.0.partial_cmp(&v.0).unwrap());
            let mut prev_t = 0.0;
            for (t, p) in splits {
                if (t - prev_t) * len > 1e-6 {
                    out.push(p);
                    prev_t = t;
                }
            }
        }
    }
    *contour = out;
}

fn emit_side_walls(mb: &mut MeshBuilder, shapes: &Shapes2D, z0: f64, z1: f64) {
    for shape in shapes {
        for contour in shape {
            emit_contour_wall(mb, contour, z0, z1);
        }
    }
}

fn emit_tapered_wall(mb: &mut MeshBuilder, bot: &[Pt2], top: &[Pt2], z0: f64, z1: f64) {
    let n = bot.len();
    if n < 3 || top.len() != n {
        return;
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let b0 = mb.vertex(bot[i][0], bot[i][1], z0);
        let b1 = mb.vertex(bot[j][0], bot[j][1], z0);
        let t1 = mb.vertex(top[j][0], top[j][1], z1);
        let t0 = mb.vertex(top[i][0], top[i][1], z1);
        mb.push_tri(b0, b1, t1);
        mb.push_tri(b0, t1, t0);
    }
}

fn emit_contour_wall(mb: &mut MeshBuilder, contour: &[Pt2], z0: f64, z1: f64) {
    let n = contour.len();
    if n < 3 {
        return;
    }
    for i in 0..n {
        let p0 = contour[i];
        let p1 = contour[(i + 1) % n];
        let b0 = mb.vertex(p0[0], p0[1], z0);
        let b1 = mb.vertex(p1[0], p1[1], z0);
        let t1 = mb.vertex(p1[0], p1[1], z1);
        let t0 = mb.vertex(p0[0], p0[1], z1);
        mb.push_tri(b0, b1, t1);
        mb.push_tri(b0, t1, t0);
    }
}

fn emit_horizontal_cap(mb: &mut MeshBuilder, shapes: &Shapes2D, z: f64, face_up: bool) {
    for shape in shapes {
        emit_shape_cap(mb, shape, z, face_up);
    }
}

fn emit_shape_cap(mb: &mut MeshBuilder, shape: &Shape2D, z: f64, face_up: bool) {
    if shape.is_empty() || shape[0].len() < 3 {
        return;
    }

    // Run checked triangulation (integer-space contour simplification, direction correction, and
    // area filtering), then re-insert all boundary vertices from `shape` into matching cap
    // triangles below so T-junctions at slab Z interfaces remain eliminated.
    let raw = shape.triangulate_as::<i64>();
    let tri = raw.to_triangulation::<u32>();
    if tri.indices.is_empty() {
        return;
    }

    let mut cap_pts: Vec<Pt2> = tri.points;
    let mut cap_tris: Vec<[usize; 3]> = tri
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|c| {
            let i0 = c[0] as usize;
            let i1 = c[1] as usize;
            let i2 = c[2] as usize;
            let a = cap_pts[i0];
            let b = cap_pts[i1];
            let d = cap_pts[i2];
            let cross_z = (b[0] - a[0]) * (d[1] - a[1]) - (b[1] - a[1]) * (d[0] - a[0]);
            if cross_z.abs() < 1e-12 {
                None
            } else if (cross_z > 0.0) == face_up {
                Some([i0, i1, i2])
            } else {
                Some([i0, i2, i1])
            }
        })
        .collect();
    if cap_tris.is_empty() {
        return;
    }

    // Ensure every vertex of `shape` lying strictly inside any triangle edge splits that triangle
    // (eliminating T-junctions both on boundaries and at interior pinch/collinear vertices).
    let mut shape_pts: Vec<(usize, Pt2)> = Vec::new();
    for contour in shape {
        for &p in contour {
            if let Some((idx, _)) = cap_pts.iter().enumerate().find(|(_, q)| {
                let dx = p[0] - q[0];
                let dy = p[1] - q[1];
                dx * dx + dy * dy < 1e-10
            }) {
                if !shape_pts.iter().any(|(i, _)| *i == idx) {
                    shape_pts.push((idx, cap_pts[idx]));
                }
            } else {
                let idx = cap_pts.len();
                cap_pts.push(p);
                shape_pts.push((idx, p));
            }
        }
    }

    for (p_idx, p) in shape_pts {
        let mut idx = 0;
        while idx < cap_tris.len() {
            let [i0, i1, i2] = cap_tris[idx];
            if i0 == p_idx || i1 == p_idx || i2 == p_idx {
                idx += 1;
                continue;
            }
            let edges = [(i0, i1, i2), (i1, i2, i0), (i2, i0, i1)];
            let mut split_edge = None;
            for (a_i, b_i, opp_i) in edges {
                let a = cap_pts[a_i];
                let b = cap_pts[b_i];
                if p[0] < a[0].min(b[0]) - 1e-5
                    || p[0] > a[0].max(b[0]) + 1e-5
                    || p[1] < a[1].min(b[1]) - 1e-5
                    || p[1] > a[1].max(b[1]) + 1e-5
                {
                    continue;
                }
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let len2 = dx * dx + dy * dy;
                if len2 < 1e-12 {
                    continue;
                }
                let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2;
                if t > 1e-7 && t < 1.0 - 1e-7 {
                    let perp = ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len2.sqrt();
                    if perp < 1e-5 {
                        split_edge = Some((a_i, b_i, opp_i));
                        break;
                    }
                }
            }
            if let Some((a_i, b_i, opp_i)) = split_edge {
                cap_tris[idx] = [a_i, p_idx, opp_i];
                cap_tris.push([p_idx, b_i, opp_i]);
            }
            idx += 1;
        }
    }

    let vert_ids: Vec<u32> = cap_pts.iter().map(|p| mb.vertex(p[0], p[1], z)).collect();

    for [i0, i1, i2] in cap_tris {
        mb.push_tri(vert_ids[i0], vert_ids[i1], vert_ids[i2]);
    }
}

/// Cardinal side face for a print-in-place horizontal cone-in-socket knuckle block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HingeFace {
    PosX,
    NegX,
    PosY,
    NegY,
}

/// Builds a closed, oriented, zero-overhang 2-manifold rectangular knuckle block covering
/// `[x0, x1] x [y0, y1] x [z0, z1]` with either a protruding male cone (`is_socket = false`)
/// or an inward-recessed female conical socket (`is_socket = true`) centered on `face`.
/// Enforces `cone_len <= radius * 0.84` so every cone/socket triangle stays below the 45-degree
/// printability overhang limit (`n.z >= -0.71`).
#[allow(clippy::too_many_arguments)]
pub fn build_cone_knuckle_block(
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    z0: f64,
    z1: f64,
    face: HingeFace,
    radius: f64,
    cone_len: f64,
    is_socket: bool,
    segments: usize,
) -> TriMesh {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let cz = (z0 + z1) * 0.5;
    let hx = ((x1 - x0) * 0.5).max(0.5);
    let hy = ((y1 - y0) * 0.5).max(0.5);
    let hz = ((z1 - z0) * 0.5).max(0.5);

    // Local right-handed orthonormal frame (u, v, n) with u x v = n pointing outward from `face`
    let (u, v, n, hu, hv, hn) = match face {
        HingeFace::PosY => (
            [1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0],
            hx,
            hz,
            hy,
        ),
        HingeFace::NegY => (
            [-1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0],
            [0.0, -1.0, 0.0],
            hx,
            hz,
            hy,
        ),
        HingeFace::PosX => (
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            hy,
            hz,
            hx,
        ),
        HingeFace::NegX => (
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            hy,
            hz,
            hx,
        ),
    };

    let r = radius.clamp(0.45, (hu.min(hv) - 0.14).max(0.45));
    let max_len = if is_socket {
        (r * 0.82).min((2.0 * hn - 0.35).max(0.35))
    } else {
        r * 0.82
    };
    let len = cone_len.clamp(0.35, max_len.max(0.35));

    let mut mb = MeshBuilder::new();
    let add_pt = |mb: &mut MeshBuilder, su: f64, sv: f64, sn: f64| -> u32 {
        mb.vertex(
            cx + u[0] * su + v[0] * sv + n[0] * sn,
            cy + u[1] * su + v[1] * sv + n[1] * sn,
            cz + u[2] * su + v[2] * sv + n[2] * sn,
        )
    };

    // 8 box corners
    let b00 = add_pt(&mut mb, -hu, -hv, -hn);
    let b10 = add_pt(&mut mb, hu, -hv, -hn);
    let b11 = add_pt(&mut mb, hu, hv, -hn);
    let b01 = add_pt(&mut mb, -hu, hv, -hn);
    let f00 = add_pt(&mut mb, -hu, -hv, hn);
    let f10 = add_pt(&mut mb, hu, -hv, hn);
    let f11 = add_pt(&mut mb, hu, hv, hn);
    let f01 = add_pt(&mut mb, -hu, hv, hn);

    // 5 non-cone box faces (exact outward CCW winding in right-handed (u, v, n))
    // Back face (-n)
    mb.push_tri(b00, b01, b11);
    mb.push_tri(b00, b11, b10);
    // +u face
    mb.push_tri(b10, b11, f11);
    mb.push_tri(b10, f11, f10);
    // -u face
    mb.push_tri(f00, f01, b01);
    mb.push_tri(f00, b01, b00);
    // +v face
    mb.push_tri(f01, f11, b11);
    mb.push_tri(f01, b11, b01);
    // -v face
    mb.push_tri(b00, b10, f10);
    mb.push_tri(b00, f10, f00);

    // Front face (+n) annulus + cone/socket: force N = 4 * m so quadrants align cleanly with 4 corners
    let per_quad = segments.max(12).div_ceil(4).max(3);
    let n_segs = per_quad * 4;
    let corners = [f11, f01, f00, f10];

    let mut ring = Vec::with_capacity(n_segs);
    for k in 0..n_segs {
        let theta = ((k as f64) + 0.5) * (2.0 * std::f64::consts::PI / n_segs as f64);
        ring.push(add_pt(&mut mb, r * theta.cos(), r * theta.sin(), hn));
    }

    // Annulus triangles between corners and circle ring (outer edges match q_j -> q_{j+1})
    for q in 0..4 {
        let c_curr = corners[q];
        let c_next = corners[(q + 1) % 4];
        let k_start = q * per_quad;
        for m in 0..(per_quad - 1) {
            let k = k_start + m;
            mb.push_tri(c_curr, ring[k + 1], ring[k]);
        }
        let k_last = k_start + per_quad - 1;
        let k_next_first = (q + 1) % 4 * per_quad;
        mb.push_tri(c_curr, c_next, ring[k_last]);
        mb.push_tri(c_next, ring[k_next_first], ring[k_last]);
    }

    let apex_sn = if is_socket { hn - len } else { hn + len };
    let apex = add_pt(&mut mb, 0.0, 0.0, apex_sn);
    for k in 0..n_segs {
        let next = (k + 1) % n_segs;
        mb.push_tri(ring[k], ring[next], apex);
    }

    mb.finish()
}
