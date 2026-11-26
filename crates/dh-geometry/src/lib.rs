pub mod builders;
pub mod extrude;
pub mod packer;
pub mod polygon2d;
pub mod validate;

pub use builders::{
    BuilderRegistry, BuiltProjectWall, ComponentBuilder, DoorLeafAsset, GeneratedWindow,
    WallMeshCache, WindowDoorBuilder, WindowSashAsset, build_window, clipped_trim_outer_contour,
    profile_kind,
};
pub use extrude::{
    HingeFace, MeshDiagnostics, MouldingConfig, Slab, TriMesh, VGroove, build_cone_knuckle_block,
    extrude_slabs, extrude_slabs_with_base_chamfer, extrude_slabs_with_moulding,
};
pub use packer::{BedPackLayout, PartPlacement, pack_part_bounds_onto_beds};
pub use polygon2d::{
    Contour2D, ProfileKind, Pt2, Shape2D, Shapes2D, circle_contour, contour_is_simple,
    difference_shapes, intersection_shapes, miter_inset_contour, miter_offset_contour,
    profile_band, profile_contour, rect_bounds, rect_centered, ring_shape, segment_bar,
    shape_from_contour, union_contours, union_shapes,
};
pub use validate::{ValidationSummary, validate_generated_piece};
