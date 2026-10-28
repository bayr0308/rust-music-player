#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod theme;
mod window;

use std::path::PathBuf;

#[derive(Default)]
pub struct Options {
    pub folders: Vec<PathBuf>,
    pub data_dir: Option<PathBuf>,
}

fn main() -> eframe::Result {
    let mut options = Options::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--data-dir") => options.data_dir = args.next().map(PathBuf::from),
            _ => options.folders.push(PathBuf::from(arg)),
        }
    }
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Mellow")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/mellow-icon-256.png"))
                    .expect("valid bundled app icon"),
            )
            .with_inner_size([280.0, 480.0])
            .with_max_inner_size([280.0, 480.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_min_inner_size([280.0, 480.0])
            .with_decorations(true)
            .with_transparent(true),
        renderer: eframe::Renderer::Glow,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Mellow",
        native,
        Box::new(move |cc| Ok(Box::new(app::PlayerApp::new(cc, options)))),
    )
}
