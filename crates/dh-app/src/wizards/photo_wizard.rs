use crate::photo_cache::WallPhotoTextureCache;
use crate::theme::StudioPalette;
use dh_core::{DisplayUnit, WallPhotoConfig};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Shape, Stroke, StrokeKind, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoWizardStep {
    OrientAndCrop,
    PerspectiveCorners,
    CalibrateScale,
}

#[derive(Debug, Clone)]
pub struct PhotoWizardState {
    pub open: bool,
    pub wall_id: String,
    pub step: PhotoWizardStep,
    pub draft: WallPhotoConfig,
    active_corner: Option<usize>,
    active_caliper: Option<usize>,
    pub cal_pts_uv: [Option<[f64; 2]>; 2],
    pub target_real_mm: f64,
    pub auto_edit_on_canvas: bool,
}

impl Default for PhotoWizardState {
    fn default() -> Self {
        Self {
            open: false,
            wall_id: String::new(),
            step: PhotoWizardStep::OrientAndCrop,
            draft: WallPhotoConfig::default(),
            active_corner: None,
            active_caliper: None,
            cal_pts_uv: [None, None],
            target_real_mm: 178.0,
            auto_edit_on_canvas: false,
        }
    }
}

impl PhotoWizardState {
    pub fn open_for(&mut self, wall_id: &str, current: &WallPhotoConfig) {
        self.open = true;
        self.wall_id = wall_id.to_string();
        self.step = PhotoWizardStep::OrientAndCrop;
        self.draft = current.clone();
        self.active_corner = None;
        self.active_caliper = None;
        self.cal_pts_uv = [None, None];
    }
}

/// Interactive triggerable Photo Setup Wizard (Orient/Crop -> 4-Corner Perspective with loupe -> 2-Point Visual Caliper).
/// Includes an immediate `"Skip to Canvas"` button so users can also work straight on Blueprint 2D.
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
) -> Option<(WallPhotoConfig, bool)> {
    if !state.open {
        return None;
    }

    let mut applied: Option<(WallPhotoConfig, bool)> = None;
    let mut open = state.open;

    let fast = state.active_corner.is_some();
    preview_cache.ensure_loaded(
        ctx,
        project_dir,
        state.draft.asset_path.as_deref(),
        &state.draft,
        fast,
    );

    egui::Window::new("Photo Setup Wizard")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .default_size(Vec2::new(720.0, 580.0))
        .show(ctx, |ui| {
            // Top step pills + Skip to Canvas
            ui.horizontal(|ui| {
                for (step, num, title) in [
                    (PhotoWizardStep::OrientAndCrop, "1", "Orient & Crop"),
                    (PhotoWizardStep::PerspectiveCorners, "2", "4-Corner Perspective"),
                    (PhotoWizardStep::CalibrateScale, "3", "Scale Caliper"),
                ] {
                    let active = state.step == step;
                    let btn = egui::Button::new(
                        RichText::new(format!("{num}. {title}"))
                            .strong()
                            .color(if active { pal.accent_text } else { pal.text_primary }),
                    )
                    .fill(if active { pal.accent } else { pal.surface_card })
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).clicked() {
                        state.step = step;
                        state.active_corner = None;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Skip to Canvas ->")
                        .on_hover_text("Skip the wizard and adjust directly with mouse handles on Blueprint 2D")
                        .clicked()
                    {
                        let mut res = state.draft.clone();
                        res.fit_to_wall(wall_w, wall_h);
                        applied = Some((res, true));
                        state.open = false;
                    }
                });
            });

            ui.separator();
            ui.add_space(6.0);

            // Preview image interactive region.
            let preview_h = 330.0;
            let (avail_r, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), preview_h),
                egui::Sense::click_and_drag(),
            );
            let painter = ui.painter_at(avail_r);
            painter.rect_filled(avail_r, 6.0, pal.canvas_bg);

            // Determine texture aspect ratio cleanly.
            let img_rect = if state.step == PhotoWizardStep::PerspectiveCorners {
                if let Some((raw_tex, pw, ph)) = preview_cache.raw_texture() {
                    let r = fit_aspect_rect(avail_r.shrink(12.0), pw as f32 / ph.max(1) as f32);
                    painter.image(
                        raw_tex.id(),
                        r,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    Some(r)
                } else {
                    None
                }
            } else if let Some((data, proc_tex)) = preview_cache.loaded_egui_texture() {
                let r = fit_aspect_rect(
                    avail_r.shrink(12.0),
                    data.width_px as f32 / data.height_px.max(1) as f32,
                );
                painter.image(
                    proc_tex.id(),
                    r,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                Some(r)
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
            if primary_released {
                state.active_corner = None;
                state.active_caliper = None;
            }

            if let Some(ir) = img_rect {
                match state.step {
                    PhotoWizardStep::OrientAndCrop => {
                        draw_crop_preview_overlay(&painter, pal, ir, &state.draft);
                    }
                    PhotoWizardStep::PerspectiveCorners => {
                        let mut quad = state.draft.effective_quad();
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
                        let pts = [to_s(quad[0]), to_s(quad[1]), to_s(quad[2]), to_s(quad[3])];
                        painter.add(Shape::closed_line(pts.to_vec(), Stroke::new(2.0, pal.gold)));

                        if primary_pressed
                            && let Some(p) = pointer_pos
                                && let Some((idx, _)) =
                                    pts.iter().enumerate().find(|(_, cp)| cp.distance(p) <= 14.0)
                                {
                                    state.active_corner = Some(idx);
                                }
                        if primary_down
                            && let (Some(p), Some(idx)) = (pointer_pos, state.active_corner) {
                                let uv = to_uv(p);
                                quad[idx] = uv;
                                state.draft.perspective_quad = Some(quad);
                                // Zoomed corner loupe
                                if let Some((raw_tex, _, _)) = preview_cache.raw_texture() {
                                    let loupe_r = Rect::from_center_size(
                                        p + Vec2::new(72.0, -68.0),
                                        Vec2::splat(96.0),
                                    );
                                    painter.rect_filled(loupe_r.expand(2.0), 6.0, pal.hud_glass);
                                    let hs = 0.08_f32;
                                    let u0 = (uv[0] as f32 - hs).clamp(0.0, 1.0 - hs * 2.0);
                                    let v0 = (uv[1] as f32 - hs).clamp(0.0, 1.0 - hs * 2.0);
                                    painter.image(
                                        raw_tex.id(),
                                        loupe_r,
                                        Rect::from_min_max(
                                            Pos2::new(u0, v0),
                                            Pos2::new(u0 + hs * 2.0, v0 + hs * 2.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                    painter.rect_stroke(
                                        loupe_r,
                                        6.0,
                                        Stroke::new(2.0, pal.gold),
                                        StrokeKind::Outside,
                                    );
                                }
                            }

                        for (idx, pt) in pts.iter().enumerate() {
                            painter.circle_filled(
                                *pt,
                                7.5,
                                if state.active_corner == Some(idx) {
                                    pal.gold
                                } else {
                                    Color32::WHITE
                                },
                            );
                            painter.circle_stroke(*pt, 7.5, Stroke::new(2.0, pal.accent));
                        }
                    }
                    PhotoWizardStep::CalibrateScale => {
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
                        if primary_pressed && ir.contains(pointer_pos.unwrap_or(Pos2::ZERO)) {
                            let p = pointer_pos.unwrap();
                            let s0 = state.cal_pts_uv[0].map(to_s);
                            let s1 = state.cal_pts_uv[1].map(to_s);
                            if s0.is_some_and(|pt| pt.distance(p) <= 12.0) {
                                state.active_caliper = Some(0);
                            } else if s1.is_some_and(|pt| pt.distance(p) <= 12.0) {
                                state.active_caliper = Some(1);
                            } else if state.cal_pts_uv[0].is_none() || state.cal_pts_uv[1].is_some()
                            {
                                let uv = to_uv(p);
                                state.cal_pts_uv = [Some(uv), Some(uv)];
                                state.active_caliper = Some(1);
                            } else {
                                state.cal_pts_uv[1] = Some(to_uv(p));
                            }
                        }
                        if primary_down
                            && let (Some(p), Some(idx)) = (pointer_pos, state.active_caliper) {
                                state.cal_pts_uv[idx] = Some(to_uv(p));
                            }
                        if let Some(u0) = state.cal_pts_uv[0] {
                            let s0 = to_s(u0);
                            painter.circle_filled(s0, 5.5, pal.gold);
                            if let Some(u1) = state.cal_pts_uv[1] {
                                let s1 = to_s(u1);
                                painter.line_segment([s0, s1], Stroke::new(2.4, pal.gold));
                                painter.circle_filled(s1, 5.5, pal.gold);
                            }
                        }
                    }
                }
            }
            let _ = resp;

            ui.add_space(10.0);

            // Step controls (100% mouse-driven)
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
                            }
                            if ui.button("Rotate +90").clicked() {
                                state.draft.rotation_deg =
                                    (state.draft.rotation_deg + 90.0).rem_euclid(360.0);
                            }
                            if ui.button("Reset Rotation").clicked() {
                                state.draft.rotation_deg = 0.0;
                            }
                            ui.add(
                                egui::Slider::new(&mut state.draft.rotation_deg, -180.0..=180.0)
                                    .text("Angle"),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Crop Margins:");
                            ui.add(egui::Slider::new(&mut state.draft.crop_left, 0.0..=0.45).text("L"));
                            ui.add(egui::Slider::new(&mut state.draft.crop_right, 0.0..=0.45).text("R"));
                            ui.add(egui::Slider::new(&mut state.draft.crop_top, 0.0..=0.45).text("T"));
                            ui.add(egui::Slider::new(&mut state.draft.crop_bottom, 0.0..=0.45).text("B"));
                        });
                    }
                    PhotoWizardStep::PerspectiveCorners => {
                        ui.horizontal(|ui| {
                            ui.label("Drag the 4 white/gold corner pins onto the real wall corners.");
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
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Known Target:").strong());
                            for (lbl, mm) in unit.calibration_presets() {
                                let active = (state.target_real_mm - mm).abs() < 0.25;
                                let b = egui::Button::new(
                                    RichText::new(lbl)
                                        .small()
                                        .color(if active { pal.accent_text } else { pal.text_primary }),
                                )
                                .fill(if active { pal.accent } else { pal.surface_panel });
                                if ui.add(b).clicked() {
                                    state.target_real_mm = mm;
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut disp = unit.from_mm(state.target_real_mm);
                            let min_d = unit.from_mm(10.0);
                            let max_d = unit.from_mm(600.0);
                            if ui.add(egui::Slider::new(&mut disp, min_d..=max_d).show_value(false)).changed() {
                                state.target_real_mm = unit.to_mm(disp);
                            }
                            ui.label(
                                RichText::new(unit.format_value(state.target_real_mm))
                                    .strong()
                                    .color(pal.gold),
                            );
                            ui.checkbox(
                                &mut state.auto_edit_on_canvas,
                                "Keep on-canvas photo handles open after Apply",
                            );
                        });
                    }
                });

            ui.add_space(10.0);
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
                    final_cfg.fit_to_wall(wall_w, wall_h);
                    if let (Some(u0), Some(u1)) = (state.cal_pts_uv[0], state.cal_pts_uv[1]) {
                        let dx_mm = (u1[0] - u0[0]) * final_cfg.width_mm;
                        let dy_mm = (u1[1] - u0[1]) * final_cfg.height_mm;
                        let measured_mm = dx_mm.hypot(dy_mm);
                        if measured_mm > 2.0 {
                            final_cfg.apply_two_point_scale(
                                measured_mm,
                                state.target_real_mm,
                                [0.0, 0.0],
                            );
                        }
                    }
                    applied = Some((final_cfg, state.auto_edit_on_canvas));
                    state.open = false;
                }

                if state.step != PhotoWizardStep::CalibrateScale && ui.button("Next Step ->").clicked() {
                    state.step = match state.step {
                        PhotoWizardStep::OrientAndCrop => PhotoWizardStep::PerspectiveCorners,
                        PhotoWizardStep::PerspectiveCorners => PhotoWizardStep::CalibrateScale,
                        PhotoWizardStep::CalibrateScale => PhotoWizardStep::CalibrateScale,
                    };
                }
                if state.step != PhotoWizardStep::OrientAndCrop && ui.button("<- Back").clicked() {
                    state.step = match state.step {
                        PhotoWizardStep::OrientAndCrop => PhotoWizardStep::OrientAndCrop,
                        PhotoWizardStep::PerspectiveCorners => PhotoWizardStep::OrientAndCrop,
                        PhotoWizardStep::CalibrateScale => PhotoWizardStep::PerspectiveCorners,
                    };
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
    painter.rect_filled(Rect::from_min_max(ir.left_top(), Pos2::new(ir.right(), crop_r.top())), 0.0, mask);
    painter.rect_filled(Rect::from_min_max(Pos2::new(ir.left(), crop_r.bottom()), ir.right_bottom()), 0.0, mask);
    painter.rect_filled(Rect::from_min_max(Pos2::new(ir.left(), crop_r.top()), Pos2::new(crop_r.left(), crop_r.bottom())), 0.0, mask);
    painter.rect_filled(Rect::from_min_max(Pos2::new(crop_r.right(), crop_r.top()), Pos2::new(ir.right(), crop_r.bottom())), 0.0, mask);
    painter.rect_stroke(crop_r, 1.0, Stroke::new(1.8, pal.gold), StrokeKind::Outside);
}
