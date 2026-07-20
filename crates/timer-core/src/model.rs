use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseType {
    Green,
    Red,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseOutcome {
    CompletedEarly,
    Completed,
    Expired,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerDefinition {
    pub id: Uuid,
    pub name: String,
    pub green_duration: Duration,
    pub red_duration: Duration,
}

impl TimerDefinition {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        green_duration: Duration,
        red_duration: Duration,
    ) -> Result<Self, TimerValidationError> {
        let definition = Self {
            id,
            name: name.into(),
            green_duration,
            red_duration,
        };
        definition.validate()?;
        Ok(definition)
    }
    pub fn validate(&self) -> Result<(), TimerValidationError> {
        if self.name.trim().is_empty() {
            return Err(TimerValidationError::EmptyName);
        }
        if self.green_duration.is_zero() {
            return Err(TimerValidationError::GreenDurationZero);
        }
        if self.red_duration.is_zero() {
            return Err(TimerValidationError::RedDurationZero);
        }
        Ok(())
    }
}

/// Runtime-only deadline data is deliberately not deserializable. Future
/// persistence must store a wall-clock deadline and reconstruct this value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivePhase {
    pub phase_type: PhaseType,
    pub started_at: SystemTime,
    #[serde(skip_serializing)]
    pub deadline: Instant,
    #[serde(skip_serializing)]
    pub started_instant: Instant,
    pub allocated_duration: Duration,
}
impl ActivePhase {
    pub fn new(
        phase_type: PhaseType,
        started_at: SystemTime,
        started_instant: Instant,
        allocated_duration: Duration,
    ) -> Result<Self, TimerValidationError> {
        if allocated_duration.is_zero() {
            return Err(TimerValidationError::AllocatedDurationZero);
        }
        let deadline = started_instant
            .checked_add(allocated_duration)
            .ok_or(TimerValidationError::DeadlineOverflow)?;
        Ok(Self {
            phase_type,
            started_at,
            deadline,
            started_instant,
            allocated_duration,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedPhase {
    pub phase_type: PhaseType,
    pub cycle_index: u32,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub allocated_duration: Duration,
    pub actual_duration: Duration,
    pub outcome: PhaseOutcome,
}
impl CompletedPhase {
    pub fn new(
        phase_type: PhaseType,
        cycle_index: u32,
        started_at: SystemTime,
        ended_at: SystemTime,
        allocated_duration: Duration,
        actual_duration: Duration,
        outcome: PhaseOutcome,
    ) -> Result<Self, TimerValidationError> {
        let phase = Self {
            phase_type,
            cycle_index,
            started_at,
            ended_at,
            allocated_duration,
            actual_duration,
            outcome,
        };
        phase.validate()?;
        Ok(phase)
    }
    pub fn validate(&self) -> Result<(), TimerValidationError> {
        if self.cycle_index == 0 {
            return Err(TimerValidationError::CycleIndexZero);
        }
        if self.ended_at.duration_since(self.started_at).is_err() {
            return Err(TimerValidationError::EndBeforeStart);
        }
        if self.actual_duration > self.allocated_duration {
            return Err(TimerValidationError::ActualDurationExceedsAllocated);
        }
        match (self.phase_type, self.outcome) {
            (PhaseType::Green, PhaseOutcome::Completed)
            | (PhaseType::Red, PhaseOutcome::CompletedEarly | PhaseOutcome::Expired) => {
                Err(TimerValidationError::InvalidPhaseOutcome)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TimerState {
    RunningGreen(ActivePhase),
    RunningRed(ActivePhase),
    Stopped,
}
impl TimerState {
    pub fn active_phase(&self) -> Option<&ActivePhase> {
        match self {
            Self::RunningGreen(p) | Self::RunningRed(p) => Some(p),
            Self::Stopped => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimerRun {
    pub id: Uuid,
    pub timer_id: Uuid,
    pub cycle_index: u32,
    pub state: TimerState,
    pub phases: Vec<CompletedPhase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompletedRunSummary {
    pub run_id: Uuid,
    pub phases: Vec<CompletedPhase>,
    pub green_completed_early: usize,
    pub green_expired: usize,
    pub red_completed: usize,
    pub interrupted: usize,
    pub total_completed_phase_records: usize,
    pub last_cycle_index: u32,
}
impl CompletedRunSummary {
    pub fn from_run(run: TimerRun) -> Self {
        let mut s = Self {
            run_id: run.id,
            phases: run.phases,
            green_completed_early: 0,
            green_expired: 0,
            red_completed: 0,
            interrupted: 0,
            total_completed_phase_records: 0,
            last_cycle_index: run.cycle_index,
        };
        for p in &s.phases {
            match (p.phase_type, p.outcome) {
                (PhaseType::Green, PhaseOutcome::CompletedEarly) => s.green_completed_early += 1,
                (PhaseType::Green, PhaseOutcome::Expired) => s.green_expired += 1,
                (PhaseType::Red, PhaseOutcome::Completed) => s.red_completed += 1,
                (_, PhaseOutcome::Interrupted) => s.interrupted += 1,
                _ => {}
            }
        }
        s.total_completed_phase_records = s.phases.len();
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TimerValidationError {
    #[error("timer name cannot be empty")]
    EmptyName,
    #[error("green duration must be greater than zero")]
    GreenDurationZero,
    #[error("red duration must be greater than zero")]
    RedDurationZero,
    #[error("cycle indices begin at one")]
    CycleIndexZero,
    #[error("active phase duration must be greater than zero")]
    AllocatedDurationZero,
    #[error("phase deadline overflowed")]
    DeadlineOverflow,
    #[error("phase ended before it started")]
    EndBeforeStart,
    #[error("actual duration exceeds allocated duration")]
    ActualDurationExceedsAllocated,
    #[error("invalid outcome for phase type")]
    InvalidPhaseOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn definition_rejects_empty_name_and_zero_durations() {
        assert!(matches!(
            TimerDefinition::new(
                Uuid::nil(),
                " ",
                Duration::from_secs(1),
                Duration::from_secs(1)
            ),
            Err(TimerValidationError::EmptyName)
        ));
        assert!(matches!(
            TimerDefinition::new(Uuid::nil(), "x", Duration::ZERO, Duration::from_secs(1)),
            Err(TimerValidationError::GreenDurationZero)
        ));
        assert!(matches!(
            TimerDefinition::new(Uuid::nil(), "x", Duration::from_secs(1), Duration::ZERO),
            Err(TimerValidationError::RedDurationZero)
        ));
    }
    #[test]
    fn completed_phase_validates_cycle_time_and_outcome() {
        let start = SystemTime::UNIX_EPOCH;
        assert!(matches!(
            CompletedPhase::new(
                PhaseType::Green,
                0,
                start,
                start,
                Duration::from_secs(1),
                Duration::ZERO,
                PhaseOutcome::Interrupted
            ),
            Err(TimerValidationError::CycleIndexZero)
        ));
        assert!(matches!(
            CompletedPhase::new(
                PhaseType::Red,
                1,
                start,
                start,
                Duration::from_secs(1),
                Duration::ZERO,
                PhaseOutcome::Expired
            ),
            Err(TimerValidationError::InvalidPhaseOutcome)
        ));
        assert!(matches!(
            CompletedPhase::new(
                PhaseType::Green,
                1,
                start + Duration::from_secs(1),
                start,
                Duration::from_secs(1),
                Duration::ZERO,
                PhaseOutcome::Interrupted
            ),
            Err(TimerValidationError::EndBeforeStart)
        ));
    }
}
