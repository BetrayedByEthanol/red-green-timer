use std::time::{Duration, Instant};

use crate::error::TimerError;
use crate::model::{Phase, TimerConfig};
use crate::state::TimerSnapshot;

/// Drives a red/green interval cycle.
///
/// `TimerEngine` is deliberately clock-agnostic about *when* it gets polled:
/// callers invoke [`TimerEngine::tick`] whenever they want the engine to
/// catch up to wall-clock time (e.g. on a 1s interval from the frontend, or
/// from a backend-side scheduler). This keeps the engine easy to drive from
/// a Tauri command without needing an internal thread/timer of its own.
pub struct TimerEngine {
    config: TimerConfig,
    phase: Phase,
    running: bool,
    remaining: Duration,
    last_tick: Option<Instant>,
    cycle_count: u64,
}

impl TimerEngine {
    /// Builds a new engine starting in the `Green` phase, paused.
    pub fn new(config: TimerConfig) -> Result<Self, TimerError> {
        if config.red_seconds == 0 || config.green_seconds == 0 {
            return Err(TimerError::InvalidConfig(
                "red and green durations must both be greater than zero".into(),
            ));
        }

        Ok(Self {
            remaining: Duration::from_secs(config.green_seconds),
            phase: Phase::Green,
            running: false,
            last_tick: None,
            cycle_count: 0,
            config,
        })
    }

    /// Starts (or resumes) the timer. Errors if already running.
    pub fn start(&mut self) -> Result<(), TimerError> {
        if self.running {
            return Err(TimerError::AlreadyRunning);
        }
        self.running = true;
        self.last_tick = Some(Instant::now());
        Ok(())
    }

    /// Pauses the timer, folding in any elapsed time first. Errors if not running.
    pub fn pause(&mut self) -> Result<(), TimerError> {
        if !self.running {
            return Err(TimerError::NotRunning);
        }
        self.tick();
        self.running = false;
        self.last_tick = None;
        Ok(())
    }

    /// Resets to the initial `Green` phase at full duration, paused, with the
    /// cycle counter cleared.
    pub fn reset(&mut self) {
        self.phase = Phase::Green;
        self.remaining = Duration::from_secs(self.config.green_seconds);
        self.running = false;
        self.last_tick = None;
        self.cycle_count = 0;
    }

    /// Advances internal state based on wall-clock time elapsed since the
    /// last tick (or since `start`, if this is the first tick), then returns
    /// a fresh snapshot. Safe to call whether or not the timer is running.
    pub fn tick(&mut self) -> TimerSnapshot {
        if self.running {
            let now = Instant::now();
            if let Some(last) = self.last_tick {
                let elapsed = now.duration_since(last);
                self.advance(elapsed);
            }
            self.last_tick = Some(now);
        }
        self.snapshot()
    }

    /// Consumes elapsed wall-clock time, rolling over phases as needed so
    /// that even long gaps between ticks (e.g. the app was backgrounded)
    /// are accounted for correctly rather than just clamped to zero.
    fn advance(&mut self, mut elapsed: Duration) {
        while elapsed > Duration::ZERO && self.running {
            if elapsed >= self.remaining {
                elapsed -= self.remaining;
                self.advance_phase();
            } else {
                self.remaining -= elapsed;
                elapsed = Duration::ZERO;
            }
        }
    }

    fn advance_phase(&mut self) {
        self.phase = self.phase.toggle();
        if self.phase == Phase::Green {
            self.cycle_count += 1;
        }
        self.remaining = Duration::from_secs(match self.phase {
            Phase::Red => self.config.red_seconds,
            Phase::Green => self.config.green_seconds,
        });
    }

    /// A point-in-time view of the engine without advancing it.
    pub fn snapshot(&self) -> TimerSnapshot {
        TimerSnapshot {
            phase: self.phase,
            remaining_seconds: self.remaining.as_secs(),
            running: self.running,
            cycle_count: self.cycle_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_in_green_paused() {
        let engine = TimerEngine::new(TimerConfig::default()).unwrap();
        let snap = engine.snapshot();
        assert_eq!(snap.phase, Phase::Green);
        assert!(!snap.running);
        assert_eq!(snap.remaining_seconds, 40);
    }

    #[test]
    fn rejects_zero_duration_config() {
        let cfg = TimerConfig {
            red_seconds: 0,
            green_seconds: 10,
        };
        assert!(TimerEngine::new(cfg).is_err());
    }

    #[test]
    fn double_start_errors() {
        let mut engine = TimerEngine::new(TimerConfig::default()).unwrap();
        engine.start().unwrap();
        assert!(matches!(engine.start(), Err(TimerError::AlreadyRunning)));
    }

    #[test]
    fn pause_without_start_errors() {
        let mut engine = TimerEngine::new(TimerConfig::default()).unwrap();
        assert!(matches!(engine.pause(), Err(TimerError::NotRunning)));
    }

    #[test]
    fn advance_rolls_over_phase_boundary() {
        let cfg = TimerConfig {
            red_seconds: 5,
            green_seconds: 5,
        };
        let mut engine = TimerEngine::new(cfg).unwrap();
        engine.running = true;
        engine.advance(Duration::from_secs(7));
        let snap = engine.snapshot();
        assert_eq!(snap.phase, Phase::Red);
        assert_eq!(snap.remaining_seconds, 3);
        assert_eq!(snap.cycle_count, 0);
    }

    #[test]
    fn cycle_count_increments_on_return_to_green() {
        let cfg = TimerConfig {
            red_seconds: 5,
            green_seconds: 5,
        };
        let mut engine = TimerEngine::new(cfg).unwrap();
        engine.running = true;
        engine.advance(Duration::from_secs(12));
        let snap = engine.snapshot();
        assert_eq!(snap.phase, Phase::Green);
        assert_eq!(snap.cycle_count, 1);
        assert_eq!(snap.remaining_seconds, 3);
    }
}
