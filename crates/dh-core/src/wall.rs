use crate::component::{ComponentDef, InstanceOverrides, OpeningSource};
use crate::params::{WindowParams, WindowStyle};
use crate::project::new_id;
use crate::units::DisplayUnit;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Optional 3D house assembly transform stored on every wall for multi-wall assembly views.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallPlacement3D {
    pub origin_mm: [f64; 3],
    pub yaw_deg: f64,
    pub floor_level: i32,
}

impl Default for WallPlacement3D {
    fn default() -> Self {
        Self {
            origin_mm: [0.0, 0.0, 0.0],
            yaw_deg: 0.0,
            floor_level: 0,
        }
    }
}

/// An opening (window or hinged door) placed on a `ProjectWall`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallOpening {
    pub id: String,
    pub name: String,
    /// Distance from wall left edge to opening left edge (mm).
    pub x_from_left: f64,
    /// Distance from wall bottom edge to opening bottom edge (mm).
    pub y_from_bottom: f64,
    #[serde(default)]
    pub lock_width: bool,
    #[serde(default)]
    pub lock_height: bool,
    #[serde(default)]
    pub lock_position: bool,
    pub source: OpeningSource,
}

impl WallOpening {
    pub fn new_unique(
        name: impl Into<String>,
        x_from_left: f64,
        y_from_bottom: f64,
        params: WindowParams,
    ) -> Self {
        Self {
            id: new_id("open"),
            name: name.into(),
            x_from_left,
            y_from_bottom,
            lock_width: false,
            lock_height: false,
            lock_position: false,
            source: OpeningSource::Unique { params },
        }
    }

    pub fn new_instance(
        name: impl Into<String>,
        x_from_left: f64,
        y_from_bottom: f64,
        component_id: impl Into<String>,
        overrides: InstanceOverrides,
    ) -> Self {
        Self {
            id: new_id("open"),
            name: name.into(),
            x_from_left,
            y_from_bottom,
            lock_width: false,
            lock_height: false,
            lock_position: false,
            source: OpeningSource::Component {
                component_id: component_id.into(),
                overrides,
            },
        }
    }

    pub fn is_component(&self) -> bool {
        self.source.is_component()
    }

    pub fn component_id(&self) -> Option<&str> {
        self.source.component_id()
    }

    pub fn resolve_params(&self, components: &[ComponentDef]) -> WindowParams {
        self.source.resolve(components)
    }

    pub fn width(&self, components: &[ComponentDef]) -> f64 {
        self.resolve_params(components).opening_width
    }

    pub fn height(&self, components: &[ComponentDef]) -> f64 {
        self.resolve_params(components).opening_height
    }

    pub fn right(&self, components: &[ComponentDef]) -> f64 {
        self.x_from_left + self.width(components)
    }

    pub fn top(&self, components: &[ComponentDef]) -> f64 {
        self.y_from_bottom + self.height(components)
    }

    /// Clamps opening position inside `[0, wall_w] x [0, wall_h]`.
    pub fn clamp_to_wall(&mut self, wall_w: f64, wall_h: f64, components: &[ComponentDef]) {
        let p = self.resolve_params(components);
        let max_x = (wall_w - p.opening_width).max(0.0);
        let max_y = (wall_h - p.opening_height).max(0.0);
        self.x_from_left = self.x_from_left.clamp(0.0, max_x);
        self.y_from_bottom = self.y_from_bottom.clamp(0.0, max_y);
    }
}

/// Reference photo underlay & 3D wall texture configuration (calibrated in millimeters).
/// Uses relative asset paths inside the project folder so projects remain portable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallPhotoConfig {
    /// Relative path inside project directory, e.g. `"assets/photos/front_wall.jpg"`.
    #[serde(default, alias = "image_path")]
    pub asset_path: Option<String>,
    #[serde(default = "default_true")]
    pub show_in_blueprint: bool,
    #[serde(default = "default_true")]
    pub show_in_3d: bool,
    #[serde(default = "default_photo_opacity")]
    pub opacity: f32,
    /// Left edge of photo in wall coordinates (mm from wall left edge).
    pub offset_x_mm: f64,
    /// Bottom edge of photo in wall coordinates (mm from wall floor Y = 0).
    pub offset_y_mm: f64,
    /// Calibrated physical width of the photo (mm).
    pub width_mm: f64,
    /// Calibrated physical height of the photo (mm).
    pub height_mm: f64,
    /// Native image pixel dimensions `[width_px, height_px]`.
    #[serde(default)]
    pub pixel_size: [u32; 2],
    #[serde(default = "default_true")]
    pub lock_aspect: bool,
    /// Rotation applied to the photo (degrees clockwise; supports 90 deg steps & fine straighten).
    #[serde(default)]
    pub rotation_deg: f64,
    /// Normalized left/right/top/bottom crop fractions (`0.0..0.45`).
    #[serde(default)]
    pub crop_left: f64,
    #[serde(default)]
    pub crop_right: f64,
    #[serde(default)]
    pub crop_top: f64,
    #[serde(default)]
    pub crop_bottom: f64,
    /// Simple horizontal keystone / perspective correction (`-0.35..0.35`).
    #[serde(default)]
    pub perspective_h: f64,
    /// Simple vertical keystone / perspective correction (`-0.35..0.35`).
    #[serde(default)]
    pub perspective_v: f64,
    /// Optional 4-corner perspective correction quad `[TL, TR, BR, BL]` in normalized `[0,1]^2` source UV space.
    #[serde(default)]
    pub perspective_quad: Option<[[f64; 2]; 4]>,
}

fn default_photo_opacity() -> f32 {
    0.72
}

impl Default for WallPhotoConfig {
    fn default() -> Self {
        Self {
            asset_path: None,
            show_in_blueprint: true,
            show_in_3d: true,
            opacity: default_photo_opacity(),
            offset_x_mm: 0.0,
            offset_y_mm: 0.0,
            width_mm: 380.0,
            height_mm: 240.0,
            pixel_size: [0, 0],
            lock_aspect: true,
            rotation_deg: 0.0,
            crop_left: 0.0,
            crop_right: 0.0,
            crop_top: 0.0,
            crop_bottom: 0.0,
            perspective_h: 0.0,
            perspective_v: 0.0,
            perspective_quad: None,
        }
    }
}

impl WallPhotoConfig {
    pub fn has_photo(&self) -> bool {
        self.asset_path.is_some()
    }

    pub fn has_edits(&self) -> bool {
        self.rotation_deg.abs() > 1e-4
            || self.crop_left.abs() > 1e-4
            || self.crop_right.abs() > 1e-4
            || self.crop_top.abs() > 1e-4
            || self.crop_bottom.abs() > 1e-4
            || self.perspective_h.abs() > 1e-4
            || self.perspective_v.abs() > 1e-4
            || self.perspective_quad.is_some()
    }

    pub fn reset_edits(&mut self) {
        self.rotation_deg = 0.0;
        self.crop_left = 0.0;
        self.crop_right = 0.0;
        self.crop_top = 0.0;
        self.crop_bottom = 0.0;
        self.perspective_h = 0.0;
        self.perspective_v = 0.0;
        self.perspective_quad = None;
    }

    pub fn clamp_edits(&mut self) {
        self.crop_left = self.crop_left.clamp(0.0, 0.45);
        self.crop_right = self.crop_right.clamp(0.0, 0.45);
        self.crop_top = self.crop_top.clamp(0.0, 0.45);
        self.crop_bottom = self.crop_bottom.clamp(0.0, 0.45);
        self.perspective_h = self.perspective_h.clamp(-0.35, 0.35);
        self.perspective_v = self.perspective_v.clamp(-0.35, 0.35);
        if !self.rotation_deg.is_finite() {
            self.rotation_deg = 0.0;
        }
    }

    pub fn transform_hash(&self) -> u64 {
        #[inline]
        fn mix_u64(h: &mut u64, b: u64) {
            *h ^= b;
            *h = h.wrapping_mul(0x100000001b3);
        }
        let mut h: u64 = 0xcbf29ce484222325;
        mix_u64(&mut h, self.rotation_deg.to_bits());
        mix_u64(&mut h, self.crop_left.to_bits());
        mix_u64(&mut h, self.crop_right.to_bits());
        mix_u64(&mut h, self.crop_top.to_bits());
        mix_u64(&mut h, self.crop_bottom.to_bits());
        mix_u64(&mut h, self.perspective_h.to_bits());
        mix_u64(&mut h, self.perspective_v.to_bits());
        if let Some(q) = self.perspective_quad {
            mix_u64(&mut h, 1);
            for pt in q {
                mix_u64(&mut h, pt[0].to_bits());
                mix_u64(&mut h, pt[1].to_bits());
            }
        }
        h
    }

    pub fn aspect_ratio(&self) -> f64 {
        if self.pixel_size[0] > 0 && self.pixel_size[1] > 0 {
            self.pixel_size[0] as f64 / self.pixel_size[1] as f64
        } else {
            (self.width_mm / self.height_mm.max(1.0)).max(0.1)
        }
    }

    /// Computes effective source quad `[TL, TR, BR, BL]` in normalized `[0, 1]^2` UV space.
    pub fn effective_quad(&self) -> [[f64; 2]; 4] {
        if let Some(q) = self.perspective_quad {
            return q;
        }
        let mut cl = self.crop_left.clamp(0.0, 0.45);
        let mut cr = 1.0 - self.crop_right.clamp(0.0, 0.45);
        let mut ct = self.crop_top.clamp(0.0, 0.45);
        let mut cb = 1.0 - self.crop_bottom.clamp(0.0, 0.45);
        if cr - cl < 0.10 {
            let m = 0.5 * (cl + cr);
            cl = (m - 0.05).max(0.0);
            cr = (m + 0.05).min(1.0);
        }
        if cb - ct < 0.10 {
            let m = 0.5 * (ct + cb);
            ct = (m - 0.05).max(0.0);
            cb = (m + 0.05).min(1.0);
        }
        let cw = cr - cl;
        let ch = cb - ct;
        let kv = self.perspective_v.clamp(-0.35, 0.35) * cw;
        let kh = self.perspective_h.clamp(-0.35, 0.35) * ch;
        let top_in = kv.max(0.0);
        let bot_in = (-kv).max(0.0);
        let right_in = kh.max(0.0);
        let left_in = (-kh).max(0.0);
        [
            [cl + top_in, ct + left_in],
            [cr - top_in, ct + right_in],
            [cr - bot_in, cb - right_in],
            [cl + bot_in, cb - left_in],
        ]
    }

    /// Fits the photo cleanly onto the given `wall_w x wall_h` area.
    pub fn fit_to_wall(&mut self, wall_w: f64, wall_h: f64) {
        if self.lock_aspect {
            let ar = self.aspect_ratio().max(0.05);
            let fit_h_by_w = wall_w / ar;
            if fit_h_by_w <= wall_h {
                self.width_mm = wall_w;
                self.height_mm = fit_h_by_w.max(10.0);
                self.offset_x_mm = 0.0;
                self.offset_y_mm = (wall_h - self.height_mm) * 0.5;
            } else {
                self.height_mm = wall_h;
                self.width_mm = (wall_h * ar).max(10.0);
                self.offset_x_mm = (wall_w - self.width_mm) * 0.5;
                self.offset_y_mm = 0.0;
            }
        } else {
            self.offset_x_mm = 0.0;
            self.offset_y_mm = 0.0;
            self.width_mm = wall_w.max(10.0);
            self.height_mm = wall_h.max(10.0);
        }
    }

    /// Stretches photo to cover the exact wall box `(0, 0) .. (wall_w, wall_h)`.
    pub fn match_wall_exact(&mut self, wall_w: f64, wall_h: f64) {
        self.offset_x_mm = 0.0;
        self.offset_y_mm = 0.0;
        self.width_mm = wall_w.max(10.0);
        self.height_mm = wall_h.max(10.0);
    }

    /// Scales the calibrated photo so `measured_mm` becomes `target_real_mm`, keeping `anchor_pt` fixed.
    pub fn apply_two_point_scale(
        &mut self,
        measured_mm: f64,
        target_real_mm: f64,
        anchor_pt: [f64; 2],
    ) {
        if measured_mm <= 0.1 || target_real_mm <= 0.1 {
            return;
        }
        let scale = (target_real_mm / measured_mm).clamp(0.02, 50.0);
        self.width_mm = (self.width_mm * scale).clamp(10.0, 5000.0);
        self.height_mm = (self.height_mm * scale).clamp(10.0, 5000.0);
        self.offset_x_mm = anchor_pt[0] - (anchor_pt[0] - self.offset_x_mm) * scale;
        self.offset_y_mm = anchor_pt[1] - (anchor_pt[1] - self.offset_y_mm) * scale;
    }
}

/// A persistent architectural dimension / measurement callout on a `ProjectWall`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallDimension {
    pub id: String,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    #[serde(default = "default_dim_offset")]
    pub offset_mm: f64,
    #[serde(default)]
    pub label: String,
}

fn default_dim_offset() -> f64 {
    10.0
}

impl WallDimension {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64, offset_mm: f64) -> Self {
        Self {
            id: new_id("dim"),
            x0,
            y0,
            x1,
            y1,
            offset_mm,
            label: String::new(),
        }
    }

    pub fn length_mm(&self) -> f64 {
        ((self.x1 - self.x0).powi(2) + (self.y1 - self.y0).powi(2)).sqrt()
    }

    pub fn display_text(&self) -> String {
        self.format_text(DisplayUnit::Mm)
    }

    pub fn format_text(&self, unit: DisplayUnit) -> String {
        let trimmed = self.label.trim();
        let val = unit.format_value(self.length_mm());
        if trimmed.is_empty() {
            val
        } else {
            format!("{trimmed}: {val}")
        }
    }
}

/// A dollhouse wall with shared thickness/acetate settings, placed openings, photo config, and dimensions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectWall {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub wall_thickness: f64,
    pub acetate_thickness: f64,
    #[serde(default = "default_true")]
    pub sync_common_settings: bool,
    #[serde(default)]
    pub placement_3d: WallPlacement3D,
    #[serde(default)]
    pub openings: Vec<WallOpening>,
    #[serde(default)]
    pub photo: WallPhotoConfig,
    #[serde(default)]
    pub dimensions: Vec<WallDimension>,
}

fn default_true() -> bool {
    true
}

impl ProjectWall {
    pub fn new_default(name: impl Into<String>) -> Self {
        Self {
            id: new_id("wall"),
            name: name.into(),
            width: 380.0,
            height: 240.0,
            wall_thickness: 9.0,
            acetate_thickness: 0.40,
            sync_common_settings: true,
            placement_3d: WallPlacement3D::default(),
            openings: Vec::new(),
            photo: WallPhotoConfig::default(),
            dimensions: Vec::new(),
        }
    }

    /// Creates a starter wall with two linked Parlor window instances and one front door.
    pub fn new_sample_facade() -> (Self, ComponentDef) {
        let mut wall = Self::new_default("Front Facade");
        let mut parlor_params = WindowParams::default();
        WindowStyle::GeorgianGrid.apply_style_defaults(&mut parlor_params);
        parlor_params.wall_thickness = wall.wall_thickness;
        parlor_params.acetate_thickness = wall.acetate_thickness;
        let parlor_comp = ComponentDef::new("Parlor Georgian Window", parlor_params);

        let mut door_params = WindowParams::default();
        WindowStyle::DoorColonial4Panel.apply_style_defaults(&mut door_params);
        door_params.wall_thickness = wall.wall_thickness;
        door_params.acetate_thickness = wall.acetate_thickness;

        wall.openings.push(WallOpening::new_instance(
            "Parlor Left",
            48.0,
            62.0,
            parlor_comp.id.clone(),
            InstanceOverrides::default(),
        ));
        wall.openings
            .push(WallOpening::new_unique("Front Entry Door", 152.0, 0.0, door_params));
        wall.openings.push(WallOpening::new_instance(
            "Parlor Right",
            274.0,
            62.0,
            parlor_comp.id.clone(),
            InstanceOverrides::default(),
        ));
        (wall, parlor_comp)
    }

    pub fn effective_params(
        &self,
        opening: &WallOpening,
        components: &[ComponentDef],
    ) -> WindowParams {
        opening.resolve_params(components)
    }

    /// Applies the wall's common `wall_thickness` and `acetate_thickness` to all openings
    /// (both unique openings and per-instance overrides on linked components).
    pub fn apply_common_settings_to_openings(&mut self) {
        self.wall_thickness = self.wall_thickness.clamp(3.0, 35.0);
        self.acetate_thickness = self.acetate_thickness.clamp(0.1, 5.0);
        for op in &mut self.openings {
            match &mut op.source {
                OpeningSource::Unique { params } => {
                    params.wall_thickness = self.wall_thickness;
                    params.acetate_thickness = self.acetate_thickness;
                }
                OpeningSource::Component { overrides, .. } => {
                    overrides.wall_thickness = Some(self.wall_thickness);
                    overrides.acetate_thickness = Some(self.acetate_thickness);
                }
            }
        }
    }

    /// Centers the bounding group of selected (unlocked) openings horizontally on the wall.
    pub fn center_openings_horizontally(
        &mut self,
        ids: &HashSet<String>,
        components: &[ComponentDef],
    ) {
        let indices: Vec<usize> = self
            .openings
            .iter()
            .enumerate()
            .filter(|(_, o)| ids.contains(&o.id) && !o.lock_position)
            .map(|(i, _)| i)
            .collect();
        if indices.is_empty() {
            return;
        }
        let min_x = indices
            .iter()
            .map(|&i| self.openings[i].x_from_left)
            .fold(f64::INFINITY, f64::min);
        let max_x = indices
            .iter()
            .map(|&i| self.openings[i].right(components))
            .fold(f64::NEG_INFINITY, f64::max);
        let group_center = (min_x + max_x) * 0.5;
        let dx = self.width * 0.5 - group_center;
        for i in indices {
            self.openings[i].x_from_left += dx;
            self.openings[i].clamp_to_wall(self.width, self.height, components);
        }
    }

    /// Evenly distributes selected openings horizontally across their outer span (for 3+)
    /// or across the wall width (for 2).
    pub fn distribute_openings_horizontally(
        &mut self,
        ids: &HashSet<String>,
        components: &[ComponentDef],
    ) {
        let mut indices: Vec<usize> = self
            .openings
            .iter()
            .enumerate()
            .filter(|(_, o)| ids.contains(&o.id))
            .map(|(i, _)| i)
            .collect();
        if indices.len() < 2 {
            return;
        }
        indices.sort_by(|&a, &b| {
            self.openings[a]
                .x_from_left
                .partial_cmp(&self.openings[b].x_from_left)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total_w: f64 = indices
            .iter()
            .map(|&i| self.openings[i].width(components))
            .sum();

        if indices.len() == 2 {
            let gap = ((self.width - total_w) / 3.0).max(0.0);
            let mut x = gap;
            for i in indices {
                let w = self.openings[i].width(components);
                if !self.openings[i].lock_position {
                    self.openings[i].x_from_left = x;
                    self.openings[i].clamp_to_wall(self.width, self.height, components);
                }
                x += w + gap;
            }
            return;
        }

        let first_x = self.openings[indices[0]].x_from_left;
        let last_r = self.openings[*indices.last().unwrap()].right(components);
        let span = (last_r - first_x).max(total_w);
        let gap = (span - total_w) / (indices.len() as f64 - 1.0);
        let mut cursor = first_x;
        for &i in &indices {
            let w = self.openings[i].width(components);
            if !self.openings[i].lock_position {
                self.openings[i].x_from_left = cursor;
                self.openings[i].clamp_to_wall(self.width, self.height, components);
            }
            cursor += w + gap;
        }
    }

    /// Mirrors selected unlocked openings across the wall's horizontal center axis.
    pub fn mirror_openings_horizontally(
        &mut self,
        ids: &HashSet<String>,
        components: &[ComponentDef],
    ) {
        for op in &mut self.openings {
            if ids.contains(&op.id) && !op.lock_position {
                let resolved = op.resolve_params(components);
                op.x_from_left = self.width - op.x_from_left - resolved.opening_width;
                if resolved.style.is_door() && !resolved.double_doors {
                    match &mut op.source {
                        OpeningSource::Unique { params } => {
                            params.hinge_right = !params.hinge_right;
                        }
                        OpeningSource::Component { overrides, .. } => {
                            let cur = overrides.hinge_right.unwrap_or(resolved.hinge_right);
                            overrides.hinge_right = Some(!cur);
                        }
                    }
                }
                op.clamp_to_wall(self.width, self.height, components);
            }
        }
    }

    /// Generates architectural horizontal chain dimensions and vertical sill/height callouts.
    pub fn generate_auto_dimensions(&mut self, components: &[ComponentDef]) {
        self.dimensions.clear();

        if self.openings.is_empty() {
            self.dimensions
                .push(WallDimension::new(0.0, self.height, self.width, self.height, 16.0));
            self.dimensions
                .push(WallDimension::new(self.width, 0.0, self.width, self.height, -16.0));
            return;
        }

        let mut sorted: Vec<(&WallOpening, WindowParams)> = self
            .openings
            .iter()
            .map(|op| (op, op.resolve_params(components)))
            .collect();
        sorted.sort_by(|a, b| {
            a.0.x_from_left
                .partial_cmp(&b.0.x_from_left)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut cursor_x = 0.0f64;
        for (op, params) in &sorted {
            let op_r = op.x_from_left + params.opening_width;
            if op.x_from_left > cursor_x + 0.5 {
                self.dimensions.push(WallDimension::new(
                    cursor_x,
                    self.height,
                    op.x_from_left,
                    self.height,
                    16.0,
                ));
            }
            self.dimensions.push(WallDimension::new(
                op.x_from_left,
                self.height,
                op_r,
                self.height,
                16.0,
            ));
            cursor_x = cursor_x.max(op_r);
        }
        if self.width > cursor_x + 0.5 {
            self.dimensions.push(WallDimension::new(
                cursor_x,
                self.height,
                self.width,
                self.height,
                16.0,
            ));
        }

        let mut seen_bands: Vec<(i64, i64)> = Vec::new();
        for (op, params) in &sorted {
            let band_key = (
                (op.y_from_bottom * 2.0).round() as i64,
                (params.opening_height * 2.0).round() as i64,
            );
            if seen_bands.contains(&band_key) {
                continue;
            }
            seen_bands.push(band_key);

            let vx = op.x_from_left + params.opening_width;
            if op.y_from_bottom > 1.0 {
                let mut sill_dim = WallDimension::new(vx, 0.0, vx, op.y_from_bottom, -7.0);
                sill_dim.label = "Sill".to_string();
                self.dimensions.push(sill_dim);
            }
            self.dimensions.push(WallDimension::new(
                vx,
                op.y_from_bottom,
                vx,
                op.y_from_bottom + params.opening_height,
                -7.0,
            ));
        }
    }
}
