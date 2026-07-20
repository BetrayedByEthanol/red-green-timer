#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod application;
mod commands;
use application::{AppState, ApplicationController};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("red-green-timer.sqlite3");
            let controller =
                tauri::async_runtime::block_on(ApplicationController::open_file(&db_path))?;
            let state: AppState = Arc::new(Mutex::new(controller));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_timers,
            commands::create_timer,
            commands::update_timer,
            commands::archive_timer,
            commands::start_timer,
            commands::stop_green,
            commands::stop_run,
            commands::tick_timer,
            commands::get_timer_snapshot,
            commands::list_recent_runs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
