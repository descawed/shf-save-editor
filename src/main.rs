#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::NativeOptions;

mod app;
mod game;
mod save;
mod uobject;

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Silent Hill f Save Editor",
        options,
        Box::new(|_cc| Ok(Box::<app::AppState>::default())),
    )
}

