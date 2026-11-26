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
    Undo,
    Redo,
    Hub,
    Duplicate,
    AlignCenter,
    Distribute,
    Mirror,
    SnapFloor,
    Rotate,
    Caliper,
    Wizard,
    AutoDim,
    Grid,
    Architrave,
    Camera,
    Cube3D,
    Blueprint,
    Explode,
    Wireframe,
    Overhang,
    Svg,
    PackPlates,
    HelpTour,
    Trash,
    FitView,
    Scale,
    OpenExternal,
    PanelLeft,
    PanelRight,
    BrowserTree,
    GridView,
    ListView,
    ArchitraveProfile,
    Shutter,
    HeaderPediment,
    Palette,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    Lock,
}

/// Renders our miniature house + 6-pane Georgian window brand emblem inside `rect`.
pub fn paint_brand_mark(painter: &egui::Painter, rect: Rect) {
    let badge_fill = Color32::from_rgb(24, 41, 68);
    let badge_stroke = Color32::from_rgb(76, 128, 198);
    let gold = Color32::from_rgb(232, 180, 76);
    let cream = Color32::from_rgb(244, 238, 224);
    let glass = Color32::from_rgb(104, 172, 232);

    painter.rect_filled(rect, 7.0, badge_fill);
    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(1.5, badge_stroke),
        egui::StrokeKind::Inside,
    );

    let r = rect.shrink(5.0);
    let mid_x = r.center().x;
    let eave_y = r.top() + r.height() * 0.42;
    let roof = vec![
        Pos2::new(r.left(), eave_y),
        Pos2::new(mid_x, r.top()),
        Pos2::new(r.right(), eave_y),
    ];
    let body = Rect::from_min_max(
        Pos2::new(r.left() + r.width() * 0.14, eave_y - 1.0),
        Pos2::new(r.right() - r.width() * 0.14, r.bottom()),
    );
    painter.rect_filled(body, 0.5, cream);
    painter.add(Shape::convex_polygon(roof, gold, Stroke::NONE));

    let win = body.shrink2(Vec2::new(body.width() * 0.22, body.height() * 0.16));
    painter.rect_filled(win, 0.5, glass);
    painter.rect_stroke(
        win,
        0.5,
        Stroke::new(1.2, badge_fill),
        egui::StrokeKind::Outside,
    );
    painter.line_segment(
        [win.center_top(), win.center_bottom()],
        Stroke::new(1.0, cream),
    );
    painter.line_segment(
        [win.left_center(), win.right_center()],
        Stroke::new(1.0, cream),
    );
}

/// Renders a crisp programmatic vector icon inside `rect`.
pub fn paint_icon(painter: &egui::Painter, rect: Rect, icon: VectorIcon, col: Color32) {
    let pad = (rect.width().min(rect.height()) * 0.12).clamp(1.5, 3.2);
    let r = rect.shrink(pad);
    let sw = (r.width().min(r.height()) / 14.0).clamp(0.85, 1.45) * 1.45;
    let s = Stroke::new(sw, col);
    let c = r.center();
    match icon {
        VectorIcon::Window => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            painter.line_segment([r.left_center(), r.right_center()], s);
        }
        VectorIcon::Door => {
            let dr = Rect::from_center_size(c, Vec2::new(r.width() * 0.72, r.height()));
            painter.rect_stroke(dr, 1.0, s, egui::StrokeKind::Inside);
            painter.circle_filled(Pos2::new(dr.right() - 2.5, dr.center().y), 1.3, col);
        }
        VectorIcon::Wall => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.line_segment([r.left_center(), r.right_center()], s);
            painter.line_segment([Pos2::new(c.x, r.top()), Pos2::new(c.x, c.y)], s);
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
                Pos2::new(c.x - 1.5, c.y),
                Pos2::new(c.x + 1.5, r.bottom() - 3.0),
                Pos2::new(r.right() - 3.5, c.y - 1.5),
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
            painter.line_segment([Pos2::new(c.x, c.y - 2.5), Pos2::new(c.x, c.y + 2.5)], s);
        }
        VectorIcon::ExportStl => {
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
            let pr = Rect::from_center_size(c, Vec2::new(r.width() * 0.8, r.height()));
            painter.rect_stroke(pr, 1.0, s, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    Pos2::new(pr.left() + 2.5, c.y - 2.0),
                    Pos2::new(pr.right() - 2.5, c.y - 2.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(pr.left() + 2.5, c.y + 2.0),
                    Pos2::new(pr.right() - 2.5, c.y + 2.0),
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
                    Pos2::new(c.x, r.top() + 0.5),
                ],
                s,
            );
        }
        VectorIcon::Plus => {
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            painter.line_segment([r.left_center(), r.right_center()], s);
        }
        VectorIcon::Undo => {
            let arc = (0..=8)
                .map(|i| {
                    let a = PI * 1.15 - (i as f32 / 8.0) * PI * 1.15;
                    c + Vec2::new(a.cos(), -a.sin()) * (r.width() * 0.36)
                })
                .collect();
            painter.add(Shape::line(arc, s));
            let tip = Pos2::new(r.left() + 2.0, c.y - 1.5);
            painter.line_segment([tip, tip + Vec2::new(3.5, -2.5)], s);
            painter.line_segment([tip, tip + Vec2::new(3.5, 2.5)], s);
        }
        VectorIcon::Redo => {
            let arc = (0..=8)
                .map(|i| {
                    let a = PI * 0.15 + (i as f32 / 8.0) * PI * 0.95;
                    c + Vec2::new(a.cos(), -a.sin()) * (r.width() * 0.36)
                })
                .collect();
            painter.add(Shape::line(arc, s));
            let tip = Pos2::new(r.right() - 2.0, c.y - 1.5);
            painter.line_segment([tip, tip + Vec2::new(-3.5, -2.5)], s);
            painter.line_segment([tip, tip + Vec2::new(-3.5, 2.5)], s);
        }
        VectorIcon::Hub => {
            let sq = r.width().min(r.height()) * 0.38;
            for dx in [-sq * 0.65, sq * 0.65] {
                for dy in [-sq * 0.65, sq * 0.65] {
                    painter.rect_stroke(
                        Rect::from_center_size(c + Vec2::new(dx, dy), Vec2::splat(sq)),
                        1.0,
                        s,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        VectorIcon::Duplicate => {
            let a = Rect::from_min_size(r.min, r.size() * 0.68);
            let b = Rect::from_min_size(r.min + r.size() * 0.32, r.size() * 0.68);
            painter.rect_stroke(a, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(b, 1.0, s, egui::StrokeKind::Inside);
        }
        VectorIcon::AlignCenter => {
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            painter.rect_stroke(
                Rect::from_center_size(c + Vec2::new(0.0, -3.0), Vec2::new(r.width() * 0.8, 3.5)),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(c + Vec2::new(0.0, 3.0), Vec2::new(r.width() * 0.55, 3.5)),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::Distribute => {
            painter.line_segment([r.left_top(), r.left_bottom()], s);
            painter.line_segment([r.right_top(), r.right_bottom()], s);
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(4.0, r.height() * 0.65)),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::Mirror => {
            painter.line_segment([r.center_top(), r.center_bottom()], s);
            let l = vec![
                Pos2::new(c.x - 2.0, r.top() + 2.0),
                Pos2::new(r.left() + 1.0, c.y),
                Pos2::new(c.x - 2.0, r.bottom() - 2.0),
            ];
            let rt = vec![
                Pos2::new(c.x + 2.0, r.top() + 2.0),
                Pos2::new(r.right() - 1.0, c.y),
                Pos2::new(c.x + 2.0, r.bottom() - 2.0),
            ];
            painter.add(Shape::closed_line(l, s));
            painter.add(Shape::closed_line(rt, s));
        }
        VectorIcon::SnapFloor => {
            painter.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.8, col));
            painter.line_segment([r.center_top(), Pos2::new(c.x, r.bottom() - 2.5)], s);
            painter.line_segment(
                [
                    Pos2::new(c.x - 3.0, r.bottom() - 5.5),
                    Pos2::new(c.x, r.bottom() - 2.5),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + 3.0, r.bottom() - 5.5),
                    Pos2::new(c.x, r.bottom() - 2.5),
                ],
                s,
            );
        }
        VectorIcon::Rotate => {
            let rad = r.width().min(r.height()) * 0.42;
            let pts: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let a = -0.5 + (i as f32 / 10.0) * 1.55 * PI;
                    c + Vec2::new(a.cos(), a.sin()) * rad
                })
                .collect();
            painter.add(Shape::line(pts, s));
            let tip = c + Vec2::new(rad, 0.0);
            painter.line_segment([tip, tip + Vec2::new(-3.0, -3.0)], s);
            painter.line_segment([tip, tip + Vec2::new(2.5, -3.0)], s);
        }
        VectorIcon::Caliper => {
            painter.line_segment([r.left_bottom(), r.right_top()], s);
            painter.circle_filled(Pos2::new(r.left() + 2.0, r.bottom() - 2.0), 2.0, col);
            painter.circle_filled(Pos2::new(r.right() - 2.0, r.top() + 2.0), 2.0, col);
        }
        VectorIcon::Wizard => {
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.bottom() - 1.0),
                    Pos2::new(r.right() - 3.0, r.top() + 3.0),
                ],
                s,
            );
            let sp = Pos2::new(r.right() - 2.5, r.top() + 2.5);
            painter.line_segment([sp + Vec2::new(-3.0, 0.0), sp + Vec2::new(3.0, 0.0)], s);
            painter.line_segment([sp + Vec2::new(0.0, -3.0), sp + Vec2::new(0.0, 3.0)], s);
        }
        VectorIcon::AutoDim => {
            painter.rect_stroke(r.shrink(2.0), 0.5, s, egui::StrokeKind::Inside);
            painter.line_segment([r.left_top(), r.right_top()], Stroke::new(1.1, col));
            painter.line_segment([r.left_top(), r.left_top() + Vec2::new(0.0, 3.0)], s);
            painter.line_segment([r.right_top(), r.right_top() + Vec2::new(0.0, 3.0)], s);
        }
        VectorIcon::Grid => {
            for t in [0.25, 0.5, 0.75] {
                let x = r.left() + r.width() * t;
                let y = r.top() + r.height() * t;
                painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], s);
                painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], s);
            }
        }
        VectorIcon::Architrave => {
            painter.rect_stroke(r, 1.0, Stroke::new(1.1, col), egui::StrokeKind::Inside);
            painter.rect_stroke(r.shrink(3.2), 0.5, s, egui::StrokeKind::Inside);
        }
        VectorIcon::Camera => {
            let body = Rect::from_center_size(
                c + Vec2::new(0.0, 1.0),
                Vec2::new(r.width(), r.height() * 0.68),
            );
            painter.rect_stroke(body, 1.5, s, egui::StrokeKind::Inside);
            painter.circle_stroke(body.center(), 2.5, s);
        }
        VectorIcon::Cube3D => {
            let rx = r.width() * 0.44;
            let ry = r.height() * 0.44;
            let pts = vec![
                Pos2::new(c.x, c.y - ry),
                Pos2::new(c.x + rx, c.y - ry * 0.45),
                Pos2::new(c.x + rx, c.y + ry * 0.45),
                Pos2::new(c.x, c.y + ry),
                Pos2::new(c.x - rx, c.y + ry * 0.45),
                Pos2::new(c.x - rx, c.y - ry * 0.45),
            ];
            painter.add(Shape::closed_line(pts, s));
            painter.line_segment([Pos2::new(c.x, c.y), Pos2::new(c.x, c.y + ry)], s);
            painter.line_segment(
                [Pos2::new(c.x, c.y), Pos2::new(c.x - rx, c.y - ry * 0.45)],
                s,
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y), Pos2::new(c.x + rx, c.y - ry * 0.45)],
                s,
            );
        }
        VectorIcon::Blueprint => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(
                Rect::from_center_size(c, r.size() * 0.45),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::Explode => {
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.top() + 1.0),
                    Pos2::new(r.left() + 1.0, r.bottom() - 1.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x, r.top() + 2.5),
                    Pos2::new(c.x, r.bottom() - 2.5),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(r.right() - 1.0, r.top() + 1.0),
                    Pos2::new(r.right() - 1.0, r.bottom() - 1.0),
                ],
                s,
            );
        }
        VectorIcon::Wireframe => {
            painter.rect_stroke(r, 0.5, s, egui::StrokeKind::Inside);
            painter.line_segment([r.left_top(), r.right_bottom()], s);
            painter.line_segment([r.right_top(), r.left_bottom()], s);
        }
        VectorIcon::Overhang => {
            let tri = vec![r.center_top(), r.right_bottom(), r.left_bottom()];
            painter.add(Shape::closed_line(tri, s));
            painter.circle_filled(Pos2::new(c.x, r.bottom() - 2.2), 1.0, col);
        }
        VectorIcon::Svg => {
            painter.circle_stroke(r.left_bottom() + Vec2::new(2.0, -2.0), 1.8, s);
            painter.circle_stroke(r.right_top() + Vec2::new(-2.0, 2.0), 1.8, s);
            painter.line_segment(
                [
                    r.left_bottom() + Vec2::new(3.0, -3.0),
                    r.right_top() + Vec2::new(-3.0, 3.0),
                ],
                s,
            );
        }
        VectorIcon::PackPlates => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(
                Rect::from_min_size(r.min + Vec2::splat(2.0), Vec2::splat(4.0)),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_size(r.min + Vec2::new(7.0, 2.0), Vec2::new(4.0, 6.0)),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::HelpTour => {
            painter.circle_stroke(c, r.width().min(r.height()) * 0.46, s);
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional((r.height() * 0.78).clamp(8.5, 16.0)),
                col,
            );
        }
        VectorIcon::Trash => {
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.top() + 3.0),
                    Pos2::new(r.right() - 1.0, r.top() + 3.0),
                ],
                s,
            );
            let body = Rect::from_min_max(
                Pos2::new(r.left() + 2.5, r.top() + 3.0),
                Pos2::new(r.right() - 2.5, r.bottom()),
            );
            painter.rect_stroke(body, 1.0, s, egui::StrokeKind::Inside);
        }
        VectorIcon::FitView => {
            let d = 3.5;
            painter.line_segment([r.left_top(), r.left_top() + Vec2::new(d, 0.0)], s);
            painter.line_segment([r.left_top(), r.left_top() + Vec2::new(0.0, d)], s);
            painter.line_segment([r.right_top(), r.right_top() + Vec2::new(-d, 0.0)], s);
            painter.line_segment([r.right_top(), r.right_top() + Vec2::new(0.0, d)], s);
            painter.line_segment([r.left_bottom(), r.left_bottom() + Vec2::new(d, 0.0)], s);
            painter.line_segment([r.left_bottom(), r.left_bottom() + Vec2::new(0.0, -d)], s);
            painter.line_segment([r.right_bottom(), r.right_bottom() + Vec2::new(-d, 0.0)], s);
            painter.line_segment([r.right_bottom(), r.right_bottom() + Vec2::new(0.0, -d)], s);
        }
        VectorIcon::Scale => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(
                Rect::from_min_max(Pos2::new(r.left(), c.y), Pos2::new(c.x, r.bottom())),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::OpenExternal => {
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(r.left(), r.top() + 3.0),
                    Pos2::new(r.right() - 3.0, r.bottom()),
                ),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([c, r.right_top()], s);
            painter.line_segment([r.right_top(), r.right_top() + Vec2::new(-4.0, 0.0)], s);
            painter.line_segment([r.right_top(), r.right_top() + Vec2::new(0.0, 4.0)], s);
        }
        VectorIcon::PanelLeft => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            let split = r.left() + r.width() * 0.36;
            painter.rect_filled(
                Rect::from_min_max(r.left_top(), Pos2::new(split, r.bottom())),
                1.0,
                col.gamma_multiply(0.32),
            );
            painter.line_segment([Pos2::new(split, r.top()), Pos2::new(split, r.bottom())], s);
        }
        VectorIcon::PanelRight => {
            painter.rect_stroke(r, 1.5, s, egui::StrokeKind::Inside);
            let split = r.right() - r.width() * 0.36;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(split, r.top()), r.right_bottom()),
                1.0,
                col.gamma_multiply(0.32),
            );
            painter.line_segment([Pos2::new(split, r.top()), Pos2::new(split, r.bottom())], s);
        }
        VectorIcon::BrowserTree => {
            let x_stem = r.left() + r.width() * 0.22;
            painter.rect_filled(
                Rect::from_min_size(r.left_top(), Vec2::new(r.width() * 0.44, r.height() * 0.24)),
                1.0,
                col,
            );
            painter.line_segment(
                [
                    Pos2::new(x_stem, r.top() + r.height() * 0.24),
                    Pos2::new(x_stem, r.bottom() - r.height() * 0.14),
                ],
                s,
            );
            for fy in [0.54, 0.86] {
                let y = r.top() + r.height() * fy;
                painter.line_segment([Pos2::new(x_stem, y), Pos2::new(r.right(), y)], s);
            }
        }
        VectorIcon::GridView => {
            let cw = r.width() * 0.42;
            let ch = r.height() * 0.42;
            for (fx, fy) in [(0.0, 0.0), (0.58, 0.0), (0.0, 0.58), (0.58, 0.58)] {
                painter.rect_stroke(
                    Rect::from_min_size(
                        Pos2::new(r.left() + r.width() * fx, r.top() + r.height() * fy),
                        Vec2::new(cw, ch),
                    ),
                    1.0,
                    s,
                    egui::StrokeKind::Inside,
                );
            }
        }
        VectorIcon::ListView => {
            for fy in [0.22, 0.5, 0.78] {
                let y = r.top() + r.height() * fy;
                painter.circle_filled(Pos2::new(r.left() + 1.5, y), 1.4, col);
                painter.line_segment([Pos2::new(r.left() + 5.0, y), Pos2::new(r.right(), y)], s);
            }
        }
        VectorIcon::ArchitraveProfile => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.rect_stroke(r.shrink(r.width() * 0.22), 0.5, s, egui::StrokeKind::Inside);
            painter.line_segment([r.left_top(), r.shrink(r.width() * 0.22).left_top()], s);
            painter.line_segment([r.right_top(), r.shrink(r.width() * 0.22).right_top()], s);
        }
        VectorIcon::Shutter => {
            let mid = Rect::from_center_size(c, Vec2::new(r.width() * 0.44, r.height() * 0.86));
            painter.rect_stroke(mid, 0.5, s, egui::StrokeKind::Inside);
            let sl = Rect::from_min_max(
                Pos2::new(r.left(), mid.top()),
                Pos2::new(mid.left() - 1.5, mid.bottom()),
            );
            let sr = Rect::from_min_max(
                Pos2::new(mid.right() + 1.5, mid.top()),
                Pos2::new(r.right(), mid.bottom()),
            );
            painter.rect_filled(sl, 0.5, col.gamma_multiply(0.55));
            painter.rect_filled(sr, 0.5, col.gamma_multiply(0.55));
        }
        VectorIcon::HeaderPediment => {
            let y_base = r.top() + r.height() * 0.48;
            painter.add(Shape::closed_line(
                vec![
                    Pos2::new(r.left(), y_base),
                    Pos2::new(c.x, r.top() + 1.0),
                    Pos2::new(r.right(), y_base),
                ],
                s,
            ));
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(r.left() + 2.0, y_base + 1.5),
                    Pos2::new(r.right() - 2.0, r.bottom()),
                ),
                0.5,
                s,
                egui::StrokeKind::Inside,
            );
        }
        VectorIcon::Palette => {
            painter.circle_stroke(c, r.width().min(r.height()) * 0.46, s);
            for (dx, dy) in [(-0.22, -0.14), (0.12, -0.22), (0.24, 0.08), (-0.1, 0.2)] {
                painter.circle_filled(
                    Pos2::new(c.x + r.width() * dx, c.y + r.height() * dy),
                    (r.width() * 0.09).max(1.3),
                    col,
                );
            }
        }
        VectorIcon::ChevronLeft => {
            let d = r.height() * 0.28;
            painter.line_segment(
                [
                    Pos2::new(c.x + d * 0.5, c.y - d),
                    Pos2::new(c.x - d * 0.6, c.y),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - d * 0.6, c.y),
                    Pos2::new(c.x + d * 0.5, c.y + d),
                ],
                s,
            );
        }
        VectorIcon::ChevronRight => {
            let d = r.height() * 0.28;
            painter.line_segment(
                [
                    Pos2::new(c.x - d * 0.5, c.y - d),
                    Pos2::new(c.x + d * 0.6, c.y),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + d * 0.6, c.y),
                    Pos2::new(c.x - d * 0.5, c.y + d),
                ],
                s,
            );
        }
        VectorIcon::ChevronDown => {
            let d = r.width() * 0.28;
            painter.line_segment(
                [
                    Pos2::new(c.x - d, c.y - d * 0.5),
                    Pos2::new(c.x, c.y + d * 0.6),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(c.x, c.y + d * 0.6),
                    Pos2::new(c.x + d, c.y - d * 0.5),
                ],
                s,
            );
        }
        VectorIcon::Lock => {
            let body = Rect::from_min_max(
                Pos2::new(r.left() + r.width() * 0.16, c.y - 1.0),
                Pos2::new(r.right() - r.width() * 0.16, r.bottom()),
            );
            painter.rect_stroke(body, 1.5, s, egui::StrokeKind::Inside);
            painter.circle_stroke(Pos2::new(c.x, c.y - r.height() * 0.18), r.width() * 0.2, s);
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
    let sc = (rect.width().min(rect.height()) / 48.0).clamp(0.32, 1.85);
    let bar_stroke = Stroke::new((1.4 * sc).max(0.9), frame_col);
    let heavy_stroke = Stroke::new((2.3 * sc).max(1.2), frame_col);
    let frame_stroke = Stroke::new((2.8 * sc).max(1.3), frame_col);
    let border_stroke = Stroke::new((1.3 * sc).max(0.8), outline_col);

    let inner = rect.shrink(5.0 * sc);

    match style {
        WindowStyle::CircularBullsEye => {
            let r = inner.width().min(inner.height()) * 0.45;
            let c = inner.center();
            painter.circle_filled(c, r, glass_col);
            painter.circle_stroke(c, r, Stroke::new((3.6 * sc).max(1.5), frame_col));
            painter.circle_stroke(c, r + 1.8 * sc, border_stroke);
            let ir = (r - 2.0 * sc).max(1.0);
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
            painter.add(Shape::closed_line(pts, frame_stroke));
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
            draw_sill(painter, cx, bot, w, sc, frame_col);
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
            painter.add(Shape::closed_line(pts, frame_stroke));
            painter.line_segment([Pos2::new(cx, top), Pos2::new(cx, bot)], bar_stroke);
            painter.line_segment(
                [Pos2::new(cx - hw, y_spring), Pos2::new(cx + hw, y_spring)],
                bar_stroke,
            );
            draw_sill(painter, cx, bot, w, sc, frame_col);
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
            painter.add(Shape::closed_line(pts, frame_stroke));
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
            draw_sill(painter, cx, bot, w, sc, frame_col);
        }
        WindowStyle::DoorArchedHeritage => {
            let w = inner.width() * 0.62;
            let h = inner.height() * 0.90;
            let door_rect = Rect::from_center_size(inner.center(), Vec2::new(w, h));
            let panel_col = Color32::from_rgb(165, 132, 96);
            let groove_col = Color32::from_rgb(95, 72, 48);
            let rx = w * 0.5;
            let y_spring = door_rect.top() + rx;
            let mut pts = vec![door_rect.left_bottom(), door_rect.right_bottom()];
            for i in 0..=14 {
                let t = (i as f32 / 14.0) * PI;
                pts.push(Pos2::new(
                    door_rect.center().x + rx * t.cos(),
                    y_spring - rx * t.sin(),
                ));
            }
            painter.add(Shape::convex_polygon(pts.clone(), panel_col, Stroke::NONE));
            painter.add(Shape::closed_line(pts, frame_stroke));
            let lower = Rect::from_min_max(
                Pos2::new(door_rect.left() + 4.0 * sc, y_spring + 2.0 * sc),
                Pos2::new(door_rect.right() - 4.0 * sc, door_rect.bottom() - 4.0 * sc),
            );
            painter.rect_stroke(
                lower,
                0.5,
                Stroke::new((1.0 * sc).max(0.8), groove_col),
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(
                Pos2::new(
                    door_rect.right() - 4.2 * sc,
                    door_rect.center().y + 2.0 * sc,
                ),
                (1.6 * sc).max(1.0),
                Color32::from_rgb(230, 190, 95),
            );
        }
        WindowStyle::DoorColonial4Panel
        | WindowStyle::DoorGeorgian6Panel
        | WindowStyle::DoorGlazedFrench
        | WindowStyle::DoorFanlightGeorgian => {
            let w = inner.width() * 0.62;
            let h = inner.height() * 0.90;
            let door_rect = Rect::from_center_size(inner.center(), Vec2::new(w, h));
            let panel_col = Color32::from_rgb(165, 132, 96);
            let groove_col = Color32::from_rgb(95, 72, 48);
            painter.rect_filled(door_rect, 1.0, panel_col);
            let p_ins = 4.0 * sc;
            if style == WindowStyle::DoorGlazedFrench {
                let tg = Rect::from_min_max(
                    Pos2::new(door_rect.left() + p_ins, door_rect.top() + p_ins),
                    Pos2::new(door_rect.right() - p_ins, door_rect.center().y + 2.0 * sc),
                );
                painter.rect_filled(tg, 0.5, glass_col);
                painter.rect_stroke(tg, 0.5, bar_stroke, egui::StrokeKind::Middle);
            } else if style == WindowStyle::DoorFanlightGeorgian {
                let fg = Rect::from_min_max(
                    Pos2::new(door_rect.left() + p_ins, door_rect.top() + p_ins),
                    Pos2::new(door_rect.right() - p_ins, door_rect.top() + h * 0.34),
                );
                painter.rect_filled(fg, (4.0 * sc).max(1.5), glass_col);
                painter.rect_stroke(
                    fg,
                    (4.0 * sc).max(1.5),
                    bar_stroke,
                    egui::StrokeKind::Middle,
                );
            } else {
                let rows = if style == WindowStyle::DoorGeorgian6Panel {
                    3
                } else {
                    2
                };
                let gap = 2.5 * sc;
                let pw = ((w - p_ins * 2.0 - gap) * 0.5).max(1.5);
                let ph = ((h - p_ins * 2.0 - (rows as f32 - 1.0) * gap) / rows as f32).max(1.5);
                for r in 0..rows {
                    let y = door_rect.top() + p_ins + r as f32 * (ph + gap);
                    for c in 0..2 {
                        let x = door_rect.left() + p_ins + c as f32 * (pw + gap);
                        painter.rect_stroke(
                            Rect::from_min_size(Pos2::new(x, y), Vec2::new(pw, ph)),
                            0.5,
                            Stroke::new((1.0 * sc).max(0.7), groove_col),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
            painter.rect_stroke(door_rect, 1.0, frame_stroke, egui::StrokeKind::Middle);
            painter.circle_filled(
                Pos2::new(
                    door_rect.right() - 4.2 * sc,
                    door_rect.center().y + 2.0 * sc,
                ),
                (1.6 * sc).max(1.0),
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
            let wr =
                Rect::from_center_size(inner.center() - Vec2::new(0.0, 1.5 * sc), Vec2::new(w, h));
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
                    let pitch = (7.0 * sc).max(3.2);
                    let span = wr.width().max(wr.height()) * 1.4;
                    for k in -6..=6 {
                        let off = k as f32 * pitch;
                        clipped.line_segment(
                            [
                                Pos2::new(wr.left() + off - span * 0.4, wr.top() - span * 0.4),
                                Pos2::new(wr.left() + off + span, wr.top() + span),
                            ],
                            bar_stroke,
                        );
                        clipped.line_segment(
                            [
                                Pos2::new(wr.right() - off + span * 0.4, wr.top() - span * 0.4),
                                Pos2::new(wr.right() - off - span, wr.top() + span),
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
            painter.rect_stroke(wr, 1.0, frame_stroke, egui::StrokeKind::Middle);
            painter.rect_stroke(
                wr.expand(1.5 * sc),
                1.5 * sc,
                border_stroke,
                egui::StrokeKind::Outside,
            );
            draw_sill(
                painter,
                wr.center().x,
                wr.bottom() + 1.5 * sc,
                w,
                sc,
                frame_col,
            );
        }
    }
}

fn draw_sill(painter: &egui::Painter, cx: f32, y_bot: f32, w: f32, sc: f32, col: Color32) {
    let sill_rect = Rect::from_center_size(
        Pos2::new(cx, y_bot + 1.5 * sc),
        Vec2::new(w + 7.5 * sc, (3.2 * sc).max(1.5)),
    );
    painter.rect_filled(sill_rect, 0.8 * sc, col);
}
