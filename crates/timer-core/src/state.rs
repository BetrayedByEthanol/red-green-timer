use serde::{Deserialize, Serialize};

use crate::model::Phase;

/// A point-in-time snapshot of the timer, suitable for sending across the
/// Tauri IPC boundary to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSnapshot {
    pub phase: Phase,
    pub remaining_seconds: u64,
    pub running: bool,
    pub cycle_count: u64,
}
