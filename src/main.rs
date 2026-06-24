#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod monitor;
mod ui;

use eframe::egui;
use single_instance::SingleInstance;
use std::sync::Arc;

fn main() -> eframe::Result {
    let instance = SingleInstance::new("BluePresence-1519354091879530506")
        .expect("single-instance guard could not be created");
    if !instance.is_single() {
        return Ok(());
    }

    let first_run = !config::config_exists();
    let icon = load_icon_data();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BluePresence")
            .with_inner_size([440.0, 360.0])
            .with_min_inner_size([400.0, 340.0])
            .with_resizable(true)
            .with_visible(first_run)
            .with_icon(Arc::new(icon.clone())),
        ..Default::default()
    };

    eframe::run_native(
        "BluePresence",
        options,
        Box::new(move |creation_context| {
            Ok(Box::new(ui::BluePresenceApp::new(
                creation_context,
                icon.clone(),
                first_run,
            )))
        }),
    )
}

fn load_icon_data() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../bluearchivelogo.ico"))
        .expect("embedded application icon must be valid")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
