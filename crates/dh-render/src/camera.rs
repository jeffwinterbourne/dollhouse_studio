use dh_core::ViewMode;
use dh_geometry::GeneratedWindow;
use eframe::egui;
use glam::{Mat4, Vec3, Vec4};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraPreset {
    Isometric,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
struct CameraGoal {
    yaw: f32,
    pitch: f32,
}

/// Interactive spherical orbit camera (Z-up world) with smooth ViewCube interpolation.
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    goal: Option<CameraGoal>,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: -0.65,
            pitch: 0.42,
            distance: 175.0,
            goal: None,
        }
    }
}

impl OrbitCamera {
    pub fn preset_angles(preset: CameraPreset) -> (f32, f32) {
        match preset {
            CameraPreset::Isometric => (-0.62, 0.36),
            CameraPreset::Front => (-FRAC_PI_2, 0.04),
            CameraPreset::Back => (FRAC_PI_2, 0.04),
            CameraPreset::Right => (0.0, 0.04),
            CameraPreset::Left => (PI, 0.04),
            CameraPreset::Top => (-FRAC_PI_2, 1.48),
            CameraPreset::Bottom => (-FRAC_PI_2, -1.48),
        }
    }

    pub fn set_preset(&mut self, preset: CameraPreset) {
        let (yaw, pitch) = Self::preset_angles(preset);
        self.yaw = yaw;
        self.pitch = pitch;
        self.goal = None;
    }

    pub fn animate_to_preset(&mut self, preset: CameraPreset) {
        let (yaw, pitch) = Self::preset_angles(preset);
        self.goal = Some(CameraGoal { yaw, pitch });
    }

    pub fn animate_to_angles(&mut self, yaw: f32, pitch: f32) {
        self.goal = Some(CameraGoal {
            yaw,
            pitch: pitch.clamp(-1.50, 1.50),
        });
    }

    /// Advances smooth animation towards `goal`; returns `true` while still animating.
    pub fn step_animation(&mut self, dt: f32) -> bool {
        let Some(g) = self.goal else {
            return false;
        };
        let alpha = (dt * 12.0).clamp(0.08, 0.65);
        let dy = shortest_angle_diff(self.yaw, g.yaw);
        let dp = g.pitch - self.pitch;
        if dy.abs() < 0.005 && dp.abs() < 0.005 {
            self.yaw = g.yaw;
            self.pitch = g.pitch;
            self.goal = None;
            false
        } else {
            self.yaw += dy * alpha;
            self.pitch += dp * alpha;
            true
        }
    }

    pub fn reset_for_mode(&mut self, mode: ViewMode, window: &GeneratedWindow) {
        let span = window
            .params
            .opening_width
            .max(window.params.opening_height) as f32;
        self.goal = None;
        match mode {
            ViewMode::PrintPlate => {
                self.target = Vec3::new(0.0, 0.0, 2.0);
                self.yaw = -0.55;
                self.pitch = 0.72;
                self.distance = (span * 2.4).max(150.0);
            }
            ViewMode::Exploded => {
                self.target = Vec3::ZERO;
                self.yaw = -0.62;
                self.pitch = 0.30;
                self.distance = (span * 2.0).max(145.0);
            }
            ViewMode::Assembled => {
                self.target = Vec3::ZERO;
                self.yaw = -0.58;
                self.pitch = 0.28;
                self.distance = (span * 1.9).max(135.0);
            }
            ViewMode::ExteriorOnly | ViewMode::InteriorOnly => {
                self.target = Vec3::new(0.0, 0.0, 2.0);
                self.yaw = -0.55;
                self.pitch = 0.68;
                self.distance = (span * 1.8).max(125.0);
            }
        }
    }

    pub fn reset_for_project_wall(&mut self, wall_w: f32, wall_h: f32) {
        let span = wall_w.max(wall_h);
        self.target = Vec3::ZERO;
        self.yaw = -0.78;
        self.pitch = 0.24;
        self.distance = (span * 1.55).max(180.0);
        self.goal = None;
    }

    pub fn eye(&self) -> Vec3 {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        self.target + Vec3::new(cp * cy, cp * sy, sp) * self.distance
    }

    pub fn view_matrix(&self) -> Mat4 {
        look_at_rh_gl(self.eye(), self.target, Vec3::Z)
    }

    pub fn projection_matrix(&self, aspect: f32, z_far: f32) -> Mat4 {
        perspective_rh_gl(40.0f32.to_radians(), aspect.max(0.01), 1.0, z_far)
    }

    pub fn handle_input(&mut self, response: &egui::Response, ui: &egui::Ui, allow_primary: bool) {
        if allow_primary && response.dragged_by(egui::PointerButton::Primary) {
            self.goal = None;
            let delta = response.drag_delta();
            let shift = ui.input(|i| i.modifiers.shift);
            if shift {
                self.pan(delta);
            } else {
                self.yaw -= delta.x * 0.008;
                self.pitch = (self.pitch + delta.y * 0.008).clamp(-1.50, 1.50);
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.goal = None;
            self.pan(response.drag_delta());
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let factor = (-scroll * 0.0025).exp();
                self.distance = (self.distance * factor).clamp(25.0, 1200.0);
            }
        }
    }

    pub fn pan(&mut self, delta: egui::Vec2) {
        let eye = self.eye();
        let forward = (self.target - eye).normalize_or_zero();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let scale = self.distance * 0.0014;
        self.target += (-right * delta.x + up * delta.y) * scale;
    }

    pub fn project_point(&self, rect: egui::Rect, world: Vec3, z_far: f32) -> Option<egui::Pos2> {
        if rect.width() <= 1.0 || rect.height() <= 1.0 {
            return None;
        }
        let vp = self.projection_matrix(rect.width() / rect.height(), z_far) * self.view_matrix();
        let clip = vp * Vec4::new(world.x, world.y, world.z, 1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(egui::pos2(
            rect.left() + (ndc.x * 0.5 + 0.5) * rect.width(),
            rect.top() + (0.5 - ndc.y * 0.5) * rect.height(),
        ))
    }

    pub fn screen_ray(
        &self,
        rect: egui::Rect,
        pos: egui::Pos2,
        z_far: f32,
    ) -> Option<(Vec3, Vec3)> {
        if rect.width() <= 1.0 || rect.height() <= 1.0 {
            return None;
        }
        let vp = self.projection_matrix(rect.width() / rect.height(), z_far) * self.view_matrix();
        let inv_vp = vp.inverse();

        let ndc_x = ((pos.x - rect.left()) / rect.width()) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((pos.y - rect.top()) / rect.height()) * 2.0;

        let near_w = inv_vp * Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far_w = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);
        if near_w.w.abs() < 1e-6 || far_w.w.abs() < 1e-6 {
            return None;
        }
        let near_pt = near_w.truncate() / near_w.w;
        let far_pt = far_w.truncate() / far_w.w;
        Some((near_pt, (far_pt - near_pt).normalize_or_zero()))
    }
}

fn shortest_angle_diff(from: f32, to: f32) -> f32 {
    let mut d = (to - from).rem_euclid(TAU);
    if d > PI {
        d -= TAU;
    }
    d
}

pub fn look_at_rh_gl(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let f = (target - eye).normalize_or_zero();
    let s = f.cross(up).normalize_or_zero();
    let u = s.cross(f);
    Mat4::from_cols(
        Vec4::new(s.x, u.x, -f.x, 0.0),
        Vec4::new(s.y, u.y, -f.y, 0.0),
        Vec4::new(s.z, u.z, -f.z, 0.0),
        Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
    )
}

pub fn perspective_rh_gl(fov_y_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    let nf = 1.0 / (z_near - z_far);
    Mat4::from_cols(
        Vec4::new(f / aspect, 0.0, 0.0, 0.0),
        Vec4::new(0.0, f, 0.0, 0.0),
        Vec4::new(0.0, 0.0, (z_far + z_near) * nf, -1.0),
        Vec4::new(0.0, 0.0, (2.0 * z_far * z_near) * nf, 0.0),
    )
}
