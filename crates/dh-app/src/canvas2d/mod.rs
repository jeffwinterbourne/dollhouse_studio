pub mod dimensions;
pub mod openings;
pub mod photo_layer;
pub mod quick_bar;

use crate::canvas2d::dimensions::{DimensionToolState, interact_and_draw_dimensions};
use crate::canvas2d::openings::{
    OpeningsInteraction, OpeningsInteractionState, interact_and_draw_openings,
    opening_screen_rect,
};
use crate::canvas2d::photo_layer::{
    BlueprintXform, PhotoLayerState, draw_caliper_control_bar,
    interact_photo_overlay, paint_photo_underlay,
};
pub use crate::canvas2d::photo_layer::PhotoCanvasTool;
use crate::canvas2d::quick_bar::{QuickBarAction, draw_opening_quick_bar};
use crate::photo_cache::WallPhotoTextureCache;
use crate::theme::StudioPalette;
use dh_core::{ComponentDef, DisplayUnit, PieceTemplate, ProjectWall, WindowStyle};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};

#[derive(Debug, Clone)]
pub enum CanvasDragPayload {
    Style(WindowStyle),
    Component(String),
    Template(Box<PieceTemplate>),
}

#[derive(Debug, Clone)]
pub enum BlueprintAction {
    SelectOpenings(Vec<String>),
    DoubleClickOpen3D(String),
    MoveOpeningsAbsolute(Vec<(String, f64, f64)>),
    ResizeOpening {
        opening_id: String,
        x_from_left: f64,
        y_from_bottom: f64,
        width: f64,
        height: f64,
    },
    ResizeWall {
        width: f64,
        height: f64,
    },
    DropStyle {
        style: WindowStyle,
        x_mm: f64,
        y_mm: f64,
    },
    DropComponent {
        component_id: String,
        x_mm: f64,
        y_mm: f64,
    },
    DropTemplate {
        template: Box<PieceTemplate>,
        x_mm: f64,
        y_mm: f64,
    },
    QuickAction(QuickBarAction),
    OpenPhotoWizard,
    CommitUndo(&'static str),
}

#[derive(Default)]
pub struct Blueprint2DState {
    pub zoom_px_per_mm: f32,
    pub center_mm: [f64; 2],
    pub fitted_wall_id: Option<String>,
    pub edit_photo_mode: bool,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub photo: PhotoLayerState,
    pub dims: DimensionToolState,
    pub openings: OpeningsInteractionState,
    pan_start: Option<(Pos2, [f64; 2])>,
}

impl Blueprint2DState {
    pub fn new() -> Self {
        Self {
            zoom_px_per_mm: 2.2,
            center_mm: [190.0, 120.0],
            fitted_wall_id: None,
            edit_photo_mode: false,
            show_rulers: true,
            show_grid: true,
            photo: PhotoLayerState::default(),
            dims: DimensionToolState::default(),
            openings: OpeningsInteractionState::default(),
            pan_start: None,
        }
    }

    pub fn fit_wall(&mut self, wall: &ProjectWall, canvas_rect: Rect) {
        let pad_w = (canvas_rect.width() - 90.0).max(100.0);
        let pad_h = (canvas_rect.height() - 100.0).max(100.0);
        let zx = pad_w / wall.width.max(40.0) as f32;
        let zy = pad_h / wall.height.max(40.0) as f32;
        self.zoom_px_per_mm = zx.min(zy).clamp(0.4, 16.0);
        self.center_mm = [wall.width * 0.5, wall.height * 0.5];
        self.fitted_wall_id = Some(wall.id.clone());
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_blueprint_2d(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    wall: &mut ProjectWall,
    components: &[ComponentDef],
    selected_ids: &[String],
    unit: DisplayUnit,
    photo_cache: &WallPhotoTextureCache,
    state: &mut Blueprint2DState,
) -> Vec<BlueprintAction> {
    let mut actions = Vec::new();
    let full_rect = ui.available_rect_before_wrap();
    if full_rect.width() < 20.0 || full_rect.height() < 20.0 {
        return actions;
    }

    // Reserve optional 24px engineering rulers on Top and Left edges.
    let ruler_thick = if state.show_rulers { 22.0 } else { 0.0 };
    let canvas_rect = Rect::from_min_max(
        Pos2::new(full_rect.left() + ruler_thick, full_rect.top() + ruler_thick),
        full_rect.right_bottom(),
    );

    if state.fitted_wall_id.as_deref() != Some(wall.id.as_str()) {
        state.fit_wall(wall, canvas_rect);
    }

    let resp = ui.allocate_rect(full_rect, Sense::click_and_drag());
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let middle_down = ui.input(|i| i.pointer.middle_down());
    let right_down = ui.input(|i| i.pointer.secondary_down());
    let space_down = ui.input(|i| i.key_down(egui::Key::Space));
    let primary_down = ui.input(|i| i.pointer.primary_down());

    let mut xform = make_xform(canvas_rect, state);

    // Zoom toward cursor on mouse wheel inside canvas.
    let canvas_hovered = resp.hovered();
    if canvas_hovered && canvas_rect.contains(pointer_pos.unwrap_or(Pos2::ZERO)) {
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() > 0.5 {
            let factor = (1.0 + scroll_y * 0.0018).clamp(0.7, 1.45);
            if let Some(p) = pointer_pos {
                let before = xform.screen_to_mm(p);
                state.zoom_px_per_mm = (state.zoom_px_per_mm * factor).clamp(0.3, 24.0);
                xform = make_xform(canvas_rect, state);
                let after = xform.screen_to_mm(p);
                state.center_mm[0] += before[0] - after[0];
                state.center_mm[1] += before[1] - after[1];
                xform = make_xform(canvas_rect, state);
            }
        }
    }

    // Pan with Middle button, Space+Drag, or Right-button drag outside dimension pills.
    let pan_active = (middle_down || (space_down && primary_down) || right_down)
        && !state.photo.is_dragging()
        && !state.openings.is_dragging()
        && !state.dims.is_busy();
    if pan_active {
        if let Some(p) = pointer_pos {
            if let Some((start_p, start_center)) = state.pan_start {
                let d = p - start_p;
                state.center_mm[0] = start_center[0] - (d.x / state.zoom_px_per_mm) as f64;
                state.center_mm[1] = start_center[1] + (d.y / state.zoom_px_per_mm) as f64;
                xform = make_xform(canvas_rect, state);
            } else if canvas_rect.contains(p) {
                state.pan_start = Some((p, state.center_mm));
            }
        }
    } else {
        state.pan_start = None;
    }

    let painter = ui.painter_at(canvas_rect);
    painter.rect_filled(canvas_rect, 0.0, pal.canvas_bg);

    // Paint calibrated wall photo UNDER the grid & wall cutout sheet.
    paint_photo_underlay(
        &painter,
        xform,
        &wall.photo,
        photo_cache,
        state.photo.tool,
        state.edit_photo_mode,
    );

    if state.show_grid {
        draw_unit_grid(&painter, pal, xform, unit);
    }

    // Wall perimeter rectangle.
    let wall_rect = Rect::from_min_max(
        xform.mm_to_screen([0.0, wall.height]),
        xform.mm_to_screen([wall.width, 0.0]),
    );
    let wall_fill = if wall.photo.has_photo() && wall.photo.show_in_blueprint {
        pal.wall_fill_over_photo
    } else {
        pal.wall_fill
    };
    painter.rect_filled(wall_rect, 2.0, wall_fill);
    painter.rect_stroke(
        wall_rect,
        2.0,
        Stroke::new(2.0, pal.wall_stroke),
        StrokeKind::Outside,
    );

    // Wall title & overall formatted dimensions above top-left corner.
    painter.text(
        Pos2::new(wall_rect.left(), wall_rect.top() - 8.0),
        Align2::LEFT_BOTTOM,
        format!("{}  ({})", wall.name, unit.format_dims(wall.width, wall.height)),
        FontId::proportional(12.5),
        pal.wall_stroke,
    );

    // Drag-and-drop drop target from Explorer / Design gallery!
    if let Some(payload) = egui::DragAndDrop::payload::<CanvasDragPayload>(ui.ctx())
        && let Some(pos) = pointer_pos
            && canvas_rect.contains(pos) {
                let mm = xform.screen_to_mm(pos);
                let (gw, gh, label) = match &*payload {
                    CanvasDragPayload::Style(s) => {
                        let mut p = dh_core::WindowParams::default();
                        s.apply_style_defaults(&mut p);
                        (p.opening_width, p.opening_height, s.name().to_string())
                    }
                    CanvasDragPayload::Component(cid) => {
                        if let Some(c) = components.iter().find(|c| &c.id == cid) {
                            (c.params.opening_width, c.params.opening_height, c.name.clone())
                        } else {
                            (60.0, 90.0, "Component".into())
                        }
                    }
                    CanvasDragPayload::Template(t) => {
                        (t.params.opening_width, t.params.opening_height, t.name.clone())
                    }
                };
                let gx = (mm[0] - gw * 0.5).clamp(0.0, (wall.width - gw).max(0.0));
                let gy = (mm[1] - gh * 0.5).clamp(0.0, (wall.height - gh).max(0.0));
                let ghost = Rect::from_min_max(
                    xform.mm_to_screen([gx, gy + gh]),
                    xform.mm_to_screen([gx + gw, gy]),
                );
                painter.rect_filled(ghost, 2.0, pal.accent.linear_multiply(0.22));
                painter.rect_stroke(ghost, 2.0, Stroke::new(2.0, pal.gold), StrokeKind::Outside);
                painter.text(
                    ghost.center(),
                    Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(11.5),
                    pal.gold,
                );

                if ui.input(|i| i.pointer.primary_released()) {
                    match &*payload {
                        CanvasDragPayload::Style(s) => actions.push(BlueprintAction::DropStyle {
                            style: *s,
                            x_mm: gx,
                            y_mm: gy,
                        }),
                        CanvasDragPayload::Component(cid) => {
                            actions.push(BlueprintAction::DropComponent {
                                component_id: cid.clone(),
                                x_mm: gx,
                                y_mm: gy,
                            });
                        }
                        CanvasDragPayload::Template(tpl) => {
                            actions.push(BlueprintAction::DropTemplate {
                                template: tpl.clone(),
                                x_mm: gx,
                                y_mm: gy,
                            });
                        }
                    }
                    egui::DragAndDrop::clear_payload(ui.ctx());
                }
            }

    // Mode routing: Photo Editing vs Annotate Dimensions vs Wall & Openings Editing.
    let mut consumed_by_photo = false;
    if state.edit_photo_mode && wall.photo.has_photo() {
        let mut p_out = interact_photo_overlay(
            ui,
            &painter,
            pal,
            xform,
            wall.width,
            wall.height,
            unit,
            &mut wall.photo,
            &mut state.photo,
            photo_cache,
            canvas_hovered,
        );
        draw_caliper_control_bar(
            ui,
            pal,
            xform,
            unit,
            &mut wall.photo,
            &mut state.photo,
            &mut p_out,
        );
        if p_out.changed && p_out.commit_label.is_none() {
            // Live drag visual feedback happens directly on `wall.photo`.
        }
        if let Some(lbl) = p_out.commit_label {
            actions.push(BlueprintAction::CommitUndo(lbl));
        }
        if p_out.request_open_wizard {
            actions.push(BlueprintAction::OpenPhotoWizard);
        }
        if p_out.exit_photo_mode {
            state.edit_photo_mode = false;
            state.photo.fast_preview = false;
        }
        consumed_by_photo = p_out.consumed_pointer;
    }

    let dim_allow = !state.edit_photo_mode && !consumed_by_photo;
    let d_out = interact_and_draw_dimensions(
        ui,
        &painter,
        pal,
        xform,
        wall,
        components,
        unit,
        &mut state.dims,
        dim_allow,
        canvas_hovered,
    );
    if let Some(lbl) = d_out.commit_label {
        actions.push(BlueprintAction::CommitUndo(lbl));
    }

    let allow_openings = !state.edit_photo_mode && !state.dims.active_draw_mode && !d_out.consumed_pointer;
    let op_events = interact_and_draw_openings(
        ui,
        &painter,
        pal,
        xform,
        wall,
        components,
        selected_ids,
        unit,
        &mut state.openings,
        allow_openings,
        canvas_hovered,
    );

    let mut cur_sel = selected_ids.to_vec();
    for ev in op_events {
        match ev {
            OpeningsInteraction::SelectSingle(id) => {
                cur_sel = vec![id];
                actions.push(BlueprintAction::SelectOpenings(cur_sel.clone()));
            }
            OpeningsInteraction::ToggleSelect(id) => {
                if let Some(pos) = cur_sel.iter().position(|x| x == &id) {
                    cur_sel.remove(pos);
                } else {
                    cur_sel.push(id);
                }
                actions.push(BlueprintAction::SelectOpenings(cur_sel.clone()));
            }
            OpeningsInteraction::SetSelection(ids) => {
                cur_sel = ids;
                actions.push(BlueprintAction::SelectOpenings(cur_sel.clone()));
            }
            OpeningsInteraction::ClearSelection => {
                cur_sel.clear();
                actions.push(BlueprintAction::SelectOpenings(Vec::new()));
            }
            OpeningsInteraction::DoubleClickOpen3D(id) => {
                actions.push(BlueprintAction::DoubleClickOpen3D(id));
            }
            OpeningsInteraction::MoveOpeningsAbsolute(moves) => {
                actions.push(BlueprintAction::MoveOpeningsAbsolute(moves));
            }
            OpeningsInteraction::ResizeOpening {
                opening_id,
                x_from_left,
                y_from_bottom,
                width,
                height,
            } => {
                actions.push(BlueprintAction::ResizeOpening {
                    opening_id,
                    x_from_left,
                    y_from_bottom,
                    width,
                    height,
                });
            }
            OpeningsInteraction::ResizeWall { width, height } => {
                actions.push(BlueprintAction::ResizeWall { width, height });
            }
            OpeningsInteraction::FinishedDrag(lbl) => {
                actions.push(BlueprintAction::CommitUndo(lbl));
            }
        }
    }

    // Floating Quick-Action Bar near selected opening(s) (when not editing photo/dimensions).
    if allow_openings && !selected_ids.is_empty() && !state.openings.is_dragging() {
        let mut bbox: Option<Rect> = None;
        let mut is_comp = false;
        let mut any_locked = false;
        for op in &wall.openings {
            if selected_ids.contains(&op.id) {
                let r = opening_screen_rect(xform, op, components);
                bbox = Some(bbox.map_or(r, |b| b.union(r)));
                is_comp = op.is_component();
                any_locked = op.lock_position || op.lock_width || op.lock_height;
            }
        }
        if let Some(sel_rect) = bbox
            && let Some(qa) = draw_opening_quick_bar(
                ui,
                pal,
                canvas_rect,
                sel_rect,
                selected_ids.len(),
                is_comp,
                any_locked,
            ) {
                actions.push(BlueprintAction::QuickAction(qa));
            }
    }

    // Paint engineering rulers along top & left margins.
    if state.show_rulers {
        draw_rulers(ui, pal, full_rect, canvas_rect, xform, unit);
    }

    // Bottom-right blueprint HUD badge ("Fit View", scale/zoom info, coordinate readout).
    draw_bottom_right_hud(ui, pal, canvas_rect, xform, wall, unit, pointer_pos, state);

    let _ = resp;
    let _ = Color32::TRANSPARENT;
    actions
}

fn make_xform(canvas_rect: Rect, state: &Blueprint2DState) -> BlueprintXform {
    let center = canvas_rect.center();
    let px = state.zoom_px_per_mm;
    let origin_screen = Pos2::new(
        center.x - state.center_mm[0] as f32 * px,
        center.y + state.center_mm[1] as f32 * px,
    );
    BlueprintXform {
        canvas_rect,
        origin_screen,
        px_per_mm: px,
    }
}

fn draw_unit_grid(
    painter: &egui::Painter,
    pal: &StudioPalette,
    xform: BlueprintXform,
    unit: DisplayUnit,
) {
    let min_mm = xform.screen_to_mm(xform.canvas_rect.left_bottom());
    let max_mm = xform.screen_to_mm(xform.canvas_rect.right_top());

    let major = unit.ruler_major_step_mm();
    let minor = unit.ruler_minor_step_mm();
    let show_minor = minor as f32 * xform.px_per_mm >= 10.0;

    if show_minor {
        let sx0 = (min_mm[0] / minor).floor() as i32;
        let sx1 = (max_mm[0] / minor).ceil() as i32;
        for i in sx0..=sx1 {
            let x = xform.mm_to_screen([i as f64 * minor, 0.0]).x;
            painter.line_segment(
                [Pos2::new(x, xform.canvas_rect.top()), Pos2::new(x, xform.canvas_rect.bottom())],
                Stroke::new(1.0, pal.canvas_grid_minor),
            );
        }
        let sy0 = (min_mm[1] / minor).floor() as i32;
        let sy1 = (max_mm[1] / minor).ceil() as i32;
        for j in sy0..=sy1 {
            let y = xform.mm_to_screen([0.0, j as f64 * minor]).y;
            painter.line_segment(
                [Pos2::new(xform.canvas_rect.left(), y), Pos2::new(xform.canvas_rect.right(), y)],
                Stroke::new(1.0, pal.canvas_grid_minor),
            );
        }
    }

    let mx0 = (min_mm[0] / major).floor() as i32;
    let mx1 = (max_mm[0] / major).ceil() as i32;
    for i in mx0..=mx1 {
        let x = xform.mm_to_screen([i as f64 * major, 0.0]).x;
        painter.line_segment(
            [Pos2::new(x, xform.canvas_rect.top()), Pos2::new(x, xform.canvas_rect.bottom())],
            Stroke::new(1.0, pal.canvas_grid_major),
        );
    }
    let my0 = (min_mm[1] / major).floor() as i32;
    let my1 = (max_mm[1] / major).ceil() as i32;
    for j in my0..=my1 {
        let y = xform.mm_to_screen([0.0, j as f64 * major]).y;
        painter.line_segment(
            [Pos2::new(xform.canvas_rect.left(), y), Pos2::new(xform.canvas_rect.right(), y)],
            Stroke::new(1.0, pal.canvas_grid_major),
        );
    }
}

fn draw_rulers(
    ui: &egui::Ui,
    pal: &StudioPalette,
    full_rect: Rect,
    canvas_rect: Rect,
    xform: BlueprintXform,
    unit: DisplayUnit,
) {
    let rp = ui.painter_at(full_rect);
    let top_rect = Rect::from_min_max(
        Pos2::new(canvas_rect.left(), full_rect.top()),
        Pos2::new(full_rect.right(), canvas_rect.top()),
    );
    let left_rect = Rect::from_min_max(
        Pos2::new(full_rect.left(), canvas_rect.top()),
        Pos2::new(canvas_rect.left(), full_rect.bottom()),
    );
    let corner_rect = Rect::from_min_max(full_rect.left_top(), canvas_rect.left_top());

    rp.rect_filled(top_rect, 0.0, pal.surface_panel);
    rp.rect_filled(left_rect, 0.0, pal.surface_panel);
    rp.rect_filled(corner_rect, 0.0, pal.surface_ribbon);
    rp.text(
        corner_rect.center(),
        Align2::CENTER_CENTER,
        unit.short_label(),
        FontId::proportional(10.0),
        pal.text_muted,
    );

    let min_mm = xform.screen_to_mm(canvas_rect.left_bottom());
    let max_mm = xform.screen_to_mm(canvas_rect.right_top());
    let major = unit.ruler_major_step_mm();

    let mx0 = (min_mm[0] / major).floor() as i32;
    let mx1 = (max_mm[0] / major).ceil() as i32;
    for i in mx0..=mx1 {
        let v_mm = i as f64 * major;
        let sx = xform.mm_to_screen([v_mm, 0.0]).x;
        if sx >= canvas_rect.left() && sx <= canvas_rect.right() {
            rp.line_segment(
                [Pos2::new(sx, top_rect.bottom() - 6.0), Pos2::new(sx, top_rect.bottom())],
                Stroke::new(1.0, pal.border_strong),
            );
            let txt = format!("{:.0}", unit.from_mm(v_mm));
            rp.text(
                Pos2::new(sx + 3.0, top_rect.top() + 2.0),
                Align2::LEFT_TOP,
                txt,
                FontId::proportional(10.0),
                pal.text_secondary,
            );
        }
    }

    let my0 = (min_mm[1] / major).floor() as i32;
    let my1 = (max_mm[1] / major).ceil() as i32;
    for j in my0..=my1 {
        let v_mm = j as f64 * major;
        let sy = xform.mm_to_screen([0.0, v_mm]).y;
        if sy >= canvas_rect.top() && sy <= canvas_rect.bottom() {
            rp.line_segment(
                [Pos2::new(left_rect.right() - 6.0, sy), Pos2::new(left_rect.right(), sy)],
                Stroke::new(1.0, pal.border_strong),
            );
            let txt = format!("{:.0}", unit.from_mm(v_mm));
            rp.text(
                Pos2::new(left_rect.left() + 2.0, sy - 1.0),
                Align2::LEFT_BOTTOM,
                txt,
                FontId::proportional(10.0),
                pal.text_secondary,
            );
        }
    }

    rp.line_segment(
        [top_rect.left_bottom(), top_rect.right_bottom()],
        Stroke::new(1.0, pal.border),
    );
    rp.line_segment(
        [left_rect.right_top(), left_rect.right_bottom()],
        Stroke::new(1.0, pal.border),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_bottom_right_hud(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    canvas_rect: Rect,
    xform: BlueprintXform,
    wall: &ProjectWall,
    unit: DisplayUnit,
    pointer_pos: Option<Pos2>,
    state: &mut Blueprint2DState,
) {
    let hud_pos = Pos2::new(canvas_rect.right() - 295.0, canvas_rect.bottom() - 36.0);
    let area_id = ui.scope_id().with("blueprint_bottom_hud");
    egui::Area::new(area_id)
        .fixed_pos(hud_pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(pal.hud_glass)
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(eframe::egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let cursor_str = if let Some(p) = pointer_pos {
                            if canvas_rect.contains(p) {
                                let mm = xform.screen_to_mm(p);
                                format!(
                                    "X {}  Y {}",
                                    unit.format_value(mm[0]),
                                    unit.format_value(mm[1])
                                )
                            } else {
                                "Cursor outside".into()
                            }
                        } else {
                            "X -  Y -".into()
                        };
                        ui.label(RichText::new(cursor_str).small().color(pal.text_secondary));
                        if ui.small_button("Fit Wall").clicked() {
                            state.fit_wall(wall, canvas_rect);
                        }
                    });
                });
        });
}
