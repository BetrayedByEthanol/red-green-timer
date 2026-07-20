export type PhaseType = "Green" | "Red";
export interface TimerSnapshot { active: boolean; phase: PhaseType | null; cycle_index: number | null; remaining_seconds: number; timer_name: string; run_id: string | null; completed_phase_count: number; green_duration_seconds: number; red_duration_seconds: number; }
export interface CompletedRunSummary { run_id: string; green_completed_early: number; green_expired: number; red_completed: number; interrupted: number; total_completed_phase_records: number; last_cycle_index: number; }
export interface TimerDefinitionDto { id: string; name: string; green_duration_seconds: number; red_duration_seconds: number; archived: boolean; }
export interface TimerRequest { name: string; green_duration_seconds: number; red_duration_seconds: number; }
export interface RunHistorySummary { run_id: string; timer_id: string; timer_name: string; started_at_unix_ms: number; ended_at_unix_ms: number; last_cycle_index: number; green_completed_early: number; green_expired: number; red_completed: number; interrupted: number; total_phase_records: number; }
