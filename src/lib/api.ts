import { invoke } from "@tauri-apps/api/core";
import type { CompletedRunSummary, TimerSnapshot } from "./types";

export const startTimer = (): Promise<TimerSnapshot> => invoke("start_timer");
export const stopGreen = (): Promise<TimerSnapshot> => invoke("stop_green");
export const stopRun = (): Promise<CompletedRunSummary> => invoke("stop_run");
export const tickTimer = (): Promise<TimerSnapshot> => invoke("tick_timer");
export const getTimerSnapshot = (): Promise<TimerSnapshot> => invoke("get_timer_snapshot");
