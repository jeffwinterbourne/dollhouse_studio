use dh_core::DisplayUnit;
use dh_project::ThemeMode;
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, Stroke, TextStyle, Vec2};

/// Semantic color palette resolved for either Light (Office/Fusion) or Dark (CAD Charcoal) mode.
#[derive(Debug, Clone, Copy)]
pub struct StudioPalette {
    pub dark_mode: bool,
    pub surface_base: Color32,
    pub surface_panel: Color32,
    pub surface_ribbon: Color32,
    pub surface_card: Color32,
    pub surface_elevated: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub gold: Color32,
    pub success: Color32,
    pub danger: Color32,
    pub hud_glass: Color32,
    pub canvas_bg: Color32,
    pub canvas_grid_minor: Color32,
    pub canvas_grid_major: Color32,
    pub wall_fill: Color32,
    pub wall_fill_over_photo: Color32,
    pub wall_stroke: Color32,
}

impl StudioPalette {
    pub fn for_mode(dark: bool) -> Self {
        if dark {
            Self {
                dark_mode: true,
                surface_base: Color32::from_rgb(20, 23, 30),
                surface_panel: Color32::from_rgb(26, 30, 40),
                surface_ribbon: Color32::from_rgb(28, 33, 44),
                surface_card: Color32::from_rgb(34, 40, 53),
                surface_elevated: Color32::from_rgb(44, 51, 68),
                border: Color32::from_rgb(64, 74, 96),
                border_strong: Color32::from_rgb(92, 106, 134),
                text_primary: Color32::from_rgb(240, 244, 250),
                text_secondary: Color32::from_rgb(206, 214, 228),
                text_muted: Color32::from_rgb(162, 174, 194),
                accent: Color32::from_rgb(245, 198, 108),
                accent_text: Color32::from_rgb(18, 22, 30),
                gold: Color32::from_rgb(245, 198, 108),
                success: Color32::from_rgb(112, 220, 145),
                danger: Color32::from_rgb(240, 102, 102),
                hud_glass: Color32::from_rgba_unmultiplied(22, 26, 36, 238),
                canvas_bg: Color32::from_rgb(17, 26, 40),
                canvas_grid_minor: Color32::from_rgba_unmultiplied(85, 115, 155, 28),
                canvas_grid_major: Color32::from_rgba_unmultiplied(115, 150, 195, 58),
                wall_fill: Color32::from_rgba_unmultiplied(36, 56, 84, 145),
                wall_fill_over_photo: Color32::from_rgba_unmultiplied(36, 56, 84, 34),
                wall_stroke: Color32::from_rgb(155, 198, 240),
            }
        } else {
            Self {
                dark_mode: false,
                surface_base: Color32::from_rgb(241, 244, 248),
                surface_panel: Color32::from_rgb(252, 253, 255),
                surface_ribbon: Color32::from_rgb(246, 248, 251),
                surface_card: Color32::from_rgb(255, 255, 255),
                surface_elevated: Color32::from_rgb(233, 238, 246),
                border: Color32::from_rgb(208, 216, 228),
                border_strong: Color32::from_rgb(162, 174, 192),
                text_primary: Color32::from_rgb(28, 35, 46),
                text_secondary: Color32::from_rgb(66, 77, 94),
                text_muted: Color32::from_rgb(106, 118, 136),
                accent: Color32::from_rgb(29, 99, 184),
                accent_text: Color32::WHITE,
                gold: Color32::from_rgb(208, 140, 24),
                success: Color32::from_rgb(22, 145, 76),
                danger: Color32::from_rgb(198, 46, 46),
                hud_glass: Color32::from_rgba_unmultiplied(255, 255, 255, 242),
                canvas_bg: Color32::from_rgb(232, 238, 246),
                canvas_grid_minor: Color32::from_rgba_unmultiplied(150, 170, 196, 55),
                canvas_grid_major: Color32::from_rgba_unmultiplied(115, 140, 175, 95),
                wall_fill: Color32::from_rgba_unmultiplied(255, 255, 255, 210),
                wall_fill_over_photo: Color32::from_rgba_unmultiplied(255, 255, 255, 45),
                wall_stroke: Color32::from_rgb(38, 72, 120),
            }
        }
    }
}

/// Resolves whether dark mode should be active given `ThemeMode` and current OS theme.
pub fn resolve_dark_mode(ctx: &egui::Context, mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => ctx.input(|i| i.raw.system_theme != Some(egui::Theme::Light)),
    }
}

/// Applies the clean Office + CAD studio theme to `ctx` and returns the resolved `StudioPalette`.
pub fn apply_studio_theme(ctx: &egui::Context, mode: ThemeMode) -> StudioPalette {
    let dark = resolve_dark_mode(ctx, mode);
    let pal = StudioPalette::for_mode(dark);

    let pref = if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    };
    ctx.set_theme(pref);

    let base_theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let mut style = (*ctx.style_of(base_theme)).clone();
    style.spacing.item_spacing = Vec2::new(6.0, 5.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.interact_size = Vec2::new(36.0, 23.0);

    style.text_styles.insert(TextStyle::Small, FontId::proportional(11.8));
    style.text_styles.insert(TextStyle::Body, FontId::proportional(13.2));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(13.0));
    style.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.5));
    style.text_styles.insert(TextStyle::Heading, FontId::proportional(16.5));

    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = pal.surface_panel;
    v.window_fill = pal.surface_card;
    v.extreme_bg_color = if dark {
        Color32::from_rgb(13, 16, 22)
    } else {
        Color32::from_rgb(236, 240, 246)
    };
    v.text_edit_bg_color = Some(if dark {
        Color32::from_rgb(15, 19, 27)
    } else {
        Color32::WHITE
    });
    v.faint_bg_color = pal.surface_elevated;
    v.code_bg_color = pal.surface_elevated;
    v.override_text_color = Some(pal.text_primary);
    v.weak_text_color = Some(pal.text_muted);
    v.hyperlink_color = pal.accent;

    v.widgets.noninteractive.bg_fill = pal.surface_card;
    v.widgets.noninteractive.weak_bg_fill = pal.surface_card;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, pal.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, pal.text_primary);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    v.widgets.inactive.bg_fill = if dark {
        Color32::from_rgb(46, 54, 72)
    } else {
        Color32::from_rgb(235, 240, 247)
    };
    v.widgets.inactive.weak_bg_fill = if dark {
        Color32::from_rgb(38, 45, 60)
    } else {
        Color32::from_rgb(242, 245, 250)
    };
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, pal.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.1, pal.text_primary);
    v.widgets.inactive.corner_radius = CornerRadius::same(5);

    v.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(60, 70, 94)
    } else {
        Color32::from_rgb(220, 231, 245)
    };
    v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
    v.widgets.hovered.bg_stroke = Stroke::new(1.2, pal.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.3, pal.text_primary);
    v.widgets.hovered.corner_radius = CornerRadius::same(5);

    v.widgets.active.bg_fill = if dark {
        Color32::from_rgb(72, 85, 114)
    } else {
        Color32::from_rgb(200, 218, 240)
    };
    v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
    v.widgets.active.bg_stroke = Stroke::new(1.4, pal.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.5, pal.text_primary);
    v.widgets.active.corner_radius = CornerRadius::same(5);

    v.widgets.open.bg_fill = pal.surface_elevated;
    v.widgets.open.weak_bg_fill = pal.surface_elevated;
    v.widgets.open.bg_stroke = Stroke::new(1.2, pal.accent);
    v.widgets.open.fg_stroke = Stroke::new(1.2, pal.text_primary);
    v.widgets.open.corner_radius = CornerRadius::same(5);

    v.selection.bg_fill = if dark {
        Color32::from_rgb(48, 92, 158)
    } else {
        Color32::from_rgb(29, 99, 184)
    };
    v.selection.stroke = Stroke::new(1.2, Color32::WHITE);
    v.window_stroke = Stroke::new(1.0, pal.border_strong);
    v.window_corner_radius = CornerRadius::same(8);
    v.menu_corner_radius = CornerRadius::same(6);

    style.visuals = v;
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
    pal
}

pub fn section_card(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    title: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(pal.surface_card)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            if let Some(t) = title {
                ui.label(RichText::new(t).strong().size(13.2).color(pal.text_primary));
                ui.add_space(4.0);
            }
            add_contents(ui);
        });
    ui.add_space(5.0);
}

pub fn segmented_pill<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    current: &mut T,
    items: &[(T, &str)],
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(pal.surface_base)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for &(val, label) in items {
                    let active = *current == val;
                    let text = if active {
                        RichText::new(label).strong().size(12.8).color(pal.accent_text)
                    } else {
                        RichText::new(label).size(12.8).color(pal.text_primary)
                    };
                    let btn = egui::Button::new(text)
                        .fill(if active { pal.accent } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(0.0, 24.0));
                    if ui.add(btn).clicked() && !active {
                        *current = val;
                        changed = true;
                    }
                }
            });
        });
    changed
}

/// Scrubbable numeric input displayed in `DisplayUnit` while storing exact millimeters in `val_mm`.
pub fn slider_unit_mm(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    label: &str,
    val_mm: &mut f64,
    range_mm: std::ops::RangeInclusive<f64>,
    unit: DisplayUnit,
) -> bool {
    let mut disp = unit.from_mm(*val_mm);
    let r_disp = unit.from_mm(*range_mm.start())..=unit.from_mm(*range_mm.end());
    let mut changed = false;
    let avail = ui.available_width();
    let slider_w = (avail - 195.0).clamp(54.0, 118.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.8).color(pal.text_primary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized(
                    [72.0, 22.0],
                    egui::DragValue::new(&mut disp)
                        .range(r_disp.clone())
                        .speed(unit.drag_speed())
                        .suffix(unit.suffix())
                        .max_decimals(unit.decimals()),
                )
                .changed()
            {
                *val_mm = unit.to_mm(disp);
                changed = true;
            }
            ui.spacing_mut().slider_width = slider_w;
            if ui
                .add(egui::Slider::new(&mut disp, r_disp).show_value(false))
                .changed()
            {
                *val_mm = unit.to_mm(disp);
                changed = true;
            }
        });
    });
    changed
}

/// Compact drag-value-only row (ideal for slim core properties without slider clutter).
pub fn drag_unit_mm(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    label: &str,
    val_mm: &mut f64,
    range_mm: std::ops::RangeInclusive<f64>,
    unit: DisplayUnit,
) -> bool {
    let mut disp = unit.from_mm(*val_mm);
    let r_disp = unit.from_mm(*range_mm.start())..=unit.from_mm(*range_mm.end());
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.8).color(pal.text_primary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized(
                    [82.0, 22.0],
                    egui::DragValue::new(&mut disp)
                        .range(r_disp)
                        .speed(unit.drag_speed())
                        .suffix(unit.suffix())
                        .max_decimals(unit.decimals()),
                )
                .changed()
            {
                *val_mm = unit.to_mm(disp);
                changed = true;
            }
        });
    });
    changed
}

pub fn slider_u32(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    label: &str,
    val: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    let mut changed = false;
    let avail = ui.available_width();
    let slider_w = (avail - 185.0).clamp(54.0, 118.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.8).color(pal.text_primary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed |= ui
                .add_sized([54.0, 22.0], egui::DragValue::new(val).range(range.clone()).speed(0.1))
                .changed();
            ui.spacing_mut().slider_width = slider_w;
            changed |= ui
                .add(egui::Slider::new(val, range).show_value(false))
                .changed();
        });
    });
    changed
}

pub fn slider_ratio_pct(
    ui: &mut egui::Ui,
    pal: &StudioPalette,
    label: &str,
    val: &mut f64,
    range: std::ops::RangeInclusive<f64>,
) -> bool {
    let mut changed = false;
    let mut pct = *val * 100.0;
    let r_pct = (*range.start() * 100.0)..=(*range.end() * 100.0);
    let avail = ui.available_width();
    let slider_w = (avail - 185.0).clamp(54.0, 118.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.8).color(pal.text_primary));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_sized(
                    [62.0, 22.0],
                    egui::DragValue::new(&mut pct)
                        .range(r_pct.clone())
                        .speed(0.5)
                        .suffix("%")
                        .max_decimals(0),
                )
                .changed()
            {
                *val = pct / 100.0;
                changed = true;
            }
            ui.spacing_mut().slider_width = slider_w;
            if ui
                .add(egui::Slider::new(&mut pct, r_pct).show_value(false))
                .changed()
            {
                *val = pct / 100.0;
                changed = true;
            }
        });
    });
    changed
}
