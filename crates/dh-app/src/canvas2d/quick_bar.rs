use crate::theme::StudioPalette;
use eframe::egui::{self, CornerRadius, Pos2, Rect, RichText, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickBarAction {
    Duplicate,
    PromoteToComponent,
    MakeUnique,
    SaveAsTemplate,
    ToggleLock,
    Inspect3D,
    Delete,
}

/// Compact Office/Figma-style floating contextual pill toolbar positioned directly above the
/// selected opening(s) on the 2D Blueprint canvas.
#[allow(clippy::too_many_arguments)]
pub fn draw_opening_quick_bar(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    canvas_rect: Rect,
    selection_screen_rect: Rect,
    selected_count: usize,
    is_component: bool,
    any_locked: bool,
) -> Option<QuickBarAction> {
    if selected_count == 0 {
        return None;
    }

    let est_w = if selected_count == 1 { 420.0 } else { 220.0 };
    let prefer_top = selection_screen_rect.top() - 40.0;
    let y = if prefer_top >= canvas_rect.top() + 34.0 {
        prefer_top
    } else {
        (selection_screen_rect.bottom() + 8.0).min(canvas_rect.bottom() - 42.0)
    };
    let x = (selection_screen_rect.center().x - est_w * 0.5).clamp(
        canvas_rect.left() + 8.0,
        (canvas_rect.right() - est_w - 8.0).max(canvas_rect.left() + 8.0),
    );

    let mut action = None;
    let area_id = ui.scope_id().with("opening_quick_action_bar");
    egui::Area::new(area_id)
        .fixed_pos(Pos2::new(x, y))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(pal.hud_glass)
                .stroke(Stroke::new(1.0, pal.border_strong))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.small_button("Duplicate").clicked() {
                            action = Some(QuickBarAction::Duplicate);
                        }

                        if selected_count == 1 {
                            if is_component {
                                if ui
                                    .small_button("Make Unique")
                                    .on_hover_text("Unlink this instance from shared Component")
                                    .clicked()
                                {
                                    action = Some(QuickBarAction::MakeUnique);
                                }
                            } else if ui
                                .small_button("Make Component")
                                .on_hover_text("Promote to reusable Component across walls")
                                .clicked()
                            {
                                action = Some(QuickBarAction::PromoteToComponent);
                            }

                            if ui.small_button("Save Template").clicked() {
                                action = Some(QuickBarAction::SaveAsTemplate);
                            }
                            if ui
                                .small_button(if any_locked { "Unlock" } else { "Lock" })
                                .clicked()
                            {
                                action = Some(QuickBarAction::ToggleLock);
                            }
                            if ui
                                .add(egui::Button::new(
                                    RichText::new("3D").strong().size(12.0).color(pal.accent),
                                ))
                                .on_hover_text("Inspect this opening in Piece 3D")
                                .clicked()
                            {
                                action = Some(QuickBarAction::Inspect3D);
                            }
                        } else {
                            ui.label(
                                RichText::new(format!("{selected_count} selected"))
                                    .small()
                                    .color(pal.text_secondary),
                            );
                        }

                        if ui
                            .add(egui::Button::new(
                                RichText::new("Delete").size(12.0).color(pal.danger),
                            ))
                            .clicked()
                        {
                            action = Some(QuickBarAction::Delete);
                        }
                    });
                });
        });

    action
}
