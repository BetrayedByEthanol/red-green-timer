/**
 * Mirrors `timer_core::model::Phase`. Serde serializes unit-variant enums
 * as their bare string tag by default, so this must stay in sync with the
 * Rust enum's variant names.
 */
export type Phase = "Red" | "Green";

/**
 * Mirrors `timer_core::state::TimerState`.
 */
export interface TimerState {
  phase: Phase;
  remaining_seconds: number;
  running: boolean;
  cycle_count: number;
}
