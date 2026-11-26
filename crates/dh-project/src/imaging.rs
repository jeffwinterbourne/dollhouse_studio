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

/// Applies crop, keystone / 4-point projective perspective rectification, and clockwise rotation
/// to a source RGBA image buffer using bilinear sampling.
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
    let [tl, tr, br, bl] = cfg.effective_quad();

    let dist_px =
        |a: [f64; 2], b: [f64; 2]| -> f64 { ((b[0] - a[0]) * sw).hypot((b[1] - a[1]) * sh) };
    let rect_w = (0.5 * (dist_px(tl, tr) + dist_px(bl, br))).max(16.0);
    let rect_h = (0.5 * (dist_px(tl, bl) + dist_px(tr, br))).max(16.0);

    let rad = cfg.rotation_deg.to_radians();
    let (sin_r, cos_r) = rad.sin_cos();
    let rot_w = rect_w * cos_r.abs() + rect_h * sin_r.abs();
    let rot_h = rect_w * sin_r.abs() + rect_h * cos_r.abs();

    let max_d = max_out_dim.max(64) as f64;
    let scale = (max_d / rot_w.max(rot_h)).min(1.0);
    let out_w = (rot_w * scale).round().clamp(16.0, max_d) as u32;
    let out_h = (rot_h * scale).round().clamp(16.0, max_d) as u32;

    let hom = compute_quad_homography(tl, tr, br, bl);
    let mut out = vec![0u8; out_w as usize * out_h as usize * 4];

    for oy in 0..out_h {
        let v_out = (oy as f64 + 0.5) / out_h as f64;
        let py = (v_out - 0.5) * rot_h;
        for ox in 0..out_w {
            let u_out = (ox as f64 + 0.5) / out_w as f64;
            let px = (u_out - 0.5) * rot_w;

            let rx = px * cos_r + py * sin_r;
            let ry = -px * sin_r + py * cos_r;
            let ur = rx / rect_w + 0.5;
            let vr = ry / rect_h + 0.5;

            let dst_idx = (oy as usize * out_w as usize + ox as usize) * 4;
            if !(0.0..=1.0).contains(&ur) || !(0.0..=1.0).contains(&vr) {
                continue;
            }

            let denom = hom[6] * ur + hom[7] * vr + hom[8];
            if denom.abs() < 1e-9 {
                continue;
            }
            let su = (hom[0] * ur + hom[1] * vr + hom[2]) / denom;
            let sv = (hom[3] * ur + hom[4] * vr + hom[5]) / denom;
            if !(0.0..=1.0).contains(&su) || !(0.0..=1.0).contains(&sv) {
                continue;
            }

            let sx = su * (sw - 1.0);
            let sy = sv * (sh - 1.0);
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

            for c in 0..4 {
                let v00 = src_rgba[idx00 + c] as f64;
                let v10 = src_rgba[idx10 + c] as f64;
                let v01 = src_rgba[idx01 + c] as f64;
                let v11 = src_rgba[idx11 + c] as f64;
                let top = v00 * (1.0 - fx) + v10 * fx;
                let bot = v01 * (1.0 - fx) + v11 * fx;
                out[dst_idx + c] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
            }
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
