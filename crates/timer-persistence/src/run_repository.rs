use crate::{
    error::PersistenceError,
    mapping::*,
    model::{PersistedRun, RunEndReason, RunHistorySummary},
    repository::TimerRepository,
};
use sqlx::Row;
use timer_core::{CompletedPhase, CompletedRunSummary};
use uuid::Uuid;

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;
impl TimerRepository {
    pub async fn insert_completed_run(
        &self,
        timer_id: Uuid,
        summary: &CompletedRunSummary,
        end_reason: RunEndReason,
    ) -> Result<(), PersistenceError> {
        let timer = self
            .get_timer(timer_id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(timer_id))?;
        if summary.phases.is_empty() {
            return Err(PersistenceError::EmptyRunHistory);
        }
        let started = summary
            .phases
            .first()
            .ok_or(PersistenceError::EmptyRunHistory)?
            .started_at;
        if timer.archived_at.is_some_and(|t| t <= started) {
            return Err(PersistenceError::TimerArchived(timer_id));
        }
        let ended = summary
            .phases
            .last()
            .ok_or(PersistenceError::EmptyRunHistory)?
            .ended_at;
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO runs (id,timer_id,started_at_ns,ended_at_ns,end_reason,last_cycle_index,created_at_ns) VALUES (?,?,?,?,?,?,?)").bind(summary.run_id.to_string()).bind(timer_id.to_string()).bind(system_time_to_unix_ns(started)?).bind(system_time_to_unix_ns(ended)?).bind(end_reason_to_db(end_reason)).bind(i64::from(summary.last_cycle_index)).bind(system_time_to_unix_ns(std::time::SystemTime::now())?).execute(&mut *tx).await?;
        for (sequence, phase) in summary.phases.iter().enumerate() {
            phase
                .validate()
                .map_err(|e| PersistenceError::InvalidStoredValue(e.to_string()))?;
            sqlx::query("INSERT INTO phases (run_id,sequence_index,cycle_index,phase_type,outcome,started_at_ns,ended_at_ns,allocated_duration_ns,actual_duration_ns) VALUES (?,?,?,?,?,?,?,?,?)").bind(summary.run_id.to_string()).bind(i64::try_from(sequence).map_err(|_| PersistenceError::IntegerOverflow)?).bind(i64::from(phase.cycle_index)).bind(phase_type_to_db(phase.phase_type)).bind(outcome_to_db(phase.outcome)).bind(system_time_to_unix_ns(phase.started_at)?).bind(system_time_to_unix_ns(phase.ended_at)?).bind(duration_to_ns(phase.allocated_duration)?).bind(duration_to_ns(phase.actual_duration)?).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn list_recent_runs(
        &self,
        timer_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<RunHistorySummary>, PersistenceError> {
        let limit = limit.clamp(1, MAX_LIMIT);
        let sql="SELECT r.*, t.name timer_name, SUM(CASE WHEN p.phase_type='green' AND p.outcome='completed_early' THEN 1 ELSE 0 END) green_completed_early, SUM(CASE WHEN p.phase_type='green' AND p.outcome='expired' THEN 1 ELSE 0 END) green_expired, SUM(CASE WHEN p.phase_type='red' AND p.outcome='completed' THEN 1 ELSE 0 END) red_completed, SUM(CASE WHEN p.outcome='interrupted' THEN 1 ELSE 0 END) interrupted, COUNT(p.id) total_phase_records FROM runs r JOIN timers t ON t.id=r.timer_id JOIN phases p ON p.run_id=r.id WHERE (? IS NULL OR r.timer_id=?) GROUP BY r.id ORDER BY r.ended_at_ns DESC LIMIT ?";
        let tid = timer_id.map(|u| u.to_string());
        let rows = sqlx::query(sql)
            .bind(tid.clone())
            .bind(tid)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_summary).collect()
    }
    pub async fn list_recent_runs_default(
        &self,
        timer_id: Option<Uuid>,
    ) -> Result<Vec<RunHistorySummary>, PersistenceError> {
        self.list_recent_runs(timer_id, DEFAULT_LIMIT).await
    }
    pub async fn get_run(&self, run_id: Uuid) -> Result<Option<PersistedRun>, PersistenceError> {
        let row=sqlx::query("SELECT r.*, t.name timer_name, 0 green_completed_early, 0 green_expired, 0 red_completed, 0 interrupted, 0 total_phase_records FROM runs r JOIN timers t ON t.id=r.timer_id WHERE r.id=?").bind(run_id.to_string()).fetch_optional(&self.pool).await?;
        let Some(row) = row else { return Ok(None) };
        let phases_rows =
            sqlx::query("SELECT * FROM phases WHERE run_id=? ORDER BY sequence_index ASC")
                .bind(run_id.to_string())
                .fetch_all(&self.pool)
                .await?;
        let phases: Vec<_> = phases_rows
            .into_iter()
            .map(row_phase)
            .collect::<Result<_, _>>()?;
        let mut summary = row_summary(row)?;
        summary.green_completed_early = phases
            .iter()
            .filter(|p| {
                p.phase_type == timer_core::PhaseType::Green
                    && p.outcome == timer_core::PhaseOutcome::CompletedEarly
            })
            .count() as u32;
        summary.green_expired = phases
            .iter()
            .filter(|p| {
                p.phase_type == timer_core::PhaseType::Green
                    && p.outcome == timer_core::PhaseOutcome::Expired
            })
            .count() as u32;
        summary.red_completed = phases
            .iter()
            .filter(|p| {
                p.phase_type == timer_core::PhaseType::Red
                    && p.outcome == timer_core::PhaseOutcome::Completed
            })
            .count() as u32;
        summary.interrupted = phases
            .iter()
            .filter(|p| p.outcome == timer_core::PhaseOutcome::Interrupted)
            .count() as u32;
        summary.total_phase_records = phases.len() as u32;
        Ok(Some(PersistedRun { summary, phases }))
    }
}
fn row_summary(row: sqlx::sqlite::SqliteRow) -> Result<RunHistorySummary, PersistenceError> {
    let run_id: String = row.get("id");
    let timer_id: String = row.get("timer_id");
    Ok(RunHistorySummary {
        run_id: uuid_from_db(run_id)?,
        timer_id: uuid_from_db(timer_id)?,
        timer_name: row.get("timer_name"),
        started_at: unix_ns_to_system_time(row.get("started_at_ns"))?,
        ended_at: unix_ns_to_system_time(row.get("ended_at_ns"))?,
        end_reason: end_reason_from_db(row.get::<String, _>("end_reason").as_str())?,
        last_cycle_index: u32::try_from(row.get::<i64, _>("last_cycle_index"))
            .map_err(|_| PersistenceError::InvalidStoredValue("last_cycle_index".into()))?,
        green_completed_early: row.get::<i64, _>("green_completed_early") as u32,
        green_expired: row.get::<i64, _>("green_expired") as u32,
        red_completed: row.get::<i64, _>("red_completed") as u32,
        interrupted: row.get::<i64, _>("interrupted") as u32,
        total_phase_records: row.get::<i64, _>("total_phase_records") as u32,
    })
}
fn row_phase(row: sqlx::sqlite::SqliteRow) -> Result<CompletedPhase, PersistenceError> {
    CompletedPhase::new(
        phase_type_from_db(row.get::<String, _>("phase_type").as_str())?,
        u32::try_from(row.get::<i64, _>("cycle_index"))
            .map_err(|_| PersistenceError::InvalidStoredValue("cycle_index".into()))?,
        unix_ns_to_system_time(row.get("started_at_ns"))?,
        unix_ns_to_system_time(row.get("ended_at_ns"))?,
        ns_to_duration(row.get("allocated_duration_ns"))?,
        ns_to_duration(row.get("actual_duration_ns"))?,
        outcome_from_db(row.get::<String, _>("outcome").as_str())?,
    )
    .map_err(|e| PersistenceError::InvalidStoredValue(e.to_string()))
}
