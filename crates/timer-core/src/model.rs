use serde::{Deserialize, Serialize};

/// The two alternating phases of a red/green interval timer.
///
/// Convention: `Green` is the "go" / work phase, `Red` is the "stop" / rest phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Red,
    Green,
}

impl Phase {
    /// Returns the opposite phase.
    pub fn toggle(self) -> Self {
        match self {
            Phase::Red => Phase::Green,
            Phase::Green => Phase::Red,
        }
    }
}

/// Static configuration for a red/green timer cycle.
///
/// Durations are expressed in whole seconds to keep the type trivially
/// serializable across the Tauri IPC boundary without pulling in a
/// `serde`-with-`Duration` shim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimerConfig {
    pub red_seconds: u64,
    pub green_seconds: u64,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            red_seconds: 20,
            green_seconds: 40,
        }
    }
}
