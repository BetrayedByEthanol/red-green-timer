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
        let active = self.take_active(PhaseType::Green)?;
        self.complete(active, PhaseOutcome::CompletedEarly)?;
        self.start_phase(
            PhaseType::Red,
            self.clock.now_instant(),
            self.clock.now_system(),
        )?;
        Ok(self.snapshot())
    }
    pub fn stop_run(&mut self) -> Result<CompletedRunSummary, TimerError> {
        let active = self.take_any_active()?;
        self.complete(active, PhaseOutcome::Interrupted)?;
        let run = self.active_run.take().ok_or(TimerError::NoActiveRun)?;
        Ok(CompletedRunSummary::from_run(run))
    }
    pub fn tick(&mut self) -> TimerSnapshot {
        let now = self.clock.now_instant();
        loop {
            let due = self
                .active_run
                .as_ref()
                .and_then(|r| r.state.active_phase())
                .is_some_and(|p| now >= p.deadline);
            if !due {
                break;
            }
            let result = (|| -> Result<(), TimerError> {
                let phase = self.take_any_active()?;
                let next_start = phase.deadline;
                let next_wall = phase
                    .started_at
                    .checked_add(phase.allocated_duration)
                    .unwrap_or(phase.started_at);
                let ty = phase.phase_type;
                self.complete(
                    phase,
                    if ty == PhaseType::Green {
                        PhaseOutcome::Expired
                    } else {
                        PhaseOutcome::Completed
                    },
                )?;
                if ty == PhaseType::Green {
                    self.start_phase(PhaseType::Red, next_start, next_wall)?;
                } else {
                    let run = self.active_run.as_mut().ok_or(TimerError::NoActiveRun)?;
                    run.cycle_index = run.cycle_index.saturating_add(1);
                    self.start_phase(PhaseType::Green, next_start, next_wall)?;
                }
                Ok(())
            })();
            if result.is_err() {
                break;
            }
        }
        self.snapshot()
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
    fn start_phase(
        &mut self,
        ty: PhaseType,
        instant: Instant,
        wall: SystemTime,
    ) -> Result<(), TimerError> {
        let active = self.new_active(ty, instant, wall)?;
        let run = self.active_run.as_mut().ok_or(TimerError::NoActiveRun)?;
        run.state = match ty {
            PhaseType::Green => TimerState::RunningGreen(active),
            PhaseType::Red => TimerState::RunningRed(active),
        };
        Ok(())
    }
    fn take_any_active(&mut self) -> Result<ActivePhase, TimerError> {
        let run = self.active_run.as_mut().ok_or(TimerError::NoActiveRun)?;
        let state = std::mem::replace(&mut run.state, TimerState::Stopped);
        match state {
            TimerState::RunningGreen(p) | TimerState::RunningRed(p) => Ok(p),
            TimerState::Stopped => Err(TimerError::InvalidTransition),
        }
    }
    fn take_active(&mut self, required: PhaseType) -> Result<ActivePhase, TimerError> {
        let active_type = self
            .active_run
            .as_ref()
            .and_then(|run| run.state.active_phase())
            .map(|phase| phase.phase_type)
            .ok_or(TimerError::NoActiveRun)?;
        if active_type != required {
            return Err(TimerError::NotInGreenPhase);
        }
        let active = self.take_any_active()?;
        Ok(active)
    }
    fn complete(&mut self, active: ActivePhase, outcome: PhaseOutcome) -> Result<(), TimerError> {
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
        let completed = CompletedPhase::new(
            active.phase_type,
            cycle,
            active.started_at,
            ended,
            active.allocated_duration,
            actual,
            outcome,
        )?;
        self.active_run
            .as_mut()
            .ok_or(TimerError::NoActiveRun)?
            .phases
            .push(completed);
        Ok(())
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
        let s = e.tick();
        assert_eq!(s.phase, Some(PhaseType::Green));
        assert_eq!(s.cycle_index, Some(2));
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 2);
        e.tick();
        assert_eq!(e.active_run.as_ref().unwrap().phases.len(), 2);
    }
    #[test]
    fn tick_before_deadline_does_not_transition() {
        let c = FakeClock::new();
        let mut e = engine(&c);
        e.start_run().unwrap();
        c.advance(Duration::from_secs(4));
        assert_eq!(e.tick().remaining_seconds, 1);
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
