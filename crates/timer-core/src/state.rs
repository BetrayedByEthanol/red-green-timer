use serde::Serialize;
use uuid::Uuid;

use crate::model::PhaseType;

/// IPC-safe point-in-time view. Durations are rounded up so a non-zero
/// remainder is never displayed as zero before its deadline is processed.
#[derive(Debug, Clone, Serialize)]
pub struct TimerSnapshot {
    pub active: bool,
    pub phase: Option<PhaseType>,
    pub cycle_index: Option<u32>,
    pub remaining_seconds: u64,
    pub timer_name: String,
    pub run_id: Option<Uuid>,
    pub completed_phase_count: usize,
    pub green_duration_seconds: u64,
    pub red_duration_seconds: u64,
}
