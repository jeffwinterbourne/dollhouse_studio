use crate::store::{read_json, write_json_atomic};
use dh_core::{PieceTemplate, WindowParams, WindowStyle, sanitize_filename};
use std::fs;
use std::path::{Path, PathBuf};

/// Lists all saved personal templates from `personal_dir`, sorted alphabetically.
pub fn list_personal_templates(personal_dir: &Path) -> Vec<PieceTemplate> {
    let mut out = Vec::new();
    if !personal_dir.is_dir() {
        return out;
    }
    if let Ok(entries) = fs::read_dir(personal_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.ends_with(".template.json"))
                && let Ok(tpl) = read_json::<PieceTemplate>(&p)
            {
                out.push(tpl);
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// Saves a template into `personal_dir` atomically.
pub fn save_personal_template(
    personal_dir: &Path,
    tpl: &PieceTemplate,
) -> Result<PathBuf, String> {
    fs::create_dir_all(personal_dir)
        .map_err(|e| format!("Failed creating personal templates dir: {e}"))?;
    let fname = format!(
        "{}_{}.template.json",
        sanitize_filename(&tpl.name),
        sanitize_filename(&tpl.id)
    );
    let path = personal_dir.join(fname);
    write_json_atomic(&path, tpl)?;
    Ok(path)
}

/// Deletes a personal template matching `template_id` from `personal_dir`.
pub fn delete_personal_template(personal_dir: &Path, template_id: &str) -> Result<(), String> {
    if !personal_dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(personal_dir)
        .map_err(|e| format!("Failed reading personal templates dir: {e}"))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file()
            && p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.ends_with(".template.json"))
            && let Ok(tpl) = read_json::<PieceTemplate>(&p)
            && tpl.id == template_id
        {
            fs::remove_file(&p)
                .map_err(|e| format!("Failed deleting template file {}: {e}", p.display()))?;
        }
    }
    Ok(())
}

/// Exports a single template to a shareable `.dhtemplate.json` file.
pub fn export_template_file(tpl: &PieceTemplate, path: &Path) -> Result<(), String> {
    write_json_atomic(path, tpl)
}

/// Imports a template from a `.dhtemplate.json` file, assigning a fresh template ID.
pub fn import_template_file(path: &Path) -> Result<PieceTemplate, String> {
    let mut tpl: PieceTemplate = read_json(path)?;
    tpl.id = dh_core::new_id("tpl");
    tpl.updated_at_epoch = dh_core::now_epoch();
    Ok(tpl)
}

/// Curated starter templates so new installations immediately have ready-to-use examples.
pub fn starter_templates() -> Vec<PieceTemplate> {
    let mut georgian = WindowParams::default();
    WindowStyle::GeorgianGrid.apply_style_defaults(&mut georgian);

    let mut sash = WindowParams::default();
    WindowStyle::SashSplit.apply_style_defaults(&mut sash);
    sash.shutter_style = dh_core::ShutterStyle::Panelled;
    sash.flower_box_enabled = true;

    let mut arch = WindowParams::default();
    WindowStyle::ArchedGeorgian.apply_style_defaults(&mut arch);
    arch.header_style = dh_core::HeaderStyle::KeystoneArch;

    let mut door = WindowParams::default();
    WindowStyle::DoorFanlightGeorgian.apply_style_defaults(&mut door);
    door.door_handle_style = dh_core::DoorHandleStyle::VictorianScroll;
    door.letterbox_enabled = true;

    vec![
        PieceTemplate::from_params(
            "Georgian 12-Pane Parlor",
            "Classic 1:12 multi-pane Georgian window with chamfered architrave",
            vec!["1:12".into(), "georgian".into()],
            georgian,
        ),
        PieceTemplate::from_params(
            "Cottage Sash + Shutters & Planter",
            "Split sash window dressed with panelled shutters and sill flower box",
            vec!["1:12".into(), "cottage".into()],
            sash,
        ),
        PieceTemplate::from_params(
            "Arched Staircase Fanlight",
            "Semicircular arch window with radial fanlight spokes",
            vec!["1:12".into(), "arched".into()],
            arch,
        ),
        PieceTemplate::from_params(
            "Georgian Entry Door + Letterbox",
            "Hinged entry door with glazed fanlight, Victorian scroll lever, and letterbox",
            vec!["1:12".into(), "door".into()],
            door,
        ),
    ]
}
