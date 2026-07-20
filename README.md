# red-green-timer

A local-first Tauri + Svelte + TypeScript desktop interval timer that alternates
between a `Green` work phase and a mandatory `Red` recovery phase.

## Current Sprint 2A capabilities

- Persistent SQLite timer definitions.
- Multiple saved timers, each with a stable UUID, name, Green duration, and Red duration.
- Create, edit, and archive timer definitions without hard-deleting historical runs.
- Start a selected saved timer while preserving the Sprint 1 Rust state machine as the only authoritative runtime timer engine.
- Persist completed runs and completed phase history transactionally when Stop Run succeeds.
- Display recent completed-run history in the Svelte UI.

Adaptation, active-run recovery after restart, notifications, system tray behavior,
late-run dropped Green cycles, and cycle-specific Green profiles are intentionally
not implemented yet.

## Layout

```text
red-green-timer/
├── Cargo.toml                         # Rust workspace
├── crates/
│   ├── timer-core/                    # Pure Rust deadline-driven timer domain logic
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs
│   │       ├── model.rs
│   │       ├── state.rs
│   │       └── error.rs
│   └── timer-persistence/             # SQLite repositories, no Tauri dependency
│       ├── migrations/
│       │   └── 0001_initial.sql       # timers, runs, and phases schema
│       └── src/
│           ├── lib.rs
│           ├── error.rs
│           ├── repository.rs
│           ├── timer_repository.rs
│           ├── run_repository.rs
│           ├── mapping.rs
│           └── model.rs
├── src/                               # Svelte + TS frontend (Vite)
│   ├── App.svelte
│   ├── main.ts
│   ├── app.css
│   └── lib/
│       ├── api.ts
│       └── types.ts
└── src-tauri/                         # Tauri shell and application controller
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json
    └── src/
        ├── main.rs
        ├── application.rs             # ApplicationController + typed app errors
        └── commands.rs                # Thin IPC command wrappers
```

## Persistence

The app stores SQLite data in the Tauri application data directory as:

```text
<app data directory>/red-green-timer.sqlite3
```

The Tauri shell resolves this directory with Tauri's path resolver and creates
it before opening the database. The persistence crate embeds and runs migrations
on startup, enables SQLite foreign keys, configures a busy timeout, and prefers
WAL mode for file-backed databases.

The initial schema stores:

- `timers` for saved timer definitions and archival timestamps.
- `runs` for completed run summaries.
- `phases` for ordered completed phase records linked to a run.

Timestamps and durations are stored as signed 64-bit Unix nanoseconds. Conversion
helpers reject pre-epoch timestamps, negative stored durations, and values that
do not fit in `i64` rather than silently clamping them.

On first launch, the application seeds one default timer only when the `timers`
table has never contained a timer:

- Name: `Red-Green Light`
- Green: 40 seconds
- Red: 20 seconds

If all timers have been archived, the app does not recreate the default
implicitly; create a new timer from the UI instead.

### Resetting development data

To reset local development data safely, close the application and delete only
the development app data database file, `red-green-timer.sqlite3`, from the app
data directory for this app. Do not delete checked-in migrations or repository
files. The database will be recreated and migrations will run on the next launch.

## Design notes

- **`timer-core` has zero Tauri or SQLite dependency.** Runtime phase
  transitions, deadline processing, Stop Green, Stop Run, and one-based cycle
  indexing remain owned by the Rust core crate.
- **`timer-persistence` has zero Tauri dependency.** It owns SQLite schema
  migrations, typed persistence errors, timer CRUD, completed-run writes, and
  run-history reads.
- **Application controller boundary.** `src-tauri` coordinates saved timer
  definitions, a single in-memory active `TimerEngine`, and persistence. Tauri
  commands are thin wrappers and do not duplicate timer business logic.
- **Stop Run persistence safety.** Completed run history is staged in memory if
  SQLite fails while persisting Stop Run. The controller returns the database
  error, keeps the completed summary recoverable as a pending write, and blocks
  new starts until `stop_run` is called again and the write succeeds. This avoids
  overwriting an unpersisted completed run and avoids duplicate records.
- **Deadline-driven behavior remains authoritative.** The frontend polls the
  backend but does not maintain its own timer engine.

## Sprint 2A limitations

- An active run is not restored after application restart or crash. If the app
  closes with an active run during Sprint 2A, that unfinished run may be lost.
  Active-run checkpointing and clean-shutdown finalization are deferred to
  Sprint 2B.
- Adaptation remains unimplemented. Green durations are the saved timer
  definition values only.
- Notifications, system tray behavior, import/export, charts, and guarded
  late-run dropped Green cycles are not implemented.

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

## Testing

```bash
cargo test --workspace
cargo test -p timer-persistence
npm run lint:frontend
npm run build
```

The core timing tests use an injected fake clock rather than real sleeps, so
exact deadlines, delayed polling, and action-before-poll races are deterministic.

## Formatting, linting, and CI

```bash
npm run format:check
npm run lint
```

GitHub Actions CI runs Rust formatting, Clippy, workspace tests, frontend
checks, and the frontend build. Full Tauri bundle builds remain a local
verification step because Linux native dependencies vary by environment.

## Building a release bundle

```bash
npm run tauri build
```

Note: `tauri.conf.json`'s `bundle.icon` is currently an empty array. Add real
icons before producing a distributable bundle.
