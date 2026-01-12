#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::NativeOptions;

mod app;
mod game;
mod save;
mod uobject;

fn main() -> eframe::Result<()> {
    let initial_path = std::env::args().nth(1).map(PathBuf::from);

    let options = NativeOptions::default();
    eframe::run_native(
        "Silent Hill f Save Editor",
        options,
        Box::new(|cc| Ok(Box::new(app::AppState::load_app(cc, initial_path)))),
    )
}

