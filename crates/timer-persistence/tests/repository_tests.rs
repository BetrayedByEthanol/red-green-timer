use std::time::{Duration, SystemTime};
use timer_core::{CompletedPhase, CompletedRunSummary, PhaseOutcome, PhaseType, TimerDefinition};
use timer_persistence::{PersistenceError, RunEndReason, TimerRepository};
use uuid::Uuid;

async fn repo() -> TimerRepository {
    TimerRepository::open("sqlite::memory:").await.unwrap()
}
fn def(name: &str) -> TimerDefinition {
    TimerDefinition::new(
        Uuid::new_v4(),
        name,
        Duration::from_secs(10),
        Duration::from_secs(5),
    )
    .unwrap()
}
fn summary(id: Uuid) -> CompletedRunSummary {
    let start = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let p1 = CompletedPhase::new(
        PhaseType::Green,
        1,
        start,
        start + Duration::from_secs(3),
        Duration::from_secs(10),
        Duration::from_secs(3),
        PhaseOutcome::CompletedEarly,
    )
    .unwrap();
    let p2 = CompletedPhase::new(
        PhaseType::Red,
        1,
        start + Duration::from_secs(3),
        start + Duration::from_secs(8),
        Duration::from_secs(5),
        Duration::from_secs(5),
        PhaseOutcome::Completed,
    )
    .unwrap();
    CompletedRunSummary {
        run_id: id,
        phases: vec![p1, p2],
        green_completed_early: 1,
        green_expired: 0,
        red_completed: 1,
        interrupted: 0,
        total_completed_phase_records: 2,
        last_cycle_index: 1,
    }
}

#[tokio::test]
async fn new_database_runs_migrations() {
    let r = repo().await;
    assert_eq!(r.timer_count().await.unwrap(), 0);
}
#[tokio::test]
async fn migrations_can_run_more_than_once() {
    let _ = TimerRepository::open("sqlite::memory:").await.unwrap();
}
#[tokio::test]
async fn foreign_keys_are_enabled() {
    let r = repo().await;
    let s: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(r.pool())
        .await
        .unwrap();
    assert_eq!(s, 1);
}
#[tokio::test]
async fn create_and_reload_timer() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    assert_eq!(
        r.get_timer(d.id).await.unwrap().unwrap().definition.name,
        "Focus"
    );
}
#[tokio::test]
async fn list_active_timers_excludes_archived() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    r.archive_timer(d.id, SystemTime::now()).await.unwrap();
    assert!(r.list_active_timers().await.unwrap().is_empty());
}
#[tokio::test]
async fn update_timer_persists_changes() {
    let r = repo().await;
    let mut d = def("Focus");
    r.create_timer(&d).await.unwrap();
    d.name = "Deep".into();
    r.update_timer(&d).await.unwrap();
    assert_eq!(
        r.get_timer(d.id).await.unwrap().unwrap().definition.name,
        "Deep"
    );
}
#[tokio::test]
async fn update_missing_timer_fails() {
    let r = repo().await;
    let d = def("Missing");
    assert!(matches!(
        r.update_timer(&d).await,
        Err(PersistenceError::TimerNotFound(_))
    ));
}
#[tokio::test]
async fn archive_timer_preserves_history() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    r.insert_completed_run(d.id, &summary(Uuid::new_v4()), RunEndReason::UserStop)
        .await
        .unwrap();
    r.archive_timer(d.id, SystemTime::now()).await.unwrap();
    assert_eq!(r.list_recent_runs(Some(d.id), 20).await.unwrap().len(), 1);
}
#[tokio::test]
async fn timer_ids_remain_stable() {
    let r = repo().await;
    let mut d = def("Focus");
    let id = d.id;
    r.create_timer(&d).await.unwrap();
    d.name = "Next".into();
    r.update_timer(&d).await.unwrap();
    assert_eq!(r.get_timer(id).await.unwrap().unwrap().definition.id, id);
}
#[tokio::test]
async fn completed_run_and_phases_persist_transactionally() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    let s = summary(Uuid::new_v4());
    r.insert_completed_run(d.id, &s, RunEndReason::UserStop)
        .await
        .unwrap();
    assert_eq!(r.get_run(s.run_id).await.unwrap().unwrap().phases.len(), 2);
}
#[tokio::test]
async fn phase_sequence_is_preserved() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    let s = summary(Uuid::new_v4());
    r.insert_completed_run(d.id, &s, RunEndReason::UserStop)
        .await
        .unwrap();
    let run = r.get_run(s.run_id).await.unwrap().unwrap();
    assert_eq!(run.phases[0].phase_type, PhaseType::Green);
    assert_eq!(run.phases[1].phase_type, PhaseType::Red);
}
#[tokio::test]
async fn run_summary_counts_match_phase_history() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    r.insert_completed_run(d.id, &summary(Uuid::new_v4()), RunEndReason::UserStop)
        .await
        .unwrap();
    let h = &r.list_recent_runs(None, 20).await.unwrap()[0];
    assert_eq!(h.green_completed_early, 1);
    assert_eq!(h.red_completed, 1);
    assert_eq!(h.total_phase_records, 2);
}
#[tokio::test]
async fn duplicate_run_id_is_rejected() {
    let r = repo().await;
    let d = def("Focus");
    r.create_timer(&d).await.unwrap();
    let s = summary(Uuid::new_v4());
    r.insert_completed_run(d.id, &s, RunEndReason::UserStop)
        .await
        .unwrap();
    assert!(matches!(
        r.insert_completed_run(d.id, &s, RunEndReason::UserStop)
            .await,
        Err(PersistenceError::Database(_))
    ));
}
#[tokio::test]
async fn history_can_be_filtered_by_timer() {
    let r = repo().await;
    let a = def("A");
    let b = def("B");
    r.create_timer(&a).await.unwrap();
    r.create_timer(&b).await.unwrap();
    r.insert_completed_run(a.id, &summary(Uuid::new_v4()), RunEndReason::UserStop)
        .await
        .unwrap();
    r.insert_completed_run(b.id, &summary(Uuid::new_v4()), RunEndReason::UserStop)
        .await
        .unwrap();
    assert_eq!(r.list_recent_runs(Some(a.id), 20).await.unwrap().len(), 1);
}
#[tokio::test]
async fn history_limit_is_enforced() {
    let r = repo().await;
    let d = def("A");
    r.create_timer(&d).await.unwrap();
    for _ in 0..3 {
        r.insert_completed_run(d.id, &summary(Uuid::new_v4()), RunEndReason::UserStop)
            .await
            .unwrap();
    }
    assert_eq!(r.list_recent_runs(None, 2).await.unwrap().len(), 2);
}
