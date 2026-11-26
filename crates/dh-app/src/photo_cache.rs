use dh_core::WallPhotoConfig;
use dh_project::{load_rgba_image, orient_photo_rgba, process_photo_rgba, resolve_asset_path};
use dh_render::LoadedPhotoGpuData;
use eframe::egui;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
struct RawPhotoData {
    key: String,
    width_px: u32,
    height_px: u32,
    rgba: Arc<Vec<u8>>,
}

/// Caches decoded project-relative reference photos and their crop/rotation/perspective GPU textures.
/// Supports fast preview resolution while dragging perspective pins, and crisp full resolution on release.
#[derive(Default)]
pub struct WallPhotoTextureCache {
    raw: Option<RawPhotoData>,
    raw_tex: Option<(String, egui::TextureHandle)>,
    oriented_tex: Option<(String, u64, u32, u32, u32, egui::TextureHandle)>,
    loaded: Option<(LoadedPhotoGpuData, u64, u32)>,
    egui_tex: Option<(String, u64, u32, egui::TextureHandle)>,
    next_version: u64,
    last_error: Option<(String, String)>,
}

impl WallPhotoTextureCache {
    pub fn clear(&mut self) {
        self.raw = None;
        self.raw_tex = None;
        self.oriented_tex = None;
        self.loaded = None;
        self.egui_tex = None;
        self.last_error = None;
    }

    fn ensure_raw_loaded(&mut self, ctx: &egui::Context, dir: &Path, rel: &str, key: &str) -> bool {
        if self.raw.as_ref().is_some_and(|r| r.key == key) {
            return true;
        }
        if self.last_error.as_ref().is_some_and(|(k, _)| k == key) {
            return false;
        }
        let full_path = match resolve_asset_path(dir, rel) {
            Ok(p) => p,
            Err(e) => {
                self.clear();
                self.last_error = Some((key.to_string(), e));
                return false;
            }
        };
        match load_rgba_image(&full_path, 2048) {
            Ok(dec) => {
                let raw_arc = Arc::new(dec.rgba);
                let raw_img = egui::ColorImage::from_rgba_unmultiplied(
                    [dec.width_px as usize, dec.height_px as usize],
                    &raw_arc,
                );
                let r_tex = ctx.load_texture(
                    format!("wall_photo_raw_{}", self.next_version.wrapping_add(1)),
                    raw_img,
                    egui::TextureOptions::LINEAR,
                );
                self.raw = Some(RawPhotoData {
                    key: key.to_string(),
                    width_px: dec.width_px,
                    height_px: dec.height_px,
                    rgba: raw_arc,
                });
                self.raw_tex = Some((key.to_string(), r_tex));
                self.oriented_tex = None;
                self.loaded = None;
                self.egui_tex = None;
                self.last_error = None;
                true
            }
            Err(err) => {
                self.clear();
                self.last_error = Some((key.to_string(), err));
                false
            }
        }
    }

    pub fn ensure_loaded(
        &mut self,
        ctx: &egui::Context,
        project_dir: Option<&Path>,
        rel_asset_path: Option<&str>,
        cfg: &WallPhotoConfig,
        fast_preview: bool,
    ) -> Option<(&LoadedPhotoGpuData, &egui::TextureHandle)> {
        let (Some(dir), Some(rel)) = (project_dir, rel_asset_path) else {
            self.clear();
            return None;
        };
        let key = format!("{}::{}", dir.display(), rel);
        if !self.ensure_raw_loaded(ctx, dir, rel, &key) {
            return None;
        }

        let target_dim = if fast_preview { 480 } else { 1400 };
        let xform_hash = cfg.transform_hash();
        let proc_ready =
            self.loaded.as_ref().is_some_and(|(_, h, d)| {
                *h == xform_hash && (*d >= target_dim || !cfg.has_edits())
            }) && self.egui_tex.as_ref().is_some_and(|(k, h, d, _)| {
                k == &key && *h == xform_hash && (*d >= target_dim || !cfg.has_edits())
            });

        if !proc_ready {
            let raw = self.raw.as_ref()?;
            self.next_version = self.next_version.wrapping_add(1).max(1);
            let (pw, ph, p_rgba) = if cfg.has_edits() {
                let (w, h, buf) =
                    process_photo_rgba(raw.width_px, raw.height_px, &raw.rgba, cfg, target_dim);
                (w, h, Arc::new(buf))
            } else {
                (raw.width_px, raw.height_px, Arc::clone(&raw.rgba))
            };
            let color_img =
                egui::ColorImage::from_rgba_unmultiplied([pw as usize, ph as usize], &p_rgba);
            let tex = ctx.load_texture(
                format!("wall_photo_proc_{}", self.next_version),
                color_img,
                egui::TextureOptions::LINEAR,
            );
            self.loaded = Some((
                LoadedPhotoGpuData {
                    width_px: pw,
                    height_px: ph,
                    rgba: p_rgba,
                    version: self.next_version,
                },
                xform_hash,
                target_dim,
            ));
            self.egui_tex = Some((key, xform_hash, target_dim, tex));
        }

        match (self.loaded.as_ref(), self.egui_tex.as_ref()) {
            (Some((data, _, _)), Some((_, _, _, tex))) => Some((data, tex)),
            _ => None,
        }
    }

    pub fn loaded_egui_texture(&self) -> Option<(&LoadedPhotoGpuData, &egui::TextureHandle)> {
        match (self.loaded.as_ref(), self.egui_tex.as_ref()) {
            (Some((data, _, _)), Some((_, _, _, tex))) => Some((data, tex)),
            _ => None,
        }
    }

    pub fn ensure_oriented_loaded(
        &mut self,
        ctx: &egui::Context,
        project_dir: Option<&Path>,
        rel_asset_path: Option<&str>,
        rotation_deg: f64,
        fast_preview: bool,
    ) -> Option<(&egui::TextureHandle, u32, u32)> {
        let (Some(dir), Some(rel)) = (project_dir, rel_asset_path) else {
            self.clear();
            return None;
        };
        let key = format!("{}::{}", dir.display(), rel);
        if !self.ensure_raw_loaded(ctx, dir, rel, &key) {
            return None;
        }
        if rotation_deg.abs() < 1e-4 {
            self.oriented_tex = None;
            return self.raw_texture();
        }
        let target_dim = if fast_preview { 640 } else { 1400 };
        let rot_bits = rotation_deg.to_bits();
        let ready = self
            .oriented_tex
            .as_ref()
            .is_some_and(|(k, r, _, _, d, _)| k == &key && *r == rot_bits && *d >= target_dim);
        if !ready {
            let raw = self.raw.as_ref()?;
            let (ow, oh, o_rgba) = orient_photo_rgba(
                raw.width_px,
                raw.height_px,
                &raw.rgba,
                rotation_deg,
                target_dim,
            );
            self.next_version = self.next_version.wrapping_add(1).max(1);
            let img = egui::ColorImage::from_rgba_unmultiplied([ow as usize, oh as usize], &o_rgba);
            let tex = ctx.load_texture(
                format!("wall_photo_orient_{}", self.next_version),
                img,
                egui::TextureOptions::LINEAR,
            );
            self.oriented_tex = Some((key, rot_bits, ow, oh, target_dim, tex));
        }
        self.oriented_tex
            .as_ref()
            .map(|(_, _, ow, oh, _, tex)| (tex, *ow, *oh))
    }

    pub fn oriented_texture(&self) -> Option<(&egui::TextureHandle, u32, u32)> {
        if let Some((_, _, ow, oh, _, tex)) = self.oriented_tex.as_ref() {
            Some((tex, *ow, *oh))
        } else {
            self.raw_texture()
        }
    }

    pub fn raw_texture(&self) -> Option<(&egui::TextureHandle, u32, u32)> {
        match (self.raw.as_ref(), self.raw_tex.as_ref()) {
            (Some(raw), Some((_, tex))) => Some((tex, raw.width_px, raw.height_px)),
            _ => None,
        }
    }

    pub fn shared_gpu_data(&self) -> Option<LoadedPhotoGpuData> {
        self.loaded.as_ref().map(|(d, _, _)| d.clone())
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_ref().map(|(_, e)| e.as_str())
    }
}
