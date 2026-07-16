import { invoke } from "@tauri-apps/api/core";
import type { TimerState } from "./types";

/**
 * Thin typed wrappers around the Tauri commands defined in
 * `src-tauri/src/commands.rs`. Keep this file as the single place that
 * knows the IPC command names/signatures.
 */

export function startTimer(): Promise<TimerState> {
  return invoke<TimerState>("start_timer");
}

export function pauseTimer(): Promise<TimerState> {
  return invoke<TimerState>("pause_timer");
}

export function resetTimer(): Promise<TimerState> {
  return invoke<TimerState>("reset_timer");
}

export function tickTimer(): Promise<TimerState> {
  return invoke<TimerState>("tick_timer");
}
