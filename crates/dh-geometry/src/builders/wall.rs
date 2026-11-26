use crate::builders::window::{GeneratedWindow, build_window, profile_kind};
use crate::extrude::{Slab, TriMesh, extrude_slabs};
use crate::polygon2d::{
    Shapes2D, difference_shapes, profile_contour, rect_centered, shape_from_contour, union_shapes,
};
use dh_core::{ComponentDef, ProjectWall, WindowParams};
use std::collections::HashMap;
use std::sync::Arc;

/// Built 3D representation of a `ProjectWall` with all punched cutouts and cached window/door meshes.
#[derive(Debug, Clone)]
pub struct BuiltProjectWall {
    pub wall: ProjectWall,
    pub wall_mesh: TriMesh,
    /// `(opening_id, center_x, center_y, generated_window)` in wall-centered coordinates.
    pub placed_openings: Vec<(String, f32, f32, Arc<GeneratedWindow>)>,
}

/// Incremental cache so moving openings or editing one component only regenerates affected openings.
#[derive(Default)]
pub struct WallMeshCache {
    opening_cache: HashMap<String, (WindowParams, Arc<GeneratedWindow>)>,
}

impl WallMeshCache {
    pub fn build_wall(
        &mut self,
        wall: &ProjectWall,
        components: &[ComponentDef],
    ) -> BuiltProjectWall {
        let mut cutouts: Shapes2D = Vec::new();
        let mut placed_openings = Vec::with_capacity(wall.openings.len());

        // Prune deleted openings from cache
        self.opening_cache
            .retain(|id, _| wall.openings.iter().any(|op| &op.id == id));

        for op in &wall.openings {
            let resolved = op.resolve_params(components);
            let win_gen = if let Some((cached_params, cached_win)) = self.opening_cache.get(&op.id)
            {
                if cached_params == &resolved {
                    Arc::clone(cached_win)
                } else {
                    let built = Arc::new(build_window(&resolved));
                    self.opening_cache
                        .insert(op.id.clone(), (resolved.clone(), Arc::clone(&built)));
                    built
                }
            } else {
                let built = Arc::new(build_window(&resolved));
                self.opening_cache
                    .insert(op.id.clone(), (resolved.clone(), Arc::clone(&built)));
                built
            };

            let cx = -wall.width * 0.5 + op.x_from_left + resolved.opening_width * 0.5;
            let cy = -wall.height * 0.5 + op.y_from_bottom + resolved.opening_height * 0.5;

            let mut contour = profile_contour(
                profile_kind(resolved.style),
                resolved.opening_width,
                resolved.opening_height,
                0.0,
                32,
            );
            for p in &mut contour {
                p[0] += cx;
                p[1] += cy;
            }
            cutouts = union_shapes(&cutouts, &shape_from_contour(contour));
            placed_openings.push((op.id.clone(), cx as f32, cy as f32, win_gen));
        }

        let wall_outer = shape_from_contour(rect_centered(0.0, 0.0, wall.width, wall.height));
        let wall_region = difference_shapes(&wall_outer, &cutouts);
        let wall_mesh = extrude_slabs(&[Slab {
            z0: 0.0,
            z1: wall.wall_thickness.clamp(2.0, 35.0),
            region: wall_region,
        }]);

        BuiltProjectWall {
            wall: wall.clone(),
            wall_mesh,
            placed_openings,
        }
    }
}
