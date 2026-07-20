/** Mirrors serializable `timer_core` snapshot and summary types. */
export type PhaseType = "Green" | "Red";
export type PhaseOutcome = "CompletedEarly" | "Completed" | "Expired" | "Interrupted";

export interface TimerSnapshot {
  active: boolean;
  phase: PhaseType | null;
  cycle_index: number | null;
  remaining_seconds: number;
  timer_name: string;
  run_id: string | null;
  completed_phase_count: number;
  green_duration_seconds: number;
  red_duration_seconds: number;
}

export interface CompletedRunSummary {
  green_completed_early: number;
  green_expired: number;
  red_completed: number;
  interrupted: number;
  total_completed_phase_records: number;
  last_cycle_index: number;
}
