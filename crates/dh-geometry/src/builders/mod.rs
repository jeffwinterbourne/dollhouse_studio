pub mod door;
pub mod wall;
pub mod window;

pub use door::DoorLeafAsset;
pub use wall::{BuiltProjectWall, WallMeshCache};
pub use window::{
    GeneratedWindow, WindowSashAsset, build_window, clipped_trim_outer_contour, profile_kind,
};

use dh_core::WindowParams;

/// Pluggable procedural CAD builder interface allowing future component families
/// (e.g. bay windows, dormers, stairs) to share the generation and validation pipeline.
pub trait ComponentBuilder {
    fn name(&self) -> &'static str;
    fn build(&self, params: &WindowParams) -> GeneratedWindow;
}

/// Default procedural builder for all window and hinged door styles.
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowDoorBuilder;

impl ComponentBuilder for WindowDoorBuilder {
    fn name(&self) -> &'static str {
        "window_and_door"
    }

    fn build(&self, params: &WindowParams) -> GeneratedWindow {
        build_window(params)
    }
}

/// Central registry of CAD builders.
pub struct BuilderRegistry {
    window_door: WindowDoorBuilder,
}

impl Default for BuilderRegistry {
    fn default() -> Self {
        Self {
            window_door: WindowDoorBuilder,
        }
    }
}

impl BuilderRegistry {
    pub fn build_window_or_door(&self, params: &WindowParams) -> GeneratedWindow {
        self.window_door.build(params)
    }

    pub fn default_builder(&self) -> &dyn ComponentBuilder {
        &self.window_door
    }
}
