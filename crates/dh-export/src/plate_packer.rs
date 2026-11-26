use dh_core::{ComponentDef, Project, ProjectKind, ProjectWall};
use dh_geometry::{TriMesh, build_window};

/// Standard 3D printer build plate size presets for auto-nested wall/project batch packing.
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

fn append_wall_parts(wall: &ProjectWall, components: &[ComponentDef], out: &mut Vec<TriMesh>) {
    for op in &wall.openings {
        let params = op.resolve_params(components);
        let built = build_window(&params);
        out.push(built.printable_exterior_assembly());
        out.push(built.interior_half);
        for leaf in built.door_leaves {
            out.push(leaf.mesh);
            if let Some(ext_leaf) = leaf.exterior_leaf {
                out.push(ext_leaf);
            }
        }
        if !built.door_handle.triangles.is_empty() {
            out.push(built.door_handle);
        }
    }
}

/// Packs every printable part from `wall` onto `(bed_w x bed_h)` build plates on `Z = 0`.
pub fn pack_wall_onto_build_plates(
    wall: &ProjectWall,
    components: &[ComponentDef],
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> Vec<TriMesh> {
    let mut parts = Vec::new();
    append_wall_parts(wall, components, &mut parts);
    pack_meshes_onto_beds(parts, bed_w, bed_h, spacing)
}

/// Packs every printable part across an entire `Project` onto `(bed_w x bed_h)` build plates.
pub fn pack_project_onto_build_plates(
    project: &Project,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> Vec<TriMesh> {
    let mut parts = Vec::new();
    match project.manifest.kind {
        ProjectKind::SinglePiece => {
            for piece in &project.pieces {
                let built = build_window(&piece.params);
                parts.push(built.printable_exterior_assembly());
                parts.push(built.interior_half);
                for leaf in built.door_leaves {
                    parts.push(leaf.mesh);
                    if let Some(ext_leaf) = leaf.exterior_leaf {
                        parts.push(ext_leaf);
                    }
                }
                if !built.door_handle.triangles.is_empty() {
                    parts.push(built.door_handle);
                }
            }
        }
        ProjectKind::MultiWall => {
            for wall in &project.walls {
                append_wall_parts(wall, &project.components, &mut parts);
            }
        }
    }
    pack_meshes_onto_beds(parts, bed_w, bed_h, spacing)
}

fn pack_meshes_onto_beds(
    parts: Vec<TriMesh>,
    bed_w: f32,
    bed_h: f32,
    spacing: f32,
) -> Vec<TriMesh> {
    if parts.is_empty() {
        return Vec::new();
    }

    let usable_w = (bed_w - 10.0).max(40.0);
    let usable_h = (bed_h - 10.0).max(40.0);
    let gap = spacing.clamp(1.0, 30.0);

    let mut sized_parts: Vec<(TriMesh, f32, f32)> = parts
        .into_iter()
        .filter(|m| !m.triangles.is_empty())
        .map(|mut m| {
            let (bmin, bmax) = m.bounds();
            let mut pw = (bmax[0] - bmin[0]).max(0.1);
            let mut ph = (bmax[1] - bmin[1]).max(0.1);
            if pw > usable_w && ph <= usable_w {
                for p in &mut m.positions {
                    let x = p[0];
                    let y = p[1];
                    p[0] = -y;
                    p[1] = x;
                }
                let (rmin, rmax) = m.bounds();
                pw = (rmax[0] - rmin[0]).max(0.1);
                ph = (rmax[1] - rmin[1]).max(0.1);
            }
            (m, pw, ph)
        })
        .collect();
    sized_parts.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let mut plates: Vec<TriMesh> = Vec::new();
    let mut current_plate = TriMesh::default();
    let mut cursor_x = 0.0f32;
    let mut shelf_y = 0.0f32;
    let mut shelf_h = 0.0f32;

    for (mut mesh, pw, ph) in sized_parts {
        if cursor_x > 0.0 && cursor_x + pw > usable_w {
            cursor_x = 0.0;
            shelf_y += shelf_h + gap;
            shelf_h = 0.0;
        }
        if shelf_y > 0.0 && shelf_y + ph > usable_h && !current_plate.triangles.is_empty() {
            center_plate_on_bed(&mut current_plate);
            plates.push(current_plate);
            current_plate = TriMesh::default();
            cursor_x = 0.0;
            shelf_y = 0.0;
            shelf_h = 0.0;
        }

        let (bmin, _bmax) = mesh.bounds();
        mesh.translate(cursor_x - bmin[0], shelf_y - bmin[1], -bmin[2]);
        current_plate.append(&mesh);

        cursor_x += pw + gap;
        shelf_h = shelf_h.max(ph);
    }

    if !current_plate.triangles.is_empty() {
        center_plate_on_bed(&mut current_plate);
        plates.push(current_plate);
    }

    plates
}

fn center_plate_on_bed(plate: &mut TriMesh) {
    let (bmin, bmax) = plate.bounds();
    let cx = (bmin[0] + bmax[0]) * 0.5;
    let cy = (bmin[1] + bmax[1]) * 0.5;
    plate.translate(-cx, -cy, -bmin[2]);
}
