use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The two alternating phases of a red/green interval timer.
///
/// Convention: `Green` is the "go" / work phase, `Red` is the "stop" / rest phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Red,
    Green,
}

impl Phase {
    /// Returns the opposite phase.
    pub fn toggle(self) -> Self {
        match self {
            Phase::Red => Phase::Green,
            Phase::Green => Phase::Red,
        }
    }
}

/// Domain-level phase type used by timer runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseType {
    Green,
    Red,
}

/// The reason a phase ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseOutcome {
    CompletedEarly,
    Completed,
    Expired,
    Interrupted,
}

/// Static definition for a timer independent of any UI framework.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerDefinition {
    pub id: Uuid,
    pub name: String,
    pub green_duration: Duration,
    pub red_duration: Duration,
}

impl TimerDefinition {
    /// Creates a validated timer definition.
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

/// Runtime state for a timer run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerRun {
    pub id: Uuid,
    pub timer_id: Uuid,
    pub cycle_index: u32,
    pub state: TimerState,
    pub phases: Vec<CompletedPhase>,
}

impl TimerRun {
    /// Creates a validated timer run. Cycle indices are one-based.
    pub fn new(
        id: Uuid,
        timer_id: Uuid,
        cycle_index: u32,
        state: TimerState,
        phases: Vec<CompletedPhase>,
    ) -> Result<Self, TimerValidationError> {
        let run = Self {
            id,
            timer_id,
            cycle_index,
            state,
            phases,
        };
        run.validate()?;
        Ok(run)
    }

    pub fn validate(&self) -> Result<(), TimerValidationError> {
        if self.cycle_index == 0 {
            return Err(TimerValidationError::CycleIndexZero);
        }
        self.state.validate()
    }
}

/// Mutually-exclusive run states. The enum shape prevents green and red phases
/// from running simultaneously.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerState {
    Idle,
    RunningGreen(ActivePhase),
    RunningRed(ActivePhase),
    Stopped,
}

impl TimerState {
    pub fn validate(&self) -> Result<(), TimerValidationError> {
        match self {
            TimerState::RunningGreen(phase) if phase.phase_type != PhaseType::Green => {
                Err(TimerValidationError::ActivePhaseTypeMismatch {
                    state: PhaseType::Green,
                    active_phase: phase.phase_type,
                })
            }
            TimerState::RunningRed(phase) if phase.phase_type != PhaseType::Red => {
                Err(TimerValidationError::ActivePhaseTypeMismatch {
                    state: PhaseType::Red,
                    active_phase: phase.phase_type,
                })
            }
            TimerState::RunningGreen(phase) | TimerState::RunningRed(phase) => phase.validate(),
            TimerState::Idle | TimerState::Stopped => Ok(()),
        }
    }
}

/// A currently running phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivePhase {
    pub phase_type: PhaseType,
    pub started_at: SystemTime,
    #[serde(skip, default = "Instant::now")]
    pub deadline: Instant,
    pub allocated_duration: Duration,
}

impl ActivePhase {
    pub fn new(
        phase_type: PhaseType,
        started_at: SystemTime,
        deadline: Instant,
        allocated_duration: Duration,
    ) -> Result<Self, TimerValidationError> {
        let phase = Self {
            phase_type,
            started_at,
            deadline,
            allocated_duration,
        };
        phase.validate()?;
        Ok(phase)
    }

    pub fn validate(&self) -> Result<(), TimerValidationError> {
        if self.allocated_duration.is_zero() {
            return Err(TimerValidationError::AllocatedDurationZero);
        }
        Ok(())
    }
}

/// A phase that has ended and can be stored or displayed in a run history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedPhase {
    pub phase_type: PhaseType,
    pub cycle_index: u32,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub allocated_duration: Duration,
    pub outcome: PhaseOutcome,
}

/// Static configuration for the existing red/green engine.
///
/// Durations are expressed in whole seconds to keep the type trivially
/// serializable across the Tauri IPC boundary without pulling in a
/// `serde`-with-`Duration` shim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimerConfig {
    pub red_seconds: u64,
    pub green_seconds: u64,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            red_seconds: 20,
            green_seconds: 40,
        }
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
    #[error("{state:?} state cannot contain a {active_phase:?} active phase")]
    ActivePhaseTypeMismatch {
        state: PhaseType,
        active_phase: PhaseType,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duration() -> Duration {
        Duration::from_secs(1)
    }

    #[test]
    fn timer_definition_rejects_empty_names() {
        let result = TimerDefinition::new(Uuid::nil(), " ", duration(), duration());
        assert!(matches!(result, Err(TimerValidationError::EmptyName)));
    }

    #[test]
    fn timer_definition_rejects_zero_green_duration() {
        let result = TimerDefinition::new(Uuid::nil(), "Timer", Duration::ZERO, duration());
        assert!(matches!(
            result,
            Err(TimerValidationError::GreenDurationZero)
        ));
    }

    #[test]
    fn timer_definition_rejects_zero_red_duration() {
        let result = TimerDefinition::new(Uuid::nil(), "Timer", duration(), Duration::ZERO);
        assert!(matches!(result, Err(TimerValidationError::RedDurationZero)));
    }

    #[test]
    fn timer_run_rejects_zero_cycle_index() {
        let result = TimerRun::new(Uuid::nil(), Uuid::nil(), 0, TimerState::Idle, Vec::new());
        assert!(matches!(result, Err(TimerValidationError::CycleIndexZero)));
    }

    #[test]
    fn running_green_requires_green_active_phase() {
        let active = ActivePhase::new(
            PhaseType::Red,
            SystemTime::UNIX_EPOCH,
            Instant::now(),
            duration(),
        )
        .unwrap();
        let state = TimerState::RunningGreen(active);
        assert!(matches!(
            state.validate(),
            Err(TimerValidationError::ActivePhaseTypeMismatch { .. })
        ));
    }

    #[test]
    fn active_phase_rejects_zero_allocated_duration() {
        let result = ActivePhase::new(
            PhaseType::Green,
            SystemTime::UNIX_EPOCH,
            Instant::now(),
            Duration::ZERO,
        );
        assert!(matches!(
            result,
            Err(TimerValidationError::AllocatedDurationZero)
        ));
    }
}
