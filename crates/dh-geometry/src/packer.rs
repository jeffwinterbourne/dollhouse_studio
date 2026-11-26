/// Placement of a single 2D bounding box onto a 3D printer build plate.
#[derive(Debug, Clone, Copy)]
pub struct PartPlacement {
    pub part_index: usize,
    pub plate_index: usize,
    /// Target minimum X on the centered plate (`plate_cx = 0`) after optional 90° CCW rotation.
    pub target_min_x: f32,
    /// Target minimum Y on the centered plate (`plate_cy = 0`) after optional 90° CCW rotation.
    pub target_min_y: f32,
    /// True if the part should be rotated 90° CCW in XY before translating to `(target_min_x, target_min_y)`.
    pub rotated_90: bool,
}

/// Multi-plate MaxRects packing result for a list of `(width, height)` bounding boxes.
#[derive(Debug, Clone, Default)]
pub struct BedPackLayout {
    pub plate_count: usize,
    pub placements: Vec<PartPlacement>,
    /// Indices of parts whose footprint exceeds the usable bed in both orientations.
    pub oversized_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy)]
struct FreeRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Debug, Clone, Copy)]
struct RawPlaced {
    part_index: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rotated_90: bool,
}

struct MaxRectsBin {
    free_rects: Vec<FreeRect>,
    placed: Vec<RawPlaced>,
}

impl MaxRectsBin {
    fn new(expanded_w: f32, expanded_h: f32) -> Self {
        Self {
            free_rects: vec![FreeRect {
                x: 0.0,
                y: 0.0,
                w: expanded_w,
                h: expanded_h,
            }],
            placed: Vec::new(),
        }
    }

    fn find_best_bssf(&self, rw: f32, rh: f32, gap: f32) -> Option<(f32, f32, f32, f32)> {
        let aw = rw + gap;
        let ah = rh + gap;
        let mut best: Option<(f32, f32, f32, f32)> = None;
        for fr in &self.free_rects {
            if aw <= fr.w + 1e-4 && ah <= fr.h + 1e-4 {
                let lw = (fr.w - aw).max(0.0);
                let lh = (fr.h - ah).max(0.0);
                let short_side = lw.min(lh);
                let long_side = lw.max(lh);
                let better = match best {
                    None => true,
                    Some((bs, bl, _, _)) => {
                        short_side < bs - 1e-4
                            || ((short_side - bs).abs() <= 1e-4 && long_side < bl - 1e-4)
                    }
                };
                if better {
                    best = Some((short_side, long_side, fr.x, fr.y));
                }
            }
        }
        best
    }

    fn place_rect(&mut self, rect: RawPlaced, gap: f32) {
        self.placed.push(rect);

        let x = rect.x;
        let y = rect.y;
        let aw = rect.w + gap;
        let ah = rect.h + gap;
        let px1 = x + aw;
        let py1 = y + ah;

        let mut next_free: Vec<FreeRect> = Vec::with_capacity(self.free_rects.len() * 2);
        for &fr in &self.free_rects {
            let fx1 = fr.x + fr.w;
            let fy1 = fr.y + fr.h;
            if x >= fx1 - 1e-4 || px1 <= fr.x + 1e-4 || y >= fy1 - 1e-4 || py1 <= fr.y + 1e-4 {
                next_free.push(fr);
                continue;
            }
            if x > fr.x + 1e-3 {
                next_free.push(FreeRect {
                    x: fr.x,
                    y: fr.y,
                    w: x - fr.x,
                    h: fr.h,
                });
            }
            if px1 < fx1 - 1e-3 {
                next_free.push(FreeRect {
                    x: px1,
                    y: fr.y,
                    w: fx1 - px1,
                    h: fr.h,
                });
            }
            if y > fr.y + 1e-3 {
                next_free.push(FreeRect {
                    x: fr.x,
                    y: fr.y,
                    w: fr.w,
                    h: y - fr.y,
                });
            }
            if py1 < fy1 - 1e-3 {
                next_free.push(FreeRect {
                    x: fr.x,
                    y: py1,
                    w: fr.w,
                    h: fy1 - py1,
                });
            }
        }

        // Prune free rectangles strictly contained inside any other free rectangle
        let len = next_free.len();
        let mut keep = vec![true; len];
        for i in 0..len {
            if !keep[i] {
                continue;
            }
            let a = next_free[i];
            for (j, &b) in next_free.iter().enumerate() {
                if i == j || !keep[j] {
                    continue;
                }
                if a.x >= b.x - 1e-4
                    && a.y >= b.y - 1e-4
                    && a.x + a.w <= b.x + b.w + 1e-4
                    && a.y + a.h <= b.y + b.h + 1e-4
                {
                    keep[i] = false;
                    break;
                }
            }
        }
        self.free_rects = next_free
            .into_iter()
            .zip(keep)
            .filter_map(|(r, k)| k.then_some(r))
            .collect();
    }
}

/// Packs 2D part bounding boxes `(width_mm, height_mm)` onto one or more `(bed_w x bed_h)` plates
/// using Best-Short-Side-Fit MaxRects with optional 90° rotation and guaranteed spacing `gap`.
pub fn pack_part_bounds_onto_beds(
    part_sizes: &[(f32, f32)],
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> BedPackLayout {
    if part_sizes.is_empty() {
        return BedPackLayout::default();
    }

    let usable_w = (bed_w - 4.0).max(40.0);
    let usable_h = (bed_h - 4.0).max(40.0);
    let gap = spacing.clamp(1.0, 25.0);
    let exp_w = usable_w + gap;
    let exp_h = usable_h + gap;

    let mut order: Vec<(usize, f32, f32)> = part_sizes
        .iter()
        .enumerate()
        .map(|(i, &(w, h))| (i, w.max(0.1), h.max(0.1)))
        .collect();
    order.sort_by(|a, b| {
        let ma = a.1.max(a.2);
        let mb = b.1.max(b.2);
        mb.partial_cmp(&ma)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                (b.1 * b.2)
                    .partial_cmp(&(a.1 * a.2))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut bins: Vec<MaxRectsBin> = Vec::new();
    let mut dedicated_oversized_bins: Vec<MaxRectsBin> = Vec::new();
    let mut oversized_indices: Vec<usize> = Vec::new();

    for (idx, pw, ph) in order {
        let fits_rot0 = pw <= usable_w + 1e-4 && ph <= usable_h + 1e-4;
        let fits_rot90 = ph <= usable_w + 1e-4 && pw <= usable_h + 1e-4;

        if !fits_rot0 && !fits_rot90 {
            oversized_indices.push(idx);
            let rot = (bed_w > bed_h && ph > pw) || (bed_h > bed_w && pw > ph);
            let (rw, rh) = if rot { (ph, pw) } else { (pw, ph) };
            let mut bin = MaxRectsBin::new(rw + gap, rh + gap);
            bin.place_rect(
                RawPlaced {
                    part_index: idx,
                    x: 0.0,
                    y: 0.0,
                    w: rw,
                    h: rh,
                    rotated_90: rot,
                },
                gap,
            );
            dedicated_oversized_bins.push(bin);
            continue;
        }

        struct Candidate {
            bin_idx: usize,
            short_side: f32,
            long_side: f32,
            x: f32,
            y: f32,
            rw: f32,
            rh: f32,
            rotated_90: bool,
        }

        let mut chosen: Option<Candidate> = None;
        for (b_idx, bin) in bins.iter().enumerate() {
            let mut try_orient = |rw: f32, rh: f32, rotated_90: bool| {
                if let Some((ss, ls, x, y)) = bin.find_best_bssf(rw, rh, gap) {
                    let take = match chosen {
                        None => true,
                        Some(ref cur) => {
                            b_idx < cur.bin_idx
                                || (b_idx == cur.bin_idx
                                    && (ss < cur.short_side - 1e-4
                                        || ((ss - cur.short_side).abs() <= 1e-4
                                            && ls < cur.long_side - 1e-4)))
                        }
                    };
                    if take {
                        chosen = Some(Candidate {
                            bin_idx: b_idx,
                            short_side: ss,
                            long_side: ls,
                            x,
                            y,
                            rw,
                            rh,
                            rotated_90,
                        });
                    }
                }
            };
            if fits_rot0 {
                try_orient(pw, ph, false);
            }
            if fits_rot90 && (pw - ph).abs() > 0.1 {
                try_orient(ph, pw, true);
            }
        }

        if let Some(c) = chosen {
            bins[c.bin_idx].place_rect(
                RawPlaced {
                    part_index: idx,
                    x: c.x,
                    y: c.y,
                    w: c.rw,
                    h: c.rh,
                    rotated_90: c.rotated_90,
                },
                gap,
            );
        } else {
            let mut new_bin = MaxRectsBin::new(exp_w, exp_h);
            let (rw, rh, rot) = if fits_rot0 && fits_rot90 {
                let s0 = new_bin.find_best_bssf(pw, ph, gap);
                let s1 = new_bin.find_best_bssf(ph, pw, gap);
                match (s0, s1) {
                    (Some((ss0, ls0, _, _)), Some((ss1, ls1, _, _)))
                        if ss1 < ss0 - 1e-4 || ((ss1 - ss0).abs() <= 1e-4 && ls1 < ls0 - 1e-4) =>
                    {
                        (ph, pw, true)
                    }
                    _ => (pw, ph, false),
                }
            } else if fits_rot0 {
                (pw, ph, false)
            } else {
                (ph, pw, true)
            };
            let (_, _, x, y) = new_bin
                .find_best_bssf(rw, rh, gap)
                .expect("part verified to fit inside empty bed");
            new_bin.place_rect(
                RawPlaced {
                    part_index: idx,
                    x,
                    y,
                    w: rw,
                    h: rh,
                    rotated_90: rot,
                },
                gap,
            );
            bins.push(new_bin);
        }
    }

    bins.extend(dedicated_oversized_bins);

    let mut placements = Vec::with_capacity(part_sizes.len());
    for (plate_idx, bin) in bins.iter().enumerate() {
        if bin.placed.is_empty() {
            continue;
        }
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for p in &bin.placed {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x + p.w);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y + p.h);
        }
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        for p in &bin.placed {
            placements.push(PartPlacement {
                part_index: p.part_index,
                plate_index: plate_idx,
                target_min_x: p.x - cx,
                target_min_y: p.y - cy,
                rotated_90: p.rotated_90,
            });
        }
    }
    placements.sort_by_key(|p| p.part_index);
    oversized_indices.sort_unstable();

    BedPackLayout {
        plate_count: bins.len(),
        placements,
        oversized_indices,
    }
}
