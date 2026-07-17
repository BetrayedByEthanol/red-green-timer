<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getTimerSnapshot, startTimer, stopGreen, stopRun, tickTimer } from "./lib/api";
  import type { CompletedRunSummary, TimerSnapshot } from "./lib/types";

  let state: TimerSnapshot = { active: false, phase: null, cycle_index: null, remaining_seconds: 0, timer_name: "", run_id: null, completed_phase_count: 0, green_duration_seconds: 0, red_duration_seconds: 0 };
  let summary: CompletedRunSummary | null = null;
  let pollHandle: ReturnType<typeof setInterval> | undefined;
  let errorMessage: string | null = null;
  const formatTime = (seconds: number) => `${Math.floor(seconds / 60).toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
  async function run(action: () => Promise<TimerSnapshot>) { try { state = await action(); errorMessage = null; } catch (error) { errorMessage = String(error); } }
  async function refresh() { await run(tickTimer); }
  async function handleStopRun() { try { summary = await stopRun(); state = await getTimerSnapshot(); errorMessage = null; } catch (error) { errorMessage = String(error); } }
  onMount(async () => { await run(getTimerSnapshot); pollHandle = setInterval(refresh, 1000); });
  onDestroy(() => { if (pollHandle !== undefined) clearInterval(pollHandle); });
</script>

<main class:phase-green={state.phase === "Green"} class:phase-red={state.phase === "Red"}>
  <div class="panel">
    <h1>{state.timer_name}</h1>
    {#if state.active}
      <span class="cycle">Cycle {state.cycle_index}</span>
      <div class="dial" aria-live="polite"><strong>{state.phase?.toUpperCase()}</strong><span>{formatTime(state.remaining_seconds)}</span></div>
      <div class="controls">
        {#if state.phase === "Green"}<button on:click={() => run(stopGreen)}>Stop Green</button>{/if}
        <button class="ghost" on:click={handleStopRun}>Stop Run</button>
      </div>
      <p>{state.completed_phase_count} completed phase record(s)</p>
    {:else}
      <p>Green: {formatTime(state.green_duration_seconds)}</p><p>Red: {formatTime(state.red_duration_seconds)}</p>
      <button on:click={() => run(startTimer)}>Start</button>
    {/if}
    {#if summary}<section><h2>Last run</h2><p>Green phases completed early: {summary.green_completed_early}</p><p>Green phases expired: {summary.green_expired}</p><p>Red phases completed: {summary.red_completed}</p><p>Interrupted phases: {summary.interrupted}</p><p>Total completed phase records: {summary.total_completed_phase_records}</p><p>Last cycle index: {summary.last_cycle_index}</p></section>{/if}
    {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}
  </div>
</main>

<style>
  :global(body) { margin: 0; font-family: Inter, system-ui, sans-serif; } main { min-height: 100vh; display:flex; align-items:center; justify-content:center; background:#20242b; color:white; } main.phase-green{background:#0f2e1a} main.phase-red{background:#3a0f0f}.panel{display:flex;flex-direction:column;align-items:center;gap:1rem;padding:2rem;text-align:center}.cycle{text-transform:uppercase;letter-spacing:.12em}.dial{width:220px;height:220px;border:5px solid #fff5;border-radius:50%;display:flex;flex-direction:column;justify-content:center;gap:.8rem;font-size:3rem}.dial strong{font-size:1rem;letter-spacing:.2em}button{padding:.65rem 1.4rem;border:0;border-radius:2rem;font-weight:700;cursor:pointer}.controls{display:flex;gap:.75rem}.ghost{background:transparent;border:1px solid #fff8;color:white}.error{color:#ffb3b3;max-width:30rem}section{border-top:1px solid #fff4;padding-top:1rem}
</style>
