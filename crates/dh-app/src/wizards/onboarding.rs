use crate::icons::{VectorIcon, paint_brand_mark, paint_icon};
use crate::theme::StudioPalette;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Stroke, StrokeKind,
    Vec2,
};

#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub open: bool,
    pub step: usize,
    pub dont_show_again: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            open: false,
            step: 0,
            dont_show_again: true,
        }
    }
}

impl OnboardingState {
    pub fn open_tour(&mut self) {
        self.open = true;
        self.step = 0;
    }
}

pub enum OnboardingOutcome {
    Dismissed { remember_seen: bool },
}

/// Interactive, skippable 4-step walkthrough introducing Dollhouse Studio's key CAD & crafting concepts.
pub fn draw_onboarding_tour(
    ctx: &egui::Context,
    pal: &StudioPalette,
    state: &mut OnboardingState,
) -> Option<OnboardingOutcome> {
    if !state.open {
        return None;
    }

    let mut outcome = None;
    let screen = ctx.content_rect();
    let modal_w = (screen.width() - 80.0).clamp(540.0, 680.0);
    let modal_h = (screen.height() - 80.0).clamp(380.0, 470.0);

    egui::Area::new(egui::Id::unique("onboarding_scrim"))
        .fixed_pos(screen.left_top())
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (r, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(10, 15, 24, 155));
        });

    let mut close_now = false;
    let modal_pos = screen.center() - Vec2::new(modal_w * 0.5, modal_h * 0.5);
    egui::Area::new(egui::Id::unique("onboarding_modal"))
        .fixed_pos(modal_pos)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(pal.surface_panel)
                .stroke(Stroke::new(1.5, pal.accent))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(modal_w - 40.0, modal_h - 40.0));
                    ui.set_max_size(Vec2::new(modal_w - 40.0, modal_h - 40.0));

                    ui.horizontal(|ui| {
                        let (mr, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
                        paint_brand_mark(ui.painter(), mr);
                        ui.label(RichText::new("Welcome to Dollhouse Studio").size(18.0).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Skip Tour").clicked() {
                                close_now = true;
                            }
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(12.0);

                    let step = state.step.min(3);
                    let (title, lead, bullets, icon) = match step {
                        0 => (
                            "1. Projects & Workspaces",
                            "Start with a single window/door study or a full multi-wall dollhouse facade — everything saves cleanly to an organized folder on disk.",
                            vec![
                                "Single Piece starts straight in 3D Preview and can be promoted into a House / Multi-Wall project anytime.",
                                "Use Blueprint 2D to lay out walls & trace photos, Wall 3D to inspect exterior/interior trims, and Piece 3D for assembly & STL check.",
                                "Switch units globally anytime between mm, cm, and inches in the top-right bar.",
                            ],
                            VectorIcon::Wall,
                        ),
                        1 => (
                            "2. Cutout Openings vs Architraves & Tolerances",
                            "Understanding wall cutouts vs overlapping exterior trim keeps neighboring windows and corners clash-free.",
                            vec![
                                "Opening Width / Height defines the hole cut through the dollhouse wall.",
                                "Architraves (outer trim), projecting sills, and shutters extend OUTSIDE that hole — toggle 'Architraves' on Blueprint 2D to inspect exact overlap tolerances.",
                                "All parts print flat on Z=0 with 45-deg-free chamfers and telescoping interior/exterior barrels.",
                            ],
                            VectorIcon::Architrave,
                        ),
                        2 => (
                            "3. Templates vs Linked Components",
                            "Two complementary ways to reuse your favorite designs without repetitive tweaking:",
                            vec![
                                "Templates: Saved starter presets in your Personal Library or Project folder. Editing one opening later does NOT alter others.",
                                "Linked Components: Select any window/door and click 'Make Component'. Place copies across one or more walls — editing the Component updates every linked instance live!",
                                "Need one sill trimmed or a door hinged opposite? Use Per-Instance Overrides without unlinking.",
                            ],
                            VectorIcon::ComponentLink,
                        ),
                        _ => (
                            "4. Photo Calibration & Smart CAD Dimensions",
                            "Build directly over real dollhouse facade photos and lock clearances with parametric dimensions.",
                            vec![
                                "Photo Setup Wizard: Rotate straight first, drag 4 corner perspective pins (with cursor loupe), then add one or more exact lines in mm/cm/in — no distorted stretching.",
                                "CAD Dimensions: Click wall or opening edges to measure. Double-click any dimension label to type an exact length and move/resize the opening automatically.",
                                "Right-click anywhere on Blueprint 2D for quick context actions: add openings, photos, dimensions, or resize the wall.",
                            ],
                            VectorIcon::Caliper,
                        ),
                    };

                    egui::Frame::NONE
                        .fill(pal.surface_card)
                        .stroke(Stroke::new(1.0, pal.border))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                            ui.set_min_height(255.0);
                            ui.horizontal(|ui| {
                                let (ir, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                                paint_icon(ui.painter(), ir, icon, pal.accent);
                                ui.label(RichText::new(title).size(16.0).strong().color(pal.accent));
                            });
                            ui.add_space(6.0);
                            ui.label(RichText::new(lead).size(13.2).color(pal.text_primary));
                            ui.add_space(10.0);
                            for b in bullets {
                                ui.horizontal_top(|ui| {
                                    ui.label(RichText::new("-").strong().color(pal.gold));
                                    ui.label(RichText::new(b).size(12.8).color(pal.text_secondary));
                                });
                                ui.add_space(4.0);
                            }

                            if step == 1 {
                                // Compact visual diagram of cutout vs architrave envelope
                                ui.add_space(6.0);
                                let (dr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 54.0), egui::Sense::hover());
                                let p = ui.painter_at(dr);
                                let trim_r = Rect::from_center_size(dr.center(), Vec2::new(170.0, 44.0));
                                let cut_r = Rect::from_center_size(dr.center(), Vec2::new(136.0, 28.0));
                                p.rect_stroke(trim_r, 2.0, Stroke::new(1.4, pal.gold), StrokeKind::Outside);
                                p.rect_filled(cut_r, 1.0, pal.accent.linear_multiply(0.22));
                                p.rect_stroke(cut_r, 1.0, Stroke::new(1.5, pal.accent), StrokeKind::Inside);
                                p.text(Pos2::new(trim_r.right() + 10.0, dr.center().y), Align2::LEFT_CENTER, "Blue = Wall Cutout\nGold = Outer Architrave + Sill Tolerance", FontId::proportional(11.0), pal.text_muted);
                            }
                        });

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.dont_show_again, "Don't show automatically on next launch");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if step < 3 {
                                let next_b = egui::Button::new(
                                    RichText::new("Next ->").strong().color(pal.accent_text),
                                )
                                .fill(pal.accent);
                                if ui.add(next_b).clicked() {
                                    state.step += 1;
                                }
                            } else {
                                let done_b = egui::Button::new(
                                    RichText::new("Start Designing").strong().color(pal.accent_text),
                                )
                                .fill(pal.accent);
                                if ui.add(done_b).clicked() {
                                    close_now = true;
                                }
                            }
                            if step > 0 && ui.button("<- Back").clicked() {
                                state.step -= 1;
                            }
                        });
                    });
                });
        });

    if close_now || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.open = false;
        outcome = Some(OnboardingOutcome::Dismissed {
            remember_seen: state.dont_show_again,
        });
    }
    outcome
}
