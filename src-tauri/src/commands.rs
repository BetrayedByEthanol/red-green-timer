use serde::Serialize;
use tauri::State;
use timer_core::{CompletedRunSummary, TimerError, TimerSnapshot};
use uuid::Uuid;

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
    engine.start_run().map_err(|e: TimerError| e.to_string())
}

#[tauri::command]
pub fn stop_green(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine.stop_green().map_err(|e: TimerError| e.to_string())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CompletedRunSummaryDto {
    pub run_id: Uuid,
    pub green_completed_early: usize,
    pub green_expired: usize,
    pub red_completed: usize,
    pub interrupted: usize,
    pub total_completed_phase_records: usize,
    pub last_cycle_index: u32,
}

impl From<CompletedRunSummary> for CompletedRunSummaryDto {
    fn from(summary: CompletedRunSummary) -> Self {
        Self {
            run_id: summary.run_id,
            green_completed_early: summary.green_completed_early,
            green_expired: summary.green_expired,
            red_completed: summary.red_completed,
            interrupted: summary.interrupted,
            total_completed_phase_records: summary.total_completed_phase_records,
            last_cycle_index: summary.last_cycle_index,
        }
    }
}

#[tauri::command]
pub fn stop_run(state: State<AppState>) -> Result<CompletedRunSummaryDto, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine
        .stop_run()
        .map(Into::into)
        .map_err(|e: TimerError| e.to_string())
}

#[tauri::command]
pub fn get_timer_snapshot(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let engine = state.engine.lock().map_err(|_| poisoned())?;
    Ok(engine.snapshot())
}

/// Called on a frontend-driven interval (e.g. every second) so the engine
/// can catch up to wall-clock time and report the latest state.
#[tauri::command]
pub fn tick_timer(state: State<AppState>) -> Result<TimerSnapshot, String> {
    let mut engine = state.engine.lock().map_err(|_| poisoned())?;
    engine.tick().map_err(|e: TimerError| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use timer_core::{CompletedPhase, PhaseOutcome, PhaseType};

    #[test]
    fn dto_omits_internal_phase_history() {
        let start = SystemTime::UNIX_EPOCH;
        let phase = CompletedPhase::new(
            PhaseType::Green,
            1,
            start,
            start + Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(1),
            PhaseOutcome::Interrupted,
        )
        .unwrap();
        let summary = CompletedRunSummary {
            run_id: Uuid::nil(),
            phases: vec![phase],
            green_completed_early: 0,
            green_expired: 0,
            red_completed: 0,
            interrupted: 1,
            total_completed_phase_records: 1,
            last_cycle_index: 1,
        };
        let dto = CompletedRunSummaryDto::from(summary);
        assert_eq!(dto.run_id, Uuid::nil());
        assert_eq!(dto.interrupted, 1);
        assert_eq!(dto.total_completed_phase_records, 1);
    }
}
