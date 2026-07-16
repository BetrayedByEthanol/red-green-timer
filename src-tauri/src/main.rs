// Prevents an additional console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;

use application::AppState;
use timer_core::TimerConfig;

fn main() {
    // TODO: load from persisted user settings instead of hardcoding once a
    // settings/config command is added.
    let config = TimerConfig::default();
    let state = AppState::new(config).expect("failed to initialize timer engine");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::start_timer,
            commands::pause_timer,
            commands::reset_timer,
            commands::tick_timer,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
