<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { archiveTimer, createTimer, getTimerSnapshot, listRecentRuns, listTimers, startTimer, stopGreen, stopRun, tickTimer, updateTimer } from "./lib/api";
  import type { RunHistorySummary, TimerDefinitionDto, TimerRequest, TimerSnapshot } from "./lib/types";
  let state: TimerSnapshot = { active: false, phase: null, cycle_index: null, remaining_seconds: 0, timer_name: "", run_id: null, completed_phase_count: 0, green_duration_seconds: 0, red_duration_seconds: 0 };
  let timers: TimerDefinitionDto[] = []; let history: RunHistorySummary[] = []; let errorMessage: string | null = null; let pollHandle: ReturnType<typeof setInterval> | undefined;
  let form: TimerRequest = { name: "", green_duration_seconds: 40, red_duration_seconds: 20 }; let editing: TimerDefinitionDto | null = null;
  const formatTime = (seconds: number) => `${Math.floor(seconds / 60).toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
  const formatDate = (ms: number) => new Date(ms).toLocaleString();
  async function load() { timers = await listTimers(); history = await listRecentRuns(undefined, 20); state = await getTimerSnapshot(); }
  async function guarded(action: () => Promise<void>) { try { await action(); errorMessage = null; } catch (error) { errorMessage = String(error); } }
  async function refresh() { await guarded(async () => { state = await tickTimer(); }); }
  function edit(timer: TimerDefinitionDto) { editing = timer; form = { name: timer.name, green_duration_seconds: timer.green_duration_seconds, red_duration_seconds: timer.red_duration_seconds }; }
  function clearForm() { editing = null; form = { name: "", green_duration_seconds: 40, red_duration_seconds: 20 }; }
  async function saveTimer() { await guarded(async () => { if (editing) await updateTimer(editing.id, form); else await createTimer(form); clearForm(); await load(); }); }
  async function begin(timer: TimerDefinitionDto) { await guarded(async () => { state = await startTimer(timer.id); }); }
  async function handleStopRun() { await guarded(async () => { await stopRun(); await load(); }); }
  onMount(async () => { await guarded(load); pollHandle = setInterval(refresh, 1000); }); onDestroy(() => { if (pollHandle !== undefined) clearInterval(pollHandle); });
</script>
<main class:phase-green={state.phase === "Green"} class:phase-red={state.phase === "Red"}>
  <div class="panel">
    <h1>Red-Green Light</h1>
    {#if state.active}
      <section class="active"><h2>{state.timer_name}</h2><span class="cycle">Cycle {state.cycle_index}</span><div class="dial" aria-live="polite"><strong>{state.phase?.toUpperCase()}</strong><span>{formatTime(state.remaining_seconds)}</span></div><div class="controls">{#if state.phase === "Green"}<button on:click={() => guarded(async () => { state = await stopGreen(); })}>Stop Green</button>{/if}<button class="ghost" on:click={handleStopRun}>Stop Run</button></div><p>{state.completed_phase_count} completed phase record(s)</p></section>
    {/if}
    <section><h2>Timers</h2>{#each timers as timer}<article><strong>{timer.name}</strong><span>Green {formatTime(timer.green_duration_seconds)} · Red {formatTime(timer.red_duration_seconds)}</span><button disabled={state.active} on:click={() => begin(timer)}>Start</button><button disabled={state.active} on:click={() => edit(timer)}>Edit</button><button disabled={state.active} class="danger" on:click={() => guarded(async () => { await archiveTimer(timer.id); await load(); })}>Archive</button></article>{/each}</section>
    <section><h2>{editing ? "Edit timer" : "Create timer"}</h2><label>Name <input bind:value={form.name} /></label><label>Green seconds <input type="number" min="1" bind:value={form.green_duration_seconds} /></label><label>Red seconds <input type="number" min="1" bind:value={form.red_duration_seconds} /></label><button on:click={saveTimer}>{editing ? "Save changes" : "Create"}</button>{#if editing}<button class="ghost" on:click={clearForm}>Cancel</button>{/if}</section>
    <section><h2>Recent runs</h2>{#if history.length === 0}<p>No completed runs yet.</p>{/if}{#each history as run}<article><strong>{run.timer_name}</strong><span>{formatDate(run.started_at_unix_ms)} → {formatDate(run.ended_at_unix_ms)}</span><span>Last cycle {run.last_cycle_index}; early {run.green_completed_early}; expired {run.green_expired}; red done {run.red_completed}; interrupted {run.interrupted}</span></article>{/each}</section>
    {#if errorMessage}<p class="error" role="alert">{errorMessage}</p>{/if}
    <p class="note">Sprint 2A limitation: active runs are not restored after restart; adaptation remains unimplemented.</p>
  </div>
</main>
<style>
:global(body){margin:0;font-family:Inter,system-ui,sans-serif}main{min-height:100vh;background:#20242b;color:white;padding:2rem}main.phase-green{background:#0f2e1a}main.phase-red{background:#3a0f0f}.panel{max-width:980px;margin:auto;display:grid;gap:1rem}.active{text-align:center}.cycle{text-transform:uppercase;letter-spacing:.12em}.dial{margin:1rem auto;width:220px;height:220px;border:5px solid #fff5;border-radius:50%;display:flex;flex-direction:column;justify-content:center;gap:.8rem;font-size:3rem}.dial strong{font-size:1rem;letter-spacing:.2em}section,article{border:1px solid #fff3;border-radius:1rem;padding:1rem}article{display:flex;gap:.75rem;align-items:center;justify-content:space-between;margin:.5rem 0;flex-wrap:wrap}button,input{padding:.65rem;border:0;border-radius:.6rem}button{font-weight:700;cursor:pointer}.controls{display:flex;gap:.75rem;justify-content:center}.ghost{background:transparent;border:1px solid #fff8;color:white}.danger{background:#7f1d1d;color:white}.error{color:#ffb3b3}.note{opacity:.8}label{display:block;margin:.5rem 0}
</style>
