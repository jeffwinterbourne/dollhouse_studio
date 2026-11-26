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

    /// Bounding pixel size after applying clockwise `rotation_deg`.
    pub fn oriented_pixel_size(&self) -> (f64, f64) {
        let (pw, ph) = if self.pixel_size[0] > 0 && self.pixel_size[1] > 0 {
            (self.pixel_size[0] as f64, self.pixel_size[1] as f64)
        } else {
            (self.width_mm.max(10.0), self.height_mm.max(10.0))
        };
        let rad = self.rotation_deg.to_radians();
        let (sin_r, cos_r) = rad.sin_cos();
        (
            (pw * cos_r.abs() + ph * sin_r.abs()).max(1.0),
            (pw * sin_r.abs() + ph * cos_r.abs()).max(1.0),
        )
    }

    /// Aspect ratio of the FINAL processed texture after rotation, crop, and perspective rectification.
    pub fn processed_aspect_ratio(&self) -> f64 {
        let (ow, oh) = self.oriented_pixel_size();
        let [tl, tr, br, bl] = self.effective_quad();
        let dist =
            |a: [f64; 2], b: [f64; 2]| -> f64 { ((b[0] - a[0]) * ow).hypot((b[1] - a[1]) * oh) };
        let rw = (0.5 * (dist(tl, tr) + dist(bl, br))).max(1.0);
        let rh = (0.5 * (dist(tl, bl) + dist(tr, br))).max(1.0);
        (rw / rh).clamp(0.05, 20.0)
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

    /// Fits the processed (rotated/cropped/rectified) photo cleanly onto `wall_w x wall_h`.
    pub fn fit_to_wall(&mut self, wall_w: f64, wall_h: f64) {
        if self.lock_aspect {
            let ar = self.processed_aspect_ratio().max(0.05);
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

    /// Scales the calibrated photo uniformly so `measured_mm` becomes `target_real_mm`, keeping `anchor_pt` fixed.
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
        self.apply_calibrated_scale_xy(scale, scale, anchor_pt);
    }

    /// Scales the calibrated photo with independent horizontal & vertical multipliers around `anchor_pt`.
    pub fn apply_calibrated_scale_xy(&mut self, scale_x: f64, scale_y: f64, anchor_pt: [f64; 2]) {
        let sx = scale_x.clamp(0.02, 50.0);
        let sy = scale_y.clamp(0.02, 50.0);
        self.width_mm = (self.width_mm * sx).clamp(10.0, 5000.0);
        self.height_mm = (self.height_mm * sy).clamp(10.0, 5000.0);
        self.offset_x_mm = anchor_pt[0] - (anchor_pt[0] - self.offset_x_mm) * sx;
        self.offset_y_mm = anchor_pt[1] - (anchor_pt[1] - self.offset_y_mm) * sy;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallEdgeKind {
    Left,
    Right,
    Bottom,
    Top,
    CenterX,
    CenterY,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CornerKind {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpeningEdgeKind {
    Left,
    Right,
    Bottom,
    Top,
    CenterX,
    CenterY,
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// CAD geometry anchor attached to each end of a `WallDimension`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DimAnchor {
    Point {
        x: f64,
        y: f64,
    },
    WallEdge {
        edge: WallEdgeKind,
    },
    WallCorner {
        corner: CornerKind,
    },
    OpeningEdge {
        opening_id: String,
        edge: OpeningEdgeKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DimOrientation {
    #[default]
    Auto,
    Horizontal,
    Vertical,
    Aligned,
}

/// A persistent associative architectural dimension / constraint callout on a `ProjectWall`.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_a: Option<DimAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_b: Option<DimAnchor>,
    #[serde(default)]
    pub orientation: DimOrientation,
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
            anchor_a: None,
            anchor_b: None,
            orientation: DimOrientation::Auto,
        }
    }

    pub fn new_anchored(
        anchor_a: DimAnchor,
        anchor_b: DimAnchor,
        orientation: DimOrientation,
        offset_mm: f64,
        p0: [f64; 2],
        p1: [f64; 2],
    ) -> Self {
        Self {
            id: new_id("dim"),
            x0: p0[0],
            y0: p0[1],
            x1: p1[0],
            y1: p1[1],
            offset_mm,
            label: String::new(),
            anchor_a: Some(anchor_a),
            anchor_b: Some(anchor_b),
            orientation,
        }
    }

    pub fn is_anchored(&self) -> bool {
        self.anchor_a.is_some() || self.anchor_b.is_some()
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
        wall.openings.push(WallOpening::new_unique(
            "Front Entry Door",
            152.0,
            0.0,
            door_params,
        ));
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
        self.acetate_thickness = self.acetate_thickness.clamp(0.1, 20.0);
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

    /// Resolves current `[p0, p1]` coordinates in mm for `dim`, tracking anchored wall/opening geometry.
    pub fn resolve_dimension_endpoints(
        &self,
        dim: &WallDimension,
        components: &[ComponentDef],
    ) -> ([f64; 2], [f64; 2]) {
        let p0 = dim
            .anchor_a
            .as_ref()
            .and_then(|a| {
                self.resolve_anchor(a, dim.anchor_b.as_ref(), [dim.x0, dim.y0], components)
            })
            .unwrap_or([dim.x0, dim.y0]);
        let p1 = dim
            .anchor_b
            .as_ref()
            .and_then(|b| {
                self.resolve_anchor(b, dim.anchor_a.as_ref(), [dim.x1, dim.y1], components)
            })
            .unwrap_or([dim.x1, dim.y1]);

        match dim.orientation {
            DimOrientation::Horizontal => (p0, [p1[0], p0[1]]),
            DimOrientation::Vertical => (p0, [p0[0], p1[1]]),
            DimOrientation::Auto | DimOrientation::Aligned => (p0, p1),
        }
    }

    fn resolve_anchor(
        &self,
        anchor: &DimAnchor,
        other: Option<&DimAnchor>,
        fallback: [f64; 2],
        components: &[ComponentDef],
    ) -> Option<[f64; 2]> {
        let other_pt = other.map(|o| match o {
            DimAnchor::Point { x, y } => [*x, *y],
            DimAnchor::WallCorner { corner } => self.corner_pt(*corner),
            DimAnchor::OpeningEdge { opening_id, edge } => self
                .openings
                .iter()
                .find(|op| &op.id == opening_id)
                .map(|op| self.opening_edge_pt(op, *edge, None, components))
                .unwrap_or(fallback),
            DimAnchor::WallEdge { edge } => match edge {
                WallEdgeKind::Left => [0.0, fallback[1]],
                WallEdgeKind::Right => [self.width, fallback[1]],
                WallEdgeKind::CenterX => [self.width * 0.5, fallback[1]],
                WallEdgeKind::Bottom => [fallback[0], 0.0],
                WallEdgeKind::Top => [fallback[0], self.height],
                WallEdgeKind::CenterY => [fallback[0], self.height * 0.5],
            },
        });
        let ref_pt = other_pt.unwrap_or(fallback);
        match anchor {
            DimAnchor::Point { x, y } => Some([*x, *y]),
            DimAnchor::WallCorner { corner } => Some(self.corner_pt(*corner)),
            DimAnchor::WallEdge { edge } => Some(match edge {
                WallEdgeKind::Left => [0.0, ref_pt[1]],
                WallEdgeKind::Right => [self.width, ref_pt[1]],
                WallEdgeKind::CenterX => [self.width * 0.5, ref_pt[1]],
                WallEdgeKind::Bottom => [ref_pt[0], 0.0],
                WallEdgeKind::Top => [ref_pt[0], self.height],
                WallEdgeKind::CenterY => [ref_pt[0], self.height * 0.5],
            }),
            DimAnchor::OpeningEdge { opening_id, edge } => {
                let op = self.openings.iter().find(|o| &o.id == opening_id)?;
                Some(self.opening_edge_pt(op, *edge, other_pt, components))
            }
        }
    }

    fn corner_pt(&self, corner: CornerKind) -> [f64; 2] {
        match corner {
            CornerKind::BottomLeft => [0.0, 0.0],
            CornerKind::BottomRight => [self.width, 0.0],
            CornerKind::TopLeft => [0.0, self.height],
            CornerKind::TopRight => [self.width, self.height],
        }
    }

    fn opening_edge_pt(
        &self,
        op: &WallOpening,
        edge: OpeningEdgeKind,
        other_pt: Option<[f64; 2]>,
        components: &[ComponentDef],
    ) -> [f64; 2] {
        let w = op.width(components);
        let h = op.height(components);
        let l = op.x_from_left;
        let r = l + w;
        let b = op.y_from_bottom;
        let t = b + h;
        let mid_x = l + w * 0.5;
        let mid_y = b + h * 0.5;
        match edge {
            OpeningEdgeKind::BottomLeft => [l, b],
            OpeningEdgeKind::BottomRight => [r, b],
            OpeningEdgeKind::TopLeft => [l, t],
            OpeningEdgeKind::TopRight => [r, t],
            OpeningEdgeKind::Left => [l, other_pt.map_or(mid_y, |p| p[1].clamp(b, t))],
            OpeningEdgeKind::Right => [r, other_pt.map_or(mid_y, |p| p[1].clamp(b, t))],
            OpeningEdgeKind::CenterX => [mid_x, other_pt.map_or(mid_y, |p| p[1].clamp(b, t))],
            OpeningEdgeKind::Bottom => [other_pt.map_or(mid_x, |p| p[0].clamp(l, r)), b],
            OpeningEdgeKind::Top => [other_pt.map_or(mid_x, |p| p[0].clamp(l, r)), t],
            OpeningEdgeKind::CenterY => [other_pt.map_or(mid_x, |p| p[0].clamp(l, r)), mid_y],
        }
    }

    /// Synchronizes stored `(x0, y0, x1, y1)` coordinates for all anchored dimensions
    /// so canvas rendering and PDF/SVG exports reflect live opening and wall geometry.
    pub fn sync_anchored_dimensions(&mut self, components: &[ComponentDef]) {
        let resolved: Vec<([f64; 2], [f64; 2])> = self
            .dimensions
            .iter()
            .map(|d| self.resolve_dimension_endpoints(d, components))
            .collect();
        for (dim, (p0, p1)) in self.dimensions.iter_mut().zip(resolved) {
            if dim.is_anchored() {
                dim.x0 = p0[0];
                dim.y0 = p0[1];
                dim.x1 = p1[0];
                dim.y1 = p1[1];
            }
        }
    }

    /// Uses a linked architectural dimension as a parametric driver: setting its target length
    /// resizes or repositions the linked wall or opening accurately.
    pub fn apply_driving_dimension(
        &mut self,
        dim_id: &str,
        target_mm: f64,
        components: &mut [ComponentDef],
    ) -> Result<&'static str, String> {
        let target = target_mm.max(0.5);
        let Some(dim) = self.dimensions.iter().find(|d| d.id == dim_id).cloned() else {
            return Err("Dimension not found".to_string());
        };
        let (Some(a), Some(b)) = (dim.anchor_a.clone(), dim.anchor_b.clone()) else {
            return Err(
                "Unanchored dimension cannot drive model geometry (anchor endpoints to wall or opening edges).".to_string(),
            );
        };

        let result = match (a, b) {
            (
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Left,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomLeft | CornerKind::TopLeft,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Right,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomRight | CornerKind::TopRight,
                },
            )
            | (
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Right,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomRight | CornerKind::TopRight,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Left,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomLeft | CornerKind::TopLeft,
                },
            ) => {
                self.width = target.clamp(60.0, 2000.0);
                for op in &mut self.openings {
                    op.clamp_to_wall(self.width, self.height, components);
                }
                Ok("Drive wall width")
            }
            (
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Bottom,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomLeft | CornerKind::BottomRight,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Top,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::TopLeft | CornerKind::TopRight,
                },
            )
            | (
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Top,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::TopLeft | CornerKind::TopRight,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Bottom,
                }
                | DimAnchor::WallCorner {
                    corner: CornerKind::BottomLeft | CornerKind::BottomRight,
                },
            ) => {
                self.height = target.clamp(60.0, 1500.0);
                for op in &mut self.openings {
                    op.clamp_to_wall(self.width, self.height, components);
                }
                Ok("Drive wall height")
            }
            (
                DimAnchor::OpeningEdge {
                    opening_id: id_a,
                    edge: ea,
                },
                DimAnchor::OpeningEdge {
                    opening_id: id_b,
                    edge: eb,
                },
            ) if id_a == id_b => {
                let horiz =
                    is_left_edge(ea) && is_right_edge(eb) || is_right_edge(ea) && is_left_edge(eb);
                let vert =
                    is_bottom_edge(ea) && is_top_edge(eb) || is_top_edge(ea) && is_bottom_edge(eb);
                if !horiz && !vert {
                    return Err(
                        "Select opposite edges of the opening to drive its width or height."
                            .to_string(),
                    );
                }
                let val = target.clamp(18.0, if horiz { self.width } else { self.height });
                self.set_opening_extent(&id_a, horiz, val, components)?;
                Ok(if horiz {
                    "Drive opening width"
                } else {
                    "Drive opening height"
                })
            }
            (
                DimAnchor::WallEdge { edge: we },
                DimAnchor::OpeningEdge {
                    opening_id,
                    edge: oe,
                },
            )
            | (
                DimAnchor::OpeningEdge {
                    opening_id,
                    edge: oe,
                },
                DimAnchor::WallEdge { edge: we },
            ) => {
                self.drive_wall_to_opening_clearance(we, &opening_id, oe, target, components)?;
                Ok("Drive opening clearance")
            }
            (
                DimAnchor::WallCorner { corner },
                DimAnchor::OpeningEdge {
                    opening_id,
                    edge: oe,
                },
            )
            | (
                DimAnchor::OpeningEdge {
                    opening_id,
                    edge: oe,
                },
                DimAnchor::WallCorner { corner },
            ) => {
                let is_h = is_left_edge(oe) || is_right_edge(oe) || oe == OpeningEdgeKind::CenterX;
                let we = match corner {
                    CornerKind::BottomLeft => {
                        if is_h {
                            WallEdgeKind::Left
                        } else {
                            WallEdgeKind::Bottom
                        }
                    }
                    CornerKind::BottomRight => {
                        if is_h {
                            WallEdgeKind::Right
                        } else {
                            WallEdgeKind::Bottom
                        }
                    }
                    CornerKind::TopLeft => {
                        if is_h {
                            WallEdgeKind::Left
                        } else {
                            WallEdgeKind::Top
                        }
                    }
                    CornerKind::TopRight => {
                        if is_h {
                            WallEdgeKind::Right
                        } else {
                            WallEdgeKind::Top
                        }
                    }
                };
                self.drive_wall_to_opening_clearance(we, &opening_id, oe, target, components)?;
                Ok("Drive opening clearance")
            }
            (
                DimAnchor::OpeningEdge {
                    opening_id: id_a,
                    edge: ea,
                },
                DimAnchor::OpeningEdge {
                    opening_id: id_b,
                    edge: eb,
                },
            ) => {
                self.drive_inter_opening_spacing(&id_a, ea, &id_b, eb, target, components)?;
                Ok("Drive spacing between openings")
            }
            _ => Err(
                "This anchor pair is read-only; anchor to wall or opening edges to drive geometry."
                    .to_string(),
            ),
        };

        if result.is_ok() {
            self.sync_anchored_dimensions(components);
        }
        result
    }

    fn set_opening_extent(
        &mut self,
        opening_id: &str,
        horiz: bool,
        new_val: f64,
        components: &mut [ComponentDef],
    ) -> Result<(), String> {
        let Some(op) = self.openings.iter_mut().find(|o| o.id == opening_id) else {
            return Err("Opening not found".to_string());
        };
        if (horiz && op.lock_width) || (!horiz && op.lock_height) {
            return Err("Opening dimension is locked.".to_string());
        }
        match &mut op.source {
            OpeningSource::Unique { params } => {
                if horiz {
                    params.opening_width = new_val;
                } else {
                    params.opening_height = new_val;
                }
            }
            OpeningSource::Component { component_id, .. } => {
                if let Some(comp) = components.iter_mut().find(|c| c.id == *component_id) {
                    if horiz {
                        comp.params.opening_width = new_val;
                    } else {
                        comp.params.opening_height = new_val;
                    }
                }
            }
        }
        let ww = self.width;
        let wh = self.height;
        if let Some(op) = self.openings.iter_mut().find(|o| o.id == opening_id) {
            op.clamp_to_wall(ww, wh, components);
        }
        Ok(())
    }

    fn drive_wall_to_opening_clearance(
        &mut self,
        we: WallEdgeKind,
        opening_id: &str,
        oe: OpeningEdgeKind,
        target: f64,
        components: &[ComponentDef],
    ) -> Result<(), String> {
        let Some(idx) = self.openings.iter().position(|o| o.id == opening_id) else {
            return Err("Opening not found".to_string());
        };
        if self.openings[idx].lock_position {
            return Err("Opening position is locked.".to_string());
        }
        let w = self.openings[idx].width(components);
        let h = self.openings[idx].height(components);
        let off_x = if is_right_edge(oe) {
            w
        } else if oe == OpeningEdgeKind::CenterX {
            w * 0.5
        } else {
            0.0
        };
        let off_y = if is_top_edge(oe) {
            h
        } else if oe == OpeningEdgeKind::CenterY {
            h * 0.5
        } else {
            0.0
        };
        match we {
            WallEdgeKind::Left => {
                self.openings[idx].x_from_left = target - off_x;
            }
            WallEdgeKind::Right => {
                self.openings[idx].x_from_left = (self.width - target) - off_x;
            }
            WallEdgeKind::CenterX => {
                let sign = if self.openings[idx].x_from_left + w * 0.5 >= self.width * 0.5 {
                    1.0
                } else {
                    -1.0
                };
                self.openings[idx].x_from_left = (self.width * 0.5 + sign * target) - off_x;
            }
            WallEdgeKind::Bottom => {
                self.openings[idx].y_from_bottom = target - off_y;
            }
            WallEdgeKind::Top => {
                self.openings[idx].y_from_bottom = (self.height - target) - off_y;
            }
            WallEdgeKind::CenterY => {
                let sign = if self.openings[idx].y_from_bottom + h * 0.5 >= self.height * 0.5 {
                    1.0
                } else {
                    -1.0
                };
                self.openings[idx].y_from_bottom = (self.height * 0.5 + sign * target) - off_y;
            }
        }
        self.openings[idx].clamp_to_wall(self.width, self.height, components);
        Ok(())
    }

    fn drive_inter_opening_spacing(
        &mut self,
        id_a: &str,
        ea: OpeningEdgeKind,
        id_b: &str,
        eb: OpeningEdgeKind,
        target: f64,
        components: &[ComponentDef],
    ) -> Result<(), String> {
        let Some(ia) = self.openings.iter().position(|o| o.id == id_a) else {
            return Err("Opening A not found".to_string());
        };
        let Some(ib) = self.openings.iter().position(|o| o.id == id_b) else {
            return Err("Opening B not found".to_string());
        };
        let (fixed_idx, fixed_e, move_idx, move_e) = if !self.openings[ib].lock_position {
            (ia, ea, ib, eb)
        } else if !self.openings[ia].lock_position {
            (ib, eb, ia, ea)
        } else {
            return Err("Both openings are position-locked.".to_string());
        };

        let fp = self.opening_edge_pt(&self.openings[fixed_idx], fixed_e, None, components);
        let mw = self.openings[move_idx].width(components);
        let mh = self.openings[move_idx].height(components);
        let mp_cur = self.opening_edge_pt(&self.openings[move_idx], move_e, None, components);

        if (mp_cur[0] - fp[0]).abs() >= (mp_cur[1] - fp[1]).abs() {
            let dir = if mp_cur[0] >= fp[0] { 1.0 } else { -1.0 };
            let desired_edge_x = fp[0] + dir * target;
            let off_x = if is_right_edge(move_e) {
                mw
            } else if move_e == OpeningEdgeKind::CenterX {
                mw * 0.5
            } else {
                0.0
            };
            self.openings[move_idx].x_from_left = desired_edge_x - off_x;
        } else {
            let dir = if mp_cur[1] >= fp[1] { 1.0 } else { -1.0 };
            let desired_edge_y = fp[1] + dir * target;
            let off_y = if is_top_edge(move_e) {
                mh
            } else if move_e == OpeningEdgeKind::CenterY {
                mh * 0.5
            } else {
                0.0
            };
            self.openings[move_idx].y_from_bottom = desired_edge_y - off_y;
        }
        self.openings[move_idx].clamp_to_wall(self.width, self.height, components);
        Ok(())
    }

    /// Generates associative architectural horizontal chain dimensions and vertical sill/height callouts.
    pub fn generate_auto_dimensions(&mut self, components: &[ComponentDef]) {
        self.dimensions.clear();

        if self.openings.is_empty() {
            self.dimensions.push(WallDimension::new_anchored(
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Left,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Right,
                },
                DimOrientation::Horizontal,
                16.0,
                [0.0, self.height],
                [self.width, self.height],
            ));
            self.dimensions.push(WallDimension::new_anchored(
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Bottom,
                },
                DimAnchor::WallEdge {
                    edge: WallEdgeKind::Top,
                },
                DimOrientation::Vertical,
                -16.0,
                [self.width, 0.0],
                [self.width, self.height],
            ));
            return;
        }

        let mut sorted: Vec<(String, f64, f64, f64, f64)> = self
            .openings
            .iter()
            .map(|op| {
                let p = op.resolve_params(components);
                (
                    op.id.clone(),
                    op.x_from_left,
                    op.y_from_bottom,
                    p.opening_width,
                    p.opening_height,
                )
            })
            .collect();
        sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut prev_anchor = DimAnchor::WallCorner {
            corner: CornerKind::TopLeft,
        };
        let mut cursor_x = 0.0f64;

        for (op_id, op_x, _op_y, op_w, _op_h) in &sorted {
            let op_r = op_x + op_w;
            if *op_x > cursor_x + 0.5 {
                self.dimensions.push(WallDimension::new_anchored(
                    prev_anchor.clone(),
                    DimAnchor::OpeningEdge {
                        opening_id: op_id.clone(),
                        edge: OpeningEdgeKind::TopLeft,
                    },
                    DimOrientation::Horizontal,
                    16.0,
                    [cursor_x, self.height],
                    [*op_x, self.height],
                ));
            }
            self.dimensions.push(WallDimension::new_anchored(
                DimAnchor::OpeningEdge {
                    opening_id: op_id.clone(),
                    edge: OpeningEdgeKind::TopLeft,
                },
                DimAnchor::OpeningEdge {
                    opening_id: op_id.clone(),
                    edge: OpeningEdgeKind::TopRight,
                },
                DimOrientation::Horizontal,
                16.0,
                [*op_x, self.height],
                [op_r, self.height],
            ));
            if op_r >= cursor_x {
                cursor_x = op_r;
                prev_anchor = DimAnchor::OpeningEdge {
                    opening_id: op_id.clone(),
                    edge: OpeningEdgeKind::TopRight,
                };
            }
        }
        if self.width > cursor_x + 0.5 {
            self.dimensions.push(WallDimension::new_anchored(
                prev_anchor,
                DimAnchor::WallCorner {
                    corner: CornerKind::TopRight,
                },
                DimOrientation::Horizontal,
                16.0,
                [cursor_x, self.height],
                [self.width, self.height],
            ));
        }

        let mut seen_bands: Vec<(i64, i64)> = Vec::new();
        for (op_id, op_x, op_y, op_w, op_h) in &sorted {
            let band_key = ((*op_y * 2.0).round() as i64, (*op_h * 2.0).round() as i64);
            if seen_bands.contains(&band_key) {
                continue;
            }
            seen_bands.push(band_key);

            let vx = op_x + op_w;
            if *op_y > 1.0 {
                let mut sill_dim = WallDimension::new_anchored(
                    DimAnchor::WallEdge {
                        edge: WallEdgeKind::Bottom,
                    },
                    DimAnchor::OpeningEdge {
                        opening_id: op_id.clone(),
                        edge: OpeningEdgeKind::BottomRight,
                    },
                    DimOrientation::Vertical,
                    -7.0,
                    [vx, 0.0],
                    [vx, *op_y],
                );
                sill_dim.label = "Sill".to_string();
                self.dimensions.push(sill_dim);
            }
            self.dimensions.push(WallDimension::new_anchored(
                DimAnchor::OpeningEdge {
                    opening_id: op_id.clone(),
                    edge: OpeningEdgeKind::BottomRight,
                },
                DimAnchor::OpeningEdge {
                    opening_id: op_id.clone(),
                    edge: OpeningEdgeKind::TopRight,
                },
                DimOrientation::Vertical,
                -7.0,
                [vx, *op_y],
                [vx, op_y + op_h],
            ));
        }
        self.sync_anchored_dimensions(components);
    }
}

fn is_left_edge(e: OpeningEdgeKind) -> bool {
    matches!(
        e,
        OpeningEdgeKind::Left | OpeningEdgeKind::BottomLeft | OpeningEdgeKind::TopLeft
    )
}

fn is_right_edge(e: OpeningEdgeKind) -> bool {
    matches!(
        e,
        OpeningEdgeKind::Right | OpeningEdgeKind::BottomRight | OpeningEdgeKind::TopRight
    )
}

fn is_bottom_edge(e: OpeningEdgeKind) -> bool {
    matches!(
        e,
        OpeningEdgeKind::Bottom | OpeningEdgeKind::BottomLeft | OpeningEdgeKind::BottomRight
    )
}

fn is_top_edge(e: OpeningEdgeKind) -> bool {
    matches!(
        e,
        OpeningEdgeKind::Top | OpeningEdgeKind::TopLeft | OpeningEdgeKind::TopRight
    )
}
