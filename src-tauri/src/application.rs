use serde::Serialize;
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use timer_core::{CompletedRunSummary, TimerDefinition, TimerEngine, TimerError, TimerSnapshot};
use timer_persistence::{PersistenceError, RunEndReason, RunHistorySummary, TimerRepository};
use tokio::sync::Mutex;
use uuid::Uuid;

pub type AppState = Arc<Mutex<ApplicationController>>;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Timer(#[from] TimerError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("timer is active: {0}")]
    TimerIsActive(Uuid),
    #[error("there is no active timer")]
    NoActiveTimer,
    #[error("a completed run is waiting to be persisted; stop_run will retry it")]
    PendingPersistenceWrite,
}

struct ActiveTimer {
    timer_id: Uuid,
    engine: TimerEngine,
}
struct PendingWrite {
    timer_id: Uuid,
    summary: CompletedRunSummary,
}

pub struct ApplicationController {
    repository: TimerRepository,
    active_timer: Option<ActiveTimer>,
    pending_write: Option<PendingWrite>,
}

impl ApplicationController {
    pub async fn open(database_url: &str) -> Result<Self, ApplicationError> {
        let repository = TimerRepository::open(database_url).await?;
        repository.seed_default_timer_if_empty().await?;
        Ok(Self {
            repository,
            active_timer: None,
            pending_write: None,
        })
    }
    pub async fn open_file(path: &Path) -> Result<Self, ApplicationError> {
        let url = format!("sqlite:{}", path.display());
        Self::open(&url).await
    }
    pub async fn list_timers(&self) -> Result<Vec<TimerDto>, ApplicationError> {
        Ok(self
            .repository
            .list_active_timers()
            .await?
            .into_iter()
            .map(TimerDto::from)
            .collect())
    }
    pub async fn create_timer(&self, req: TimerRequest) -> Result<TimerDto, ApplicationError> {
        let def = req.definition(Uuid::new_v4())?;
        self.repository.create_timer(&def).await?;
        Ok(self
            .repository
            .get_timer(def.id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(def.id))?
            .into())
    }
    pub async fn update_timer(
        &self,
        id: Uuid,
        req: TimerRequest,
    ) -> Result<TimerDto, ApplicationError> {
        if self.active_timer.as_ref().is_some_and(|a| a.timer_id == id) {
            return Err(ApplicationError::TimerIsActive(id));
        }
        let def = req.definition(id)?;
        self.repository.update_timer(&def).await?;
        Ok(self
            .repository
            .get_timer(id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(id))?
            .into())
    }
    pub async fn archive_timer(&self, id: Uuid) -> Result<(), ApplicationError> {
        if self.active_timer.as_ref().is_some_and(|a| a.timer_id == id) {
            return Err(ApplicationError::TimerIsActive(id));
        }
        self.repository.archive_timer(id, SystemTime::now()).await?;
        Ok(())
    }
    pub async fn start_timer(&mut self, timer_id: Uuid) -> Result<TimerSnapshot, ApplicationError> {
        if self.pending_write.is_some() {
            return Err(ApplicationError::PendingPersistenceWrite);
        }
        if self.active_timer.is_some() {
            return Err(TimerError::RunAlreadyActive.into());
        }
        let timer = self
            .repository
            .get_timer(timer_id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(timer_id))?;
        if timer.archived_at.is_some() {
            return Err(PersistenceError::TimerArchived(timer_id).into());
        }
        let mut engine = TimerEngine::new(timer.definition)?;
        let snap = engine.start_run()?;
        self.active_timer = Some(ActiveTimer { timer_id, engine });
        Ok(snap)
    }
    pub fn stop_green(&mut self) -> Result<TimerSnapshot, ApplicationError> {
        Ok(self
            .active_timer
            .as_mut()
            .ok_or(ApplicationError::NoActiveTimer)?
            .engine
            .stop_green()?)
    }
    pub async fn stop_run(&mut self) -> Result<CompletedRunSummaryDto, ApplicationError> {
        if let Some(p) = &self.pending_write {
            self.repository
                .insert_completed_run(p.timer_id, &p.summary, RunEndReason::UserStop)
                .await?;
            let summary = self.pending_write.take().expect("pending exists").summary;
            return Ok(summary.into());
        }
        let active = self
            .active_timer
            .as_mut()
            .ok_or(ApplicationError::NoActiveTimer)?;
        let summary = active.engine.stop_run()?;
        let timer_id = active.timer_id;
        match self
            .repository
            .insert_completed_run(timer_id, &summary, RunEndReason::UserStop)
            .await
        {
            Ok(()) => {
                self.active_timer = None;
                Ok(summary.into())
            }
            Err(e) => {
                self.pending_write = Some(PendingWrite { timer_id, summary });
                self.active_timer = None;
                Err(e.into())
            }
        }
    }
    pub fn snapshot(&self) -> TimerSnapshot {
        self.active_timer
            .as_ref()
            .map(|a| a.engine.snapshot())
            .unwrap_or(TimerSnapshot {
                active: false,
                phase: None,
                cycle_index: None,
                remaining_seconds: 0,
                timer_name: String::new(),
                run_id: None,
                completed_phase_count: 0,
                green_duration_seconds: 0,
                red_duration_seconds: 0,
            })
    }
    pub fn tick(&mut self) -> Result<TimerSnapshot, ApplicationError> {
        Ok(match self.active_timer.as_mut() {
            Some(a) => a.engine.tick()?,
            None => self.snapshot(),
        })
    }
    pub async fn list_recent_runs(
        &self,
        timer_id: Option<Uuid>,
        limit: Option<u32>,
    ) -> Result<Vec<RunHistoryDto>, ApplicationError> {
        Ok(self
            .repository
            .list_recent_runs(timer_id, limit.unwrap_or(20))
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TimerRequest {
    pub name: String,
    pub green_duration_seconds: u64,
    pub red_duration_seconds: u64,
}
impl TimerRequest {
    fn definition(self, id: Uuid) -> Result<TimerDefinition, TimerError> {
        Ok(TimerDefinition::new(
            id,
            self.name,
            Duration::from_secs(self.green_duration_seconds),
            Duration::from_secs(self.red_duration_seconds),
        )?)
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TimerDto {
    pub id: Uuid,
    pub name: String,
    pub green_duration_seconds: u64,
    pub red_duration_seconds: u64,
    pub archived: bool,
}
impl From<timer_persistence::PersistedTimer> for TimerDto {
    fn from(t: timer_persistence::PersistedTimer) -> Self {
        Self {
            id: t.definition.id,
            name: t.definition.name,
            green_duration_seconds: t.definition.green_duration.as_secs(),
            red_duration_seconds: t.definition.red_duration.as_secs(),
            archived: t.archived_at.is_some(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
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
    fn from(s: CompletedRunSummary) -> Self {
        Self {
            run_id: s.run_id,
            green_completed_early: s.green_completed_early,
            green_expired: s.green_expired,
            red_completed: s.red_completed,
            interrupted: s.interrupted,
            total_completed_phase_records: s.total_completed_phase_records,
            last_cycle_index: s.last_cycle_index,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct RunHistoryDto {
    pub run_id: Uuid,
    pub timer_id: Uuid,
    pub timer_name: String,
    pub started_at_unix_ms: i64,
    pub ended_at_unix_ms: i64,
    pub last_cycle_index: u32,
    pub green_completed_early: u32,
    pub green_expired: u32,
    pub red_completed: u32,
    pub interrupted: u32,
    pub total_phase_records: u32,
}
impl From<RunHistorySummary> for RunHistoryDto {
    fn from(r: RunHistorySummary) -> Self {
        fn ms(t: SystemTime) -> i64 {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(i64::MAX)
        }
        Self {
            run_id: r.run_id,
            timer_id: r.timer_id,
            timer_name: r.timer_name,
            started_at_unix_ms: ms(r.started_at),
            ended_at_unix_ms: ms(r.ended_at),
            last_cycle_index: r.last_cycle_index,
            green_completed_early: r.green_completed_early,
            green_expired: r.green_expired,
            red_completed: r.red_completed,
            interrupted: r.interrupted,
            total_phase_records: r.total_phase_records,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn app() -> ApplicationController {
        ApplicationController::open("sqlite::memory:")
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn start_selected_timer() {
        let mut a = app().await;
        let t = a.list_timers().await.unwrap()[0].clone();
        assert!(a.start_timer(t.id).await.unwrap().active);
    }
    #[tokio::test]
    async fn only_one_timer_can_run() {
        let mut a = app().await;
        let t = a.list_timers().await.unwrap()[0].clone();
        a.start_timer(t.id).await.unwrap();
        assert!(matches!(
            a.start_timer(t.id).await,
            Err(ApplicationError::Timer(TimerError::RunAlreadyActive))
        ));
    }
    #[tokio::test]
    async fn active_timer_cannot_be_updated_or_archived() {
        let mut a = app().await;
        let t = a.list_timers().await.unwrap()[0].clone();
        a.start_timer(t.id).await.unwrap();
        let req = TimerRequest {
            name: "x".into(),
            green_duration_seconds: 1,
            red_duration_seconds: 1,
        };
        assert!(matches!(
            a.update_timer(t.id, req).await,
            Err(ApplicationError::TimerIsActive(_))
        ));
        assert!(matches!(
            a.archive_timer(t.id).await,
            Err(ApplicationError::TimerIsActive(_))
        ));
    }
    #[tokio::test]
    async fn stop_run_persists_history() {
        let mut a = app().await;
        let t = a.list_timers().await.unwrap()[0].clone();
        a.start_timer(t.id).await.unwrap();
        a.stop_run().await.unwrap();
        assert_eq!(a.list_recent_runs(None, Some(20)).await.unwrap().len(), 1);
    }
}
