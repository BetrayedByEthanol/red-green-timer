# red-green-timer

A minimal Tauri + Svelte + TypeScript desktop app: an interval timer that
alternates between a `Green` (go/work) phase and a `Red` (stop/rest) phase.

## Layout

```
red-green-timer/
├── Cargo.toml                 # Rust workspace (timer-core + src-tauri)
├── crates/timer-core/         # Pure Rust domain logic, no Tauri dependency
│   └── src/
│       ├── lib.rs
│       ├── engine.rs          # Deadline-driven TimerEngine run state machine
│       ├── model.rs           # Run/phase domain models and validation
│       ├── state.rs           # TimerSnapshot IPC snapshot (serde)
│       └── error.rs           # TimerError (thiserror)
├── src/                       # Svelte + TS frontend (Vite)
│   ├── App.svelte
│   ├── main.ts
│   ├── app.css
│   └── lib/
│       ├── api.ts             # Typed invoke() wrappers
│       └── types.ts           # TS mirrors of the Rust types
└── src-tauri/                 # Tauri shell
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── main.rs
        ├── application.rs     # AppState (Mutex<TimerEngine>)
        └── commands.rs        # #[tauri::command] functions
```

## Design notes

- **`timer-core` has zero Tauri dependency.** It's a plain Rust library so
  the timer logic is independently testable (see `engine.rs`'s `#[cfg(test)]`
  module) and could be reused from a CLI or a different frontend later.
- **Deadline-driven, not thread driven.** `TimerEngine::tick()` measures
  elapsed time against `Instant`s rather than running its own background
  thread/timer. The frontend polls `tick_timer` on a 1s `setInterval`, and
  because elapsed time is computed from real timestamps, the state stays
  correct even if a tick is delayed or the window was backgrounded.
- **Exact-deadline semantics.** A phase is due when `now >= deadline`; at the
  exact deadline Green is expired and Red is completed. Before any user action,
  the engine first catches up every overdue deadline transition. Stop Green can
  therefore record `CompletedEarly` only while `now < deadline`; once the Green
  deadline is reached it processes the expiry and returns `NotInGreenPhase`
  because Red is active. Stop Run likewise catches up overdue Green/Red
  transitions first, then records only the phase active after catch-up as
  `Interrupted`.
- **Authoritative timestamps.** Sprint 1 runtime duration is derived from the
  injected monotonic clock. Automatic deadline transitions use the scheduled
  full allocated duration; manual early completion and interruption use elapsed
  monotonic duration and derive `ended_at` from `started_at + actual_duration`
  so persisted-compatible records remain internally consistent.
- **Sprint 1 run flow.** Start Run begins Green cycle 1. Stop Green records an
  early Green completion and immediately starts mandatory Red. Green expiry
  records `Expired`; Red expiry records `Completed` and starts the next
  one-based Green cycle. Stop Run records the active phase after deadline
  catch-up as `Interrupted`. Only one run can be active at a time.
- **Current scope.** History is in memory only. Persistence, adaptation,
  notifications, and dropped cycles are intentionally not implemented yet.
- **IPC types kept in sync manually.** `src/lib/types.ts` mirrors
  `timer_core::state::TimerSnapshot` and `Phase` by hand. If this grows, look at
  `tauri-specta` to generate the TS types directly from the Rust structs.

## Prerequisites

- Rust toolchain (`rustup`) with `cargo`
- Node.js (LTS) + npm
- Tauri's platform-specific system dependencies — see
  https://v2.tauri.app/start/prerequisites/

## Running

```bash
npm install
npm run tauri dev
```

This starts the Vite dev server on port 1420 and launches the Tauri window
pointed at it (both configured in `tauri.conf.json` / `vite.config.ts`).

## Testing the Rust core

```bash
cargo test -p timer-core
```

The core timing tests use an injected fake clock rather than real sleeps, so
exact deadlines, delayed polling, and action-before-poll races are deterministic.

## Formatting, linting, and CI

```bash
npm run format:check
npm run lint
```

- `format:check` runs `cargo fmt --all --check` using the repository `rustfmt.toml`.
- `lint` runs Svelte/TypeScript diagnostics with `svelte-check` and Rust lints with Clippy.
- Shared whitespace defaults live in `.editorconfig`; Clippy thresholds live in `clippy.toml`.
- GitHub Actions CI runs on pushes to `master` and on pull requests. It runs
  Rust formatting, Clippy, workspace tests, frontend checks, and the frontend
  build. Full Tauri bundle builds remain a local verification step because
  Linux native dependencies vary by environment.

## Building a release bundle

```bash
npm run tauri build
```

Note: `tauri.conf.json`'s `bundle.icon` is currently an empty array. Add real
icons (e.g. via `npm run tauri icon path/to/source.png`) before producing a
distributable bundle — the app builds and runs in dev without them, but a
release bundle typically expects app icons to be present.
