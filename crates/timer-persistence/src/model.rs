use std::time::SystemTime;
use timer_core::{CompletedPhase, TimerDefinition};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedTimer {
    pub definition: TimerDefinition,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub archived_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEndReason {
    UserStop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHistorySummary {
    pub run_id: Uuid,
    pub timer_id: Uuid,
    pub timer_name: String,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub end_reason: RunEndReason,
    pub last_cycle_index: u32,
    pub green_completed_early: u32,
    pub green_expired: u32,
    pub red_completed: u32,
    pub interrupted: u32,
    pub total_phase_records: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedRun {
    pub summary: RunHistorySummary,
    pub phases: Vec<CompletedPhase>,
}
