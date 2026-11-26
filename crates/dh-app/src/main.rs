#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dh_app::app::DollhouseStudioApp;
use eframe::egui;
use std::sync::Arc;

fn load_window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../../../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Dollhouse Studio")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1024.0, 680.0]);
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        depth_buffer: 24,
        multisampling: 4,
        ..Default::default()
    };

    eframe::run_native(
        "Dollhouse Studio",
        options,
        Box::new(|cc| Ok(Box::new(DollhouseStudioApp::new(cc)))),
    )
}
