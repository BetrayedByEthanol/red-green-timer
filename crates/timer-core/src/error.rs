use thiserror::Error;

use crate::TimerValidationError;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("a timer run is already active")]
    RunAlreadyActive,
    #[error("there is no active timer run")]
    NoActiveRun,
    #[error("the active phase is not Green")]
    NotInGreenPhase,
    #[error("invalid timer transition")]
    InvalidTransition,
    #[error(transparent)]
    TimerValidation(#[from] TimerValidationError),
}
