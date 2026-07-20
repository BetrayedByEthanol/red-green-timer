use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("timer not found: {0}")]
    TimerNotFound(Uuid),
    #[error("timer is archived: {0}")]
    TimerArchived(Uuid),
    #[error("run not found: {0}")]
    RunNotFound(Uuid),
    #[error("completed run has no phase history")]
    EmptyRunHistory,
    #[error("invalid UUID: {0}")]
    InvalidUuid(String),
    #[error("time is before Unix epoch")]
    TimeBeforeUnixEpoch,
    #[error("integer overflow while converting time or duration")]
    IntegerOverflow,
    #[error("invalid stored value: {0}")]
    InvalidStoredValue(String),
}
