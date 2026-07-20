pub mod engine;
pub mod error;
pub mod model;
pub mod state;

pub use engine::{Clock, SystemClock, TimerEngine};
pub use error::TimerError;
pub use model::{
    ActivePhase, CompletedPhase, CompletedRunSummary, PhaseOutcome, PhaseType, TimerDefinition,
    TimerRun, TimerState, TimerValidationError,
};
pub use state::TimerSnapshot;
