use crate::{
    ActivePhase, CompletedPhase, CompletedRunSummary, PhaseOutcome, PhaseType, TimerDefinition,
    TimerError, TimerRun, TimerSnapshot, TimerState,
};
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

pub trait Clock {
    fn now_instant(&self) -> Instant;
    fn now_system(&self) -> SystemTime;
}
#[derive(Default)]
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_instant(&self) -> Instant {
        Instant::now()
    }
    fn now_system(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// Deadline-driven run state machine. It has no UI or Tauri dependency.
pub struct TimerEngine<C: Clock = SystemClock> {
    definition: TimerDefinition,
    active_run: Option<TimerRun>,
    clock: C,
}
impl TimerEngine<SystemClock> {
    pub fn new(definition: TimerDefinition) -> Result<Self, TimerError> {
        Self::with_clock(definition, SystemClock)
    }
}
impl<C: Clock> TimerEngine<C> {
    pub fn with_clock(definition: TimerDefinition, clock: C) -> Result<Self, TimerError> {
        definition.validate()?;
        Ok(Self {
            definition,
            active_run: None,
            clock,
        })
    }
    pub fn start_run(&mut self) -> Result<TimerSnapshot, TimerError> {
        if self.active_run.is_some() {
            return Err(TimerError::RunAlreadyActive);
        }
        let active = self.new_active(
            PhaseType::Green,
            self.clock.now_instant(),
            self.clock.now_system(),
        )?;
        self.active_run = Some(TimerRun {
            id: Uuid::new_v4(),
            timer_id: self.definition.id,
            cycle_index: 1,
            state: TimerState::RunningGreen(active),
            phases: vec![],
        });
        Ok(self.snapshot())
    }
    pub fn stop_green(&mut self) -> Result<TimerSnapshot, TimerError> {
        // Exact-deadline semantics are `now >= deadline`: overdue transitions
        // are caught up before user actions, so Green cannot be completed early
        // at or after its scheduled deadline.
        self.process_due_transitions()?;
        let active = self.require_active(PhaseType::Green)?.clone();
        let completed = self.completed_phase(&active, PhaseOutcome::CompletedEarly)?;
        let next_active = self.new_active(
            PhaseType::Red,
            self.clock.now_instant(),
            active.started_at + completed.actual_duration,
        )?;
        let run = self.active_run.as_mut().ok_or(TimerError::NoActiveRun)?;
        run.phases.push(completed);
        run.state = TimerState::RunningRed(next_active);
        Ok(self.snapshot())
    }
    pub fn stop_run(&mut self) -> Result<CompletedRunSummary, TimerError> {
        // Stop catches up overdue deadlines first. The active phase after that
        // catch-up is interrupted, so expired Green phases are never rewritten
        // as interruptions by a delayed frontend action.
        self.process_due_transitions()?;
        let active = self.require_any_active()?.clone();
        let completed = self.completed_phase(&active, PhaseOutcome::Interrupted)?;
        let mut run = self.active_run.take().ok_or(TimerError::NoActiveRun)?;
        run.phases.push(completed);
        run.state = TimerState::Stopped;
        Ok(CompletedRunSummary::from_run(run))
    }
    pub fn tick(&mut self) -> Result<TimerSnapshot, TimerError> {
        self.process_due_transitions()?;
        Ok(self.snapshot())
    }
    pub fn snapshot(&self) -> TimerSnapshot {
        let (active, phase, cycle_index, remaining_seconds, run_id, count) =
            if let Some(run) = &self.active_run {
                let p = run.state.active_phase();
                let remaining = p
                    .map(|p| {
                        p.deadline
                            .saturating_duration_since(self.clock.now_instant())
                    })
                    .unwrap_or_default();
                (
                    true,
                    p.map(|p| p.phase_type),
                    Some(run.cycle_index),
                    ceil_seconds(remaining),
                    Some(run.id),
                    run.phases.len(),
                )
            } else {
                (false, None, None, 0, None, 0)
            };
        TimerSnapshot {
            active,
            phase,
            cycle_index,
            remaining_seconds,
            timer_name: self.definition.name.clone(),
            run_id,
            completed_phase_count: count,
            green_duration_seconds: self.definition.green_duration.as_secs(),
            red_duration_seconds: self.definition.red_duration.as_secs(),
        }
    }
    fn new_active(
        &self,
        ty: PhaseType,
        instant: Instant,
        wall: SystemTime,
    ) -> Result<ActivePhase, TimerError> {
        ActivePhase::new(
            ty,
            wall,
            instant,
            match ty {
                PhaseType::Green => self.definition.green_duration,
                PhaseType::Red => self.definition.red_duration,
            },
        )
        .map_err(Into::into)
    }
    fn require_any_active(&self) -> Result<&ActivePhase, TimerError> {
        self.active_run
            .as_ref()
            .and_then(|run| run.state.active_phase())
            .ok_or(TimerError::NoActiveRun)
    }
    fn require_active(&self, required: PhaseType) -> Result<&ActivePhase, TimerError> {
        let active = self.require_any_active()?;
        if active.phase_type != required {
            return Err(TimerError::NotInGreenPhase);
        }
        Ok(active)
    }
    fn completed_phase(
        &self,
        active: &ActivePhase,
        outcome: PhaseOutcome,
    ) -> Result<CompletedPhase, TimerError> {
        let now = self.clock.now_instant();
        let actual = if outcome == PhaseOutcome::Expired || outcome == PhaseOutcome::Completed {
            active.allocated_duration
        } else {
            now.saturating_duration_since(active.started_instant)
                .min(active.allocated_duration)
        };
        let ended = active
            .started_at
            .checked_add(actual)
            .unwrap_or(active.started_at);
        let cycle = self
            .active_run
            .as_ref()
            .ok_or(TimerError::NoActiveRun)?
            .cycle_index;
        Ok(CompletedPhase::new(
            active.phase_type,
            cycle,
            active.started_at,
            ended,
            active.allocated_duration,
            actual,
            outcome,
        )?)
    }

    fn process_due_transitions(&mut self) -> Result<(), TimerError> {
        loop {
            let active = match self
                .active_run
                .as_ref()
                .and_then(|run| run.state.active_phase())
            {
                Some(active) if self.clock.now_instant() >= active.deadline => active.clone(),
                _ => return Ok(()),
            };
            let next_start = active.deadline;
            let next_wall = active.started_at + active.allocated_duration;
            let cycle_increment = active.phase_type == PhaseType::Red;
            let outcome = if active.phase_type == PhaseType::Green {
                PhaseOutcome::Expired
            } else {
                PhaseOutcome::Completed
            };
            let completed = self.completed_phase(&active, outcome)?;
            let next_type = if active.phase_type == PhaseType::Green {
                PhaseType::Red
            } else {
                PhaseType::Green
            };
            let next_active = self.new_active(next_type, next_start, next_wall)?;

            let run = self.active_run.as_mut().ok_or(TimerError::NoActiveRun)?;
            run.phases.push(completed);
            if cycle_increment {
                run.cycle_index = run.cycle_index.saturating_add(1);
            }
            run.state = match next_type {
                PhaseType::Green => TimerState::RunningGreen(next_active),
                PhaseType::Red => TimerState::RunningRed(next_active),
            };
        }
    }
}
fn ceil_seconds(d: Duration) -> u64 {
    d.as_secs().saturating_add(u64::from(d.subsec_nanos() > 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::time::{Duration, Instant, SystemTime};
    struct FakeClock {
        instant: Instant,
        elapsed: Cell<Duration>,
    }
    impl FakeClock {
        fn new() -> Self {
            Self {
                instant: Instant::now(),
                elapsed: Cell::new(Duration::ZERO),
            }
        }
        fn advance(&self, d: Duration) {
            self.elapsed.set(self.elapsed.get() + d);
        }
    }
    impl Clock for &FakeClock {
        fn now_instant(&self) -> Instant {
            self.instant + self.elapsed.get()
        }
        fn now_system(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH + self.elapsed.get()
        }
    }
    fn engine(c: &FakeClock) -> TimerEngine<&FakeClock> {
        TimerEngine::with_clock(
            TimerDefinition::new(
                Uuid::nil(),
                "Focus",
                Duration::from_secs(5),
                Duration::from_secs(3),
            )
            .unwrap(),
            c,
        )
        .unwrap()
    }
    fn outcomes(e: &TimerEngine<&FakeClock>) -> Vec<(PhaseType, PhaseOutcome, u32)> {
        e.active_run
            .as_ref()
            .unwrap()
            .phases
            .iter()
            .map(|p| (p.phase_type, p.outcome, p.cycle_index))
            .collect()
    }

    #[test]
    fn stop_green_before_deadline_records_completed_early() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(4));
        let s = e.stop_green().unwrap();
        assert_eq!(s.phase, Some(PhaseType::Red));
        assert_eq!(
            outcomes(&e),
            vec![(PhaseType::Green, PhaseOutcome::CompletedEarly, 1)]
        );
    }

    #[test]
    fn stop_green_exactly_at_deadline_records_expired() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(5));
        assert!(matches!(e.stop_green(), Err(TimerError::NotInGreenPhase)));
        assert_eq!(e.snapshot().phase, Some(PhaseType::Red));
        assert_eq!(
            outcomes(&e),
            vec![(PhaseType::Green, PhaseOutcome::Expired, 1)]
        );
    }

    #[test]
    fn stop_green_after_deadline_before_tick_records_expired() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(6));
        assert!(matches!(e.stop_green(), Err(TimerError::NotInGreenPhase)));
        assert_eq!(e.snapshot().phase, Some(PhaseType::Red));
        assert_eq!(
            outcomes(&e),
            vec![(PhaseType::Green, PhaseOutcome::Expired, 1)]
        );
    }

    #[test]
    fn stop_green_after_expiry_returns_not_in_green_phase() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(5));
        e.tick().unwrap();
        assert!(matches!(e.stop_green(), Err(TimerError::NotInGreenPhase)));
    }

    #[test]
    fn stop_run_before_green_deadline_interrupts_green() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(4));
        let summary = e.stop_run().unwrap();
        assert_eq!(
            summary
                .phases
                .iter()
                .map(|p| (p.phase_type, p.outcome, p.cycle_index))
                .collect::<Vec<_>>(),
            vec![(PhaseType::Green, PhaseOutcome::Interrupted, 1)]
        );
    }

    #[test]
    fn stop_run_exactly_at_green_deadline_records_green_expired_then_interrupts_red() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(5));
        let summary = e.stop_run().unwrap();
        assert_eq!(
            summary
                .phases
                .iter()
                .map(|p| (p.phase_type, p.outcome, p.cycle_index))
                .collect::<Vec<_>>(),
            vec![
                (PhaseType::Green, PhaseOutcome::Expired, 1),
                (PhaseType::Red, PhaseOutcome::Interrupted, 1)
            ]
        );
    }

    #[test]
    fn stop_run_after_green_and_red_deadlines_catches_up_then_interrupts_next_green() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(9));
        let summary = e.stop_run().unwrap();
        assert_eq!(
            summary
                .phases
                .iter()
                .map(|p| (p.phase_type, p.outcome, p.cycle_index))
                .collect::<Vec<_>>(),
            vec![
                (PhaseType::Green, PhaseOutcome::Expired, 1),
                (PhaseType::Red, PhaseOutcome::Completed, 1),
                (PhaseType::Green, PhaseOutcome::Interrupted, 2)
            ]
        );
    }

    #[test]
    fn stop_run_during_red_interrupts_red() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(2));
        e.stop_green().unwrap();
        c.advance(Duration::from_secs(1));
        let summary = e.stop_run().unwrap();
        assert_eq!(
            summary
                .phases
                .iter()
                .map(|p| (p.phase_type, p.outcome, p.cycle_index))
                .collect::<Vec<_>>(),
            vec![
                (PhaseType::Green, PhaseOutcome::CompletedEarly, 1),
                (PhaseType::Red, PhaseOutcome::Interrupted, 1)
            ]
        );
    }

    #[test]
    fn tick_exactly_at_green_deadline_transitions_to_red() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(5));
        assert_eq!(e.tick().unwrap().phase, Some(PhaseType::Red));
    }

    #[test]
    fn tick_exactly_at_red_deadline_transitions_to_next_green() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(8));
        let s = e.tick().unwrap();
        assert_eq!(s.phase, Some(PhaseType::Green));
        assert_eq!(s.cycle_index, Some(2));
    }

    #[test]
    fn delayed_tick_crosses_multiple_complete_cycles() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(14));
        let s = e.tick().unwrap();
        assert_eq!(s.phase, Some(PhaseType::Red));
        assert_eq!(s.cycle_index, Some(2));
        assert_eq!(s.remaining_seconds, 2);
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 3);
    }

    #[test]
    fn failed_stop_green_after_deadline_leaves_valid_red_state() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(5));
        assert!(e.stop_green().is_err());
        assert_eq!(e.snapshot().phase, Some(PhaseType::Red));
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 1);
    }

    #[test]
    fn starts_green_cycle_one_and_rejects_second_run() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        let s = e.start_run().unwrap();
        assert_eq!(s.phase, Some(PhaseType::Green));
        assert_eq!(s.cycle_index, Some(1));
        assert!(matches!(e.start_run(), Err(TimerError::RunAlreadyActive)));
    }
    #[test]
    fn stopping_green_records_early_and_starts_red() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(2));
        assert_eq!(e.stop_green().unwrap().phase, Some(PhaseType::Red));
        assert_eq!(
            e.active_run.as_ref().unwrap().phases[0].outcome,
            PhaseOutcome::CompletedEarly
        );
        assert!(matches!(e.stop_green(), Err(TimerError::NotInGreenPhase)));
        assert_eq!(e.snapshot().phase, Some(PhaseType::Red));
    }
    #[test]
    fn delayed_tick_crosses_boundaries_without_duplicates() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(8));
        let s = e.tick().unwrap();
        assert_eq!(s.phase, Some(PhaseType::Green));
        assert_eq!(s.cycle_index, Some(2));
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 2);
        e.tick().unwrap();
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 2);
    }
    #[test]
    fn tick_before_deadline_does_not_transition() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(4));
        assert_eq!(e.tick().unwrap().remaining_seconds, 1);
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 0);
    }
    #[test]
    fn stopping_run_interrupts_and_allows_restart() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        assert!(matches!(e.stop_run(), Err(TimerError::NoActiveRun)));
        e.start_run().unwrap();
        assert_eq!(e.stop_run().unwrap().interrupted, 1);
        e.start_run().unwrap();
    }
}
