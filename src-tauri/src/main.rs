// Prevents an additional console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod commands;

use application::AppState;
use std::time::Duration;
use timer_core::TimerDefinition;
use uuid::Uuid;

fn main() {
    // TODO: load from persisted user settings instead of hardcoding once a
    // settings/config command is added.
    let definition = TimerDefinition::new(
        Uuid::new_v4(),
        "Red-Green Light",
        Duration::from_secs(40),
        Duration::from_secs(20),
    )
    .expect("default timer definition must be valid");
    let state = AppState::new(definition).expect("failed to initialize timer engine");

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::start_timer,
            commands::stop_green,
            commands::stop_run,
            commands::tick_timer,
            commands::get_timer_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
