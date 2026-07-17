use tauri::State;
use timer_core::{TimerError, TimerSnapshot};

use crate::application::AppState;

/// Maps a poisoned-mutex error to a plain string, since Tauri command
/// errors must cross the IPC boundary as `Serialize` (a poisoned lock
/// means a prior command panicked while holding it, which shouldn't
/// happen in normal operation but is handled defensively here).
fn poisoned() -> String {
    "timer engine lock was poisoned".to_string()
}

#[tauri::command]
pub fn start_timer(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine.start().map_err(|e: TimerError| e.to_string())?;
    Ok(engine.snapshot())
}

#[tauri::command]
pub fn pause_timer(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine.pause().map_err(|e: TimerError| e.to_string())?;
    Ok(engine.snapshot())
}

#[tauri::command]
pub fn reset_timer(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine.reset();
    Ok(engine.snapshot())
}

/// Called on a frontend-driven interval (e.g. every second) so the engine
/// can catch up to wall-clock time and report the latest state.
#[tauri::command]
pub fn tick_timer(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    Ok(engine.tick())
}
