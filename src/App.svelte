<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { pauseTimer, resetTimer, startTimer, tickTimer } from "./lib/api";
  import type { TimerState } from "./lib/types";

  let state: TimerState = {
    phase: "Green",
    remaining_seconds: 40,
    running: false,
    cycle_count: 0,
  };

  let pollHandle: ReturnType<typeof setInterval> | undefined;
  let errorMessage: string | null = null;

  function formatTime(totalSeconds: number): string {
    const m = Math.floor(totalSeconds / 60)
      .toString()
      .padStart(2, "0");
    const s = (totalSeconds % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  }

  async function refresh() {
    try {
      state = await tickTimer();
      errorMessage = null;
    } catch (err) {
      errorMessage = String(err);
    }
  }

  function ensurePolling() {
    if (pollHandle === undefined) {
      pollHandle = setInterval(refresh, 1000);
    }
  }

  function stopPolling() {
    if (pollHandle !== undefined) {
      clearInterval(pollHandle);
      pollHandle = undefined;
    }
  }

  async function handleStart() {
    try {
      state = await startTimer();
      errorMessage = null;
      ensurePolling();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  async function handlePause() {
    try {
      state = await pauseTimer();
      errorMessage = null;
      stopPolling();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  async function handleReset() {
    try {
      state = await resetTimer();
      errorMessage = null;
      stopPolling();
    } catch (err) {
      errorMessage = String(err);
    }
  }

  onMount(() => {
    refresh();
  });

  onDestroy(() => {
    stopPolling();
  });

  $: phaseLabel = state.phase === "Green" ? "GO" : "STOP";
</script>

<main class="phase-{state.phase.toLowerCase()}">
  <div class="panel">
    <span class="cycle">cycle {state.cycle_count}</span>

    <div class="dial" aria-live="polite">
      <span class="phase-label">{phaseLabel}</span>
      <span class="time">{formatTime(state.remaining_seconds)}</span>
    </div>

    <div class="controls">
      {#if state.running}
        <button class="btn" on:click={handlePause}>Pause</button>
      {:else}
        <button class="btn" on:click={handleStart}>Start</button>
      {/if}
      <button class="btn ghost" on:click={handleReset}>Reset</button>
    </div>

    {#if errorMessage}
      <p class="error">{errorMessage}</p>
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: "Inter", "Segoe UI", system-ui, sans-serif;
  }

  main {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 400ms ease;
  }

  main.phase-green {
    background-color: #0f2e1a;
  }

  main.phase-red {
    background-color: #3a0f0f;
  }

  .panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    padding: 2.5rem 3rem;
  }

  .cycle {
    font-size: 0.8rem;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.55);
  }

  .dial {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 220px;
    height: 220px;
    border-radius: 50%;
    border: 6px solid rgba(255, 255, 255, 0.15);
  }

  .phase-green .dial {
    border-color: #3ddc84;
    box-shadow: 0 0 40px rgba(61, 220, 132, 0.25);
  }

  .phase-red .dial {
    border-color: #ff5c5c;
    box-shadow: 0 0 40px rgba(255, 92, 92, 0.25);
  }

  .phase-label {
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.3em;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 0.35rem;
  }

  .time {
    font-family: "SF Mono", "JetBrains Mono", monospace;
    font-size: 3rem;
    font-variant-numeric: tabular-nums;
    color: #ffffff;
  }

  .controls {
    display: flex;
    gap: 0.75rem;
  }

  .btn {
    padding: 0.6rem 1.6rem;
    border-radius: 999px;
    border: none;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    background: #ffffff;
    color: #1a1a1a;
    transition: transform 120ms ease, opacity 120ms ease;
  }

  .btn:hover {
    transform: translateY(-1px);
  }

  .btn:active {
    transform: translateY(0);
    opacity: 0.85;
  }

  .btn.ghost {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.35);
    color: rgba(255, 255, 255, 0.85);
  }

  .error {
    color: #ffb3b3;
    font-size: 0.85rem;
    max-width: 260px;
    text-align: center;
  }
</style>
