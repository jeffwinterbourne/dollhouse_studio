use crate::photo_cache::WallPhotoTextureCache;
use crate::theme::StudioPalette;
use dh_core::{DisplayUnit, WallPhotoConfig};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, LayerId, Margin, Order, Pos2, Rect, RichText,
    Shape, Stroke, StrokeKind, TextureId, Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoWizardStep {
    OrientAndCrop,
    PerspectiveCorners,
    CalibrateScale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotoCalibrationLine {
    pub id: usize,
    pub label: String,
    pub uv_a: [f64; 2],
    pub uv_b: [f64; 2],
    pub target_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CropHandle {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardDrag {
    Crop(CropHandle),
    Corner(usize),
    LinePoint { line_id: usize, end_idx: usize },
    Pan,
}

#[derive(Debug, Clone)]
pub struct PhotoWizardState {
    pub open: bool,
    pub wall_id: String,
    pub step: PhotoWizardStep,
    pub draft: WallPhotoConfig,
    pub lines: Vec<PhotoCalibrationLine>,
    pub active_line_id: Option<usize>,
    pub next_line_id: usize,
    pub allow_independent_hv_scale: bool,
    pub resize_wall_to_match: bool,
    pub auto_edit_on_canvas: bool,
    pub drawing_new_line: bool,
    pub view_zoom: f32,
    pub view_center_uv: [f32; 2],
    active_drag: Option<WizardDrag>,
    pending_first_pt: Option<[f64; 2]>,
}

impl Default for PhotoWizardState {
    fn default() -> Self {
        Self {
            open: false,
            wall_id: String::new(),
            step: PhotoWizardStep::OrientAndCrop,
            draft: WallPhotoConfig::default(),
            lines: Vec::new(),
            active_line_id: None,
            next_line_id: 1,
            allow_independent_hv_scale: false,
            resize_wall_to_match: false,
            auto_edit_on_canvas: false,
            drawing_new_line: false,
            view_zoom: 1.0,
            view_center_uv: [0.5, 0.5],
            active_drag: None,
            pending_first_pt: None,
        }
    }
}

impl PhotoWizardState {
    pub fn open_for(&mut self, wall_id: &str, current: &WallPhotoConfig) {
        self.open = true;
        self.wall_id = wall_id.to_string();
        self.step = PhotoWizardStep::OrientAndCrop;
        self.draft = current.clone();
        self.lines.clear();
        self.active_line_id = None;
        self.next_line_id = 1;
        self.allow_independent_hv_scale = false;
        self.resize_wall_to_match = false;
        self.drawing_new_line = true;
        self.pending_first_pt = None;
        self.active_drag = None;
        self.reset_view();
    }

    pub fn reset_view(&mut self) {
        self.view_zoom = 1.0;
        self.view_center_uv = [0.5, 0.5];
    }

    pub fn set_step(&mut self, step: PhotoWizardStep) {
        if self.step != step {
            self.step = step;
            self.active_drag = None;
            self.pending_first_pt = None;
            self.reset_view();
            if step == PhotoWizardStep::CalibrateScale && self.lines.is_empty() {
                self.drawing_new_line = true;
            }
        }
    }

    fn is_dragging_crop_or_pin(&self) -> bool {
        matches!(
            self.active_drag,
            Some(WizardDrag::Corner(_) | WizardDrag::Crop(_))
        )
    }
}

const LINE_COLORS: [Color32; 6] = [
    Color32::from_rgb(232, 180, 62),
    Color32::from_rgb(56, 189, 248),
    Color32::from_rgb(52, 211, 153),
    Color32::from_rgb(251, 113, 133),
    Color32::from_rgb(192, 132, 252),
    Color32::from_rgb(251, 146, 60),
];

fn line_color(id: usize) -> Color32 {
    LINE_COLORS[(id.saturating_sub(1)) % LINE_COLORS.len()]
}

/// Computes calibrated `(width_mm, height_mm)` for `cfg` given reference calibration lines
/// in the post-rotation, post-crop, perspective-rectified `[0, 1]^2` texture space.
pub fn compute_calibrated_size_mm(
    cfg: &WallPhotoConfig,
    wall_w: f64,
    wall_h: f64,
    lines: &[PhotoCalibrationLine],
    allow_independent_hv: bool,
) -> (f64, f64, Vec<(usize, f64)>) {
    let mut base = cfg.clone();
    base.clamp_edits();
    base.fit_to_wall(wall_w, wall_h);

    let mut valid: Vec<(usize, f64, f64, f64, f64)> = Vec::new();
    for line in lines {
        let dx0 = (line.uv_b[0] - line.uv_a[0]) * base.width_mm;
        let dy0 = (line.uv_b[1] - line.uv_a[1]) * base.height_mm;
        let len0 = dx0.hypot(dy0);
        if len0 > 1.0 && line.target_mm > 0.5 {
            valid.push((line.id, dx0, dy0, len0, line.target_mm));
        }
    }

    if valid.is_empty() {
        return (base.width_mm, base.height_mm, Vec::new());
    }

    // Uniform least-squares scale: minimize sum((s * len0 - target)^2)
    let sum_lt: f64 = valid.iter().map(|(_, _, _, l, t)| l * t).sum();
    let sum_ll: f64 = valid.iter().map(|(_, _, _, l, _)| l * l).sum();
    let uniform_s = (sum_lt / sum_ll.max(1e-9)).clamp(0.02, 50.0);

    let (sx, sy) = if allow_independent_hv && valid.len() >= 2 {
        // Solve 2x2 least squares for (u, v) = (sx^2, sy^2) where a_i * u + b_i * v = c_i
        let mut s_aa = 0.0;
        let mut s_ab = 0.0;
        let mut s_bb = 0.0;
        let mut r_a = 0.0;
        let mut r_b = 0.0;
        for (_, dx, dy, _, t) in &valid {
            let a = dx * dx;
            let b = dy * dy;
            let c = t * t;
            s_aa += a * a;
            s_ab += a * b;
            s_bb += b * b;
            r_a += a * c;
            r_b += b * c;
        }
        let det = s_aa * s_bb - s_ab * s_ab;
        if det > 1e-6 * (s_aa * s_bb).max(1.0) {
            let u = (r_a * s_bb - r_b * s_ab) / det;
            let v = (s_aa * r_b - s_ab * r_a) / det;
            if u > 0.0004 && v > 0.0004 {
                (u.sqrt().clamp(0.02, 50.0), v.sqrt().clamp(0.02, 50.0))
            } else {
                (uniform_s, uniform_s)
            }
        } else {
            (uniform_s, uniform_s)
        }
    } else {
        (uniform_s, uniform_s)
    };

    base.apply_calibrated_scale_xy(sx, sy, [0.0, 0.0]);
    let residuals = lines
        .iter()
        .map(|l| {
            let dx = (l.uv_b[0] - l.uv_a[0]) * base.width_mm;
            let dy = (l.uv_b[1] - l.uv_a[1]) * base.height_mm;
            (l.id, dx.hypot(dy))
        })
        .collect();
    (base.width_mm, base.height_mm, residuals)
}

pub type PhotoWizardApply = (WallPhotoConfig, bool, Option<(f64, f64)>);

/// Interactive large-canvas Photo Setup Wizard (Orient/Crop -> Oriented 4-Corner Perspective -> Multi-Line Exact Caliper).
/// Returns `Some((updated_config, open_canvas_photo_handles, optional_new_wall_size))`.
#[allow(clippy::too_many_arguments)]
pub fn draw_photo_setup_wizard(
    ctx: &egui::Context,
    pal: &StudioPalette,
    project_dir: Option<&std::path::Path>,
    wall_w: f64,
    wall_h: f64,
    unit: DisplayUnit,
    preview_cache: &mut WallPhotoTextureCache,
    state: &mut PhotoWizardState,
) -> Option<PhotoWizardApply> {
    if !state.open {
        return None;
    }

    let mut applied: Option<PhotoWizardApply> = None;
    let mut open = state.open;

    let fast = state.is_dragging_crop_or_pin();
    match state.step {
        PhotoWizardStep::OrientAndCrop | PhotoWizardStep::PerspectiveCorners => {
            preview_cache.ensure_oriented_loaded(
                ctx,
                project_dir,
                state.draft.asset_path.as_deref(),
                state.draft.rotation_deg,
                fast,
            );
        }
        PhotoWizardStep::CalibrateScale => {
            preview_cache.ensure_loaded(
                ctx,
                project_dir,
                state.draft.asset_path.as_deref(),
                &state.draft,
                false,
            );
        }
    }
    if state.draft.pixel_size == [0, 0]
        && let Some((_, rw, rh)) = preview_cache.raw_texture()
    {
        state.draft.pixel_size = [rw, rh];
    }

    let screen = ctx.content_rect();
    let win_w = (screen.width() - 36.0).clamp(760.0, 1480.0);
    let win_h = (screen.height() - 36.0).clamp(540.0, 960.0);

    egui::Window::new("Photo Setup Wizard")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .default_size(Vec2::new(win_w, win_h))
        .min_size(Vec2::new(680.0, 500.0))
        .show(ctx, |ui| {
            // Top step pills, zoom controls, and Skip to Canvas
            ui.horizontal(|ui| {
                for (step, num, title) in [
                    (PhotoWizardStep::OrientAndCrop, "1", "Orient & Crop"),
                    (
                        PhotoWizardStep::PerspectiveCorners,
                        "2",
                        "4-Corner Perspective",
                    ),
                    (PhotoWizardStep::CalibrateScale, "3", "Multi-Line Scale"),
                ] {
                    let active = state.step == step;
                    let btn = egui::Button::new(
                        RichText::new(format!("{num}. {title}"))
                            .strong()
                            .color(if active {
                                pal.accent_text
                            } else {
                                pal.text_primary
                            }),
                    )
                    .fill(if active {
                        pal.accent
                    } else {
                        pal.surface_card
                    })
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).clicked() {
                        state.set_step(step);
                    }
                }

                ui.separator();
                if ui.small_button("-").on_hover_text("Zoom out preview").clicked() {
                    state.view_zoom = (state.view_zoom / 1.25).clamp(0.6, 12.0);
                }
                if ui
                    .small_button(format!("{:.0}%", state.view_zoom * 100.0))
                    .on_hover_text("Reset zoom & pan to fit")
                    .clicked()
                {
                    state.reset_view();
                }
                if ui.small_button("+").on_hover_text("Zoom in preview").clicked() {
                    state.view_zoom = (state.view_zoom * 1.25).clamp(0.6, 12.0);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Skip to Canvas ->")
                        .on_hover_text(
                            "Skip wizard and adjust directly with mouse handles on Blueprint 2D",
                        )
                        .clicked()
                    {
                        let mut res = state.draft.clone();
                        res.clamp_edits();
                        res.fit_to_wall(wall_w, wall_h);
                        applied = Some((res, true, None));
                        state.open = false;
                    }
                });
            });

            ui.separator();
            ui.add_space(4.0);

            let controls_reserve = match state.step {
                PhotoWizardStep::OrientAndCrop => 130.0,
                PhotoWizardStep::PerspectiveCorners => 96.0,
                PhotoWizardStep::CalibrateScale => 230.0,
            };
            let preview_h = (ui.available_height() - controls_reserve).max(260.0);
            let (avail_r, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), preview_h),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(avail_r);
            painter.rect_filled(avail_r, 6.0, pal.canvas_bg);

            let active_tex: Option<(TextureId, u32, u32)> = match state.step {
                PhotoWizardStep::OrientAndCrop | PhotoWizardStep::PerspectiveCorners => {
                    preview_cache
                        .oriented_texture()
                        .map(|(tex, w, h)| (tex.id(), w, h))
                }
                PhotoWizardStep::CalibrateScale => preview_cache
                    .loaded_egui_texture()
                    .map(|(d, tex)| (tex.id(), d.width_px, d.height_px)),
            };

            let img_rect = if let Some((tex_id, pw, ph)) = active_tex {
                let aspect = pw as f32 / ph.max(1) as f32;
                let base_r = fit_aspect_rect(avail_r.shrink(14.0), aspect);
                let scaled_size = base_r.size() * state.view_zoom;

                // Scroll-to-cursor zoom & middle/right/space drag pan
                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                if resp.hovered() {
                    let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll_y.abs() > 0.1 {
                        let factor = (1.0 + scroll_y * 0.0022).clamp(0.75, 1.35);
                        let old_zoom = state.view_zoom;
                        let new_zoom = (old_zoom * factor).clamp(0.6, 12.0);
                        if (new_zoom - old_zoom).abs() > 1e-4 {
                            let cur_ir = Rect::from_min_size(
                                avail_r.center()
                                    - Vec2::new(
                                        state.view_center_uv[0] * scaled_size.x,
                                        state.view_center_uv[1] * scaled_size.y,
                                    ),
                                scaled_size,
                            );
                            let focus = pointer_pos.unwrap_or(avail_r.center());
                            let uv_p = [
                                (focus.x - cur_ir.left()) / scaled_size.x.max(1.0),
                                (focus.y - cur_ir.top()) / scaled_size.y.max(1.0),
                            ];
                            let new_scaled = base_r.size() * new_zoom;
                            state.view_center_uv = [
                                (uv_p[0] - (focus.x - avail_r.center().x) / new_scaled.x.max(1.0))
                                    .clamp(-0.25, 1.25),
                                (uv_p[1] - (focus.y - avail_r.center().y) / new_scaled.y.max(1.0))
                                    .clamp(-0.25, 1.25),
                            ];
                            state.view_zoom = new_zoom;
                        }
                    }
                }

                let pan_trigger = ui.input(|i| {
                    i.pointer.middle_down()
                        || i.pointer.secondary_down()
                        || (i.modifiers.ctrl && i.pointer.primary_down())
                        || (i.key_down(egui::Key::Space) && i.pointer.primary_down())
                });
                if pan_trigger && resp.hovered() {
                    state.active_drag = Some(WizardDrag::Pan);
                }
                if state.active_drag == Some(WizardDrag::Pan) {
                    if pan_trigger {
                        let delta = ui.input(|i| i.pointer.delta());
                        let cur_sz = base_r.size() * state.view_zoom;
                        state.view_center_uv[0] = (state.view_center_uv[0]
                            - delta.x / cur_sz.x.max(1.0))
                        .clamp(-0.25, 1.25);
                        state.view_center_uv[1] = (state.view_center_uv[1]
                            - delta.y / cur_sz.y.max(1.0))
                        .clamp(-0.25, 1.25);
                    } else {
                        state.active_drag = None;
                    }
                }

                let final_size = base_r.size() * state.view_zoom;
                let ir = Rect::from_min_size(
                    avail_r.center()
                        - Vec2::new(
                            state.view_center_uv[0] * final_size.x,
                            state.view_center_uv[1] * final_size.y,
                        ),
                    final_size,
                );
                painter.image(
                    tex_id,
                    ir,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                Some((tex_id, ir))
            } else {
                painter.text(
                    avail_r.center(),
                    Align2::CENTER_CENTER,
                    "Loading wall photo...",
                    FontId::proportional(14.0),
                    pal.text_muted,
                );
                None
            };

            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
            let primary_down = ui.input(|i| i.pointer.primary_down());
            let primary_released = ui.input(|i| i.pointer.primary_released());
            if primary_released && state.active_drag != Some(WizardDrag::Pan) {
                state.active_drag = None;
            }

            if let Some((tex_id, ir)) = img_rect {
                let to_s = |uv: [f64; 2]| {
                    Pos2::new(
                        ir.left() + uv[0] as f32 * ir.width(),
                        ir.top() + uv[1] as f32 * ir.height(),
                    )
                };
                let to_uv = |p: Pos2| {
                    [
                        ((p.x - ir.left()) / ir.width().max(1.0)).clamp(0.0, 1.0) as f64,
                        ((p.y - ir.top()) / ir.height().max(1.0)).clamp(0.0, 1.0) as f64,
                    ]
                };

                match state.step {
                    PhotoWizardStep::OrientAndCrop => {
                        draw_crop_preview_overlay(&painter, pal, ir, &state.draft);
                        let cl = state.draft.crop_left.clamp(0.0, 0.45) as f32;
                        let cr = 1.0 - state.draft.crop_right.clamp(0.0, 0.45) as f32;
                        let ct = state.draft.crop_top.clamp(0.0, 0.45) as f32;
                        let cb = 1.0 - state.draft.crop_bottom.clamp(0.0, 0.45) as f32;
                        let crop_r = Rect::from_min_max(
                            Pos2::new(ir.left() + cl * ir.width(), ir.top() + ct * ir.height()),
                            Pos2::new(ir.left() + cr * ir.width(), ir.top() + cb * ir.height()),
                        );
                        let handles = [
                            (CropHandle::Left, crop_r.left_center()),
                            (CropHandle::Right, crop_r.right_center()),
                            (CropHandle::Top, crop_r.center_top()),
                            (CropHandle::Bottom, crop_r.center_bottom()),
                        ];
                        if primary_pressed
                            && state.active_drag.is_none()
                            && let Some(p) = pointer_pos
                            && avail_r.contains(p)
                            && let Some((h, _)) = handles
                                .iter()
                                .find(|(_, hp)| hp.distance(p) <= 15.0)
                        {
                            state.active_drag = Some(WizardDrag::Crop(*h));
                        }
                        if primary_down
                            && let (Some(p), Some(WizardDrag::Crop(h))) =
                                (pointer_pos, state.active_drag)
                        {
                            let uv = to_uv(p);
                            match h {
                                CropHandle::Left => {
                                    state.draft.crop_left = uv[0].clamp(0.0, 0.45);
                                }
                                CropHandle::Right => {
                                    state.draft.crop_right = (1.0 - uv[0]).clamp(0.0, 0.45);
                                }
                                CropHandle::Top => {
                                    state.draft.crop_top = uv[1].clamp(0.0, 0.45);
                                }
                                CropHandle::Bottom => {
                                    state.draft.crop_bottom = (1.0 - uv[1]).clamp(0.0, 0.45);
                                }
                            }
                            state.draft.perspective_quad = None;
                        }
                        for (h, hp) in handles {
                            let active = state.active_drag == Some(WizardDrag::Crop(h));
                            let sz = match h {
                                CropHandle::Left | CropHandle::Right => Vec2::new(10.0, 28.0),
                                CropHandle::Top | CropHandle::Bottom => Vec2::new(28.0, 10.0),
                            };
                            let hr = Rect::from_center_size(hp, sz);
                            painter.rect_filled(
                                hr,
                                4.0,
                                if active { pal.gold } else { Color32::WHITE },
                            );
                            painter.rect_stroke(
                                hr,
                                4.0,
                                Stroke::new(1.5, pal.accent),
                                StrokeKind::Outside,
                            );
                        }
                    }
                    PhotoWizardStep::PerspectiveCorners => {
                        let mut quad = state.draft.effective_quad();
                        let pts = [to_s(quad[0]), to_s(quad[1]), to_s(quad[2]), to_s(quad[3])];
                        painter.add(Shape::closed_line(pts.to_vec(), Stroke::new(2.0, pal.gold)));
                        // Center reference crosshairs to judge straightness
                        let mid_t = pts[0].lerp(pts[1], 0.5);
                        let mid_b = pts[3].lerp(pts[2], 0.5);
                        let mid_l = pts[0].lerp(pts[3], 0.5);
                        let mid_r = pts[1].lerp(pts[2], 0.5);
                        let guide = Stroke::new(1.0, pal.gold.gamma_multiply(0.55));
                        painter.line_segment([mid_t, mid_b], guide);
                        painter.line_segment([mid_l, mid_r], guide);

                        if primary_pressed
                            && state.active_drag.is_none()
                            && let Some(p) = pointer_pos
                            && avail_r.contains(p)
                            && let Some((idx, _)) =
                                pts.iter().enumerate().find(|(_, cp)| cp.distance(p) <= 16.0)
                        {
                            state.active_drag = Some(WizardDrag::Corner(idx));
                        }
                        if primary_down
                            && let (Some(p), Some(WizardDrag::Corner(idx))) =
                                (pointer_pos, state.active_drag)
                        {
                            let uv = to_uv(p);
                            quad[idx] = uv;
                            state.draft.perspective_quad = Some(quad);
                            draw_top_layer_loupe(ctx, pal, tex_id, p, uv);
                        }

                        for (idx, pt) in pts.iter().enumerate() {
                            let active = state.active_drag == Some(WizardDrag::Corner(idx));
                            painter.circle_filled(
                                *pt,
                                8.0,
                                if active { pal.gold } else { Color32::WHITE },
                            );
                            painter.circle_stroke(*pt, 8.0, Stroke::new(2.0, pal.accent));
                        }
                    }
                    PhotoWizardStep::CalibrateScale => {
                        if primary_pressed
                            && state.active_drag.is_none()
                            && let Some(p) = pointer_pos
                            && avail_r.contains(p)
                        {
                            // First check if user clicked an existing endpoint handle
                            let mut hit = None;
                            for line in &state.lines {
                                let p0 = to_s(line.uv_a);
                                let p1 = to_s(line.uv_b);
                                if p0.distance(p) <= 14.0 {
                                    hit = Some((line.id, 0));
                                    break;
                                }
                                if p1.distance(p) <= 14.0 {
                                    hit = Some((line.id, 1));
                                    break;
                                }
                            }
                            if let Some((line_id, end_idx)) = hit {
                                state.active_line_id = Some(line_id);
                                state.active_drag = Some(WizardDrag::LinePoint { line_id, end_idx });
                            } else if state.drawing_new_line || state.lines.is_empty() {
                                let uv = to_uv(p);
                                if let Some(first_uv) = state.pending_first_pt.take() {
                                    let id = state.next_line_id;
                                    state.next_line_id += 1;
                                    state.lines.push(PhotoCalibrationLine {
                                        id,
                                        label: format!("Ref #{id}"),
                                        uv_a: first_uv,
                                        uv_b: uv,
                                        target_mm: 178.0,
                                    });
                                    state.active_line_id = Some(id);
                                    state.drawing_new_line = false;
                                } else {
                                    // Allow click-and-drag directly to finish line on release, or two clicks
                                    state.pending_first_pt = Some(uv);
                                }
                            } else if let Some(line) = state.lines.iter().min_by(|a, b| {
                                point_seg_dist(p, to_s(a.uv_a), to_s(a.uv_b))
                                    .total_cmp(&point_seg_dist(p, to_s(b.uv_a), to_s(b.uv_b)))
                            }) && point_seg_dist(p, to_s(line.uv_a), to_s(line.uv_b)) <= 14.0
                            {
                                state.active_line_id = Some(line.id);
                            }
                        }

                        if primary_down
                            && let (Some(p), Some(WizardDrag::LinePoint { line_id, end_idx })) =
                                (pointer_pos, state.active_drag)
                        {
                            let uv = to_uv(p);
                            if let Some(line) = state.lines.iter_mut().find(|l| l.id == line_id) {
                                if end_idx == 0 {
                                    line.uv_a = uv;
                                } else {
                                    line.uv_b = uv;
                                }
                            }
                            draw_top_layer_loupe(ctx, pal, tex_id, p, uv);
                        }

                        // Support click-drag to draw a new line in one fluid motion
                        if primary_released
                            && let (Some(first_uv), Some(p)) =
                                (state.pending_first_pt, pointer_pos)
                            && to_s(first_uv).distance(p) >= 14.0
                        {
                            let uv = to_uv(p);
                            let id = state.next_line_id;
                            state.next_line_id += 1;
                            state.lines.push(PhotoCalibrationLine {
                                id,
                                label: format!("Ref #{id}"),
                                uv_a: first_uv,
                                uv_b: uv,
                                target_mm: 178.0,
                            });
                            state.active_line_id = Some(id);
                            state.pending_first_pt = None;
                            state.drawing_new_line = false;
                        }

                        // Preview pending point / rubber band line
                        if let Some(first_uv) = state.pending_first_pt {
                            let s0 = to_s(first_uv);
                            painter.circle_filled(s0, 6.0, pal.gold);
                            if let Some(p) = pointer_pos {
                                painter.line_segment(
                                    [s0, p],
                                    Stroke::new(2.0, pal.gold.gamma_multiply(0.8)),
                                );
                                let uv = to_uv(p);
                                draw_top_layer_loupe(ctx, pal, tex_id, p, uv);
                            }
                        }

                        // Draw all calibration lines
                        for line in &state.lines {
                            let active = state.active_line_id == Some(line.id);
                            let col = line_color(line.id);
                            let s0 = to_s(line.uv_a);
                            let s1 = to_s(line.uv_b);
                            painter.line_segment(
                                [s0, s1],
                                Stroke::new(if active { 3.0 } else { 2.0 }, col),
                            );
                            for pt in [s0, s1] {
                                painter.circle_filled(
                                    pt,
                                    if active { 6.5 } else { 5.0 },
                                    if active { Color32::WHITE } else { col },
                                );
                                painter.circle_stroke(pt, if active { 6.5 } else { 5.0 }, Stroke::new(2.0, col));
                            }
                            let mid = s0.lerp(s1, 0.5);
                            let tag = format!("#{}  {}", line.id, unit.format_value(line.target_mm));
                            let g = painter.layout_no_wrap(tag, FontId::proportional(11.5), Color32::WHITE);
                            let pill = Rect::from_center_size(mid, g.size() + Vec2::new(12.0, 6.0));
                            painter.rect_filled(pill, 4.0, pal.hud_glass);
                            painter.rect_stroke(pill, 4.0, Stroke::new(1.0, col), StrokeKind::Outside);
                            painter.galley(pill.min + Vec2::new(6.0, 3.0), g, Color32::WHITE);
                        }
                    }
                }
            }
            let _ = resp;

            // Subtle pan/zoom hint chip in bottom-left of canvas
            let hint_txt = match state.step {
                PhotoWizardStep::OrientAndCrop => {
                    "Drag crop handles • Scroll: Zoom • Right/Middle-drag: Pan"
                }
                PhotoWizardStep::PerspectiveCorners => {
                    "Drag 4 pins onto real corners • Scroll: Zoom • Right/Middle-drag: Pan"
                }
                PhotoWizardStep::CalibrateScale => {
                    if state.pending_first_pt.is_some() {
                        "Click or release at second endpoint • Scroll: Zoom • Right-drag: Pan"
                    } else if state.drawing_new_line {
                        "Click & drag (or click A then B) on photo to place reference line • Right-drag: Pan"
                    } else {
                        "Drag endpoints or click '+ Add Line' • Right-drag: Pan"
                    }
                }
            };
            let hg = painter.layout_no_wrap(
                hint_txt.to_string(),
                FontId::proportional(11.0),
                pal.text_muted,
            );
            let hpill = Rect::from_min_size(
                Pos2::new(avail_r.left() + 8.0, avail_r.bottom() - hg.size().y - 12.0),
                hg.size() + Vec2::new(14.0, 6.0),
            );
            painter.rect_filled(hpill, 4.0, pal.hud_glass.gamma_multiply(0.9));
            painter.galley(hpill.min + Vec2::new(7.0, 3.0), hg, pal.text_primary);

            ui.add_space(6.0);

            // Step controls card
            egui::Frame::NONE
                .fill(pal.surface_card)
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10))
                .show(ui, |ui| match state.step {
                    PhotoWizardStep::OrientAndCrop => {
                        ui.horizontal(|ui| {
                            if ui.button("Rotate -90").clicked() {
                                state.draft.rotation_deg =
                                    (state.draft.rotation_deg - 90.0).rem_euclid(360.0);
                                state.draft.perspective_quad = None;
                            }
                            if ui.button("Rotate +90").clicked() {
                                state.draft.rotation_deg =
                                    (state.draft.rotation_deg + 90.0).rem_euclid(360.0);
                                state.draft.perspective_quad = None;
                            }
                            if ui.button("Reset Rotation").clicked() {
                                state.draft.rotation_deg = 0.0;
                                state.draft.perspective_quad = None;
                            }
                            ui.add(
                                egui::Slider::new(&mut state.draft.rotation_deg, -180.0..=180.0)
                                    .text("Angle"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Crop Margins (or drag handles on image):");
                            ui.add(
                                egui::Slider::new(&mut state.draft.crop_left, 0.0..=0.45).text("L"),
                            );
                            ui.add(
                                egui::Slider::new(&mut state.draft.crop_right, 0.0..=0.45)
                                    .text("R"),
                            );
                            ui.add(
                                egui::Slider::new(&mut state.draft.crop_top, 0.0..=0.45).text("T"),
                            );
                            ui.add(
                                egui::Slider::new(&mut state.draft.crop_bottom, 0.0..=0.45)
                                    .text("B"),
                            );
                        });
                    }
                    PhotoWizardStep::PerspectiveCorners => {
                        ui.horizontal(|ui| {
                            ui.label(
                                "Corners operate in the rotated image frame. Drag the 4 pins onto real wall corners (top-layer magnifier follows your cursor).",
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Reset Corners").clicked() {
                                    state.draft.perspective_quad = None;
                                    state.draft.perspective_h = 0.0;
                                    state.draft.perspective_v = 0.0;
                                }
                            });
                        });
                    }
                    PhotoWizardStep::CalibrateScale => {
                        let (cal_w, cal_h, residuals) = compute_calibrated_size_mm(
                            &state.draft,
                            wall_w,
                            wall_h,
                            &state.lines,
                            state.allow_independent_hv_scale,
                        );
                        ui.horizontal(|ui| {
                            let add_btn = egui::Button::new(
                                RichText::new("+ Add Reference Line")
                                    .strong()
                                    .color(if state.drawing_new_line {
                                        pal.accent_text
                                    } else {
                                        pal.text_primary
                                    }),
                            )
                            .fill(if state.drawing_new_line {
                                pal.accent
                            } else {
                                pal.surface_panel
                            });
                            if ui.add(add_btn).clicked() {
                                state.drawing_new_line = true;
                                state.pending_first_pt = None;
                            }
                            ui.separator();
                            ui.label(
                                RichText::new(format!(
                                    "Calibrated Photo: {}",
                                    unit.format_dims(cal_w, cal_h)
                                ))
                                .strong()
                                .color(pal.gold),
                            );
                            ui.separator();
                            ui.checkbox(
                                &mut state.allow_independent_hv_scale,
                                "Independent H & V scale (2+ lines)",
                            );
                            ui.checkbox(
                                &mut state.resize_wall_to_match,
                                "Resize wall to match photo",
                            );
                        });

                        ui.add_space(4.0);
                        if let Some(active_id) = state.active_line_id {
                            if let Some(active_line) =
                                state.lines.iter_mut().find(|l| l.id == active_id)
                            {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("Presets for active line:").small());
                                    for (lbl, mm) in unit.calibration_presets() {
                                        let is_cur = (active_line.target_mm - mm).abs() < 0.05;
                                        let b = egui::Button::new(
                                            RichText::new(lbl).small().color(if is_cur {
                                                pal.accent_text
                                            } else {
                                                pal.text_primary
                                            }),
                                        )
                                        .fill(if is_cur {
                                            pal.accent
                                        } else {
                                            pal.surface_panel
                                        });
                                        if ui.add(b).clicked() {
                                            active_line.target_mm = mm;
                                        }
                                    }
                                });
                            }
                        } else {
                            ui.label(
                                RichText::new(
                                    "Draw one or more known lengths (e.g. door height, window width) directly on the photo, then type exact measurements below.",
                                )
                                .small()
                                .color(pal.text_muted),
                            );
                        }

                        ui.add_space(4.0);
                        let mut remove_id = None;
                        egui::ScrollArea::vertical()
                            .max_height(78.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for line in &mut state.lines {
                                    let is_active = state.active_line_id == Some(line.id);
                                    let col = line_color(line.id);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(
                                                is_active,
                                                RichText::new(format!("#{}", line.id))
                                                    .strong()
                                                    .color(col),
                                            )
                                            .clicked()
                                        {
                                            state.active_line_id = Some(line.id);
                                        }
                                        ui.label("Real length:");
                                        let mut disp = unit.from_mm(line.target_mm);
                                        let resp = ui.add(
                                            egui::DragValue::new(&mut disp)
                                                .speed(unit.drag_speed() * 0.5)
                                                .range(unit.from_mm(2.0)..=unit.from_mm(2500.0))
                                                .max_decimals(unit.decimals().max(2))
                                                .suffix(unit.suffix()),
                                        );
                                        if resp.changed() {
                                            line.target_mm = unit.to_mm(disp).max(1.0);
                                            state.active_line_id = Some(line.id);
                                        }
                                        if let Some((_, fit_mm)) =
                                            residuals.iter().find(|(id, _)| *id == line.id)
                                        {
                                            let diff = fit_mm - line.target_mm;
                                            let err_txt = if diff.abs() < 0.08 {
                                                "exact".to_string()
                                            } else {
                                                format!(
                                                    "fit {} ({:+.1} mm)",
                                                    unit.format_value(*fit_mm),
                                                    diff
                                                )
                                            };
                                            ui.label(
                                                RichText::new(err_txt)
                                                    .small()
                                                    .color(pal.text_muted),
                                            );
                                        }
                                        if ui.small_button("Delete").clicked() {
                                            remove_id = Some(line.id);
                                        }
                                    });
                                }
                            });
                        if let Some(id) = remove_id {
                            state.lines.retain(|l| l.id != id);
                            if state.active_line_id == Some(id) {
                                state.active_line_id = state.lines.first().map(|l| l.id);
                            }
                        }

                        ui.checkbox(
                            &mut state.auto_edit_on_canvas,
                            "Keep on-canvas photo handles open after Apply",
                        );
                    }
                });

            ui.add_space(6.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let finish_btn = egui::Button::new(
                    RichText::new("Apply & Place on Wall")
                        .strong()
                        .color(pal.accent_text),
                )
                .fill(pal.accent);
                if ui.add(finish_btn).clicked() {
                    let mut final_cfg = state.draft.clone();
                    final_cfg.clamp_edits();
                    let (cw, ch, _) = compute_calibrated_size_mm(
                        &final_cfg,
                        wall_w,
                        wall_h,
                        &state.lines,
                        state.allow_independent_hv_scale,
                    );
                    final_cfg.fit_to_wall(wall_w, wall_h);
                    if !state.lines.is_empty() {
                        if state.allow_independent_hv_scale
                            && ((cw / ch.max(1.0)) - final_cfg.processed_aspect_ratio()).abs() > 1e-3
                        {
                            final_cfg.lock_aspect = false;
                        }
                        final_cfg.width_mm = cw;
                        final_cfg.height_mm = ch;
                        if state.resize_wall_to_match {
                            final_cfg.offset_x_mm = 0.0;
                            final_cfg.offset_y_mm = 0.0;
                        } else {
                            final_cfg.offset_x_mm = (wall_w - cw) * 0.5;
                            final_cfg.offset_y_mm = (wall_h - ch) * 0.5;
                        }
                    }
                    let new_wall = if state.resize_wall_to_match && !state.lines.is_empty() {
                        Some((cw.clamp(80.0, 2500.0), ch.clamp(80.0, 2500.0)))
                    } else {
                        None
                    };
                    applied = Some((final_cfg, state.auto_edit_on_canvas, new_wall));
                    state.open = false;
                }

                if state.step != PhotoWizardStep::CalibrateScale
                    && ui.button("Next Step ->").clicked()
                {
                    let next = match state.step {
                        PhotoWizardStep::OrientAndCrop => PhotoWizardStep::PerspectiveCorners,
                        PhotoWizardStep::PerspectiveCorners => PhotoWizardStep::CalibrateScale,
                        PhotoWizardStep::CalibrateScale => PhotoWizardStep::CalibrateScale,
                    };
                    state.set_step(next);
                }
                if state.step != PhotoWizardStep::OrientAndCrop
                    && ui.button("<- Back").clicked()
                {
                    let prev = match state.step {
                        PhotoWizardStep::OrientAndCrop => PhotoWizardStep::OrientAndCrop,
                        PhotoWizardStep::PerspectiveCorners => PhotoWizardStep::OrientAndCrop,
                        PhotoWizardStep::CalibrateScale => PhotoWizardStep::PerspectiveCorners,
                    };
                    state.set_step(prev);
                }
                if ui.button("Cancel").clicked() {
                    state.open = false;
                }
            });
        });

    if !open {
        state.open = false;
    }
    applied
}

fn draw_top_layer_loupe(
    ctx: &egui::Context,
    pal: &StudioPalette,
    tex_id: TextureId,
    cursor_pos: Pos2,
    uv: [f64; 2],
) {
    let bounds = ctx.content_rect().shrink(10.0);
    let size = Vec2::splat(132.0);
    let mut center = cursor_pos + Vec2::new(98.0, -96.0);
    // Flip quadrant cleanly if pushing against screen edges so loupe never hides cursor or goes off-screen
    if center.x + size.x * 0.5 > bounds.right() {
        center.x = cursor_pos.x - 98.0;
    }
    if center.y - size.y * 0.5 < bounds.top() {
        center.y = cursor_pos.y + 96.0;
    }
    let mut loupe_r = Rect::from_center_size(center, size);
    loupe_r = loupe_r.translate(Vec2::new(
        (bounds.left() - loupe_r.left()).max(0.0) - (loupe_r.right() - bounds.right()).max(0.0),
        (bounds.top() - loupe_r.top()).max(0.0) - (loupe_r.bottom() - bounds.bottom()).max(0.0),
    ));

    let painter = ctx.layer_painter(LayerId::new(
        Order::Tooltip,
        Id::unique("photo_wizard_loupe"),
    ));
    painter.rect_filled(loupe_r.expand(3.0), 8.0, pal.hud_glass);
    let hs = 0.055_f32;
    let u0 = (uv[0] as f32 - hs).clamp(0.0, 1.0 - hs * 2.0);
    let v0 = (uv[1] as f32 - hs).clamp(0.0, 1.0 - hs * 2.0);
    painter.image(
        tex_id,
        loupe_r,
        Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u0 + hs * 2.0, v0 + hs * 2.0)),
        Color32::WHITE,
    );
    // Crisp crosshair at exact cursor sample point
    let c_uv = Pos2::new(
        loupe_r.left() + ((uv[0] as f32 - u0) / (hs * 2.0)) * loupe_r.width(),
        loupe_r.top() + ((uv[1] as f32 - v0) / (hs * 2.0)) * loupe_r.height(),
    );
    let ch = Stroke::new(1.2, pal.gold);
    painter.line_segment(
        [
            Pos2::new(loupe_r.left(), c_uv.y),
            Pos2::new(loupe_r.right(), c_uv.y),
        ],
        ch,
    );
    painter.line_segment(
        [
            Pos2::new(c_uv.x, loupe_r.top()),
            Pos2::new(c_uv.x, loupe_r.bottom()),
        ],
        ch,
    );
    painter.rect_stroke(
        loupe_r,
        8.0,
        Stroke::new(2.0, pal.gold),
        StrokeKind::Outside,
    );
}

fn fit_aspect_rect(bounds: Rect, aspect: f32) -> Rect {
    let a = aspect.max(0.05);
    let bw = bounds.width();
    let bh = bounds.height();
    let (w, h) = if bw / bh.max(1.0) > a {
        (bh * a, bh)
    } else {
        (bw, bw / a)
    };
    Rect::from_center_size(bounds.center(), Vec2::new(w, h))
}

fn point_seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 <= 1e-4 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

fn draw_crop_preview_overlay(
    painter: &egui::Painter,
    pal: &StudioPalette,
    ir: Rect,
    cfg: &WallPhotoConfig,
) {
    let cl = cfg.crop_left.clamp(0.0, 0.45) as f32;
    let cr = 1.0 - cfg.crop_right.clamp(0.0, 0.45) as f32;
    let ct = cfg.crop_top.clamp(0.0, 0.45) as f32;
    let cb = 1.0 - cfg.crop_bottom.clamp(0.0, 0.45) as f32;
    let crop_r = Rect::from_min_max(
        Pos2::new(ir.left() + cl * ir.width(), ir.top() + ct * ir.height()),
        Pos2::new(ir.left() + cr * ir.width(), ir.top() + cb * ir.height()),
    );
    let mask = Color32::from_rgba_unmultiplied(12, 16, 22, 140);
    painter.rect_filled(
        Rect::from_min_max(ir.left_top(), Pos2::new(ir.right(), crop_r.top())),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(ir.left(), crop_r.bottom()), ir.right_bottom()),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(ir.left(), crop_r.top()),
            Pos2::new(crop_r.left(), crop_r.bottom()),
        ),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(crop_r.right(), crop_r.top()),
            Pos2::new(ir.right(), crop_r.bottom()),
        ),
        0.0,
        mask,
    );
    painter.rect_stroke(crop_r, 1.0, Stroke::new(1.8, pal.gold), StrokeKind::Outside);
}
