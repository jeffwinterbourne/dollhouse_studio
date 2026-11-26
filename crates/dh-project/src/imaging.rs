use dh_core::WallPhotoConfig;
use std::path::Path;

/// Decoded RGBA image buffer independent of any GUI framework.
#[derive(Debug, Clone)]
pub struct DecodedPhoto {
    pub width_px: u32,
    pub height_px: u32,
    pub rgba: Vec<u8>,
}

/// Loads an image from disk and caps its longest dimension to `max_dim` pixels.
pub fn load_rgba_image(path: &Path, max_dim: u32) -> Result<DecodedPhoto, String> {
    let mut img =
        image::open(path).map_err(|e| format!("Failed to load photo '{}': {e}", path.display()))?;
    let cap = max_dim.max(64);
    if img.width() > cap || img.height() > cap {
        img = img.resize(cap, cap, image::imageops::FilterType::Triangle);
    }
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(DecodedPhoto {
        width_px: w,
        height_px: h,
        rgba: rgba.into_raw(),
    })
}

/// Bilinearly samples a source RGBA buffer at normalized UV `(su, sv) in [0, 1]^2`.
#[inline]
fn sample_bilinear(src_w: u32, src_h: u32, src_rgba: &[u8], su: f64, sv: f64, out_rgba: &mut [u8]) {
    let sx = su.clamp(0.0, 1.0) * (src_w - 1) as f64;
    let sy = sv.clamp(0.0, 1.0) * (src_h - 1) as f64;
    let x0 = sx.floor() as u32;
    let y0 = sy.floor() as u32;
    let x1 = (x0 + 1).min(src_w - 1);
    let y1 = (y0 + 1).min(src_h - 1);
    let fx = sx - x0 as f64;
    let fy = sy - y0 as f64;

    let idx00 = (y0 as usize * src_w as usize + x0 as usize) * 4;
    let idx10 = (y0 as usize * src_w as usize + x1 as usize) * 4;
    let idx01 = (y1 as usize * src_w as usize + x0 as usize) * 4;
    let idx11 = (y1 as usize * src_w as usize + x1 as usize) * 4;

    for (c, dst) in out_rgba.iter_mut().enumerate().take(4) {
        let v00 = src_rgba[idx00 + c] as f64;
        let v10 = src_rgba[idx10 + c] as f64;
        let v01 = src_rgba[idx01 + c] as f64;
        let v11 = src_rgba[idx11 + c] as f64;
        let top = v00 * (1.0 - fx) + v10 * fx;
        let bot = v01 * (1.0 - fx) + v11 * fx;
        *dst = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
}

/// Rotates `src_rgba` clockwise by `rotation_deg` into the oriented image coordinate frame
/// where crop and 4-corner perspective pins operate.
pub fn orient_photo_rgba(
    src_w: u32,
    src_h: u32,
    src_rgba: &[u8],
    rotation_deg: f64,
    max_out_dim: u32,
) -> (u32, u32, Vec<u8>) {
    if src_w == 0 || src_h == 0 || src_rgba.len() < (src_w as usize * src_h as usize * 4) {
        return (src_w, src_h, src_rgba.to_vec());
    }
    if rotation_deg.abs() <= 1e-4 && src_w <= max_out_dim && src_h <= max_out_dim {
        return (src_w, src_h, src_rgba.to_vec());
    }

    let sw = src_w as f64;
    let sh = src_h as f64;
    let rad = rotation_deg.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();
    let rot_w = (sw * cos_r.abs() + sh * sin_r.abs()).max(16.0);
    let rot_h = (sw * sin_r.abs() + sh * cos_r.abs()).max(16.0);

    let max_d = max_out_dim.max(64) as f64;
    let scale = (max_d / rot_w.max(rot_h)).min(1.0);
    let out_w = (rot_w * scale).round().clamp(16.0, max_d) as u32;
    let out_h = (rot_h * scale).round().clamp(16.0, max_d) as u32;

    let mut out = vec![0u8; out_w as usize * out_h as usize * 4];
    for oy in 0..out_h {
        let ov = (oy as f64 + 0.5) / out_h as f64;
        let py_o = (ov - 0.5) * rot_h;
        for ox in 0..out_w {
            let ou = (ox as f64 + 0.5) / out_w as f64;
            let px_o = (ou - 0.5) * rot_w;
            let px_s = px_o * cos_r + py_o * sin_r;
            let py_s = -px_o * sin_r + py_o * cos_r;
            let su = px_s / sw + 0.5;
            let sv = py_s / sh + 0.5;
            if !(0.0..=1.0).contains(&su) || !(0.0..=1.0).contains(&sv) {
                continue;
            }
            let dst = (oy as usize * out_w as usize + ox as usize) * 4;
            sample_bilinear(src_w, src_h, src_rgba, su, sv, &mut out[dst..dst + 4]);
        }
    }
    (out_w, out_h, out)
}

/// Applies clockwise rotation FIRST, then crop & 4-corner projective perspective rectification
/// defined in the oriented frame, using a single bilinear sampling pass over `src_rgba` (zero double-blur).
/// `max_out_dim` controls output resolution (`1400` for crisp render, `480` for fast live drag preview).
pub fn process_photo_rgba(
    src_w: u32,
    src_h: u32,
    src_rgba: &[u8],
    cfg: &WallPhotoConfig,
    max_out_dim: u32,
) -> (u32, u32, Vec<u8>) {
    if src_w == 0 || src_h == 0 || src_rgba.len() < (src_w as usize * src_h as usize * 4) {
        return (src_w, src_h, src_rgba.to_vec());
    }
    if !cfg.has_edits() && src_w <= max_out_dim && src_h <= max_out_dim {
        return (src_w, src_h, src_rgba.to_vec());
    }

    let sw = src_w as f64;
    let sh = src_h as f64;
    let rad = cfg.rotation_deg.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();

    // Step 1 frame: oriented image bounding box dimensions.
    let rot_w = (sw * cos_r.abs() + sh * sin_r.abs()).max(16.0);
    let rot_h = (sw * sin_r.abs() + sh * cos_r.abs()).max(16.0);

    // Step 2 quad operates inside the oriented [0, 1]^2 UV space.
    let [tl, tr, br, bl] = cfg.effective_quad();
    let dist_o =
        |a: [f64; 2], b: [f64; 2]| -> f64 { ((b[0] - a[0]) * rot_w).hypot((b[1] - a[1]) * rot_h) };
    let rect_w = (0.5 * (dist_o(tl, tr) + dist_o(bl, br))).max(16.0);
    let rect_h = (0.5 * (dist_o(tl, bl) + dist_o(tr, br))).max(16.0);

    let max_d = max_out_dim.max(64) as f64;
    let scale = (max_d / rect_w.max(rect_h)).min(1.0);
    let out_w = (rect_w * scale).round().clamp(16.0, max_d) as u32;
    let out_h = (rect_h * scale).round().clamp(16.0, max_d) as u32;

    let hom = compute_quad_homography(tl, tr, br, bl);
    let mut out = vec![0u8; out_w as usize * out_h as usize * 4];

    for oy in 0..out_h {
        let vr = (oy as f64 + 0.5) / out_h as f64;
        for ox in 0..out_w {
            let ur = (ox as f64 + 0.5) / out_w as f64;

            let denom = hom[6] * ur + hom[7] * vr + hom[8];
            if denom.abs() < 1e-9 {
                continue;
            }
            let ou = (hom[0] * ur + hom[1] * vr + hom[2]) / denom;
            let ov = (hom[3] * ur + hom[4] * vr + hom[5]) / denom;
            if !(0.0..=1.0).contains(&ou) || !(0.0..=1.0).contains(&ov) {
                continue;
            }

            // Map oriented UV back through inverse rotation to raw source UV in a single lookup.
            let px_o = (ou - 0.5) * rot_w;
            let py_o = (ov - 0.5) * rot_h;
            let px_s = px_o * cos_r + py_o * sin_r;
            let py_s = -px_o * sin_r + py_o * cos_r;
            let su = px_s / sw + 0.5;
            let sv = py_s / sh + 0.5;
            if !(0.0..=1.0).contains(&su) || !(0.0..=1.0).contains(&sv) {
                continue;
            }

            let dst = (oy as usize * out_w as usize + ox as usize) * 4;
            sample_bilinear(src_w, src_h, src_rgba, su, sv, &mut out[dst..dst + 4]);
        }
    }

    (out_w, out_h, out)
}

/// Solves for the 3x3 projective homography mapping unit square `(0,0),(1,0),(1,1),(0,1)`
/// to `(tl, tr, br, bl)`.
pub fn compute_quad_homography(tl: [f64; 2], tr: [f64; 2], br: [f64; 2], bl: [f64; 2]) -> [f64; 9] {
    let unit = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let dst = [tl, tr, br, bl];
    let mut a = [[0.0f64; 9]; 8];

    for i in 0..4 {
        let [x, y] = unit[i];
        let [u, v] = dst[i];
        let r0 = i * 2;
        let r1 = r0 + 1;
        a[r0] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        a[r1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }

    for col in 0..8 {
        let mut best_row = col;
        let mut best_val = a[col][col].abs();
        for (r, row) in a.iter().enumerate().skip(col + 1) {
            if row[col].abs() > best_val {
                best_val = row[col].abs();
                best_row = r;
            }
        }
        a.swap(col, best_row);
        let pivot = a[col][col];
        if pivot.abs() < 1e-12 {
            return [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        }
        for cell in &mut a[col][col..9] {
            *cell /= pivot;
        }
        let pivot_row = a[col];
        for (r, row) in a.iter_mut().enumerate() {
            if r != col {
                let f = row[col];
                for c in col..9 {
                    row[c] -= f * pivot_row[c];
                }
            }
        }
    }

    [
        a[0][8], a[1][8], a[2][8], a[3][8], a[4][8], a[5][8], a[6][8], a[7][8], 1.0,
    ]
}
