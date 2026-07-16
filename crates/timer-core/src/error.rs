use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimerError {
    #[error("timer is already running")]
    AlreadyRunning,

    #[error("timer is not running")]
    NotRunning,

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}
