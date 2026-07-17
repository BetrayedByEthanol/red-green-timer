# AGENTS.md

## Project

**Red-Green Light** is a local-first desktop timer application for repeated work/rest cycles.

Each timer alternates between:

- **Green** — active work phase with an adaptive duration.
- **Red** — mandatory recovery phase with a fixed duration.

The application records completed runs and uses previous results to conservatively adjust future Green durations. Correct timer behavior, recoverable adaptation, and trustworthy history are more important than aggressive optimization.

## Technology Stack

- **Desktop shell:** Tauri 2
- **Frontend:** Svelte 4, TypeScript, Vite
- **Core domain logic:** Rust in a separate `timer-core` crate
- **Async runtime:** Tokio where asynchronous work is required
- **Persistence:** SQLite through `sqlx`
- **Serialization:** Serde
- **Time handling:** `time` or `chrono`; use one consistently within a crate
- **Errors:** `thiserror`
- **Randomization:** `rand`, with injectable or seeded randomness in tests

Do not upgrade Tauri, Svelte, Vite, or `@sveltejs/vite-plugin-svelte` independently. Keep their supported version ranges compatible and preserve the repository lockfile.

## Architectural Boundaries

Keep the timer engine independent from the UI and operating-system integration.

### Rust core owns

- Timer state and phase transitions
- Run and cycle lifecycle
- Deadline calculations
- Completion and expiry records
- Adaptive Green-duration calculations
- Minimum and maximum duration constraints
- Dropped-cycle decisions and safeguards
- Domain validation

### Tauri layer owns

- Commands and event transport between Svelte and Rust
- Application lifecycle integration
- Shutdown/finalization handling
- Persistence orchestration
- Desktop notifications and platform-specific behavior

### Svelte frontend owns

- Rendering state received from the backend
- User input and configuration forms
- Start, pause, reset, stop, and finish controls
- History and settings views
- Presentation-only countdown formatting

Do not duplicate timer or adaptation logic in TypeScript. The backend state is authoritative.

## Core Behavioral Invariants

These rules are product requirements. Do not change them without an explicit decision.

1. Only one timer runs at a time by default.
2. Green expiry immediately transitions to Red; there is no Green overtime.
3. Red duration is fixed for the configured timer.
4. Red cannot be skipped.
5. Green adaptation is calculated only between runs, never during an active run.
6. Green duration may differ by cycle position after adaptation.
7. Users cannot manually restore or increase an adapted Green duration.
8. Adaptation can be disabled while observations are still recorded.
9. Reductions must be reversible when repeated Green phases expire.
10. Closing the application or computer must finalize and record the current result as safely as possible.
11. Do not implement saved-time credits or manual skip credits.
12. A rare automatic dropped Green cycle may be considered only when a run exceeds its historically expected cycle count. It must not end the run and must have strict safeguards.

## Timing Rules

Use deadline-based timing rather than decrementing a counter once per second.

- Store a monotonic start instant and a target deadline.
- Derive remaining time from the current monotonic clock.
- UI update frequency must not affect correctness.
- Pausing stores the actual remaining duration.
- Resuming creates a new deadline from the stored remaining duration.
- Wall-clock changes, rendering delays, sleep, and temporary UI stalls must not extend a phase accidentally.

Inject the clock into core logic so tests can advance time deterministically.

## Run and Cycle Model

Use explicit domain types rather than loosely related flags.

Suggested concepts:

- `TimerConfig`
- `TimerState`
- `Phase` (`Green`, `Red`, `Idle`, and any explicit terminal state)
- `Run`
- `CycleRecord`
- `CompletionSignal`
- `AdaptationSettings`
- `CycleProfile`

A cycle record should distinguish at least:

- Green completed early by the user
- Green expired
- Run stopped or interrupted
- Red completed
- Application shutdown or abnormal interruption

Do not treat an interrupted observation as a successful early completion.

## Adaptive Green-Duration Rules

The algorithm must be conservative and explainable.

### Required behavior

- Adapt each cycle position separately.
- Earlier cycle results may influence later cycle positions in the next run.
- Later cycle results must not influence earlier cycle positions.
- An early finish in cycle 1 may affect cycles 1 and later.
- An early finish in the last observed cycle affects only that cycle position.
- Use small bounded reductions after sufficient evidence of early completion.
- Increase durations after repeated expiry evidence.
- Clamp every duration to configured minimum and maximum values.
- Preserve reversibility; no duration is permanently reduced.

### Avoid cumulative overcorrection

Do not add the same upstream reduction repeatedly for every preceding cycle.

Example: if cycle 1 appears 10 seconds too long, cycle 5 must not automatically be reduced by `10 seconds × 5`. Use bounded propagation, weighted evidence, or a single capped look-forward influence.

### Randomization

Random steps may prevent adaptation from becoming mechanically predictable, but randomness must remain bounded.

- Keep deterministic tests by injecting a seeded RNG or random-step provider.
- Never let randomization bypass duration limits or evidence thresholds.
- Persist the resulting decision, not merely the raw random seed.

### Dropped-cycle safeguard

A dropped Green cycle is not a user-controlled skip and is not earned from saved time.

Only consider it when all of the following are true:

- The current run has exceeded the established expected cycle count.
- There is enough completed-run history to establish that expectation.
- The drop probability is low and bounded.
- The Red phase remains mandatory.
- The run continues afterward.
- The decision is logged explicitly.

Do not include this feature in early MVP work unless the surrounding run-history and safety logic already exists.

## Persistence

Use SQLite for durable state. Migrations must be checked into the repository.

Persist at least:

- Timer configurations
- Per-cycle Green profiles
- Completed runs
- Cycle observations
- Adaptation enabled/disabled state
- Adaptation decisions and relevant inputs
- Schema/application version metadata

Persistence rules:

- Never silently discard a completed cycle.
- Write completed observations transactionally.
- Keep raw observations separate from derived adaptation settings.
- Rebuild or audit derived profiles from stored observations where practical.
- Use UTC timestamps for persistence and convert only for display.
- Handle schema migrations explicitly; do not mutate production tables ad hoc.

## Current Implementation State

The prototype currently includes:

- Fixed Green/Red phase transitions
- Start, pause, and reset behavior
- Deadline-based ticking
- Cycle counter
- Basic full-screen UI
- Initial Rust unit tests

Major remaining work includes:

- Configurable timers and durations
- Explicit run start and finish workflow
- Completion signals and cycle records
- SQLite persistence and migrations
- Adaptive Green-duration engine
- Per-cycle look-forward adaptation
- Minimum/maximum constraints
- History and settings UI
- Shutdown finalization
- Notifications
- Frontend tests
- Continuous integration
- Late-run dropped-cycle safeguards

Prefer completing the run-recording foundation before implementing advanced adaptation.

## Development Priorities

Work in this order unless the task explicitly requires otherwise:

1. Stabilize domain types and invariants in `timer-core`.
2. Add deterministic clock and RNG abstractions.
3. Implement explicit run/cycle recording.
4. Add SQLite persistence and migrations.
5. Add conservative adaptation with unit tests.
6. Expose backend behavior through typed Tauri commands/events.
7. Build configuration, active-run, history, and settings UI.
8. Add shutdown handling and notifications.
9. Add guarded late-run dropped-cycle behavior.

## Coding Standards

### Rust

- Prefer explicit domain enums and newtypes over booleans and primitive tuples.
- Keep pure calculation functions free of I/O.
- Return typed errors; do not use `unwrap()` or `expect()` in production paths.
- Use checked or saturating duration arithmetic where appropriate.
- Document non-obvious adaptation formulas and invariants.
- Keep Tauri command payloads serializable and versionable.

### TypeScript and Svelte

- Enable and preserve strict TypeScript checking.
- Avoid `any`; model command responses and events explicitly.
- Keep components small and move reusable presentation state into stores or modules.
- Do not create a second frontend timer engine.
- Unsubscribe from event listeners and timers during component teardown.
- Treat backend errors as visible application states, not console-only messages.

### General

- Make the smallest coherent change that completes the task.
- Preserve existing public behavior unless the task changes a documented requirement.
- Avoid unrelated dependency upgrades and broad refactors.
- Add comments for rationale, not for obvious syntax.
- Never commit generated build output, local databases, secrets, or machine-specific paths.

## Testing Requirements

Every behavior change must include appropriate tests.

### Core unit tests

Cover at least:

- Green expiry transitions to Red
- Red completion transitions correctly
- Pause/resume preserves remaining time
- One-active-timer enforcement
- Early completion recording
- Expiry recording
- Interrupted-run recording
- Adaptation disabled but recording enabled
- Per-cycle adaptation isolation
- Look-forward influence without backward influence
- No cumulative multiplication of upstream reductions
- Minimum and maximum clamping
- Reversal after repeated expiry
- Deterministic random steps
- Dropped-cycle eligibility and rejection cases

Use fake clocks and seeded randomness. Do not use real sleeps in unit tests.

### Persistence tests

- Run migrations against a temporary database.
- Verify transactional writes.
- Verify stored timestamps and duration units.
- Verify completed observations survive reload.
- Verify derived adaptation state can be audited against source records.

### Frontend tests

Test user-visible behavior, especially:

- Correct controls for each phase
- Red cannot be skipped
- Backend errors are displayed
- Reloaded state renders correctly
- Event subscriptions are cleaned up

## Verification Commands

Use the package manager represented by the repository lockfile. For the current npm-based setup, run:

```bash
npm ci
npm run check
npm run build
npm run tauri build
```

Run Rust checks from the workspace root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

During development:

```bash
npm run tauri dev
```

If scripts differ in the repository, use the existing script names rather than creating aliases solely to match this file.

## Dependency Changes

Before changing dependencies:

1. Confirm the change is required for the task.
2. Check compatibility across Tauri, Svelte, Vite, and the Svelte Vite plugin.
3. Update the relevant lockfile.
4. Run frontend and Rust checks.
5. Record any migration or platform impact.

Do not solve peer-dependency conflicts with `--force` or `--legacy-peer-deps` unless the repository explicitly documents that policy. Fix the incompatible versions instead.

## Agent Workflow

When modifying the project:

1. Read this file and the nearest relevant source files.
2. Identify which behavior invariants apply.
3. Inspect existing tests before changing public behavior.
4. Keep domain logic in Rust core.
5. Add or update tests with the implementation.
6. Run the narrowest relevant checks, then the full verification suite when feasible.
7. Summarize changed files, behavior, tests run, and any remaining limitations.

Do not claim a command passed unless it was actually executed successfully.

## Definition of Done

A change is complete when:

- The requested behavior is implemented.
- Product invariants remain intact.
- Core logic is not duplicated in the frontend.
- New behavior has deterministic tests.
- Persistence changes include migrations and tests.
- Formatting, linting, type checking, and relevant tests pass.
- Errors and interrupted states are handled explicitly.
- Documentation is updated when architecture or behavior changes.

## Non-Goals for Early MVP

Unless explicitly requested, do not prioritize:

- Cloud synchronization
- Accounts or authentication
- Multi-device coordination
- Concurrent active timers
- Manual Green increases
- Skippable Red phases
- Saved-time or skip-credit systems
- Complex analytics dashboards
- Aggressive machine-learning adaptation

The MVP should remain local, deterministic, auditable, and conservative.
