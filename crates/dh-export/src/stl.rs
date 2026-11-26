use dh_geometry::TriMesh;
use glam::Vec3;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Writes a `TriMesh` to a binary STL file (units: millimeters, Z-up outside-down orientation).
pub fn write_binary_stl(mesh: &TriMesh, path: &Path, header_label: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_binary_stl_to_writer(mesh, &mut writer, header_label)?;
    writer.flush()
}

pub fn write_binary_stl_to_writer<W: Write>(
    mesh: &TriMesh,
    writer: &mut W,
    header_label: &str,
) -> std::io::Result<()> {
    let mut header = [b' '; 80];
    let prefix = format!("Dollhouse Studio STL ({header_label}) - Units: mm");
    let bytes = prefix.as_bytes();
    let copy_len = bytes.len().min(80);
    header[..copy_len].copy_from_slice(&bytes[..copy_len]);
    writer.write_all(&header)?;

    let tri_count = mesh.triangles.len() as u32;
    writer.write_all(&tri_count.to_le_bytes())?;

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

        for v in [n.x, n.y, n.z] {
            writer.write_all(&v.to_le_bytes())?;
        }
        for p in [p0, p1, p2] {
            for v in [p.x, p.y, p.z] {
                writer.write_all(&v.to_le_bytes())?;
            }
        }
        writer.write_all(&0u16.to_le_bytes())?;
    }

    Ok(())
}
