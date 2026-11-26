pub mod commands;
pub mod component;
pub mod params;
pub mod project;
pub mod template;
pub mod units;
pub mod wall;

pub use commands::{UndoEntry, UndoHistory};
pub use component::{ComponentDef, InstanceOverrides, OpeningSource};
pub use params::{
    ArchitraveStyle, DoorHandleStyle, FilamentColor, FlushTopMode, GrooveStyle, HeaderStyle,
    PrinterPreset, ScalePreset, ShutterStyle, ViewMode, WindowOpeningMode, WindowParams,
    WindowSpecs, WindowStyle,
};
pub use project::{
    Project, ProjectKind, ProjectManifest, SCHEMA_VERSION, StandalonePiece, new_id, now_epoch,
    sanitize_filename,
};
pub use template::{PieceTemplate, TemplateCategory, TemplateScope};
pub use units::DisplayUnit;
pub use wall::{
    CornerKind, DimAnchor, DimOrientation, OpeningEdgeKind, ProjectWall, WallDimension,
    WallEdgeKind, WallOpening, WallPhotoConfig, WallPlacement3D,
};
