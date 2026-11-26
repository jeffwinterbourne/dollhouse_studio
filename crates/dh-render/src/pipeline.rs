use crate::camera::OrbitCamera;
use crate::gizmos::{GizmoState, Viewport3DEvent, handle_and_draw_wall_gizmos};
use crate::picking::pick_wall_opening;
use crate::viewcube::draw_viewcube;
use dh_core::{FilamentColor, ViewMode};
use dh_geometry::{BuiltProjectWall, GeneratedWindow, TriMesh};
use eframe::egui;
use eframe::egui_glow::glow::{self, HasContext};
use glam::{Mat4, Vec3};
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::sync::{Arc, Mutex};

/// Decoded RGBA image buffer ready for OpenGL wall texture upload.
#[derive(Clone)]
pub struct LoadedPhotoGpuData {
    pub width_px: u32,
    pub height_px: u32,
    pub rgba: Arc<Vec<u8>>,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct ViewportSettings {
    pub view_mode: ViewMode,
    pub filament_color: FilamentColor,
    pub door_color: FilamentColor,
    pub wall_color: FilamentColor,
    pub explode_distance: f32,
    pub door_open_deg: f32,
    pub show_wall_cutaway: bool,
    pub show_wireframe: bool,
    pub highlight_overhangs: bool,
    pub show_grid: bool,
    pub dark_mode: bool,
}

impl Default for ViewportSettings {
    fn default() -> Self {
        Self {
            view_mode: ViewMode::Exploded,
            filament_color: FilamentColor::HeritageWhite,
            door_color: FilamentColor::HeritageRed,
            wall_color: FilamentColor::WarmCream,
            explode_distance: 22.0,
            door_open_deg: 25.0,
            show_wall_cutaway: true,
            show_wireframe: false,
            highlight_overhangs: false,
            show_grid: true,
            dark_mode: true,
        }
    }
}

struct MeshGpu {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertex_count: i32,
}

struct LeafGpu {
    main_gpu: MeshGpu,
    ext_gpu: Option<MeshGpu>,
    ac_gpu: Option<MeshGpu>,
    hardware_gpu: Option<MeshGpu>,
    accessory_gpu: Option<MeshGpu>,
}

struct WindowSashGpu {
    mesh_gpu: MeshGpu,
    ac_gpu: Option<MeshGpu>,
}

struct PlacedOpeningGpu {
    id: String,
    ext_gpu: Option<MeshGpu>,
    int_gpu: Option<MeshGpu>,
    ac_gpu: Option<MeshGpu>,
    leaf_gpus: Vec<LeafGpu>,
    sash_gpus: Vec<WindowSashGpu>,
}

struct GlPipeline {
    mesh_program: glow::Program,
    line_program: glow::Program,
    u_mvp: Option<glow::UniformLocation>,
    u_model: Option<glow::UniformLocation>,
    u_eye: Option<glow::UniformLocation>,
    u_color: Option<glow::UniformLocation>,
    u_alpha: Option<glow::UniformLocation>,
    u_overhang_mode: Option<glow::UniformLocation>,
    u_use_wall_tex: Option<glow::UniformLocation>,
    u_wall_tex: Option<glow::UniformLocation>,
    u_photo_bounds: Option<glow::UniformLocation>,
    u_wall_size: Option<glow::UniformLocation>,
    u_photo_opacity: Option<glow::UniformLocation>,
    u_line_mvp: Option<glow::UniformLocation>,
    u_line_color: Option<glow::UniformLocation>,

    ext_gpu: Option<MeshGpu>,
    int_gpu: Option<MeshGpu>,
    ac_gpu: Option<MeshGpu>,
    wall_gpu: Option<MeshGpu>,
    leaf_gpus: Vec<LeafGpu>,
    sash_gpus: Vec<WindowSashGpu>,
    handle_set_gpu: Option<MeshGpu>,
    project_wall_gpu: Option<MeshGpu>,
    project_openings_gpu: Vec<PlacedOpeningGpu>,
    uploaded_wall_version: u64,
    wall_photo_tex: Option<glow::Texture>,
    uploaded_photo_version: u64,
    grid_vao: glow::VertexArray,
    grid_vbo: glow::Buffer,
    grid_vertex_count: i32,
    uploaded_version: u64,
}

/// Thread-safe holder shared between the UI and the `egui_glow` paint callback.
pub struct Viewport3DState {
    pub camera: OrbitCamera,
    pub settings: ViewportSettings,
    pub project_wall_mode: bool,
    pub wall_solid: bool,
    pub wall_selected_ids: HashSet<String>,
    pub hovered_opening_id: Option<String>,
    pub events: Vec<Viewport3DEvent>,
    pub gizmo: GizmoState,
    photo_data: Option<LoadedPhotoGpuData>,
    photo_bounds_mm: [f32; 4],
    photo_show_3d: bool,
    photo_opacity: f32,
    window_data: Option<Arc<GeneratedWindow>>,
    data_version: u64,
    wall_data: Option<Arc<BuiltProjectWall>>,
    wall_version: u64,
    gl_pipeline: Option<GlPipeline>,
}

impl Default for Viewport3DState {
    fn default() -> Self {
        Self {
            camera: OrbitCamera::default(),
            settings: ViewportSettings::default(),
            project_wall_mode: false,
            wall_solid: true,
            wall_selected_ids: HashSet::new(),
            hovered_opening_id: None,
            events: Vec::new(),
            gizmo: GizmoState::default(),
            photo_data: None,
            photo_bounds_mm: [0.0, 0.0, 380.0, 240.0],
            photo_show_3d: true,
            photo_opacity: 0.72,
            window_data: None,
            data_version: 0,
            wall_data: None,
            wall_version: 0,
            gl_pipeline: None,
        }
    }
}

impl Viewport3DState {
    pub fn set_window(&mut self, window: Arc<GeneratedWindow>) {
        self.window_data = Some(window);
        self.data_version = self.data_version.wrapping_add(1);
    }

    pub fn set_project_wall(&mut self, built: Arc<BuiltProjectWall>) {
        self.wall_data = Some(built);
        self.wall_version = self.wall_version.wrapping_add(1);
    }

    pub fn set_wall_photo(
        &mut self,
        data: Option<LoadedPhotoGpuData>,
        bounds_mm: [f32; 4],
        show_3d: bool,
        opacity: f32,
    ) {
        self.photo_data = data;
        self.photo_bounds_mm = bounds_mm;
        self.photo_show_3d = show_3d;
        self.photo_opacity = opacity;
    }

    pub fn destroy_gl(&mut self, gl: &glow::Context) {
        if let Some(p) = self.gl_pipeline.take() {
            unsafe {
                p.destroy(gl);
            }
        }
    }
}

/// Renders the interactive 3D viewport (mesh + ViewCube + 3D Wall gizmos) inside `ui`.
pub fn draw_viewport(ui: &mut egui::Ui, shared: Arc<Mutex<Viewport3DState>>) {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

    let dark_mode;
    let animating;
    {
        let mut st = shared.lock().unwrap();
        let dt = ui.input(|i| i.stable_dt).clamp(0.005, 0.05);
        animating = st.camera.step_animation(dt);
        dark_mode = st.settings.dark_mode;

        let bg = if dark_mode {
            egui::Color32::from_rgb(22, 26, 35)
        } else {
            egui::Color32::from_rgb(230, 236, 244)
        };
        ui.painter().rect_filled(rect, 0.0, bg);
    }

    // Queue GL paint callback first so egui overlays (ViewCube + 3D Gizmos) paint crisply above it
    let shared_for_cb = Arc::clone(&shared);
    let cb = eframe::egui_glow::CallbackFn::new(move |info, painter| {
        let gl = painter.gl();
        let mut st = shared_for_cb.lock().unwrap();
        render_gl(gl, &info, &mut st);
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });

    // Interactive overlays & mouse handling
    {
        let mut st = shared.lock().unwrap();
        let cube_consumed = draw_viewcube(ui, rect, &mut st.camera, dark_mode);

        let mut gizmo_consumed = false;
        if st.project_wall_mode
            && let Some(built_wall) = st.wall_data.clone()
        {
            let selected = st.wall_selected_ids.clone();
            let camera = st.camera.clone();
            let mut gizmo = std::mem::take(&mut st.gizmo);
            let mut events = Vec::new();
            gizmo_consumed = handle_and_draw_wall_gizmos(
                ui,
                rect,
                &response,
                &camera,
                &built_wall,
                &selected,
                &mut gizmo,
                &mut events,
            );
            st.gizmo = gizmo;
            st.events.extend(events);

            st.hovered_opening_id = if !cube_consumed && !gizmo_consumed {
                ui.input(|i| i.pointer.hover_pos())
                    .filter(|p| rect.contains(*p))
                    .and_then(|pos| pick_wall_opening(&built_wall, &st.camera, rect, pos))
            } else {
                None
            };
        } else {
            st.hovered_opening_id = None;
        }

        st.camera
            .handle_input(&response, ui, !cube_consumed && !gizmo_consumed);

        if st.project_wall_mode && !cube_consumed && !gizmo_consumed {
            if response.double_clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(built_wall) = &st.wall_data
                && let Some(hit) = pick_wall_opening(built_wall, &st.camera, rect, pos)
            {
                st.events.push(Viewport3DEvent::DoubleClickedOpening(hit));
            } else if response.clicked_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(built_wall) = &st.wall_data
            {
                let toggle = ui.input(|i| i.modifiers.shift || i.modifiers.ctrl);
                let hit = pick_wall_opening(built_wall, &st.camera, rect, pos);
                st.events
                    .push(Viewport3DEvent::PickedOpening { id: hit, toggle });
            }
        }
    }

    if animating {
        ui.ctx().request_repaint();
    }
}

fn render_gl(gl: &glow::Context, info: &egui::PaintCallbackInfo, st: &mut Viewport3DState) {
    let vp = info.viewport_in_pixels();
    if vp.width_px <= 0 || vp.height_px <= 0 {
        return;
    }

    if st.gl_pipeline.is_none() {
        st.gl_pipeline = Some(unsafe { GlPipeline::new(gl) });
    }

    if st.project_wall_mode {
        render_project_wall_gl(gl, info, st);
        return;
    }

    let Some(window) = st.window_data.clone() else {
        return;
    };
    let version = st.data_version;
    let camera_view = st.camera.view_matrix();
    let camera_eye = st.camera.eye();
    let settings = st.settings.clone();

    let pipeline = st.gl_pipeline.as_mut().unwrap();
    if pipeline.uploaded_version != version {
        unsafe {
            pipeline.upload_meshes(gl, &window);
        }
        pipeline.uploaded_version = version;
    }

    let aspect = (vp.width_px as f32 / vp.height_px as f32).max(0.01);
    let proj = st.camera.projection_matrix(aspect, 2500.0);
    let vp_mat = proj * camera_view;
    let clip = info.clip_rect_in_pixels();

    unsafe {
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(
            clip.left_px,
            clip.from_bottom_px,
            clip.width_px,
            clip.height_px,
        );
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);

        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.clear_depth(1.0);
        gl.clear(glow::DEPTH_BUFFER_BIT);

        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::BACK);
        gl.front_face(glow::CCW);

        let specs = &window.specs;
        let params = &window.params;
        let (ext_min, ext_max) = window.exterior_half.bounds();
        let (int_min, int_max) = window.interior_half.bounds();
        let ext_cx = (ext_min[0] + ext_max[0]) * 0.5;
        let ext_cy = (ext_min[1] + ext_max[1]) * 0.5;
        let int_cx = (int_min[0] + int_max[0]) * 0.5;
        let int_cy = (int_min[1] + int_max[1]) * 0.5;

        let grid_z = match settings.view_mode {
            ViewMode::Exploded | ViewMode::Assembled => -(params.opening_height as f32 * 0.5 + 14.0),
            _ => 0.0,
        };

        if settings.show_grid {
            let grid_model = Mat4::from_translation(Vec3::new(0.0, 0.0, grid_z));
            let grid_mvp = vp_mat * grid_model;
            gl.use_program(Some(pipeline.line_program));
            gl.uniform_matrix_4_f32_slice(pipeline.u_line_mvp.as_ref(), false, grid_mvp.as_ref());
            if settings.dark_mode {
                gl.uniform_3_f32(pipeline.u_line_color.as_ref(), 0.28, 0.32, 0.39);
            } else {
                gl.uniform_3_f32(pipeline.u_line_color.as_ref(), 0.72, 0.77, 0.84);
            }
            gl.bind_vertex_array(Some(pipeline.grid_vao));
            gl.draw_arrays(glow::LINES, 0, pipeline.grid_vertex_count);
        }

        let stand_upright = Mat4::from_rotation_x(FRAC_PI_2);
        let flip_y = Mat4::from_rotation_y(std::f32::consts::PI);
        let total_assembled_z = (params.trim_thickness
            + specs.effective_wall_thickness
            + params.interior_trim_thickness) as f32;
        let center_z_shift = -total_assembled_z * 0.5;

        let ext_color = settings.filament_color.rgb_exterior();
        let int_color = settings.filament_color.rgb_interior();
        let door_color = settings.door_color.rgb_exterior();
        let wall_color = settings.wall_color.rgb_exterior();
        let brass_color = [0.85, 0.68, 0.28];

        let overhang_flag = if settings.highlight_overhangs
            && matches!(
                settings.view_mode,
                ViewMode::PrintPlate | ViewMode::ExteriorOnly | ViewMode::InteriorOnly
            ) {
            1
        } else {
            0
        };

        let draw_gpu = |m: &MeshGpu,
                        model: Mat4,
                        color: [f32; 3],
                        alpha: f32,
                        wireframe: bool,
                        overhang_chk: i32| {
            let mvp = vp_mat * model;
            gl.use_program(Some(pipeline.mesh_program));
            gl.uniform_matrix_4_f32_slice(pipeline.u_mvp.as_ref(), false, mvp.as_ref());
            gl.uniform_matrix_4_f32_slice(pipeline.u_model.as_ref(), false, model.as_ref());
            gl.uniform_3_f32(pipeline.u_eye.as_ref(), camera_eye.x, camera_eye.y, camera_eye.z);
            gl.uniform_3_f32(pipeline.u_color.as_ref(), color[0], color[1], color[2]);
            gl.uniform_1_f32(pipeline.u_alpha.as_ref(), alpha);
            gl.uniform_1_i32(pipeline.u_overhang_mode.as_ref(), overhang_chk);
            gl.uniform_1_i32(pipeline.u_use_wall_tex.as_ref(), 0);

            gl.bind_vertex_array(Some(m.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, m.vertex_count);

            if wireframe {
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
                gl.enable(glow::POLYGON_OFFSET_LINE);
                gl.polygon_offset(-1.0, -1.0);
                gl.uniform_3_f32(pipeline.u_color.as_ref(), 0.08, 0.10, 0.14);
                gl.draw_arrays(glow::TRIANGLES, 0, m.vertex_count);
                gl.disable(glow::POLYGON_OFFSET_LINE);
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            }
        };

        let draw_part = |gpu: &Option<MeshGpu>,
                         model: Mat4,
                         color: [f32; 3],
                         alpha: f32,
                         wireframe: bool,
                         overhang_chk: i32| {
            if let Some(m) = gpu {
                draw_gpu(m, model, color, alpha, wireframe, overhang_chk);
            }
        };

        match settings.view_mode {
            ViewMode::PrintPlate => {
                if window.is_door {
                    type PlateGpuItem<'a> = (&'a MeshGpu, ([f32; 3], [f32; 3]), [f32; 3]);
                    let mut items: Vec<PlateGpuItem<'_>> = Vec::new();
                    if let Some(m) = &pipeline.ext_gpu {
                        items.push((m, window.exterior_half.bounds(), ext_color));
                    }
                    if let Some(m) = &pipeline.int_gpu {
                        items.push((m, window.interior_half.bounds(), int_color));
                    }
                    for (idx, lg) in pipeline.leaf_gpus.iter().enumerate() {
                        items.push((&lg.main_gpu, window.door_leaves[idx].mesh.bounds(), door_color));
                        if let (Some(em), Some(em_mesh)) =
                            (&lg.ext_gpu, &window.door_leaves[idx].exterior_leaf)
                        {
                            items.push((em, em_mesh.bounds(), ext_color));
                        }
                    }
                    if let Some(hm) = &pipeline.handle_set_gpu {
                        items.push((hm, window.door_handle.bounds(), brass_color));
                    }

                    let spacing = params.plate_spacing as f32;
                    let total_w: f32 = items
                        .iter()
                        .map(|(_, (bmin, bmax), _)| (bmax[0] - bmin[0]).max(0.1))
                        .sum::<f32>()
                        + spacing * items.len().saturating_sub(1) as f32;
                    let mut cursor_x = -total_w * 0.5;
                    for (gpu, (bmin, bmax), col) in items {
                        let w = (bmax[0] - bmin[0]).max(0.1);
                        let cx = (bmin[0] + bmax[0]) * 0.5;
                        let cy = (bmin[1] + bmax[1]) * 0.5;
                        let m = Mat4::from_translation(Vec3::new(
                            cursor_x + w * 0.5 - cx,
                            -cy,
                            -bmin[2],
                        ));
                        draw_gpu(gpu, m, col, 1.0, settings.show_wireframe, overhang_flag);
                        cursor_x += w + spacing;
                    }
                } else {
                    let ext_w = ext_max[0] - ext_min[0];
                    let int_w = int_max[0] - int_min[0];
                    let total_w = ext_w + int_w + params.plate_spacing as f32;
                    let m_ext = Mat4::from_translation(Vec3::new(
                        -total_w * 0.5 + ext_w * 0.5 - ext_cx,
                        -ext_cy,
                        0.0,
                    ));
                    let m_int = Mat4::from_translation(Vec3::new(
                        total_w * 0.5 - int_w * 0.5 - int_cx,
                        -int_cy,
                        0.0,
                    ));
                    draw_part(&pipeline.ext_gpu, m_ext, ext_color, 1.0, settings.show_wireframe, overhang_flag);
                    let sash_color = [
                        (ext_color[0] * 0.90 + 0.08).min(1.0),
                        (ext_color[1] * 0.90 + 0.07).min(1.0),
                        (ext_color[2] * 0.90 + 0.04).min(1.0),
                    ];
                    for sg in &pipeline.sash_gpus {
                        draw_gpu(&sg.mesh_gpu, m_ext, sash_color, 1.0, settings.show_wireframe, overhang_flag);
                    }
                    draw_part(&pipeline.int_gpu, m_int, int_color, 1.0, settings.show_wireframe, overhang_flag);
                }
            }
            ViewMode::ExteriorOnly => {
                let m_ext = Mat4::from_translation(Vec3::new(-ext_cx, -ext_cy, 0.0));
                draw_part(&pipeline.ext_gpu, m_ext, ext_color, 1.0, settings.show_wireframe, overhang_flag);
                let sash_color = [
                    (ext_color[0] * 0.90 + 0.08).min(1.0),
                    (ext_color[1] * 0.90 + 0.07).min(1.0),
                    (ext_color[2] * 0.90 + 0.04).min(1.0),
                ];
                for sg in &pipeline.sash_gpus {
                    draw_gpu(&sg.mesh_gpu, m_ext, sash_color, 1.0, settings.show_wireframe, overhang_flag);
                }
            }
            ViewMode::InteriorOnly => {
                let m_int = Mat4::from_translation(Vec3::new(-int_cx, -int_cy, 0.0));
                draw_part(&pipeline.int_gpu, m_int, int_color, 1.0, settings.show_wireframe, overhang_flag);
            }
            ViewMode::Exploded | ViewMode::Assembled => {
                let is_exploded = settings.view_mode == ViewMode::Exploded;
                let gap = if is_exploded { settings.explode_distance } else { 0.0 };

                let m_ext = stand_upright
                    * Mat4::from_translation(Vec3::new(0.0, 0.0, center_z_shift - gap));
                let int_z = if window.is_door {
                    center_z_shift + total_assembled_z + gap
                } else {
                    center_z_shift
                        + (specs.ext_seat_z + params.acetate_thickness + specs.int_total_height)
                            as f32
                        + gap
                };
                let m_int =
                    stand_upright * Mat4::from_translation(Vec3::new(0.0, 0.0, int_z)) * flip_y;

                draw_part(&pipeline.ext_gpu, m_ext, ext_color, 1.0, settings.show_wireframe, 0);
                draw_part(&pipeline.int_gpu, m_int, int_color, 1.0, settings.show_wireframe, 0);

                if window.is_door {
                    let z_leaf_int = center_z_shift
                        + (params.trim_thickness + specs.effective_wall_thickness) as f32
                        - 0.65;
                    let open_rad = if is_exploded {
                        (settings.door_open_deg * 0.4).to_radians()
                    } else {
                        settings.door_open_deg.to_radians()
                    };

                    for (idx, lg) in pipeline.leaf_gpus.iter().enumerate() {
                        let meta = &window.door_leaves[idx];
                        let px = meta.assembled_x_offset + meta.pivot_x;
                        let pz = z_leaf_int - meta.pivot_z;
                        let swing = Mat4::from_translation(Vec3::new(px, 0.0, pz))
                            * Mat4::from_rotation_y(-open_rad * meta.swing_dir)
                            * Mat4::from_translation(Vec3::new(-px, 0.0, -pz));

                        let leaf_base = stand_upright
                            * swing
                            * Mat4::from_translation(Vec3::new(
                                meta.assembled_x_offset,
                                0.0,
                                z_leaf_int,
                            ))
                            * flip_y;

                        draw_gpu(&lg.main_gpu, leaf_base, door_color, 1.0, settings.show_wireframe, 0);

                        if let Some(ext_leaf) = &lg.ext_gpu {
                            let ext_shift = if is_exploded { -gap * 0.55 } else { 0.0 };
                            let m_ext_leaf = stand_upright
                                * swing
                                * Mat4::from_translation(Vec3::new(
                                    meta.assembled_x_offset,
                                    0.0,
                                    z_leaf_int - meta.leaf_thickness + ext_shift,
                                ));
                            draw_gpu(ext_leaf, m_ext_leaf, ext_color, 1.0, settings.show_wireframe, 0);
                        }

                        let hw_base = stand_upright
                            * swing
                            * Mat4::from_translation(Vec3::new(
                                meta.assembled_x_offset,
                                0.0,
                                z_leaf_int,
                            ));
                        if let Some(hm) = &lg.hardware_gpu {
                            let hm_m = if is_exploded {
                                hw_base * Mat4::from_translation(Vec3::new(0.0, 0.0, gap * 0.25))
                            } else {
                                hw_base
                            };
                            draw_gpu(hm, hm_m, brass_color, 1.0, false, 0);
                        }
                        if let Some(am) = &lg.accessory_gpu {
                            draw_gpu(am, hw_base, ext_color, 1.0, settings.show_wireframe, 0);
                        }
                    }

                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    gl.depth_mask(false);
                    for (idx, lg) in pipeline.leaf_gpus.iter().enumerate() {
                        if let Some(ac_m) = &lg.ac_gpu {
                            let meta = &window.door_leaves[idx];
                            let px = meta.assembled_x_offset + meta.pivot_x;
                            let pz = z_leaf_int - meta.pivot_z;
                            let swing = Mat4::from_translation(Vec3::new(px, 0.0, pz))
                                * Mat4::from_rotation_y(-open_rad * meta.swing_dir)
                                * Mat4::from_translation(Vec3::new(-px, 0.0, -pz));
                            let m_ac = stand_upright
                                * swing
                                * Mat4::from_translation(Vec3::new(
                                    meta.assembled_x_offset,
                                    0.0,
                                    z_leaf_int
                                        - meta.leaf_thickness * 0.5
                                        - params.acetate_thickness as f32 * 0.5,
                                ));
                            draw_gpu(ac_m, m_ac, [0.42, 0.78, 0.96], 0.42, false, 0);
                        }
                    }
                    if !is_exploded && settings.show_wall_cutaway {
                        let m_wall = stand_upright
                            * Mat4::from_translation(Vec3::new(
                                0.0,
                                0.0,
                                center_z_shift + params.trim_thickness as f32,
                            ));
                        draw_part(&pipeline.wall_gpu, m_wall, wall_color, 0.28, false, 0);
                    }
                    gl.depth_mask(true);
                    gl.disable(glow::BLEND);
                } else {
                    let open_rad = if is_exploded {
                        (settings.door_open_deg * 0.4).to_radians()
                    } else {
                        settings.door_open_deg.to_radians()
                    };
                    let sash_color = [
                        (ext_color[0] * 0.90 + 0.08).min(1.0),
                        (ext_color[1] * 0.90 + 0.07).min(1.0),
                        (ext_color[2] * 0.90 + 0.04).min(1.0),
                    ];
                    let sash_t =
                        (params.trim_thickness + 1.2).clamp(3.6, specs.ext_seat_z.clamp(3.6, 4.6)) as f32;
                    let sash_ac_z = (sash_t - params.acetate_thickness as f32 - 0.15).max(1.0);

                    for (s_idx, sg) in pipeline.sash_gpus.iter().enumerate() {
                        if let Some(meta) = window.window_sashes.get(s_idx) {
                            let piv = Vec3::from(meta.pivot_origin);
                            let axis = Vec3::from(meta.pivot_axis);
                            let sash_rot = Mat4::from_translation(piv)
                                * Mat4::from_axis_angle(axis, open_rad * meta.swing_dir)
                                * Mat4::from_translation(-piv);
                            draw_gpu(&sg.mesh_gpu, m_ext * sash_rot, sash_color, 1.0, settings.show_wireframe, 0);
                        }
                    }

                    let m_ac = stand_upright
                        * Mat4::from_translation(Vec3::new(
                            0.0,
                            0.0,
                            center_z_shift + specs.ext_seat_z as f32,
                        ));

                    gl.enable(glow::BLEND);
                    gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                    gl.depth_mask(false);
                    draw_part(&pipeline.ac_gpu, m_ac, [0.42, 0.78, 0.96], 0.42, false, 0);
                    for (s_idx, sg) in pipeline.sash_gpus.iter().enumerate() {
                        if let (Some(ac_m), Some(meta)) =
                            (&sg.ac_gpu, window.window_sashes.get(s_idx))
                        {
                            let piv = Vec3::from(meta.pivot_origin);
                            let axis = Vec3::from(meta.pivot_axis);
                            let sash_rot = Mat4::from_translation(piv)
                                * Mat4::from_axis_angle(axis, open_rad * meta.swing_dir)
                                * Mat4::from_translation(-piv);
                            let m_sash_ac =
                                m_ext * sash_rot * Mat4::from_translation(Vec3::new(0.0, 0.0, sash_ac_z));
                            draw_gpu(ac_m, m_sash_ac, [0.42, 0.78, 0.96], 0.42, false, 0);
                        }
                    }
                    if !is_exploded && settings.show_wall_cutaway {
                        let m_wall = stand_upright
                            * Mat4::from_translation(Vec3::new(
                                0.0,
                                0.0,
                                center_z_shift + params.trim_thickness as f32,
                            ));
                        draw_part(&pipeline.wall_gpu, m_wall, wall_color, 0.28, false, 0);
                    }
                    gl.depth_mask(true);
                    gl.disable(glow::BLEND);
                }
            }
        }

        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::SCISSOR_TEST);
    }
}

fn highlight_color(base: [f32; 3], selected: bool, hovered: bool) -> [f32; 3] {
    if selected {
        [
            (base[0] * 0.65 + 0.35).min(1.0),
            (base[1] * 0.65 + 0.28).min(1.0),
            (base[2] * 0.65 + 0.10).min(1.0),
        ]
    } else if hovered {
        [
            (base[0] * 0.80 + 0.16).min(1.0),
            (base[1] * 0.80 + 0.20).min(1.0),
            (base[2] * 0.80 + 0.24).min(1.0),
        ]
    } else {
        base
    }
}

fn render_project_wall_gl(
    gl: &glow::Context,
    info: &egui::PaintCallbackInfo,
    st: &mut Viewport3DState,
) {
    let vp = info.viewport_in_pixels();
    if vp.width_px <= 0 || vp.height_px <= 0 {
        return;
    }
    let Some(built_wall) = st.wall_data.clone() else {
        return;
    };
    let wall_ver = st.wall_version;
    let camera_view = st.camera.view_matrix();
    let camera_eye = st.camera.eye();
    let settings = st.settings.clone();
    let wall_solid = st.wall_solid;
    let selected_ids = st.wall_selected_ids.clone();
    let hovered_id = st.hovered_opening_id.clone();
    let photo_data = st.photo_data.clone();
    let photo_bounds = st.photo_bounds_mm;
    let photo_show_3d = st.photo_show_3d;
    let photo_opacity = st.photo_opacity;

    let aspect = (vp.width_px as f32 / vp.height_px as f32).max(0.01);
    let proj = st.camera.projection_matrix(aspect, 4000.0);
    let vp_mat = proj * camera_view;
    let clip = info.clip_rect_in_pixels();

    let pipeline = st.gl_pipeline.as_mut().unwrap();
    if pipeline.uploaded_wall_version != wall_ver {
        unsafe {
            pipeline.upload_project_wall(gl, &built_wall);
        }
        pipeline.uploaded_wall_version = wall_ver;
    }
    if let Some(ref pdata) = photo_data {
        if pipeline.uploaded_photo_version != pdata.version {
            unsafe {
                pipeline.upload_wall_photo(gl, pdata);
            }
            pipeline.uploaded_photo_version = pdata.version;
        }
    } else if pipeline.wall_photo_tex.is_some() {
        unsafe {
            if let Some(tex) = pipeline.wall_photo_tex.take() {
                gl.delete_texture(tex);
            }
        }
        pipeline.uploaded_photo_version = 0;
    }

    unsafe {
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(
            clip.left_px,
            clip.from_bottom_px,
            clip.width_px,
            clip.height_px,
        );
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);

        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.clear_depth(1.0);
        gl.clear(glow::DEPTH_BUFFER_BIT);

        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::BACK);
        gl.front_face(glow::CCW);

        let wall_w = built_wall.wall.width as f32;
        let wall_h = built_wall.wall.height as f32;
        let wall_t = built_wall.wall.wall_thickness as f32;
        let stand_upright = Mat4::from_rotation_x(FRAC_PI_2);
        let flip_y = Mat4::from_rotation_y(std::f32::consts::PI);

        if settings.show_grid {
            let grid_model = Mat4::from_translation(Vec3::new(0.0, 0.0, -wall_h * 0.5))
                * Mat4::from_scale(Vec3::new(2.2, 2.2, 1.0));
            let grid_mvp = vp_mat * grid_model;
            gl.use_program(Some(pipeline.line_program));
            gl.uniform_matrix_4_f32_slice(pipeline.u_line_mvp.as_ref(), false, grid_mvp.as_ref());
            if settings.dark_mode {
                gl.uniform_3_f32(pipeline.u_line_color.as_ref(), 0.28, 0.32, 0.39);
            } else {
                gl.uniform_3_f32(pipeline.u_line_color.as_ref(), 0.72, 0.77, 0.84);
            }
            gl.bind_vertex_array(Some(pipeline.grid_vao));
            gl.draw_arrays(glow::LINES, 0, pipeline.grid_vertex_count);
        }

        let draw_gpu = |m: &MeshGpu,
                        model: Mat4,
                        color: [f32; 3],
                        alpha: f32,
                        wireframe: bool,
                        use_wall_tex: bool| {
            let mvp = vp_mat * model;
            gl.use_program(Some(pipeline.mesh_program));
            gl.uniform_matrix_4_f32_slice(pipeline.u_mvp.as_ref(), false, mvp.as_ref());
            gl.uniform_matrix_4_f32_slice(pipeline.u_model.as_ref(), false, model.as_ref());
            gl.uniform_3_f32(pipeline.u_eye.as_ref(), camera_eye.x, camera_eye.y, camera_eye.z);
            gl.uniform_3_f32(pipeline.u_color.as_ref(), color[0], color[1], color[2]);
            gl.uniform_1_f32(pipeline.u_alpha.as_ref(), alpha);
            gl.uniform_1_i32(pipeline.u_overhang_mode.as_ref(), 0);
            if use_wall_tex && let Some(tex) = pipeline.wall_photo_tex {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                gl.uniform_1_i32(pipeline.u_wall_tex.as_ref(), 0);
                gl.uniform_1_i32(pipeline.u_use_wall_tex.as_ref(), 1);
                gl.uniform_4_f32(
                    pipeline.u_photo_bounds.as_ref(),
                    photo_bounds[0],
                    photo_bounds[1],
                    photo_bounds[2],
                    photo_bounds[3],
                );
                gl.uniform_2_f32(pipeline.u_wall_size.as_ref(), wall_w, wall_h);
                gl.uniform_1_f32(pipeline.u_photo_opacity.as_ref(), photo_opacity);
            } else {
                gl.uniform_1_i32(pipeline.u_use_wall_tex.as_ref(), 0);
            }
            gl.bind_vertex_array(Some(m.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, m.vertex_count);

            if wireframe {
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
                gl.enable(glow::POLYGON_OFFSET_LINE);
                gl.polygon_offset(-1.0, -1.0);
                gl.uniform_1_i32(pipeline.u_use_wall_tex.as_ref(), 0);
                gl.uniform_3_f32(pipeline.u_color.as_ref(), 0.08, 0.10, 0.14);
                gl.draw_arrays(glow::TRIANGLES, 0, m.vertex_count);
                gl.disable(glow::POLYGON_OFFSET_LINE);
                gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            }
            if use_wall_tex {
                gl.bind_texture(glow::TEXTURE_2D, None);
            }
        };

        let wall_color = settings.wall_color.rgb_exterior();
        let wall_mesh_col = [
            wall_color[0] * 0.85,
            wall_color[1] * 0.83,
            wall_color[2] * 0.79,
        ];
        let wall_z_shift = -wall_t * 0.5;
        let use_tex_on_wall = photo_show_3d && pipeline.wall_photo_tex.is_some();
        if wall_solid && let Some(wm) = &pipeline.project_wall_gpu {
            let m_wall = stand_upright * Mat4::from_translation(Vec3::new(0.0, 0.0, wall_z_shift));
            draw_gpu(wm, m_wall, wall_mesh_col, 1.0, settings.show_wireframe, use_tex_on_wall);
        }

        let ext_base = settings.filament_color.rgb_exterior();
        let int_base = settings.filament_color.rgb_interior();
        let door_base = settings.door_color.rgb_exterior();
        let brass_color = [0.85, 0.68, 0.28];
        let open_rad = settings.door_open_deg.to_radians();

        for (idx, op_gpu) in pipeline.project_openings_gpu.iter().enumerate() {
            let Some((_, cx, cy, win_gen)) = built_wall.placed_openings.get(idx) else {
                continue;
            };
            let is_sel = selected_ids.contains(&op_gpu.id);
            let is_hov = hovered_id.as_deref() == Some(op_gpu.id.as_str());
            let ext_col = highlight_color(ext_base, is_sel, is_hov);
            let int_col = highlight_color(int_base, is_sel, is_hov);

            let op_z0 = wall_z_shift - win_gen.params.trim_thickness as f32;
            let op_int_z = if win_gen.is_door {
                op_z0
                    + (win_gen.params.trim_thickness
                        + win_gen.specs.effective_wall_thickness
                        + win_gen.params.interior_trim_thickness) as f32
            } else {
                op_z0
                    + (win_gen.specs.ext_seat_z
                        + win_gen.params.acetate_thickness
                        + win_gen.specs.int_total_height) as f32
            };

            if let Some(m) = &op_gpu.ext_gpu {
                let model = stand_upright * Mat4::from_translation(Vec3::new(*cx, *cy, op_z0));
                draw_gpu(m, model, ext_col, 1.0, settings.show_wireframe, false);
            }
            if let Some(m) = &op_gpu.int_gpu {
                let model = stand_upright
                    * Mat4::from_translation(Vec3::new(*cx, *cy, op_int_z))
                    * flip_y;
                draw_gpu(m, model, int_col, 1.0, settings.show_wireframe, false);
            }

            if win_gen.is_door {
                let z_leaf_int = op_z0
                    + (win_gen.params.trim_thickness + win_gen.specs.effective_wall_thickness) as f32
                    - 0.65;
                let door_col = highlight_color(door_base, is_sel, is_hov);

                for (l_idx, lg) in op_gpu.leaf_gpus.iter().enumerate() {
                    let meta = &win_gen.door_leaves[l_idx];
                    let px = *cx + meta.assembled_x_offset + meta.pivot_x;
                    let pz = z_leaf_int - meta.pivot_z;
                    let swing = Mat4::from_translation(Vec3::new(px, *cy, pz))
                        * Mat4::from_rotation_y(-open_rad * meta.swing_dir)
                        * Mat4::from_translation(Vec3::new(-px, -*cy, -pz));

                    let leaf_model = stand_upright
                        * swing
                        * Mat4::from_translation(Vec3::new(
                            *cx + meta.assembled_x_offset,
                            *cy,
                            z_leaf_int,
                        ))
                        * flip_y;
                    draw_gpu(&lg.main_gpu, leaf_model, door_col, 1.0, settings.show_wireframe, false);

                    if let Some(ext_l) = &lg.ext_gpu {
                        let m_ext_l = stand_upright
                            * swing
                            * Mat4::from_translation(Vec3::new(
                                *cx + meta.assembled_x_offset,
                                *cy,
                                z_leaf_int - meta.leaf_thickness,
                            ));
                        draw_gpu(ext_l, m_ext_l, ext_col, 1.0, settings.show_wireframe, false);
                    }
                    let hw_model = stand_upright
                        * swing
                        * Mat4::from_translation(Vec3::new(
                            *cx + meta.assembled_x_offset,
                            *cy,
                            z_leaf_int,
                        ));
                    if let Some(hm) = &lg.hardware_gpu {
                        draw_gpu(hm, hw_model, brass_color, 1.0, false, false);
                    }
                    if let Some(am) = &lg.accessory_gpu {
                        draw_gpu(am, hw_model, ext_col, 1.0, settings.show_wireframe, false);
                    }
                }
            } else if !op_gpu.sash_gpus.is_empty() {
                let m_op_ext = stand_upright * Mat4::from_translation(Vec3::new(*cx, *cy, op_z0));
                let sash_col = [
                    (ext_col[0] * 0.90 + 0.08).min(1.0),
                    (ext_col[1] * 0.90 + 0.07).min(1.0),
                    (ext_col[2] * 0.90 + 0.04).min(1.0),
                ];
                for (s_idx, sg) in op_gpu.sash_gpus.iter().enumerate() {
                    if let Some(meta) = win_gen.window_sashes.get(s_idx) {
                        let piv = Vec3::from(meta.pivot_origin);
                        let axis = Vec3::from(meta.pivot_axis);
                        let sash_rot = Mat4::from_translation(piv)
                            * Mat4::from_axis_angle(axis, open_rad * meta.swing_dir)
                            * Mat4::from_translation(-piv);
                        draw_gpu(&sg.mesh_gpu, m_op_ext * sash_rot, sash_col, 1.0, settings.show_wireframe, false);
                    }
                }
            }
        }

        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.depth_mask(false);

        for (idx, op_gpu) in pipeline.project_openings_gpu.iter().enumerate() {
            let Some((_, cx, cy, win_gen)) = built_wall.placed_openings.get(idx) else {
                continue;
            };
            let op_z0 = wall_z_shift - win_gen.params.trim_thickness as f32;
            if win_gen.is_door {
                let z_leaf_int = op_z0
                    + (win_gen.params.trim_thickness + win_gen.specs.effective_wall_thickness) as f32
                    - 0.65;
                for (l_idx, lg) in op_gpu.leaf_gpus.iter().enumerate() {
                    if let Some(ac_m) = &lg.ac_gpu {
                        let meta = &win_gen.door_leaves[l_idx];
                        let px = *cx + meta.assembled_x_offset + meta.pivot_x;
                        let pz = z_leaf_int - meta.pivot_z;
                        let swing = Mat4::from_translation(Vec3::new(px, *cy, pz))
                            * Mat4::from_rotation_y(-open_rad * meta.swing_dir)
                            * Mat4::from_translation(Vec3::new(-px, -*cy, -pz));
                        let m_ac = stand_upright
                            * swing
                            * Mat4::from_translation(Vec3::new(
                                *cx + meta.assembled_x_offset,
                                *cy,
                                z_leaf_int
                                    - meta.leaf_thickness * 0.5
                                    - win_gen.params.acetate_thickness as f32 * 0.5,
                            ));
                        draw_gpu(ac_m, m_ac, [0.42, 0.78, 0.96], 0.42, false, false);
                    }
                }
            } else if !op_gpu.sash_gpus.is_empty() {
                let m_op_ext = stand_upright * Mat4::from_translation(Vec3::new(*cx, *cy, op_z0));
                let sash_t = (win_gen.params.trim_thickness + 1.2)
                    .clamp(3.6, win_gen.specs.ext_seat_z.clamp(3.6, 4.6)) as f32;
                let sash_ac_z = (sash_t - win_gen.params.acetate_thickness as f32 - 0.15).max(1.0);
                for (s_idx, sg) in op_gpu.sash_gpus.iter().enumerate() {
                    if let (Some(ac_m), Some(meta)) =
                        (&sg.ac_gpu, win_gen.window_sashes.get(s_idx))
                    {
                        let piv = Vec3::from(meta.pivot_origin);
                        let axis = Vec3::from(meta.pivot_axis);
                        let sash_rot = Mat4::from_translation(piv)
                            * Mat4::from_axis_angle(axis, open_rad * meta.swing_dir)
                            * Mat4::from_translation(-piv);
                        let m_sash_ac =
                            m_op_ext * sash_rot * Mat4::from_translation(Vec3::new(0.0, 0.0, sash_ac_z));
                        draw_gpu(ac_m, m_sash_ac, [0.42, 0.78, 0.96], 0.42, false, false);
                    }
                }
            } else if let Some(ac_m) = &op_gpu.ac_gpu {
                let m_ac = stand_upright
                    * Mat4::from_translation(Vec3::new(
                        *cx,
                        *cy,
                        op_z0 + win_gen.specs.ext_seat_z as f32,
                    ));
                draw_gpu(ac_m, m_ac, [0.42, 0.78, 0.96], 0.42, false, false);
            }
        }

        if !wall_solid && let Some(wm) = &pipeline.project_wall_gpu {
            let m_wall = stand_upright * Mat4::from_translation(Vec3::new(0.0, 0.0, wall_z_shift));
            draw_gpu(wm, m_wall, wall_mesh_col, 0.28, false, use_tex_on_wall);
        }

        gl.depth_mask(true);
        gl.disable(glow::BLEND);

        gl.bind_vertex_array(None);
        gl.use_program(None);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::SCISSOR_TEST);
    }
}

impl GlPipeline {
    unsafe fn new(gl: &glow::Context) -> Self {
        let mesh_vs = r#"
            #version 150
            in vec3 a_pos;
            in vec3 a_norm;
            uniform mat4 u_mvp;
            uniform mat4 u_model;
            out vec3 v_world_pos;
            out vec3 v_world_norm;
            out vec2 v_local_xy;
            out float v_local_z;
            out float v_local_nz;
            void main() {
                gl_Position = u_mvp * vec4(a_pos, 1.0);
                v_world_pos = (u_model * vec4(a_pos, 1.0)).xyz;
                v_world_norm = normalize(mat3(u_model) * a_norm);
                v_local_xy = a_pos.xy;
                v_local_z = a_pos.z;
                v_local_nz = a_norm.z;
            }
        "#;

        let mesh_fs = r#"
            #version 150
            in vec3 v_world_pos;
            in vec3 v_world_norm;
            in vec2 v_local_xy;
            in float v_local_z;
            in float v_local_nz;
            uniform vec3 u_eye;
            uniform vec3 u_color;
            uniform float u_alpha;
            uniform int u_overhang_mode;
            uniform int u_use_wall_tex;
            uniform sampler2D u_wall_tex;
            uniform vec4 u_photo_bounds;
            uniform vec2 u_wall_size;
            uniform float u_photo_opacity;
            out vec4 frag_color;
            void main() {
                vec3 n = normalize(v_world_norm);
                vec3 v = normalize(u_eye - v_world_pos);
                vec3 key_dir = normalize(vec3(0.45, -0.65, 0.78));
                vec3 fill_dir = normalize(vec3(-0.60, 0.40, 0.45));

                float key = max(dot(n, key_dir), 0.0);
                float fill = max(dot(n, fill_dir), 0.0);
                float hemi = 0.5 + 0.5 * n.z;

                vec3 h = normalize(key_dir + v);
                float spec = pow(max(dot(n, h), 0.0), 36.0) * 0.25;

                vec3 base = u_color;
                if (u_use_wall_tex == 1) {
                    float wx = v_local_xy.x + u_wall_size.x * 0.5;
                    float wy = v_local_xy.y + u_wall_size.y * 0.5;
                    float u = (wx - u_photo_bounds.x) / max(u_photo_bounds.z, 1.0);
                    float v_uv = 1.0 - (wy - u_photo_bounds.y) / max(u_photo_bounds.w, 1.0);
                    if (u >= 0.0 && u <= 1.0 && v_uv >= 0.0 && v_uv <= 1.0) {
                        vec4 tex_c = texture(u_wall_tex, vec2(u, v_uv));
                        base = mix(base, tex_c.rgb, u_photo_opacity * tex_c.a);
                    }
                }
                if (u_overhang_mode == 1 && v_local_nz < -0.71 && v_local_z > 0.01) {
                    base = vec3(0.95, 0.16, 0.16);
                }

                vec3 lit = base * (0.26 + 0.16 * hemi + 0.62 * key + 0.22 * fill) + vec3(spec);
                frag_color = vec4(lit, u_alpha);
            }
        "#;

        let line_vs = r#"
            #version 150
            in vec3 a_pos;
            uniform mat4 u_mvp;
            void main() {
                gl_Position = u_mvp * vec4(a_pos, 1.0);
            }
        "#;

        let line_fs = r#"
            #version 150
            uniform vec3 u_color;
            out vec4 frag_color;
            void main() {
                frag_color = vec4(u_color, 1.0);
            }
        "#;

        let mesh_program = unsafe { compile_program(gl, mesh_vs, mesh_fs) };
        let line_program = unsafe { compile_program(gl, line_vs, line_fs) };

        let u_mvp = unsafe { gl.get_uniform_location(mesh_program, "u_mvp") };
        let u_model = unsafe { gl.get_uniform_location(mesh_program, "u_model") };
        let u_eye = unsafe { gl.get_uniform_location(mesh_program, "u_eye") };
        let u_color = unsafe { gl.get_uniform_location(mesh_program, "u_color") };
        let u_alpha = unsafe { gl.get_uniform_location(mesh_program, "u_alpha") };
        let u_overhang_mode = unsafe { gl.get_uniform_location(mesh_program, "u_overhang_mode") };
        let u_use_wall_tex = unsafe { gl.get_uniform_location(mesh_program, "u_use_wall_tex") };
        let u_wall_tex = unsafe { gl.get_uniform_location(mesh_program, "u_wall_tex") };
        let u_photo_bounds = unsafe { gl.get_uniform_location(mesh_program, "u_photo_bounds") };
        let u_wall_size = unsafe { gl.get_uniform_location(mesh_program, "u_wall_size") };
        let u_photo_opacity = unsafe { gl.get_uniform_location(mesh_program, "u_photo_opacity") };

        let u_line_mvp = unsafe { gl.get_uniform_location(line_program, "u_mvp") };
        let u_line_color = unsafe { gl.get_uniform_location(line_program, "u_color") };

        let (grid_vao, grid_vbo, grid_vertex_count) = unsafe { build_grid_buffers(gl) };

        Self {
            mesh_program,
            line_program,
            u_mvp,
            u_model,
            u_eye,
            u_color,
            u_alpha,
            u_overhang_mode,
            u_use_wall_tex,
            u_wall_tex,
            u_photo_bounds,
            u_wall_size,
            u_photo_opacity,
            u_line_mvp,
            u_line_color,
            ext_gpu: None,
            int_gpu: None,
            ac_gpu: None,
            wall_gpu: None,
            leaf_gpus: Vec::new(),
            sash_gpus: Vec::new(),
            handle_set_gpu: None,
            project_wall_gpu: None,
            project_openings_gpu: Vec::new(),
            uploaded_wall_version: 0,
            wall_photo_tex: None,
            uploaded_photo_version: 0,
            grid_vao,
            grid_vbo,
            grid_vertex_count,
            uploaded_version: 0,
        }
    }

    unsafe fn upload_wall_photo(&mut self, gl: &glow::Context, data: &LoadedPhotoGpuData) {
        unsafe {
            let tex = match self.wall_photo_tex {
                Some(t) => t,
                None => {
                    let t = gl.create_texture().unwrap();
                    self.wall_photo_tex = Some(t);
                    t
                }
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                data.width_px as i32,
                data.height_px as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&data.rgba)),
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
    }

    unsafe fn upload_project_wall(&mut self, gl: &glow::Context, built: &BuiltProjectWall) {
        unsafe {
            replace_mesh_gpu(gl, &mut self.project_wall_gpu, &built.wall_mesh);
            for old in self.project_openings_gpu.drain(..) {
                for m in [old.ext_gpu, old.int_gpu, old.ac_gpu].into_iter().flatten() {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
                for lg in old.leaf_gpus {
                    gl.delete_vertex_array(lg.main_gpu.vao);
                    gl.delete_buffer(lg.main_gpu.vbo);
                    for m in [lg.ext_gpu, lg.ac_gpu, lg.hardware_gpu, lg.accessory_gpu]
                        .into_iter()
                        .flatten()
                    {
                        gl.delete_vertex_array(m.vao);
                        gl.delete_buffer(m.vbo);
                    }
                }
                for sg in old.sash_gpus {
                    gl.delete_vertex_array(sg.mesh_gpu.vao);
                    gl.delete_buffer(sg.mesh_gpu.vbo);
                    if let Some(m) = sg.ac_gpu {
                        gl.delete_vertex_array(m.vao);
                        gl.delete_buffer(m.vbo);
                    }
                }
            }
            for (id, _, _, win_gen) in &built.placed_openings {
                let ext_gpu = (!win_gen.exterior_half.triangles.is_empty())
                    .then(|| upload_tri_mesh(gl, &win_gen.exterior_half));
                let int_gpu = (!win_gen.interior_half.triangles.is_empty())
                    .then(|| upload_tri_mesh(gl, &win_gen.interior_half));
                let ac_gpu = (!win_gen.acetate_sheet.triangles.is_empty())
                    .then(|| upload_tri_mesh(gl, &win_gen.acetate_sheet));
                let mut leaf_gpus = Vec::with_capacity(win_gen.door_leaves.len());
                for leaf in &win_gen.door_leaves {
                    leaf_gpus.push(LeafGpu {
                        main_gpu: upload_tri_mesh(gl, &leaf.mesh),
                        ext_gpu: leaf.exterior_leaf.as_ref().map(|m| upload_tri_mesh(gl, m)),
                        ac_gpu: leaf.leaf_acetate.as_ref().map(|m| upload_tri_mesh(gl, m)),
                        hardware_gpu: (!leaf.hardware_preview.triangles.is_empty())
                            .then(|| upload_tri_mesh(gl, &leaf.hardware_preview)),
                        accessory_gpu: (!leaf.accessory_preview.triangles.is_empty())
                            .then(|| upload_tri_mesh(gl, &leaf.accessory_preview)),
                    });
                }
                let mut sash_gpus = Vec::with_capacity(win_gen.window_sashes.len());
                for sash in &win_gen.window_sashes {
                    sash_gpus.push(WindowSashGpu {
                        mesh_gpu: upload_tri_mesh(gl, &sash.mesh),
                        ac_gpu: (!sash.sash_acetate.triangles.is_empty())
                            .then(|| upload_tri_mesh(gl, &sash.sash_acetate)),
                    });
                }
                self.project_openings_gpu.push(PlacedOpeningGpu {
                    id: id.clone(),
                    ext_gpu,
                    int_gpu,
                    ac_gpu,
                    leaf_gpus,
                    sash_gpus,
                });
            }
        }
    }

    unsafe fn upload_meshes(&mut self, gl: &glow::Context, w: &GeneratedWindow) {
        unsafe {
            replace_mesh_gpu(gl, &mut self.ext_gpu, &w.exterior_half);
            replace_mesh_gpu(gl, &mut self.int_gpu, &w.interior_half);
            replace_mesh_gpu(gl, &mut self.ac_gpu, &w.acetate_sheet);
            replace_mesh_gpu(gl, &mut self.wall_gpu, &w.wall_cutaway);
            replace_mesh_gpu(gl, &mut self.handle_set_gpu, &w.door_handle);

            for lg in self.leaf_gpus.drain(..) {
                gl.delete_vertex_array(lg.main_gpu.vao);
                gl.delete_buffer(lg.main_gpu.vbo);
                for m in [lg.ext_gpu, lg.ac_gpu, lg.hardware_gpu, lg.accessory_gpu]
                    .into_iter()
                    .flatten()
                {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
            }
            for leaf in &w.door_leaves {
                self.leaf_gpus.push(LeafGpu {
                    main_gpu: upload_tri_mesh(gl, &leaf.mesh),
                    ext_gpu: leaf.exterior_leaf.as_ref().map(|m| upload_tri_mesh(gl, m)),
                    ac_gpu: leaf.leaf_acetate.as_ref().map(|m| upload_tri_mesh(gl, m)),
                    hardware_gpu: (!leaf.hardware_preview.triangles.is_empty())
                        .then(|| upload_tri_mesh(gl, &leaf.hardware_preview)),
                    accessory_gpu: (!leaf.accessory_preview.triangles.is_empty())
                        .then(|| upload_tri_mesh(gl, &leaf.accessory_preview)),
                });
            }

            for sg in self.sash_gpus.drain(..) {
                gl.delete_vertex_array(sg.mesh_gpu.vao);
                gl.delete_buffer(sg.mesh_gpu.vbo);
                if let Some(m) = sg.ac_gpu {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
            }
            for sash in &w.window_sashes {
                self.sash_gpus.push(WindowSashGpu {
                    mesh_gpu: upload_tri_mesh(gl, &sash.mesh),
                    ac_gpu: (!sash.sash_acetate.triangles.is_empty())
                        .then(|| upload_tri_mesh(gl, &sash.sash_acetate)),
                });
            }
        }
    }

    unsafe fn destroy(self, gl: &glow::Context) {
        unsafe {
            for m in [
                self.ext_gpu,
                self.int_gpu,
                self.ac_gpu,
                self.wall_gpu,
                self.handle_set_gpu,
                self.project_wall_gpu,
            ]
            .into_iter()
            .flatten()
            {
                gl.delete_vertex_array(m.vao);
                gl.delete_buffer(m.vbo);
            }
            for op in self.project_openings_gpu {
                for m in [op.ext_gpu, op.int_gpu, op.ac_gpu].into_iter().flatten() {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
                for lg in op.leaf_gpus {
                    gl.delete_vertex_array(lg.main_gpu.vao);
                    gl.delete_buffer(lg.main_gpu.vbo);
                    for m in [lg.ext_gpu, lg.ac_gpu, lg.hardware_gpu, lg.accessory_gpu]
                        .into_iter()
                        .flatten()
                    {
                        gl.delete_vertex_array(m.vao);
                        gl.delete_buffer(m.vbo);
                    }
                }
                for sg in op.sash_gpus {
                    gl.delete_vertex_array(sg.mesh_gpu.vao);
                    gl.delete_buffer(sg.mesh_gpu.vbo);
                    if let Some(m) = sg.ac_gpu {
                        gl.delete_vertex_array(m.vao);
                        gl.delete_buffer(m.vbo);
                    }
                }
            }
            for lg in self.leaf_gpus {
                gl.delete_vertex_array(lg.main_gpu.vao);
                gl.delete_buffer(lg.main_gpu.vbo);
                for m in [lg.ext_gpu, lg.ac_gpu, lg.hardware_gpu, lg.accessory_gpu]
                    .into_iter()
                    .flatten()
                {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
            }
            for sg in self.sash_gpus {
                gl.delete_vertex_array(sg.mesh_gpu.vao);
                gl.delete_buffer(sg.mesh_gpu.vbo);
                if let Some(m) = sg.ac_gpu {
                    gl.delete_vertex_array(m.vao);
                    gl.delete_buffer(m.vbo);
                }
            }
            if let Some(t) = self.wall_photo_tex {
                gl.delete_texture(t);
            }
            gl.delete_vertex_array(self.grid_vao);
            gl.delete_buffer(self.grid_vbo);
            gl.delete_program(self.mesh_program);
            gl.delete_program(self.line_program);
        }
    }
}

unsafe fn replace_mesh_gpu(gl: &glow::Context, slot: &mut Option<MeshGpu>, mesh: &TriMesh) {
    unsafe {
        if let Some(old) = slot.take() {
            gl.delete_vertex_array(old.vao);
            gl.delete_buffer(old.vbo);
        }
        if mesh.triangles.is_empty() {
            return;
        }
        *slot = Some(upload_tri_mesh(gl, mesh));
    }
}

unsafe fn upload_tri_mesh(gl: &glow::Context, mesh: &TriMesh) -> MeshGpu {
    let mut floats: Vec<f32> = Vec::with_capacity(mesh.triangles.len() * 18);
    for tri in &mesh.triangles {
        let p0 = Vec3::from(mesh.positions[tri[0] as usize]);
        let p1 = Vec3::from(mesh.positions[tri[1] as usize]);
        let p2 = Vec3::from(mesh.positions[tri[2] as usize]);
        let raw_n = (p1 - p0).cross(p2 - p0);
        let n = if raw_n.length_squared() > 1e-12 {
            raw_n.normalize()
        } else {
            Vec3::Z
        };
        for p in [p0, p1, p2] {
            floats.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z]);
        }
    }
    let vertex_count = (floats.len() / 6) as i32;
    let bytes = floats_to_bytes(&floats);

    unsafe {
        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);

        let stride = 6 * 4;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 3 * 4);

        gl.bind_vertex_array(None);
        MeshGpu {
            vao,
            vbo,
            vertex_count,
        }
    }
}

unsafe fn build_grid_buffers(gl: &glow::Context) -> (glow::VertexArray, glow::Buffer, i32) {
    let mut verts: Vec<f32> = Vec::new();
    let half = 120.0f32;
    let step = 10.0f32;
    let count = (half / step) as i32;
    for i in -count..=count {
        let c = i as f32 * step;
        verts.extend_from_slice(&[-half, c, 0.0, half, c, 0.0]);
        verts.extend_from_slice(&[c, -half, 0.0, c, half, 0.0]);
    }
    let vertex_count = (verts.len() / 3) as i32;
    let bytes = floats_to_bytes(&verts);

    unsafe {
        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 3 * 4, 0);
        gl.bind_vertex_array(None);
        (vao, vbo, vertex_count)
    }
}

unsafe fn compile_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> glow::Program {
    unsafe {
        let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        gl.shader_source(vs, vs_src);
        gl.compile_shader(vs);
        assert!(
            gl.get_shader_compile_status(vs),
            "VS compile error: {}",
            gl.get_shader_info_log(vs)
        );

        let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(fs, fs_src);
        gl.compile_shader(fs);
        assert!(
            gl.get_shader_compile_status(fs),
            "FS compile error: {}",
            gl.get_shader_info_log(fs)
        );

        let prog = gl.create_program().unwrap();
        gl.attach_shader(prog, vs);
        gl.attach_shader(prog, fs);
        gl.bind_attrib_location(prog, 0, "a_pos");
        gl.bind_attrib_location(prog, 1, "a_norm");
        gl.link_program(prog);
        assert!(
            gl.get_program_link_status(prog),
            "Program link error: {}",
            gl.get_program_info_log(prog)
        );
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        prog
    }
}

fn floats_to_bytes(floats: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(floats.len() * 4);
    for &f in floats {
        bytes.extend_from_slice(&f.to_ne_bytes());
    }
    bytes
}
