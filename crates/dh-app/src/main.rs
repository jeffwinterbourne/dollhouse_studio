#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dh_app::app::DollhouseStudioApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dollhouse Studio")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 680.0]),
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
