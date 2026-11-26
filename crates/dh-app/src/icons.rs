use crate::theme::StudioPalette;
use dh_core::WindowStyle;
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub enum VectorIcon {
    Window,
    Door,
    Wall,
    ComponentLink,
    Template,
    Photo,
    Perspective,
    Crop,
    Ruler,
    ExportStl,
    ExportPdf,
    Save,
    Folder,
    Plus,
}

/// Renders a crisp 16x16 programmatic vector icon inside `rect`.
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: VectorIcon, col: Color32) {
    let s = Stroke::new(1.4, col);
    let r = rect.shrink(2.0);
    match icon {
        VectorIcon::Window => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            painter.line_segment([r.left_center(), r.right_center()], s);
        }
        VectorIcon::Door => {
            let dr = Rect::from_center_size(r.center(), Vec2::new(r.width() * 0.72, r.height()));
            painter.rect_stroke(dr, 1.0, s, egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(dr.right() - 2.5, dr.center().y), 1.3, col);
        }
        VectorIcon::Wall => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.line_segment([r.left_center(), r.right_center()], s);
            painter.line_segment(
                [
                    Pos2::new(r.center().x, r.top()),
                    Pos2::new(r.center().x, r.center().y),
                ],
                s,
            );
        }
        VectorIcon::ComponentLink => {
            let a = Rect::from_min_size(r.min, r.size() * 0.62);
            let b = Rect::from_min_size(r.min + r.size() * 0.38, r.size() * 0.62);
            painter.rect_stroke(a, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(b, 1.0, s, egui::StrokeKind::Inside);
        }
        VectorIcon::Template => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            painter.rect_filled(
                Rect::from_min_size(r.min + Vec2::new(2.0, 2.0), Vec2::new(r.width() - 4.0, 3.0)),
                0.5,
                col,
            );
        }
        VectorIcon::Photo => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            let pts = vec![
                Pos2::new(r.left() + 1.5, r.bottom() - 2.0),
                Pos2::new(r.center().x - 1.5, r.center().y),
                Pos2::new(r.center().x + 1.5, r.bottom() - 3.0),
                Pos2::new(r.right() - 3.5, r.center().y - 1.5),
                Pos2::new(r.right() - 1.5, r.bottom() - 2.0),
            ];
            painter.add(Shape::line(pts, s));
        }
        VectorIcon::Perspective => {
            let pts = vec![
                Pos2::new(r.left() + 2.0, r.top() + 1.0),
                Pos2::new(r.right() - 0.5, r.top() + 2.5),
                Pos2::new(r.right() - 2.0, r.bottom() - 1.0),
                Pos2::new(r.left() + 0.5, r.bottom() - 2.0),
            ];
            painter.add(Shape::closed_line(pts, s));
        }
        VectorIcon::Crop => {
            painter.line_segment(
                [
                    Pos2::new(r.left(), r.top() + 3.0),
                    Pos2::new(r.right() - 3.0, r.top() + 3.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + 3.0, r.top()),
                    Pos2::new(r.left() + 3.0, r.bottom() - 3.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(r.right() - 3.0, r.top() + 3.0),
                    Pos2::new(r.right() - 3.0, r.bottom()),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + 3.0, r.bottom() - 3.0),
                    Pos2::new(r.right(), r.bottom() - 3.0),
                ],
                s,
            );
        }
        VectorIcon::Ruler => {
            painter.line_segment([r.left_center(), r.right_center()], s);
            painter.line_segment(
                [
                    Pos2::new(r.left(), r.top() + 2.0),
                    Pos2::new(r.left(), r.bottom() - 2.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(r.right(), r.top() + 2.0),
                    Pos2::new(r.right(), r.bottom() - 2.0),
                ],
                s,
            );
        }
        VectorIcon::ExportStl => {
            let c = r.center();
            let rx = r.width() * 0.45;
            let ry = r.height() * 0.45;
            let hex = vec![
                Pos2::new(c.x, c.y - ry),
                Pos2::new(c.x + rx, c.y - ry * 0.45),
                Pos2::new(c.x + rx, c.y + ry * 0.45),
                Pos2::new(c.x, c.y + ry),
                Pos2::new(c.x - rx, c.y + ry * 0.45),
                Pos2::new(c.x - rx, c.y - ry * 0.45),
            ];
            painter.add(Shape::closed_line(hex, s));
        }
        VectorIcon::ExportPdf => {
            let pr = Rect::from_center_size(r.center(), Vec2::new(r.width() * 0.8, r.height()));
            painter.rect_stroke(pr, 1.0, s, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(pr.left() + 2.5, pr.center().y - 2.0),
                    Pos2::new(pr.right() - 2.5, pr.center().y - 2.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(pr.left() + 2.5, pr.center().y + 2.0),
                    Pos2::new(pr.right() - 2.5, pr.center().y + 2.0),
                ],
                s,
            );
        }
        VectorIcon::Save => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(r.left() + 3.0, r.top() + 1.0),
                    Vec2::new(r.width() - 6.0, 4.0),
                ),
                0.5,
                col,
            );
        }
        VectorIcon::Folder => {
            painter.rect_stroke(
                Rect::from_min_max(Pos2::new(r.left(), r.top() + 2.5), r.right_bottom()),
                1.5,
                s,
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.top() + 2.5),
                    Pos2::new(r.center().x, r.top() + 0.5),
                ],
                s,
            );
        }
        VectorIcon::Plus => {
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            painter.line_segment([r.left_center(), r.right_center()], s);
        }
    }
}

/// Draws a crisp architectural vector preview of a `WindowStyle` inside `rect` adapted to Light/Dark theme.
pub fn draw_style_thumbnail(
    painter: &egui::Painter,
    rect: Rect,
    style: WindowStyle,
    selected: bool,
    pal: &StudioPalette,
) {
    let frame_col = if pal.dark_mode {
        if selected {
            Color32::from_rgb(245, 240, 230)
        } else {
            Color32::from_rgb(214, 210, 204)
        }
    } else if selected {
        Color32::from_rgb(28, 42, 66)
    } else {
        Color32::from_rgb(66, 78, 96)
    };
    let glass_col = if pal.dark_mode {
        Color32::from_rgba_unmultiplied(110, 175, 215, 72)
    } else {
        Color32::from_rgba_unmultiplied(85, 160, 220, 75)
    };
    let outline_col = if selected {
        pal.accent
    } else {
        pal.border_strong
    };
    let bar_stroke = Stroke::new(1.4, frame_col);
    let heavy_stroke = Stroke::new(2.3, frame_col);
    let border_stroke = Stroke::new(1.3, outline_col);

    let inner = rect.shrink(5.0);

    match style {
        WindowStyle::CircularBullsEye => {
            let r = inner.width().min(inner.height()) * 0.45;
            let c = inner.center();
            painter.circle_filled(c, r, glass_col);
            painter.circle_stroke(c, r, Stroke::new(3.6, frame_col));
            painter.circle_stroke(c, r + 1.8, border_stroke);
            let ir = r - 2.0;
            for i in 0..4 {
                let a = (i as f32) * PI / 4.0;
                let d = Vec2::new(a.cos() * ir, a.sin() * ir);
                painter.line_segment([c - d, c + d], bar_stroke);
            }
            painter.circle_stroke(c, ir * 0.42, bar_stroke);
        }
        WindowStyle::ArchedGeorgian => {
            let w = inner.width() * 0.68;
            let h = inner.height() * 0.88;
            let cx = inner.center().x;
            let top = inner.center().y - h * 0.5;
            let bot = inner.center().y + h * 0.5;
            let r = w * 0.5;
            let y_spring = top + r;
            let mut pts = vec![Pos2::new(cx - r, bot), Pos2::new(cx + r, bot)];
            for i in 0..=18 {
                let t = (i as f32 / 18.0) * PI;
                pts.push(Pos2::new(cx + r * t.cos(), y_spring - r * t.sin()));
            }
            painter.add(Shape::convex_polygon(pts.clone(), glass_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, Stroke::new(2.8, frame_col)));
            painter.line_segment(
                [Pos2::new(cx - r, y_spring), Pos2::new(cx + r, y_spring)],
                bar_stroke,
            );
            for col in 1..=2 {
                let x = cx - r + w * (col as f32 / 3.0);
                painter.line_segment([Pos2::new(x, y_spring), Pos2::new(x, bot)], bar_stroke);
            }
            for i in 1..=3 {
                let t = (i as f32 / 4.0) * PI;
                painter.line_segment(
                    [
                        Pos2::new(cx, y_spring),
                        Pos2::new(cx + r * t.cos(), y_spring - r * t.sin()),
                    ],
                    bar_stroke,
                );
            }
            draw_sill(painter, cx, bot, w, frame_col);
        }
        WindowStyle::GothicPointed => {
            let w = inner.width() * 0.64;
            let h = inner.height() * 0.90;
            let cx = inner.center().x;
            let top = inner.center().y - h * 0.5;
            let bot = inner.center().y + h * 0.5;
            let hw = w * 0.5;
            let y_spring = top + w * 0.72;
            let pts = vec![
                Pos2::new(cx - hw, bot),
                Pos2::new(cx + hw, bot),
                Pos2::new(cx + hw, y_spring),
                Pos2::new(cx, top),
                Pos2::new(cx - hw, y_spring),
            ];
            painter.add(Shape::convex_polygon(pts.clone(), glass_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, Stroke::new(2.8, frame_col)));
            painter.line_segment([Pos2::new(cx, top), Pos2::new(cx, bot)], bar_stroke);
            painter.line_segment(
                [Pos2::new(cx - hw, y_spring), Pos2::new(cx + hw, y_spring)],
                bar_stroke,
            );
            draw_sill(painter, cx, bot, w, frame_col);
        }
        WindowStyle::HalfRoundLunette => {
            let w = inner.width() * 0.88;
            let h = inner.height() * 0.48;
            let cx = inner.center().x;
            let bot = inner.center().y + h * 0.45;
            let rx = w * 0.5;
            let mut pts = vec![Pos2::new(cx - rx, bot), Pos2::new(cx + rx, bot)];
            for i in 0..=18 {
                let t = (i as f32 / 18.0) * PI;
                pts.push(Pos2::new(cx + rx * t.cos(), bot - h * t.sin()));
            }
            painter.add(Shape::convex_polygon(pts.clone(), glass_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, Stroke::new(2.8, frame_col)));
            for i in 1..=3 {
                let t = (i as f32 / 4.0) * PI;
                painter.line_segment(
                    [
                        Pos2::new(cx, bot),
                        Pos2::new(cx + rx * t.cos(), bot - h * t.sin()),
                    ],
                    bar_stroke,
                );
            }
            draw_sill(painter, cx, bot, w, frame_col);
        }
        WindowStyle::DoorColonial4Panel
        | WindowStyle::DoorGeorgian6Panel
        | WindowStyle::DoorGlazedFrench
        | WindowStyle::DoorArchedHeritage
        | WindowStyle::DoorFanlightGeorgian => {
            let w = inner.width() * 0.62;
            let h = inner.height() * 0.90;
            let door_rect = Rect::from_center_size(inner.center(), Vec2::new(w, h));
            let panel_col = Color32::from_rgb(165, 132, 96);
            let groove_col = Color32::from_rgb(95, 72, 48);
            painter.rect_filled(door_rect, 1.0, panel_col);
            if style == WindowStyle::DoorGlazedFrench {
                let tg = Rect::from_min_max(
                    Pos2::new(door_rect.left() + 4.0, door_rect.top() + 4.0),
                    Pos2::new(door_rect.right() - 4.0, door_rect.center().y + 2.0),
                );
                painter.rect_filled(tg, 0.5, glass_col);
                painter.rect_stroke(tg, 0.5, bar_stroke, egui::StrokeKind::Middle);
            } else if style == WindowStyle::DoorFanlightGeorgian {
                let fg = Rect::from_min_max(
                    Pos2::new(door_rect.left() + 4.0, door_rect.top() + 4.0),
                    Pos2::new(door_rect.right() - 4.0, door_rect.top() + h * 0.34),
                );
                painter.rect_filled(fg, 4.0, glass_col);
                painter.rect_stroke(fg, 4.0, bar_stroke, egui::StrokeKind::Middle);
            } else {
                let rows = if style == WindowStyle::DoorGeorgian6Panel {
                    3
                } else {
                    2
                };
                let pw = (w - 10.0) * 0.5;
                let ph = (h - 8.0 - (rows as f32 - 1.0) * 3.0) / rows as f32;
                for r in 0..rows {
                    let y = door_rect.top() + 4.0 + r as f32 * (ph + 3.0);
                    for c in 0..2 {
                        let x = door_rect.left() + 4.0 + c as f32 * (pw + 2.0);
                        painter.rect_stroke(
                            Rect::from_min_size(Pos2::new(x, y), Vec2::new(pw, ph)),
                            0.5,
                            Stroke::new(1.0, groove_col),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
            painter.rect_stroke(
                door_rect,
                1.0,
                Stroke::new(2.5, frame_col),
                egui::StrokeKind::Middle,
            );
            painter.circle_filled(
                Pos2::new(door_rect.right() - 4.5, door_rect.center().y + 2.0),
                1.6,
                Color32::from_rgb(230, 190, 95),
            );
        }
        _ => {
            let aspect = if matches!(
                style,
                WindowStyle::CasementSplit
                    | WindowStyle::PictureFixed
                    | WindowStyle::PalladianVenetian
            ) {
                0.86
            } else {
                0.68
            };
            let w = inner.width() * aspect;
            let h = inner.height() * 0.82;
            let wr = Rect::from_center_size(inner.center() - Vec2::new(0.0, 1.5), Vec2::new(w, h));
            painter.rect_filled(wr, 1.0, glass_col);

            match style {
                WindowStyle::GeorgianGrid => {
                    for c in 1..=2 {
                        let x = wr.left() + w * (c as f32 / 3.0);
                        painter.line_segment(
                            [Pos2::new(x, wr.top()), Pos2::new(x, wr.bottom())],
                            bar_stroke,
                        );
                    }
                    for r in 1..=3 {
                        let y = wr.top() + h * (r as f32 / 4.0);
                        painter.line_segment(
                            [Pos2::new(wr.left(), y), Pos2::new(wr.right(), y)],
                            bar_stroke,
                        );
                    }
                }
                WindowStyle::SashSplit => {
                    let ym = wr.center().y;
                    for c in 1..=2 {
                        let x = wr.left() + w * (c as f32 / 3.0);
                        painter.line_segment(
                            [Pos2::new(x, wr.top()), Pos2::new(x, wr.bottom())],
                            bar_stroke,
                        );
                    }
                    painter.line_segment(
                        [Pos2::new(wr.left(), ym), Pos2::new(wr.right(), ym)],
                        heavy_stroke,
                    );
                }
                WindowStyle::CasementSplit => {
                    let yt = wr.top() + h * 0.28;
                    painter.line_segment(
                        [Pos2::new(wr.left(), yt), Pos2::new(wr.right(), yt)],
                        heavy_stroke,
                    );
                    for c in 1..=2 {
                        let x = wr.left() + w * (c as f32 / 3.0);
                        painter.line_segment(
                            [Pos2::new(x, wr.top()), Pos2::new(x, wr.bottom())],
                            heavy_stroke,
                        );
                    }
                }
                WindowStyle::CottageLattice => {
                    let clipped = painter.with_clip_rect(wr);
                    for k in -6..=6 {
                        let off = k as f32 * 7.0;
                        clipped.line_segment(
                            [
                                Pos2::new(wr.left() + off - 16.0, wr.top() - 16.0),
                                Pos2::new(wr.left() + off + 48.0, wr.top() + 48.0),
                            ],
                            bar_stroke,
                        );
                        clipped.line_segment(
                            [
                                Pos2::new(wr.right() - off + 16.0, wr.top() - 16.0),
                                Pos2::new(wr.right() - off - 48.0, wr.top() + 48.0),
                            ],
                            bar_stroke,
                        );
                    }
                }
                WindowStyle::CraftsmanPrairie => {
                    let yt = wr.top() + h * 0.28;
                    let xl = wr.left() + w * 0.22;
                    let xr = wr.right() - w * 0.22;
                    painter.line_segment(
                        [Pos2::new(wr.left(), yt), Pos2::new(wr.right(), yt)],
                        heavy_stroke,
                    );
                    painter.line_segment(
                        [Pos2::new(xl, wr.top()), Pos2::new(xl, wr.bottom())],
                        bar_stroke,
                    );
                    painter.line_segment(
                        [Pos2::new(xr, wr.top()), Pos2::new(xr, wr.bottom())],
                        bar_stroke,
                    );
                }
                WindowStyle::PalladianVenetian => {
                    let xl = wr.center().x - w * 0.22;
                    let xr = wr.center().x + w * 0.22;
                    let yt = wr.top() + h * 0.34;
                    painter.line_segment(
                        [Pos2::new(xl, wr.top()), Pos2::new(xl, wr.bottom())],
                        heavy_stroke,
                    );
                    painter.line_segment(
                        [Pos2::new(xr, wr.top()), Pos2::new(xr, wr.bottom())],
                        heavy_stroke,
                    );
                    painter.line_segment(
                        [Pos2::new(wr.left(), yt), Pos2::new(wr.right(), yt)],
                        heavy_stroke,
                    );
                }
                _ => {}
            }
            painter.rect_stroke(
                wr,
                1.0,
                Stroke::new(2.8, frame_col),
                egui::StrokeKind::Middle,
            );
            painter.rect_stroke(
                wr.expand(1.5),
                1.5,
                border_stroke,
                egui::StrokeKind::Outside,
            );
            draw_sill(painter, wr.center().x, wr.bottom() + 1.5, w, frame_col);
        }
    }
}

fn draw_sill(painter: &egui::Painter, cx: f32, y_bot: f32, w: f32, col: Color32) {
    let sill_rect = Rect::from_center_size(Pos2::new(cx, y_bot + 1.5), Vec2::new(w + 8.0, 3.2));
    painter.rect_filled(sill_rect, 0.8, col);
}
