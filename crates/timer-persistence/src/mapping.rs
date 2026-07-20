use crate::error::PersistenceError;
use crate::model::RunEndReason;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use timer_core::{PhaseOutcome, PhaseType};
use uuid::Uuid;

pub fn system_time_to_unix_ns(value: SystemTime) -> Result<i64, PersistenceError> {
    let duration = value
        .duration_since(UNIX_EPOCH)
        .map_err(|_| PersistenceError::TimeBeforeUnixEpoch)?;
    duration_to_ns(duration)
}
pub fn unix_ns_to_system_time(value: i64) -> Result<SystemTime, PersistenceError> {
    Ok(UNIX_EPOCH + ns_to_duration(value)?)
}
pub fn duration_to_ns(value: Duration) -> Result<i64, PersistenceError> {
    i64::try_from(value.as_nanos()).map_err(|_| PersistenceError::IntegerOverflow)
}
pub fn ns_to_duration(value: i64) -> Result<Duration, PersistenceError> {
    let nanos = u64::try_from(value).map_err(|_| {
        PersistenceError::InvalidStoredValue("negative duration/timestamp nanoseconds".into())
    })?;
    Ok(Duration::from_nanos(nanos))
}
pub fn uuid_from_db(value: String) -> Result<Uuid, PersistenceError> {
    Uuid::parse_str(&value).map_err(|_| PersistenceError::InvalidUuid(value))
}
pub fn phase_type_to_db(v: PhaseType) -> &'static str {
    match v {
        PhaseType::Green => "green",
        PhaseType::Red => "red",
    }
}
pub fn phase_type_from_db(v: &str) -> Result<PhaseType, PersistenceError> {
    match v {
        "green" => Ok(PhaseType::Green),
        "red" => Ok(PhaseType::Red),
        _ => Err(PersistenceError::InvalidStoredValue(format!(
            "phase_type {v}"
        ))),
    }
}
pub fn outcome_to_db(v: PhaseOutcome) -> &'static str {
    match v {
        PhaseOutcome::CompletedEarly => "completed_early",
        PhaseOutcome::Completed => "completed",
        PhaseOutcome::Expired => "expired",
        PhaseOutcome::Interrupted => "interrupted",
    }
}
pub fn outcome_from_db(v: &str) -> Result<PhaseOutcome, PersistenceError> {
    match v {
        "completed_early" => Ok(PhaseOutcome::CompletedEarly),
        "completed" => Ok(PhaseOutcome::Completed),
        "expired" => Ok(PhaseOutcome::Expired),
        "interrupted" => Ok(PhaseOutcome::Interrupted),
        _ => Err(PersistenceError::InvalidStoredValue(format!("outcome {v}"))),
    }
}
pub fn end_reason_to_db(v: RunEndReason) -> &'static str {
    match v {
        RunEndReason::UserStop => "user_stop",
    }
}
pub fn end_reason_from_db(v: &str) -> Result<RunEndReason, PersistenceError> {
    match v {
        "user_stop" => Ok(RunEndReason::UserStop),
        _ => Err(PersistenceError::InvalidStoredValue(format!(
            "end_reason {v}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_time_round_trip() {
        let t = UNIX_EPOCH + Duration::new(123, 456);
        assert_eq!(
            unix_ns_to_system_time(system_time_to_unix_ns(t).unwrap()).unwrap(),
            t
        );
    }
    #[test]
    fn duration_round_trip() {
        let d = Duration::new(5, 7);
        assert_eq!(ns_to_duration(duration_to_ns(d).unwrap()).unwrap(), d);
    }
    #[test]
    fn overflow_is_rejected() {
        assert!(matches!(
            duration_to_ns(Duration::MAX),
            Err(PersistenceError::IntegerOverflow)
        ));
    }
    #[test]
    fn negative_database_duration_is_rejected() {
        assert!(matches!(
            ns_to_duration(-1),
            Err(PersistenceError::InvalidStoredValue(_))
        ));
    }
}
