use dh_core::{ComponentDef, Project, ProjectKind, ProjectWall};
use dh_geometry::{GeneratedWindow, TriMesh, build_window, pack_part_bounds_onto_beds};
use std::collections::HashMap;

/// Standard 3D printer build plate size presets for auto-nested batch packing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedSizePreset {
    Mini180,
    Standard220,
    Bambu256,
    Large300,
}

impl BedSizePreset {
    pub const ALL: [BedSizePreset; 4] = [
        BedSizePreset::Mini180,
        BedSizePreset::Standard220,
        BedSizePreset::Bambu256,
        BedSizePreset::Large300,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BedSizePreset::Mini180 => "180 x 180 mm (Mini)",
            BedSizePreset::Standard220 => "220 x 220 mm (Ender/Prusa)",
            BedSizePreset::Bambu256 => "256 x 256 mm (Bambu/CoreXY)",
            BedSizePreset::Large300 => "300 x 300 mm (Large)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            BedSizePreset::Mini180 => "180²",
            BedSizePreset::Standard220 => "220²",
            BedSizePreset::Bambu256 => "256²",
            BedSizePreset::Large300 => "300²",
        }
    }

    pub fn dimensions_mm(self) -> (f32, f32) {
        match self {
            BedSizePreset::Mini180 => (180.0, 180.0),
            BedSizePreset::Standard220 => (220.0, 220.0),
            BedSizePreset::Bambu256 => (256.0, 256.0),
            BedSizePreset::Large300 => (300.0, 300.0),
        }
    }
}

/// Output of multi-plate 2D build-plate bin packing.
#[derive(Debug, Clone, Default)]
pub struct PackedPlatesReport {
    /// Combined watertight `TriMesh` plates (each centered at `(0, 0)` on `Z = 0`).
    pub plates: Vec<TriMesh>,
    /// Names of individual parts whose own footprint exceeded usable bed dimensions in both orientations.
    pub oversized_parts: Vec<String>,
}

fn sanitize_stem(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_sep = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        } else if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "part".to_string()
    } else {
        trimmed
    }
}

fn append_named_window_parts(
    prefix: &str,
    built: &GeneratedWindow,
    out: &mut Vec<(String, TriMesh)>,
) {
    let clean_prefix = sanitize_stem(prefix);
    for (part_name, mesh) in built.named_printable_parts() {
        if !mesh.triangles.is_empty() {
            out.push((format!("{clean_prefix}_{part_name}"), mesh));
        }
    }
}

fn append_named_wall_parts(
    wall: &ProjectWall,
    wall_prefix: Option<&str>,
    components: &[ComponentDef],
    out: &mut Vec<(String, TriMesh)>,
) {
    for (idx, op) in wall.openings.iter().enumerate() {
        let params = op.resolve_params(components);
        let built = build_window(&params);
        let op_clean = sanitize_stem(&op.name);
        let prefix = match wall_prefix {
            Some(wp) => format!("{wp}_{:02}_{op_clean}", idx + 1),
            None => format!("{:02}_{op_clean}", idx + 1),
        };
        append_named_window_parts(&prefix, &built, out);
    }
}

fn deduplicate_part_names(parts: Vec<(String, TriMesh)>) -> Vec<(String, TriMesh)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(parts.len());
    for (stem, mesh) in parts {
        let n = counts.entry(stem.clone()).or_insert(0);
        *n += 1;
        let final_name = if *n == 1 {
            stem
        } else {
            format!("{stem}_{}", *n)
        };
        out.push((final_name, mesh));
    }
    out
}

/// Packs a single built piece onto one or more `(bed_w x bed_h)` plates on `Z = 0`.
pub fn pack_piece_onto_build_plates(
    window: &GeneratedWindow,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> PackedPlatesReport {
    pack_meshes_onto_beds_named(window.named_printable_parts(), bed_w, bed_h, spacing)
}

/// Packs every printable part from `wall` onto `(bed_w x bed_h)` build plates with oversized-part diagnostics.
pub fn pack_wall_onto_build_plates_with_report(
    wall: &ProjectWall,
    components: &[ComponentDef],
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> PackedPlatesReport {
    let mut parts = Vec::new();
    append_named_wall_parts(wall, None, components, &mut parts);
    pack_meshes_onto_beds_named(parts, bed_w, bed_h, spacing)
}

/// Packs every printable part from `wall` onto `(bed_w x bed_h)` build plates on `Z = 0`.
pub fn pack_wall_onto_build_plates(
    wall: &ProjectWall,
    components: &[ComponentDef],
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> Vec<TriMesh> {
    pack_wall_onto_build_plates_with_report(wall, components, bed_w, bed_h, spacing).plates
}

/// Packs every printable part across an entire `Project` onto `(bed_w x bed_h)` build plates
/// with oversized-part diagnostics.
pub fn pack_project_onto_build_plates_with_report(
    project: &Project,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> PackedPlatesReport {
    let mut parts = Vec::new();
    match project.manifest.kind {
        ProjectKind::SinglePiece => {
            for (idx, piece) in project.pieces.iter().enumerate() {
                let built = build_window(&piece.params);
                let prefix = format!("{:02}_{}", idx + 1, sanitize_stem(&piece.name));
                append_named_window_parts(&prefix, &built, &mut parts);
            }
        }
        ProjectKind::MultiWall => {
            for (w_idx, wall) in project.walls.iter().enumerate() {
                let wp = format!("w{:02}_{}", w_idx + 1, sanitize_stem(&wall.name));
                append_named_wall_parts(wall, Some(&wp), &project.components, &mut parts);
            }
        }
    }
    pack_meshes_onto_beds_named(parts, bed_w, bed_h, spacing)
}

/// Packs every printable part across an entire `Project` onto `(bed_w x bed_h)` build plates.
pub fn pack_project_onto_build_plates(
    project: &Project,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> Vec<TriMesh> {
    pack_project_onto_build_plates_with_report(project, bed_w, bed_h, spacing).plates
}

/// Collects individual named STLs (one file per printable part, centered on `Z = 0`) for a single piece.
pub fn collect_piece_individual_stls(
    piece_prefix: &str,
    window: &GeneratedWindow,
) -> Vec<(String, TriMesh)> {
    let mut parts = Vec::new();
    append_named_window_parts(piece_prefix, window, &mut parts);
    deduplicate_part_names(parts)
}

/// Collects individual named STLs (one file per printable part, centered on `Z = 0`) for a wall.
pub fn collect_wall_individual_stls(
    wall: &ProjectWall,
    components: &[ComponentDef],
) -> Vec<(String, TriMesh)> {
    let mut parts = Vec::new();
    append_named_wall_parts(wall, None, components, &mut parts);
    deduplicate_part_names(parts)
}

/// Collects individual named STLs (one file per printable part, centered on `Z = 0`) for an entire project.
pub fn collect_project_individual_stls(project: &Project) -> Vec<(String, TriMesh)> {
    let mut parts = Vec::new();
    match project.manifest.kind {
        ProjectKind::SinglePiece => {
            for (idx, piece) in project.pieces.iter().enumerate() {
                let built = build_window(&piece.params);
                let prefix = format!("{:02}_{}", idx + 1, sanitize_stem(&piece.name));
                append_named_window_parts(&prefix, &built, &mut parts);
            }
        }
        ProjectKind::MultiWall => {
            for (w_idx, wall) in project.walls.iter().enumerate() {
                let wp = format!("w{:02}_{}", w_idx + 1, sanitize_stem(&wall.name));
                append_named_wall_parts(wall, Some(&wp), &project.components, &mut parts);
            }
        }
    }
    deduplicate_part_names(parts)
}

fn pack_meshes_onto_beds_named(
    parts: Vec<(String, TriMesh)>,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> PackedPlatesReport {
    let non_empty: Vec<(String, TriMesh)> = parts
        .into_iter()
        .filter(|(_, m)| !m.triangles.is_empty())
        .collect();
    if non_empty.is_empty() {
        return PackedPlatesReport::default();
    }

    let sizes: Vec<(f32, f32)> = non_empty
        .iter()
        .map(|(_, m)| {
            let (bmin, bmax) = m.bounds();
            ((bmax[0] - bmin[0]).max(0.1), (bmax[1] - bmin[1]).max(0.1))
        })
        .collect();

    let layout = pack_part_bounds_onto_beds(&sizes, bed_w, bed_h, spacing);
    let mut plates = vec![TriMesh::default(); layout.plate_count];
    for pl in layout.placements {
        let mut mesh = non_empty[pl.part_index].1.clone();
        if pl.rotated_90 {
            mesh.rotate_z_90_ccw();
        }
        let (bmin, _) = mesh.bounds();
        mesh.translate(
            pl.target_min_x - bmin[0],
            pl.target_min_y - bmin[1],
            -bmin[2],
        );
        plates[pl.plate_index].append(&mesh);
    }
    plates.retain(|p| !p.triangles.is_empty());

    let oversized_parts = layout
        .oversized_indices
        .into_iter()
        .map(|i| non_empty[i].0.clone())
        .collect();

    PackedPlatesReport {
        plates,
        oversized_parts,
    }
}
