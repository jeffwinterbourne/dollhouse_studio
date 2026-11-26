use crate::builders::GeneratedWindow;

/// Summary of manifold and printability checks across every printable part of a generated piece.
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub is_valid: bool,
    pub total_triangles: usize,
    pub total_overhangs: usize,
    pub summary_line: String,
    pub errors: Vec<String>,
}

/// Verifies that every printable part in `piece` is a closed 2-manifold sitting flat on `Z = 0`
/// with zero unprintable overhangs exceeding 45 degrees.
pub fn validate_generated_piece(piece: &GeneratedWindow) -> ValidationSummary {
    let mut errors = Vec::new();
    let mut total_triangles = 0usize;
    let mut total_overhangs = 0usize;

    let mut check_part = |label: &str, mesh: &crate::extrude::TriMesh| {
        if mesh.triangles.is_empty() {
            return;
        }
        match mesh.verify_printable_manifold() {
            Ok(diag) => {
                total_triangles += diag.triangle_count;
                total_overhangs += diag.overhang_triangles;
                if diag.overhang_triangles > 0 {
                    errors.push(format!("{label}: {} overhang triangle(s)", diag.overhang_triangles));
                }
                if diag.min_z.abs() > 1e-3 {
                    errors.push(format!("{label}: not seated on Z=0 (min_z={:.3})", diag.min_z));
                }
            }
            Err(e) => errors.push(format!("{label}: {e}")),
        }
    };

    check_part("Exterior half", &piece.exterior_half);
    check_part("Interior half", &piece.interior_half);
    for (idx, sash) in piece.window_sashes.iter().enumerate() {
        check_part(&format!("Window sash {}", idx + 1), &sash.mesh);
    }
    for (idx, leaf) in piece.door_leaves.iter().enumerate() {
        check_part(&format!("Door leaf {}", idx + 1), &leaf.mesh);
        if let Some(ext) = &leaf.exterior_leaf {
            check_part(&format!("Door exterior leaf {}", idx + 1), ext);
        }
    }
    check_part("Door hardware", &piece.door_handle);

    let is_valid = errors.is_empty() && piece.printable_ok;
    let summary_line = if is_valid {
        format!("{total_triangles} tris | watertight | 0 overhangs")
    } else {
        errors.join("; ")
    };

    ValidationSummary {
        is_valid,
        total_triangles,
        total_overhangs,
        summary_line,
        errors,
    }
}
