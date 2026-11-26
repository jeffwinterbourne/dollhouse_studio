use dh_core::sanitize_filename;
use std::fs;
use std::path::{Component, Path, PathBuf};

const ALLOWED_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "ico"];

/// Copies `source_path` into `<project_dir>/assets/photos/` using a collision-safe filename
/// and returns the portable relative POSIX path (e.g. `"assets/photos/front_wall.jpg"`).
pub fn import_wall_photo(project_dir: &Path, source_path: &Path) -> Result<String, String> {
    if !source_path.is_file() {
        return Err(format!("Photo file does not exist: {}", source_path.display()));
    }
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "png".to_string());
    if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("Unsupported image format '.{ext}'"));
    }

    let photos_dir = project_dir.join("assets").join("photos");
    fs::create_dir_all(&photos_dir)
        .map_err(|e| format!("Failed creating project assets directory: {e}"))?;

    let raw_stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("wall_photo");
    let stem = sanitize_filename(raw_stem);

    let mut candidate_name = format!("{stem}.{ext}");
    let mut counter = 2usize;
    while photos_dir.join(&candidate_name).exists() {
        candidate_name = format!("{stem}_{counter}.{ext}");
        counter += 1;
    }

    let dest = photos_dir.join(&candidate_name);
    fs::copy(source_path, &dest)
        .map_err(|e| format!("Failed copying photo into project assets: {e}"))?;

    Ok(format!("assets/photos/{candidate_name}"))
}

/// Safely resolves a project-relative asset path inside `project_dir`, blocking absolute
/// paths or `..` directory traversal segments.
pub fn resolve_asset_path(project_dir: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let rel = Path::new(rel_path);
    if rel.is_absolute() {
        return Err("Asset path must be relative to project folder".to_string());
    }
    let mut resolved = project_dir.to_path_buf();
    for comp in rel.components() {
        match comp {
            Component::Normal(seg) => resolved.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Unsafe relative asset path: '{rel_path}'"));
            }
        }
    }
    Ok(resolved)
}
