use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use std::f64::consts::PI;

pub type Pt2 = [f64; 2];
pub type Contour2D = Vec<Pt2>;
/// One polygon with outer contour at index 0 and optional holes at indices 1..
pub type Shape2D = Vec<Contour2D>;
pub type Shapes2D = Vec<Shape2D>;

/// Architectural outer boundary family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    Rectangle,
    RoundArch,
    HalfRoundArch,
    GothicArch,
    CircleOrEllipse,
}

/// Generates a CCW rectangle contour covering `[x0, x1] x [y0, y1]`.
pub fn rect_bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour2D {
    let (lx, hx) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let (ly, hy) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    vec![[lx, ly], [hx, ly], [hx, hy], [lx, hy]]
}

/// Generates a CCW rectangle centered at `(cx, cy)` with dimensions `(w, h)`.
pub fn rect_centered(cx: f64, cy: f64, w: f64, h: f64) -> Contour2D {
    rect_bounds(cx - w * 0.5, cy - h * 0.5, cx + w * 0.5, cy + h * 0.5)
}

/// Generates a CCW thick line segment (bar) from `p0` to `p1` with total thickness `width`.
pub fn segment_bar(p0: Pt2, p1: Pt2, width: f64) -> Contour2D {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return rect_centered(p0[0], p0[1], width, width);
    }
    let nx = -dy / len * (width * 0.5);
    let ny = dx / len * (width * 0.5);
    vec![
        [p0[0] - nx, p0[1] - ny],
        [p1[0] - nx, p1[1] - ny],
        [p1[0] + nx, p1[1] + ny],
        [p0[0] + nx, p0[1] + ny],
    ]
}

/// Generates a CCW circle centered at `(cx, cy)`.
pub fn circle_contour(cx: f64, cy: f64, radius: f64, segments: usize) -> Contour2D {
    let r = radius.max(0.2);
    let segs = segments.max(16);
    (0..segs)
        .map(|i| {
            let a = (i as f64 / segs as f64) * 2.0 * PI;
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

/// Generates a CCW concentric ring (annulus) between `inner_r` and `outer_r`.
pub fn ring_shape(cx: f64, cy: f64, inner_r: f64, outer_r: f64, segments: usize) -> Shape2D {
    let outer = circle_contour(cx, cy, outer_r, segments);
    let mut inner = circle_contour(cx, cy, inner_r, segments);
    inner.reverse(); // CW hole
    vec![outer, inner]
}

/// Generates the outer CCW contour for a given `ProfileKind` of nominal size `w x h`
/// offset outward by `offset` (or inward if negative).
pub fn profile_contour(
    kind: ProfileKind,
    w: f64,
    h: f64,
    offset: f64,
    arc_segments: usize,
) -> Contour2D {
    match kind {
        ProfileKind::Rectangle => rect_centered(
            0.0,
            0.0,
            (w + 2.0 * offset).max(0.5),
            (h + 2.0 * offset).max(0.5),
        ),
        ProfileKind::RoundArch => {
            let rx_base = w * 0.5;
            let ry_base = (w * 0.5).min(h * 0.72);
            let y_bot = -h * 0.5 - offset;
            let y_spring = h * 0.5 - ry_base;
            let half_w = (rx_base + offset).max(0.3);
            let ry = (ry_base + offset).max(0.3);
            let segs = arc_segments.max(24);

            let mut pts = Vec::with_capacity(segs + 4);
            pts.push([-half_w, y_bot]);
            pts.push([half_w, y_bot]);

            if (rx_base - ry_base).abs() < 1e-4 {
                // Exact semicircle: parallel offset is an exact concentric semicircle of radius `half_w`.
                for i in 0..=segs {
                    let t = (i as f64 / segs as f64) * PI;
                    pts.push([half_w * t.cos(), y_spring + half_w * t.sin()]);
                }
            } else {
                // Elliptical arch for unusually short/wide proportions: exact normal offset.
                for i in 0..=segs {
                    let t = (i as f64 / segs as f64) * PI;
                    let ct = t.cos();
                    let st = t.sin();
                    let bx = rx_base * ct;
                    let by = y_spring + ry_base * st;
                    let nx = ry_base * ct;
                    let ny = rx_base * st;
                    let nlen = (nx * nx + ny * ny).sqrt().max(1e-6);
                    pts.push([bx + offset * nx / nlen, by + offset * ny / nlen]);
                }
            }
            let _ = ry;
            dedup_consecutive(&mut pts);
            pts
        }
        ProfileKind::HalfRoundArch => {
            let y_bot = -h * 0.5 - offset;
            let a = (w * 0.5 + offset).max(0.4);
            let b = (h + 2.0 * offset).max(0.4);
            let segs = arc_segments.max(24);
            let mut pts = Vec::with_capacity(segs + 4);
            pts.push([-a, y_bot]);
            pts.push([a, y_bot]);
            for i in 0..=segs {
                let t = (i as f64 / segs as f64) * PI;
                pts.push([a * t.cos(), y_bot + b * t.sin()]);
            }
            dedup_consecutive(&mut pts);
            pts
        }
        ProfileKind::GothicArch => {
            let half_w_base = w * 0.5;
            let h_arch = (w * 0.72).min(h * 0.76).max(w * 0.52);
            let y_bot = -h * 0.5 - offset;
            let y_spring = h * 0.5 - h_arch;
            let c = (h_arch * h_arch - half_w_base * half_w_base) / w;
            let r_base = half_w_base + c;
            let r = (r_base + offset).max(c + 0.35);
            let half_w = (half_w_base + offset).max(0.3);
            let segs_half = (arc_segments / 2).max(14);

            let apex_angle = (c / r).clamp(-0.999, 0.999).acos();
            let y_apex = y_spring + (r * r - c * c).max(0.0).sqrt();

            let mut pts = Vec::with_capacity(segs_half * 2 + 5);
            pts.push([-half_w, y_bot]);
            pts.push([half_w, y_bot]);

            // Right lancet flank: center at (-c, y_spring), angle 0 -> apex_angle
            for i in 0..segs_half {
                let a = (i as f64 / segs_half as f64) * apex_angle;
                pts.push([-c + r * a.cos(), y_spring + r * a.sin()]);
            }
            pts.push([0.0, y_apex]);
            // Left lancet flank: center at (+c, y_spring), angle (PI - apex_angle) -> PI
            for i in 1..=segs_half {
                let a = (PI - apex_angle) + (i as f64 / segs_half as f64) * apex_angle;
                pts.push([c + r * a.cos(), y_spring + r * a.sin()]);
            }
            dedup_consecutive(&mut pts);
            pts
        }
        ProfileKind::CircleOrEllipse => {
            let rx_base = w * 0.5;
            let ry_base = h * 0.5;
            let segs = (arc_segments * 2).max(48);
            if (rx_base - ry_base).abs() < 1e-4 {
                circle_contour(0.0, 0.0, rx_base + offset, segs)
            } else {
                let mut pts = Vec::with_capacity(segs);
                for i in 0..segs {
                    let t = (i as f64 / segs as f64) * 2.0 * PI;
                    let ct = t.cos();
                    let st = t.sin();
                    let bx = rx_base * ct;
                    let by = ry_base * st;
                    let nx = ry_base * ct;
                    let ny = rx_base * st;
                    let nlen = (nx * nx + ny * ny).sqrt().max(1e-6);
                    pts.push([bx + offset * nx / nlen, by + offset * ny / nlen]);
                }
                pts
            }
        }
    }
}

fn dedup_consecutive(pts: &mut Vec<Pt2>) {
    if pts.len() < 2 {
        return;
    }
    let mut out: Vec<Pt2> = Vec::with_capacity(pts.len());
    for &p in pts.iter() {
        if let Some(&last) = out.last() {
            let dx = p[0] - last[0];
            let dy = p[1] - last[1];
            if dx * dx + dy * dy < 1e-10 {
                continue;
            }
        }
        out.push(p);
    }
    if out.len() >= 2 {
        let first = out[0];
        let last = *out.last().unwrap();
        let dx = first[0] - last[0];
        let dy = first[1] - last[1];
        if dx * dx + dy * dy < 1e-10 {
            out.pop();
        }
    }
    *pts = out;
}

/// Computes the exact 2D union of a list of contours (self-intersecting/overlapping bars etc.)
/// using `FillRule::NonZero` and `i64` precision.
pub fn union_contours(contours: &[Contour2D]) -> Shapes2D {
    if contours.is_empty() {
        return Vec::new();
    }
    FloatOverlay::<Pt2, i64>::from_subj(contours).overlay(OverlayRule::Subject, FillRule::NonZero)
}

/// Computes the exact 2D union of two `Shapes2D` regions.
pub fn union_shapes(a: &Shapes2D, b: &Shapes2D) -> Shapes2D {
    if a.is_empty() {
        return b.clone();
    }
    if b.is_empty() {
        return a.clone();
    }
    FloatOverlay::<Pt2, i64>::from_subj_and_clip(a, b)
        .overlay(OverlayRule::Union, FillRule::EvenOdd)
}

/// Computes `a \ b` (exact 2D boolean difference).
pub fn difference_shapes(a: &Shapes2D, b: &Shapes2D) -> Shapes2D {
    if a.is_empty() || b.is_empty() {
        return a.clone();
    }
    FloatOverlay::<Pt2, i64>::from_subj_and_clip(a, b)
        .overlay(OverlayRule::Difference, FillRule::EvenOdd)
}

/// Computes `a ∩ b` (exact 2D boolean intersection).
pub fn intersection_shapes(a: &Shapes2D, b: &Shapes2D) -> Shapes2D {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    FloatOverlay::<Pt2, i64>::from_subj_and_clip(a, b)
        .overlay(OverlayRule::Intersect, FillRule::EvenOdd)
}

/// Helper to wrap a single outer contour into `Shapes2D`.
pub fn shape_from_contour(c: Contour2D) -> Shapes2D {
    vec![vec![c]]
}

/// Offsets a simple closed contour by `inward_offset` using mitered vertex normals.
/// For a CCW contour, positive `inward_offset` shrinks the contour; negative expands it.
/// For a CW hole contour, negative `inward_offset` expands the hole (shrinking the surrounding bar).
/// Preserves exact 1-to-1 vertex correspondence for clean chamfered bevel extrusion.
pub fn miter_offset_contour(contour: &Contour2D, inward_offset: f64) -> Option<Contour2D> {
    let n = contour.len();
    if n < 3 || inward_offset.abs() <= 1e-6 {
        return None;
    }

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = contour[(i + n - 1) % n];
        let curr = contour[i];
        let next = contour[(i + 1) % n];

        let e0x = curr[0] - prev[0];
        let e0y = curr[1] - prev[1];
        let l0 = (e0x * e0x + e0y * e0y).sqrt();

        let e1x = next[0] - curr[0];
        let e1y = next[1] - curr[1];
        let l1 = (e1x * e1x + e1y * e1y).sqrt();

        if l0 < 1e-9 || l1 < 1e-9 {
            return None;
        }

        // Left-hand normals
        let n0 = [-e0y / l0, e0x / l0];
        let n1 = [-e1y / l1, e1x / l1];

        let avg_x = n0[0] + n1[0];
        let avg_y = n0[1] + n1[1];
        let avg_len = (avg_x * avg_x + avg_y * avg_y).sqrt();
        let sign = inward_offset.signum();
        let mag = inward_offset.abs();
        if avg_len < 1e-6 {
            out.push([
                curr[0] + n0[0] * inward_offset,
                curr[1] + n0[1] * inward_offset,
            ]);
        } else {
            let mx = avg_x / avg_len;
            let my = avg_y / avg_len;
            let denom = (mx * n0[0] + my * n0[1]).max(0.45);
            let miter_len = (mag / denom).min(mag * 2.2) * sign;
            out.push([curr[0] + mx * miter_len, curr[1] + my * miter_len]);
        }
    }

    // Verify no edge flipped direction (prevents self-intersection on tiny geometry)
    for i in 0..n {
        let j = (i + 1) % n;
        let old_dx = contour[j][0] - contour[i][0];
        let old_dy = contour[j][1] - contour[i][1];
        let new_dx = out[j][0] - out[i][0];
        let new_dy = out[j][1] - out[i][1];
        if old_dx * new_dx + old_dy * new_dy <= 1e-10 {
            return None;
        }
    }

    if !contour_is_simple(&out) {
        return None;
    }

    Some(out)
}

/// Checks that no two non-adjacent segments of a closed contour intersect.
pub fn contour_is_simple(c: &[Pt2]) -> bool {
    let n = c.len();
    if n < 4 {
        return n == 3;
    }
    for i in 0..n {
        let a0 = c[i];
        let a1 = c[(i + 1) % n];
        let min_ax = a0[0].min(a1[0]);
        let max_ax = a0[0].max(a1[0]);
        let min_ay = a0[1].min(a1[1]);
        let max_ay = a0[1].max(a1[1]);

        for j in (i + 2)..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let b0 = c[j];
            let b1 = c[(j + 1) % n];
            if b0[0].max(b1[0]) < min_ax - 1e-9
                || b0[0].min(b1[0]) > max_ax + 1e-9
                || b0[1].max(b1[1]) < min_ay - 1e-9
                || b0[1].min(b1[1]) > max_ay + 1e-9
            {
                continue;
            }
            if segments_intersect(a0, a1, b0, b1) {
                return false;
            }
        }
    }
    true
}

fn segments_intersect(a0: Pt2, a1: Pt2, b0: Pt2, b1: Pt2) -> bool {
    let cross = |p: Pt2, q: Pt2, r: Pt2| -> f64 {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let d1 = cross(b0, b1, a0);
    let d2 = cross(b0, b1, a1);
    let d3 = cross(a0, a1, b0);
    let d4 = cross(a0, a1, b1);

    ((d1 > 1e-9 && d2 < -1e-9) || (d1 < -1e-9 && d2 > 1e-9))
        && ((d3 > 1e-9 && d4 < -1e-9) || (d3 < -1e-9 && d4 > 1e-9))
}

/// Backward-compatible alias for inward CCW contour offset.
pub fn miter_inset_contour(contour: &Contour2D, inset: f64) -> Option<Contour2D> {
    miter_offset_contour(contour, inset)
}

/// Builds an annular band between `profile(offset_inner)` and `profile(offset_outer)`.
pub fn profile_band(
    kind: ProfileKind,
    w: f64,
    h: f64,
    offset_inner: f64,
    offset_outer: f64,
    arc_segments: usize,
) -> Shapes2D {
    let outer = shape_from_contour(profile_contour(kind, w, h, offset_outer, arc_segments));
    let inner = shape_from_contour(profile_contour(kind, w, h, offset_inner, arc_segments));
    difference_shapes(&outer, &inner)
}
