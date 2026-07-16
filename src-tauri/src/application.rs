use std::sync::Mutex;

use timer_core::{TimerConfig, TimerEngine, TimerError};

/// Shared, Tauri-managed application state.
///
/// The engine is wrapped in a `Mutex` because Tauri commands run on a
/// thread pool: multiple frontend calls (start/pause/tick/etc.) can arrive
/// concurrently and must serialize access to the single timer instance.
pub struct AppState {
    pub engine: Mutex<TimerEngine>,
}

impl AppState {
    pub fn new(config: TimerConfig) -> Result<Self, TimerError> {
        Ok(Self {
            engine: Mutex::new(TimerEngine::new(config)?),
        })
    }
}
