use crate::photo_cache::WallPhotoTextureCache;
use crate::theme::StudioPalette;
use dh_core::{DisplayUnit, WallPhotoConfig};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Shape, Stroke, StrokeKind,
    Vec2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhotoCanvasTool {
    #[default]
    MoveStretch,
    Rotate,
    PerspectivePins,
    Crop,
    TwoPointCalibrate,
}

impl PhotoCanvasTool {
    pub fn label(self) -> &'static str {
        match self {
            PhotoCanvasTool::MoveStretch => "Move & Stretch",
            PhotoCanvasTool::Rotate => "Rotate",
            PhotoCanvasTool::PerspectivePins => "Perspective Pins",
            PhotoCanvasTool::Crop => "Crop",
            PhotoCanvasTool::TwoPointCalibrate => "2-Pt Calibrate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivePhotoHandle {
    MovePhoto,
    /// `0..=7`: NW, N, NE, E, SE, S, SW, W
    ResizeEdge(usize),
    RotateKnob,
    /// `0..=3`: TL, TR, BR, BL in normalized image UV `[0,1]^2`
    PerspCorner(usize),
    /// `0..=3`: Left, Top, Right, Bottom
    CropSide(usize),
    /// `0` or `1`
    CaliperEndpoint(usize),
    DraggingNewCaliperB,
}

#[derive(Debug, Clone)]
pub struct PhotoLayerState {
    pub tool: PhotoCanvasTool,
    active_handle: Option<ActivePhotoHandle>,
    drag_start_photo: Option<WallPhotoConfig>,
    drag_start_mm: [f64; 2],
    pub caliper_pts_mm: [Option<[f64; 2]>; 2],
    pub caliper_target_mm: f64,
    pub anchor_origin_on_apply: bool,
    pub fast_preview: bool,
}

impl Default for PhotoLayerState {
    fn default() -> Self {
        Self {
            tool: PhotoCanvasTool::MoveStretch,
            active_handle: None,
            drag_start_photo: None,
            drag_start_mm: [0.0, 0.0],
            caliper_pts_mm: [None, None],
            caliper_target_mm: 178.0,
            anchor_origin_on_apply: true,
            fast_preview: false,
        }
    }
}

impl PhotoLayerState {
    pub fn reset_caliper(&mut self) {
        self.caliper_pts_mm = [None, None];
    }

    pub fn is_dragging(&self) -> bool {
        self.active_handle.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlueprintXform {
    pub canvas_rect: Rect,
    pub origin_screen: Pos2,
    pub px_per_mm: f32,
}

impl BlueprintXform {
    pub fn mm_to_screen(self, pt: [f64; 2]) -> Pos2 {
        Pos2::new(
            self.origin_screen.x + pt[0] as f32 * self.px_per_mm,
            self.origin_screen.y - pt[1] as f32 * self.px_per_mm,
        )
    }

    pub fn screen_to_mm(self, pos: Pos2) -> [f64; 2] {
        [
            ((pos.x - self.origin_screen.x) / self.px_per_mm) as f64,
            ((self.origin_screen.y - pos.y) / self.px_per_mm) as f64,
        ]
    }

    pub fn photo_screen_rect(self, cfg: &WallPhotoConfig) -> Rect {
        let min_s = self.mm_to_screen([cfg.offset_x_mm, cfg.offset_y_mm + cfg.height_mm]);
        let max_s = self.mm_to_screen([cfg.offset_x_mm + cfg.width_mm, cfg.offset_y_mm]);
        Rect::from_min_max(min_s, max_s)
    }
}

#[derive(Default)]
pub struct PhotoLayerOutput {
    pub changed: bool,
    pub commit_label: Option<&'static str>,
    pub consumed_pointer: bool,
    pub request_open_wizard: bool,
    pub exit_photo_mode: bool,
}

/// Renders the calibrated underlay image beneath the blueprint wall grid and handles.
pub fn paint_photo_underlay(
    painter: &egui::Painter,
    xform: BlueprintXform,
    cfg: &WallPhotoConfig,
    cache: &WallPhotoTextureCache,
    tool: PhotoCanvasTool,
    active: bool,
) {
    if !cfg.has_photo() || (!cfg.show_in_blueprint && !active) {
        return;
    }
    let rect = xform.photo_screen_rect(cfg);
    let alpha = ((cfg.opacity.clamp(0.05, 1.0)) * 255.0).round() as u8;
    let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha);

    // When directly picking 4 perspective pins on-canvas, show the oriented texture (respecting rotation)
    // underneath so corners do not shift beneath the pointer while adjusting pins.
    if active
        && tool == PhotoCanvasTool::PerspectivePins
        && let Some((orient_tex, _, _)) = cache.oriented_texture()
    {
        painter.image(
            orient_tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
        return;
    }

    if let Some((_, proc_tex)) = cache.loaded_egui_texture() {
        painter.image(
            proc_tex.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    }
}

/// Renders interactive mouse-only overlays for photo Move/Stretch, Rotate, Perspective Pins, Crop, and 2-Point Caliper.
#[allow(clippy::too_many_arguments)]
pub fn interact_photo_overlay(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall_w: f64,
    wall_h: f64,
    unit: DisplayUnit,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    cache: &WallPhotoTextureCache,
    canvas_hovered: bool,
) -> PhotoLayerOutput {
    let mut out = PhotoLayerOutput::default();
    if !cfg.has_photo() {
        return out;
    }

    let photo_rect = xform.photo_screen_rect(cfg);
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let primary_down = ui.input(|i| i.pointer.primary_down());
    let primary_pressed = canvas_hovered && ui.input(|i| i.pointer.primary_pressed());
    let primary_released = ui.input(|i| i.pointer.primary_released());
    let alt_held = ui.input(|i| i.modifiers.alt);
    let shift_held = ui.input(|i| i.modifiers.shift);

    // Floating quick-tool & action HUD anchored near top of viewport.
    draw_photo_hud_bar(ui, pal, xform, wall_w, wall_h, unit, cfg, state, &mut out);

    // Release active handle drag.
    if primary_released && state.active_handle.is_some() {
        let finished = state.active_handle.take();
        state.fast_preview = false;
        state.drag_start_photo = None;
        if let Some(h) = finished {
            out.commit_label = Some(match h {
                ActivePhotoHandle::MovePhoto => "Move wall photo",
                ActivePhotoHandle::ResizeEdge(_) => "Stretch wall photo",
                ActivePhotoHandle::RotateKnob => "Rotate wall photo",
                ActivePhotoHandle::PerspCorner(_) => "Adjust perspective corners",
                ActivePhotoHandle::CropSide(_) => "Crop wall photo",
                ActivePhotoHandle::CaliperEndpoint(_) | ActivePhotoHandle::DraggingNewCaliperB => {
                    ""
                }
            });
            if out.commit_label == Some("") {
                out.commit_label = None;
            }
            out.consumed_pointer = true;
        }
    }

    match state.tool {
        PhotoCanvasTool::MoveStretch => {
            interact_move_stretch(
                painter,
                pal,
                xform,
                photo_rect,
                wall_w,
                wall_h,
                unit,
                cfg,
                state,
                pointer_pos,
                primary_down,
                primary_pressed,
                alt_held,
                shift_held,
                &mut out,
            );
        }
        PhotoCanvasTool::Rotate => {
            interact_rotate(
                painter,
                pal,
                photo_rect,
                cfg,
                state,
                pointer_pos,
                primary_down,
                primary_pressed,
                shift_held,
                &mut out,
            );
        }
        PhotoCanvasTool::PerspectivePins => {
            interact_perspective_pins(
                ui.ctx(),
                painter,
                pal,
                photo_rect,
                cfg,
                state,
                cache,
                pointer_pos,
                primary_down,
                primary_pressed,
                &mut out,
            );
        }
        PhotoCanvasTool::Crop => {
            interact_crop(
                painter,
                pal,
                photo_rect,
                cfg,
                state,
                pointer_pos,
                primary_down,
                primary_pressed,
                &mut out,
            );
        }
        PhotoCanvasTool::TwoPointCalibrate => {
            interact_caliper(
                ui.ctx(),
                painter,
                pal,
                xform,
                photo_rect,
                unit,
                state,
                cache,
                pointer_pos,
                primary_down,
                primary_pressed,
                &mut out,
            );
        }
    }

    out
}

#[allow(clippy::too_many_arguments)]
fn draw_photo_hud_bar(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    xform: BlueprintXform,
    wall_w: f64,
    wall_h: f64,
    unit: DisplayUnit,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    out: &mut PhotoLayerOutput,
) {
    let hud_pos = Pos2::new(
        xform.canvas_rect.center().x - 280.0,
        xform.canvas_rect.top() + 34.0,
    );
    let hud_id = ui.scope_id().with("photo_canvas_hud");
    egui::Area::new(hud_id)
        .fixed_pos(hud_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(pal.hud_glass)
                .stroke(Stroke::new(1.0, pal.border_strong))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for tool in [
                            PhotoCanvasTool::MoveStretch,
                            PhotoCanvasTool::Rotate,
                            PhotoCanvasTool::PerspectivePins,
                            PhotoCanvasTool::Crop,
                            PhotoCanvasTool::TwoPointCalibrate,
                        ] {
                            let active = state.tool == tool;
                            let btn = egui::Button::new(
                                RichText::new(tool.label())
                                    .size(12.2)
                                    .strong()
                                    .color(if active { pal.accent_text } else { pal.text_primary }),
                            )
                            .fill(if active { pal.accent } else { pal.surface_card })
                            .stroke(Stroke::new(1.0, if active { pal.accent } else { pal.border }))
                            .corner_radius(CornerRadius::same(5));
                            if ui.add(btn).clicked() {
                                state.tool = tool;
                                state.active_handle = None;
                                state.fast_preview = false;
                            }
                        }

                        ui.separator();

                        if ui.button("Fit Wall").on_hover_text("Fit photo inside wall boundaries").clicked() {
                            cfg.fit_to_wall(wall_w, wall_h);
                            out.changed = true;
                            out.commit_label = Some("Fit photo to wall");
                        }
                        if ui.button("Wizard...").on_hover_text("Open guided Photo Setup Wizard").clicked() {
                            out.request_open_wizard = true;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Done Photo").strong().color(pal.accent_text))
                                    .fill(pal.success),
                            )
                            .clicked()
                        {
                            out.exit_photo_mode = true;
                        }
                    });

                    // Secondary contextual sub-strip per active tool (mouse-only pills & scrubbers).
                    ui.add_space(3.0);
                    ui.horizontal(|ui| match state.tool {
                        PhotoCanvasTool::MoveStretch => {
                            let mut lock = cfg.lock_aspect;
                            if ui.checkbox(&mut lock, "Lock Aspect (Hold Alt for Free Stretch)").changed() {
                                cfg.lock_aspect = lock;
                                out.changed = true;
                            }
                            if ui.small_button("Stretch Exact").clicked() {
                                cfg.match_wall_exact(wall_w, wall_h);
                                out.changed = true;
                                out.commit_label = Some("Stretch photo to match wall");
                            }
                            ui.label(
                                RichText::new(format!("Size: {}", unit.format_dims(cfg.width_mm, cfg.height_mm)))
                                    .small()
                                    .color(pal.text_secondary),
                            );
                        }
                        PhotoCanvasTool::Rotate => {
                            if ui.small_button("-90 deg").clicked() {
                                cfg.rotation_deg = (cfg.rotation_deg - 90.0).rem_euclid(360.0);
                                out.changed = true;
                                out.commit_label = Some("Rotate wall photo");
                            }
                            if ui.small_button("+90 deg").clicked() {
                                cfg.rotation_deg = (cfg.rotation_deg + 90.0).rem_euclid(360.0);
                                out.changed = true;
                                out.commit_label = Some("Rotate wall photo");
                            }
                            if ui.small_button("0 deg").clicked() {
                                cfg.rotation_deg = 0.0;
                                out.changed = true;
                                out.commit_label = Some("Reset photo rotation");
                            }
                            ui.label(
                                RichText::new(format!("Angle: {:.1} deg (drag compass ring)", cfg.rotation_deg))
                                    .small()
                                    .color(pal.text_secondary),
                            );
                        }
                        PhotoCanvasTool::PerspectivePins => {
                            if ui.small_button("Rectify & Fit").clicked() {
                                cfg.fit_to_wall(wall_w, wall_h);
                                state.tool = PhotoCanvasTool::MoveStretch;
                                out.changed = true;
                                out.commit_label = Some("Rectify and fit photo");
                            }
                            if ui.small_button("Reset Corners").clicked() {
                                cfg.perspective_quad = None;
                                cfg.perspective_h = 0.0;
                                cfg.perspective_v = 0.0;
                                out.changed = true;
                                out.commit_label = Some("Reset perspective corners");
                            }
                            ui.label(
                                RichText::new("Drag the 4 corner pins onto real wall corners (magnifier follows cursor)")
                                    .small()
                                    .color(pal.text_secondary),
                            );
                        }
                        PhotoCanvasTool::Crop => {
                            if ui.small_button("Reset Crop").clicked() {
                                cfg.crop_left = 0.0;
                                cfg.crop_right = 0.0;
                                cfg.crop_top = 0.0;
                                cfg.crop_bottom = 0.0;
                                out.changed = true;
                                out.commit_label = Some("Reset photo crop");
                            }
                            ui.label(
                                RichText::new("Drag the L-brackets on any edge to crop margins")
                                    .small()
                                    .color(pal.text_secondary),
                            );
                        }
                        PhotoCanvasTool::TwoPointCalibrate => {
                            ui.label(
                                RichText::new(
                                    "Click/drag Point A to Point B on any known span, then pick a preset or slide scale below",
                                )
                                .small()
                                .color(pal.text_secondary),
                            );
                        }
                    });
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn interact_move_stretch(
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    photo_rect: Rect,
    wall_w: f64,
    wall_h: f64,
    unit: DisplayUnit,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    pointer_pos: Option<Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    alt_held: bool,
    shift_held: bool,
    out: &mut PhotoLayerOutput,
) {
    let border_col = pal.accent;
    painter.rect_stroke(
        photo_rect,
        2.0,
        Stroke::new(2.0, border_col),
        StrokeKind::Outside,
    );

    let handle_pts = [
        photo_rect.left_top(),
        photo_rect.center_top(),
        photo_rect.right_top(),
        photo_rect.right_center(),
        photo_rect.right_bottom(),
        photo_rect.center_bottom(),
        photo_rect.left_bottom(),
        photo_rect.left_center(),
    ];

    if primary_pressed && let Some(pos) = pointer_pos {
        if let Some((idx, _)) = handle_pts
            .iter()
            .enumerate()
            .find(|(_, pt)| pt.distance(pos) <= 10.0)
        {
            state.active_handle = Some(ActivePhotoHandle::ResizeEdge(idx));
            state.drag_start_photo = Some(cfg.clone());
            state.drag_start_mm = xform.screen_to_mm(pos);
            out.consumed_pointer = true;
        } else if photo_rect.contains(pos) {
            state.active_handle = Some(ActivePhotoHandle::MovePhoto);
            state.drag_start_photo = Some(cfg.clone());
            state.drag_start_mm = xform.screen_to_mm(pos);
            out.consumed_pointer = true;
        }
    }

    if primary_down
        && let (Some(pos), Some(handle), Some(start)) = (
            pointer_pos,
            state.active_handle,
            state.drag_start_photo.as_ref(),
        )
    {
        let cur_mm = xform.screen_to_mm(pos);
        let dx = cur_mm[0] - state.drag_start_mm[0];
        let dy = cur_mm[1] - state.drag_start_mm[1];
        match handle {
            ActivePhotoHandle::MovePhoto => {
                let mut nx = start.offset_x_mm + dx;
                let mut ny = start.offset_y_mm + dy;
                if !shift_held {
                    let snap = 4.0;
                    if nx.abs() < snap {
                        nx = 0.0;
                    } else if ((nx + start.width_mm) - wall_w).abs() < snap {
                        nx = wall_w - start.width_mm;
                    }
                    if ny.abs() < snap {
                        ny = 0.0;
                    } else if ((ny + start.height_mm) - wall_h).abs() < snap {
                        ny = wall_h - start.height_mm;
                    }
                }
                cfg.offset_x_mm = nx;
                cfg.offset_y_mm = ny;
                out.changed = true;
                out.consumed_pointer = true;
            }
            ActivePhotoHandle::ResizeEdge(idx) => {
                let mut left = start.offset_x_mm;
                let mut bottom = start.offset_y_mm;
                let mut right = start.offset_x_mm + start.width_mm;
                let mut top = start.offset_y_mm + start.height_mm;
                let aspect = (start.width_mm / start.height_mm.max(1.0)).max(0.05);
                let is_corner = matches!(idx, 0 | 2 | 4 | 6);
                let keep_aspect = is_corner && cfg.lock_aspect && !alt_held;

                match idx {
                    0 => {
                        left = (left + dx).min(right - 10.0);
                        top = (top + dy).max(bottom + 10.0);
                    }
                    1 => top = (top + dy).max(bottom + 10.0),
                    2 => {
                        right = (right + dx).max(left + 10.0);
                        top = (top + dy).max(bottom + 10.0);
                    }
                    3 => right = (right + dx).max(left + 10.0),
                    4 => {
                        right = (right + dx).max(left + 10.0);
                        bottom = (bottom + dy).min(top - 10.0);
                    }
                    5 => bottom = (bottom + dy).min(top - 10.0),
                    6 => {
                        left = (left + dx).min(right - 10.0);
                        bottom = (bottom + dy).min(top - 10.0);
                    }
                    7 => left = (left + dx).min(right - 10.0),
                    _ => {}
                }

                if keep_aspect {
                    let w = (right - left).max(10.0);
                    let h_from_w = (w / aspect).max(10.0);
                    match idx {
                        0 | 2 => top = bottom + h_from_w,
                        4 | 6 => bottom = top - h_from_w,
                        _ => {}
                    }
                }

                cfg.offset_x_mm = left;
                cfg.offset_y_mm = bottom;
                cfg.width_mm = (right - left).max(10.0);
                cfg.height_mm = (top - bottom).max(10.0);
                out.changed = true;
                out.consumed_pointer = true;
            }
            _ => {}
        }
    }

    // Paint the 8 handles cleanly.
    for (idx, pt) in handle_pts.iter().enumerate() {
        let hovered = pointer_pos.is_some_and(|p| p.distance(*pt) <= 10.0);
        let fill = if hovered { pal.gold } else { Color32::WHITE };
        let r = if idx % 2 == 0 { 5.5 } else { 4.5 };
        painter.circle_filled(*pt, r, fill);
        painter.circle_stroke(*pt, r, Stroke::new(1.5, border_col));
    }

    // Size readout badge near bottom-right corner.
    let label = unit.format_dims(cfg.width_mm, cfg.height_mm);
    painter.text(
        Pos2::new(photo_rect.right() - 6.0, photo_rect.bottom() - 6.0),
        Align2::RIGHT_BOTTOM,
        label,
        FontId::proportional(11.5),
        pal.accent_text,
    );
}

#[allow(clippy::too_many_arguments)]
fn interact_rotate(
    painter: &egui::Painter,
    pal: &StudioPalette,
    photo_rect: Rect,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    pointer_pos: Option<Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    shift_held: bool,
    out: &mut PhotoLayerOutput,
) {
    let center = photo_rect.center();
    let radius = (photo_rect.width().min(photo_rect.height()) * 0.42).clamp(48.0, 150.0);
    let rad = (cfg.rotation_deg as f32 - 90.0).to_radians();
    let knob = center + Vec2::new(rad.cos(), rad.sin()) * radius;

    painter.circle_stroke(center, radius, Stroke::new(1.8, pal.accent));
    // Alignment crosshair through center.
    painter.line_segment(
        [
            Pos2::new(photo_rect.left(), center.y),
            Pos2::new(photo_rect.right(), center.y),
        ],
        Stroke::new(1.0, pal.accent.linear_multiply(0.5)),
    );
    painter.line_segment(
        [
            Pos2::new(center.x, photo_rect.top()),
            Pos2::new(center.x, photo_rect.bottom()),
        ],
        Stroke::new(1.0, pal.accent.linear_multiply(0.5)),
    );
    painter.line_segment([center, knob], Stroke::new(2.0, pal.gold));

    let hovered = pointer_pos.is_some_and(|p| p.distance(knob) <= 13.0);
    if primary_pressed && hovered {
        state.active_handle = Some(ActivePhotoHandle::RotateKnob);
        state.drag_start_photo = Some(cfg.clone());
        state.fast_preview = true;
        out.consumed_pointer = true;
    }

    if primary_down
        && state.active_handle == Some(ActivePhotoHandle::RotateKnob)
        && let Some(pos) = pointer_pos
    {
        let v = pos - center;
        let mut deg = (v.y.atan2(v.x).to_degrees() + 90.0) as f64;
        if shift_held {
            deg = (deg / 5.0).round() * 5.0;
        }
        if deg > 180.0 {
            deg -= 360.0;
        }
        cfg.rotation_deg = deg;
        out.changed = true;
        out.consumed_pointer = true;
    }

    painter.circle_filled(knob, 7.0, if hovered { pal.gold } else { Color32::WHITE });
    painter.circle_stroke(knob, 7.0, Stroke::new(2.0, pal.accent));
}

#[allow(clippy::too_many_arguments)]
fn interact_perspective_pins(
    ctx: &egui::Context,
    painter: &egui::Painter,
    pal: &StudioPalette,
    photo_rect: Rect,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    cache: &WallPhotoTextureCache,
    pointer_pos: Option<Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    out: &mut PhotoLayerOutput,
) {
    let mut quad = cfg.effective_quad();
    let uv_to_screen = |uv: [f64; 2]| -> Pos2 {
        Pos2::new(
            photo_rect.left() + uv[0] as f32 * photo_rect.width(),
            photo_rect.top() + uv[1] as f32 * photo_rect.height(),
        )
    };
    let screen_to_uv = |pos: Pos2| -> [f64; 2] {
        [
            ((pos.x - photo_rect.left()) / photo_rect.width().max(1.0)).clamp(0.0, 1.0) as f64,
            ((pos.y - photo_rect.top()) / photo_rect.height().max(1.0)).clamp(0.0, 1.0) as f64,
        ]
    };

    let pts: [Pos2; 4] = [
        uv_to_screen(quad[0]),
        uv_to_screen(quad[1]),
        uv_to_screen(quad[2]),
        uv_to_screen(quad[3]),
    ];

    // Quad outline and interior 3x3 calibration grid.
    painter.add(Shape::closed_line(pts.to_vec(), Stroke::new(2.2, pal.gold)));
    let grid_stroke = Stroke::new(1.0, pal.accent.linear_multiply(0.75));
    for step in [0.333f32, 0.667f32] {
        let top = pts[0].lerp(pts[1], step);
        let bot = pts[3].lerp(pts[2], step);
        painter.line_segment([top, bot], grid_stroke);
        let left = pts[0].lerp(pts[3], step);
        let right = pts[1].lerp(pts[2], step);
        painter.line_segment([left, right], grid_stroke);
    }

    if primary_pressed
        && let Some(pos) = pointer_pos
        && let Some((idx, _)) = pts
            .iter()
            .enumerate()
            .find(|(_, pt)| pt.distance(pos) <= 13.0)
    {
        state.active_handle = Some(ActivePhotoHandle::PerspCorner(idx));
        state.drag_start_photo = Some(cfg.clone());
        state.fast_preview = true;
        out.consumed_pointer = true;
    }

    let mut active_uv: Option<[f64; 2]> = None;
    let mut active_pos: Option<Pos2> = None;
    if primary_down
        && let (Some(pos), Some(ActivePhotoHandle::PerspCorner(idx))) =
            (pointer_pos, state.active_handle)
    {
        let uv = screen_to_uv(pos);
        quad[idx] = uv;
        cfg.perspective_quad = Some(quad);
        active_uv = Some(uv);
        active_pos = Some(pos);
        out.changed = true;
        out.consumed_pointer = true;
    }

    let labels = ["TL", "TR", "BR", "BL"];
    for (idx, pt) in pts.iter().enumerate() {
        let hovered = pointer_pos.is_some_and(|p| p.distance(*pt) <= 13.0)
            || state.active_handle == Some(ActivePhotoHandle::PerspCorner(idx));
        painter.circle_filled(*pt, 7.5, if hovered { pal.gold } else { Color32::WHITE });
        painter.circle_stroke(*pt, 7.5, Stroke::new(2.0, pal.accent));
        painter.text(
            *pt + Vec2::new(10.0, -10.0),
            Align2::LEFT_BOTTOM,
            labels[idx],
            FontId::proportional(11.0),
            pal.gold,
        );
    }

    // Zoomed Cursor Magnifier Loupe on top tooltip layer when dragging any corner pin.
    if let (Some(uv), Some(pos)) = (active_uv, active_pos)
        && let Some((orient_tex, _, _)) = cache.oriented_texture()
    {
        draw_cursor_loupe(ctx, pal, orient_tex.id(), pos, uv);
    }
}

#[allow(clippy::too_many_arguments)]
fn interact_crop(
    painter: &egui::Painter,
    pal: &StudioPalette,
    photo_rect: Rect,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    pointer_pos: Option<Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    out: &mut PhotoLayerOutput,
) {
    let cl = cfg.crop_left.clamp(0.0, 0.45) as f32;
    let cr = 1.0 - cfg.crop_right.clamp(0.0, 0.45) as f32;
    let ct = cfg.crop_top.clamp(0.0, 0.45) as f32;
    let cb = 1.0 - cfg.crop_bottom.clamp(0.0, 0.45) as f32;

    let crop_rect = Rect::from_min_max(
        Pos2::new(
            photo_rect.left() + cl * photo_rect.width(),
            photo_rect.top() + ct * photo_rect.height(),
        ),
        Pos2::new(
            photo_rect.left() + cr * photo_rect.width(),
            photo_rect.top() + cb * photo_rect.height(),
        ),
    );

    // Dimmed masks outside crop_rect.
    let mask = Color32::from_rgba_unmultiplied(10, 14, 20, 145);
    painter.rect_filled(
        Rect::from_min_max(
            photo_rect.left_top(),
            Pos2::new(photo_rect.right(), crop_rect.top()),
        ),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(photo_rect.left(), crop_rect.bottom()),
            photo_rect.right_bottom(),
        ),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(photo_rect.left(), crop_rect.top()),
            Pos2::new(crop_rect.left(), crop_rect.bottom()),
        ),
        0.0,
        mask,
    );
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(crop_rect.right(), crop_rect.top()),
            Pos2::new(photo_rect.right(), crop_rect.bottom()),
        ),
        0.0,
        mask,
    );

    painter.rect_stroke(
        crop_rect,
        0.0,
        Stroke::new(2.0, pal.gold),
        StrokeKind::Outside,
    );

    let side_handles = [
        crop_rect.left_center(),   // 0: Left
        crop_rect.center_top(),    // 1: Top
        crop_rect.right_center(),  // 2: Right
        crop_rect.center_bottom(), // 3: Bottom
    ];

    if primary_pressed
        && let Some(pos) = pointer_pos
        && let Some((idx, _)) = side_handles
            .iter()
            .enumerate()
            .find(|(_, pt)| pt.distance(pos) <= 12.0)
    {
        state.active_handle = Some(ActivePhotoHandle::CropSide(idx));
        state.fast_preview = true;
        out.consumed_pointer = true;
    }

    if primary_down
        && let (Some(pos), Some(ActivePhotoHandle::CropSide(idx))) =
            (pointer_pos, state.active_handle)
    {
        match idx {
            0 => {
                let frac = ((pos.x - photo_rect.left()) / photo_rect.width().max(1.0)) as f64;
                cfg.crop_left = frac.clamp(0.0, 0.45);
            }
            1 => {
                let frac = ((pos.y - photo_rect.top()) / photo_rect.height().max(1.0)) as f64;
                cfg.crop_top = frac.clamp(0.0, 0.45);
            }
            2 => {
                let frac = ((photo_rect.right() - pos.x) / photo_rect.width().max(1.0)) as f64;
                cfg.crop_right = frac.clamp(0.0, 0.45);
            }
            3 => {
                let frac = ((photo_rect.bottom() - pos.y) / photo_rect.height().max(1.0)) as f64;
                cfg.crop_bottom = frac.clamp(0.0, 0.45);
            }
            _ => {}
        }
        cfg.clamp_edits();
        out.changed = true;
        out.consumed_pointer = true;
    }

    for pt in side_handles {
        painter.rect_filled(Rect::from_center_size(pt, Vec2::splat(9.0)), 2.0, pal.gold);
    }
}

#[allow(clippy::too_many_arguments)]
fn interact_caliper(
    ctx: &egui::Context,
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    photo_rect: Rect,
    unit: DisplayUnit,
    state: &mut PhotoLayerState,
    cache: &WallPhotoTextureCache,
    pointer_pos: Option<Pos2>,
    primary_down: bool,
    primary_pressed: bool,
    out: &mut PhotoLayerOutput,
) {
    // Allow dragging existing A/B endpoints or placing A -> B cleanly.
    if primary_pressed
        && let Some(pos) = pointer_pos
        && xform.canvas_rect.contains(pos)
    {
        let s0 = state.caliper_pts_mm[0].map(|p| xform.mm_to_screen(p));
        let s1 = state.caliper_pts_mm[1].map(|p| xform.mm_to_screen(p));
        if s0.is_some_and(|p| p.distance(pos) <= 12.0) {
            state.active_handle = Some(ActivePhotoHandle::CaliperEndpoint(0));
            out.consumed_pointer = true;
        } else if s1.is_some_and(|p| p.distance(pos) <= 12.0) {
            state.active_handle = Some(ActivePhotoHandle::CaliperEndpoint(1));
            out.consumed_pointer = true;
        } else if state.caliper_pts_mm[0].is_none() || state.caliper_pts_mm[1].is_some() {
            let mm = xform.screen_to_mm(pos);
            state.caliper_pts_mm[0] = Some(mm);
            state.caliper_pts_mm[1] = Some(mm);
            state.active_handle = Some(ActivePhotoHandle::DraggingNewCaliperB);
            out.consumed_pointer = true;
        } else {
            state.caliper_pts_mm[1] = Some(xform.screen_to_mm(pos));
            out.consumed_pointer = true;
        }
    }

    if primary_down && let (Some(pos), Some(h)) = (pointer_pos, state.active_handle) {
        let mm = xform.screen_to_mm(pos);
        let show_loupe = match h {
            ActivePhotoHandle::CaliperEndpoint(0) => {
                state.caliper_pts_mm[0] = Some(mm);
                out.consumed_pointer = true;
                true
            }
            ActivePhotoHandle::CaliperEndpoint(1) | ActivePhotoHandle::DraggingNewCaliperB => {
                state.caliper_pts_mm[1] = Some(mm);
                out.consumed_pointer = true;
                true
            }
            _ => false,
        };
        if show_loupe
            && photo_rect.width() > 4.0
            && photo_rect.height() > 4.0
            && let Some((_, proc_tex)) = cache.loaded_egui_texture()
        {
            let uv = [
                ((pos.x - photo_rect.left()) / photo_rect.width()).clamp(0.0, 1.0) as f64,
                ((pos.y - photo_rect.top()) / photo_rect.height()).clamp(0.0, 1.0) as f64,
            ];
            draw_cursor_loupe(ctx, pal, proc_tex.id(), pos, uv);
        }
    }

    // Paint caliper line & endpoints.
    if let Some(p0) = state.caliper_pts_mm[0] {
        let s0 = xform.mm_to_screen(p0);
        painter.circle_filled(s0, 5.5, pal.gold);
        painter.circle_stroke(s0, 5.5, Stroke::new(1.5, pal.accent_text));
        painter.text(
            s0 + Vec2::new(8.0, -8.0),
            Align2::LEFT_BOTTOM,
            "A",
            FontId::proportional(11.5),
            pal.gold,
        );

        if let Some(p1) = state.caliper_pts_mm[1] {
            let s1 = xform.mm_to_screen(p1);
            painter.line_segment([s0, s1], Stroke::new(2.5, pal.gold));
            painter.circle_filled(s1, 5.5, pal.gold);
            painter.circle_stroke(s1, 5.5, Stroke::new(1.5, pal.accent_text));
            painter.text(
                s1 + Vec2::new(8.0, -8.0),
                Align2::LEFT_BOTTOM,
                "B",
                FontId::proportional(11.5),
                pal.gold,
            );

            let measured_mm = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();
            let mid = s0.lerp(s1, 0.5);
            painter.text(
                mid + Vec2::new(0.0, -12.0),
                Align2::CENTER_BOTTOM,
                format!("Measured: {}", unit.format_value(measured_mm)),
                FontId::proportional(12.0),
                pal.gold,
            );
        }
    }
}

/// Companion calibration floating bar with preset chips + visual scale slider so zero typing is ever required.
#[allow(clippy::too_many_arguments)]
pub fn draw_caliper_control_bar(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    xform: BlueprintXform,
    unit: DisplayUnit,
    cfg: &mut WallPhotoConfig,
    state: &mut PhotoLayerState,
    out: &mut PhotoLayerOutput,
) {
    if state.tool != PhotoCanvasTool::TwoPointCalibrate || !cfg.has_photo() {
        return;
    }

    let measured_mm = match (state.caliper_pts_mm[0], state.caliper_pts_mm[1]) {
        (Some(a), Some(b)) => ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt(),
        _ => 0.0,
    };

    let bar_pos = Pos2::new(
        (xform.canvas_rect.center().x - 290.0).max(xform.canvas_rect.left() + 12.0),
        (xform.canvas_rect.bottom() - 98.0).max(xform.canvas_rect.top() + 80.0),
    );
    let area_id = ui.scope_id().with("photo_caliper_apply_bar");
    egui::Area::new(area_id)
        .fixed_pos(bar_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(pal.hud_glass)
                .stroke(Stroke::new(1.0, pal.border_strong))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    // Row 1: quick preset chips in active DisplayUnit.
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Known Target:").strong().size(12.2));
                        for (label, mm) in unit.calibration_presets() {
                            let active = (state.caliper_target_mm - mm).abs() < 0.25;
                            let btn = egui::Button::new(RichText::new(label).size(11.8).color(
                                if active {
                                    pal.accent_text
                                } else {
                                    pal.text_primary
                                },
                            ))
                            .fill(if active { pal.accent } else { pal.surface_card })
                            .stroke(Stroke::new(1.0, pal.border))
                            .corner_radius(CornerRadius::same(5));
                            if ui.add(btn).clicked() {
                                state.caliper_target_mm = mm;
                            }
                        }
                    });

                    // Row 2: visual slider + micro-nudge buttons + Apply button.
                    ui.horizontal(|ui| {
                        let mut disp = unit.from_mm(state.caliper_target_mm);
                        let min_d = unit.from_mm(10.0);
                        let max_d = unit.from_mm(600.0);
                        ui.spacing_mut().slider_width = 160.0;
                        if ui
                            .add(egui::Slider::new(&mut disp, min_d..=max_d).show_value(false))
                            .changed()
                        {
                            state.caliper_target_mm = unit.to_mm(disp);
                        }
                        if ui.small_button("-5").clicked() {
                            state.caliper_target_mm = (state.caliper_target_mm - 5.0).max(5.0);
                        }
                        if ui.small_button("-1").clicked() {
                            state.caliper_target_mm = (state.caliper_target_mm - 1.0).max(5.0);
                        }
                        let mut exact_disp = unit.from_mm(state.caliper_target_mm);
                        if ui
                            .add(
                                egui::DragValue::new(&mut exact_disp)
                                    .speed(unit.drag_speed() * 0.5)
                                    .range(unit.from_mm(2.0)..=unit.from_mm(2500.0))
                                    .max_decimals(unit.decimals().max(2))
                                    .suffix(unit.suffix()),
                            )
                            .changed()
                        {
                            state.caliper_target_mm = unit.to_mm(exact_disp).max(1.0);
                        }
                        if ui.small_button("+1").clicked() {
                            state.caliper_target_mm = (state.caliper_target_mm + 1.0).min(1200.0);
                        }
                        if ui.small_button("+5").clicked() {
                            state.caliper_target_mm = (state.caliper_target_mm + 5.0).min(1200.0);
                        }

                        let ready = measured_mm > 2.0;
                        let apply_btn = egui::Button::new(
                            RichText::new("Apply Scale").strong().color(if ready {
                                pal.accent_text
                            } else {
                                pal.text_muted
                            }),
                        )
                        .fill(if ready {
                            pal.accent
                        } else {
                            pal.surface_card
                        });
                        if ui.add_enabled(ready, apply_btn).clicked() {
                            let anchor = if state.anchor_origin_on_apply {
                                [0.0, 0.0]
                            } else {
                                state.caliper_pts_mm[0].unwrap_or([0.0, 0.0])
                            };
                            cfg.apply_two_point_scale(measured_mm, state.caliper_target_mm, anchor);
                            state.reset_caliper();
                            out.changed = true;
                            out.commit_label = Some("Calibrate photo scale");
                        }
                        if ui.small_button("Clear").clicked() {
                            state.reset_caliper();
                        }
                    });
                });
        });
}

fn draw_cursor_loupe(
    ctx: &egui::Context,
    pal: &StudioPalette,
    tex_id: egui::TextureId,
    cursor_pos: Pos2,
    uv: [f64; 2],
) {
    let bounds = ctx.content_rect().shrink(10.0);
    let size = Vec2::splat(124.0);
    let mut center = cursor_pos + Vec2::new(92.0, -88.0);
    if center.x + size.x * 0.5 > bounds.right() {
        center.x = cursor_pos.x - 92.0;
    }
    if center.y - size.y * 0.5 < bounds.top() {
        center.y = cursor_pos.y + 88.0;
    }
    let mut loupe_rect = Rect::from_center_size(center, size);
    loupe_rect = loupe_rect.translate(Vec2::new(
        (bounds.left() - loupe_rect.left()).max(0.0)
            - (loupe_rect.right() - bounds.right()).max(0.0),
        (bounds.top() - loupe_rect.top()).max(0.0)
            - (loupe_rect.bottom() - bounds.bottom()).max(0.0),
    ));

    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::unique("blueprint_photo_loupe"),
    ));
    painter.rect_filled(loupe_rect.expand(3.0), 8.0, pal.hud_glass);
    let half_span = 0.06_f32;
    let u0 = (uv[0] as f32 - half_span).clamp(0.0, 1.0 - half_span * 2.0);
    let v0 = (uv[1] as f32 - half_span).clamp(0.0, 1.0 - half_span * 2.0);
    let uv_rect = Rect::from_min_max(
        Pos2::new(u0, v0),
        Pos2::new(u0 + half_span * 2.0, v0 + half_span * 2.0),
    );
    painter.image(tex_id, loupe_rect, uv_rect, Color32::WHITE);
    painter.rect_stroke(
        loupe_rect,
        6.0,
        Stroke::new(2.0, pal.gold),
        StrokeKind::Outside,
    );

    // Crosshair inside loupe aligned with exact pointer sample.
    let cx = loupe_rect.left() + ((uv[0] as f32 - u0) / (half_span * 2.0)) * loupe_rect.width();
    let cy = loupe_rect.top() + ((uv[1] as f32 - v0) / (half_span * 2.0)) * loupe_rect.height();
    let ch = Stroke::new(1.3, pal.gold);
    painter.line_segment(
        [
            Pos2::new(loupe_rect.left() + 6.0, cy),
            Pos2::new(loupe_rect.right() - 6.0, cy),
        ],
        ch,
    );
    painter.line_segment(
        [
            Pos2::new(cx, loupe_rect.top() + 6.0),
            Pos2::new(cx, loupe_rect.bottom() - 6.0),
        ],
        ch,
    );
}
