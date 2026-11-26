use crate::icons::draw_style_thumbnail;
use crate::theme::{StudioPalette, segmented_pill};
use dh_core::{DisplayUnit, TemplateScope, WindowParams};
use eframe::egui::{self, RichText, Vec2};

#[derive(Debug, Clone)]
pub struct SaveTemplateModalState {
    pub open: bool,
    pub name: String,
    pub scope: TemplateScope,
    pub tags_text: String,
    pub notes: String,
    pub params: WindowParams,
}

impl Default for SaveTemplateModalState {
    fn default() -> Self {
        Self {
            open: false,
            name: "Custom Window Template".to_string(),
            scope: TemplateScope::Personal,
            tags_text: String::new(),
            notes: String::new(),
            params: WindowParams::default(),
        }
    }
}

impl SaveTemplateModalState {
    pub fn open_for(&mut self, suggested_name: &str, params: WindowParams) {
        self.open = true;
        self.name = suggested_name.to_string();
        self.tags_text = params.style.name().to_lowercase();
        self.notes.clear();
        self.params = params;
    }
}

#[derive(Debug, Clone)]
pub struct SaveTemplateRequest {
    pub name: String,
    pub scope: TemplateScope,
    pub tags: Vec<String>,
    pub notes: String,
    pub params: WindowParams,
}

pub fn draw_save_template_modal(
    ctx: &egui::Context,
    pal: &StudioPalette,
    unit: DisplayUnit,
    state: &mut SaveTemplateModalState,
) -> Option<SaveTemplateRequest> {
    if !state.open {
        return None;
    }
    let mut out = None;
    let mut open = state.open;

    egui::Window::new("Save As Reusable Template")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(430.0, 320.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (tr, _) = ui.allocate_exact_size(Vec2::splat(52.0), egui::Sense::hover());
                draw_style_thumbnail(ui.painter(), tr, state.params.style, true, pal);
                ui.vertical(|ui| {
                    ui.label(RichText::new(state.params.style.name()).strong().size(14.5));
                    ui.label(
                        RichText::new(unit.format_dims(
                            state.params.opening_width,
                            state.params.opening_height,
                        ))
                        .small()
                        .color(pal.text_secondary),
                    );
                });
            });

            ui.add_space(10.0);
            ui.label(RichText::new("Template Name").strong());
            ui.add(egui::TextEdit::singleline(&mut state.name).desired_width(ui.available_width()));

            ui.add_space(8.0);
            ui.label(RichText::new("Save To").strong());
            segmented_pill(
                ui,
                pal,
                &mut state.scope,
                &[
                    (TemplateScope::Personal, "Personal Library (All Projects)"),
                    (TemplateScope::Project, "This Project Folder"),
                ],
            );

            ui.add_space(8.0);
            ui.label(RichText::new("Tags (comma separated)").small().color(pal.text_muted));
            ui.add(
                egui::TextEdit::singleline(&mut state.tags_text)
                    .desired_width(ui.available_width()),
            );

            ui.add_space(6.0);
            ui.label(RichText::new("Notes (optional)").small().color(pal.text_muted));
            ui.add(
                egui::TextEdit::multiline(&mut state.notes)
                    .desired_rows(2)
                    .desired_width(ui.available_width()),
            );

            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    RichText::new("Save Template").strong().color(pal.accent_text),
                )
                .fill(pal.accent);
                let valid = !state.name.trim().is_empty();
                if ui.add_enabled(valid, save_btn).clicked() {
                    let tags: Vec<String> = state
                        .tags_text
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    out = Some(SaveTemplateRequest {
                        name: state.name.trim().to_string(),
                        scope: state.scope,
                        tags,
                        notes: state.notes.trim().to_string(),
                        params: state.params.clone(),
                    });
                    state.open = false;
                }
                if ui.button("Cancel").clicked() {
                    state.open = false;
                }
            });
        });

    if !open {
        state.open = false;
    }
    out
}
