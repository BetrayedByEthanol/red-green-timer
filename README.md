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
│       ├── engine.rs          # TimerEngine: start/pause/reset/tick
│       ├── model.rs           # Phase, TimerConfig
│       ├── state.rs           # TimerState snapshot (serde)
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
- **Wall-clock driven, not thread driven.** `TimerEngine::tick()` measures
  elapsed time against `Instant`s rather than running its own background
  thread/timer. The frontend polls `tick_timer` on a 1s `setInterval`, and
  because elapsed time is computed from real timestamps, the state stays
  correct even if a tick is delayed or the window was backgrounded.
- **IPC types kept in sync manually.** `src/lib/types.ts` mirrors
  `timer_core::state::TimerState` and `Phase` by hand. If this grows, look at
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

## Formatting and linting

```bash
npm run format:check
npm run lint
```

- `format:check` runs `cargo fmt --all --check` using the repository `rustfmt.toml`.
- `lint` runs Svelte/TypeScript diagnostics with `svelte-check` and Rust lints with Clippy.
- Shared whitespace defaults live in `.editorconfig`; Clippy thresholds live in `clippy.toml`.

## Building a release bundle

```bash
npm run tauri build
```

Note: `tauri.conf.json`'s `bundle.icon` is currently an empty array. Add real
icons (e.g. via `npm run tauri icon path/to/source.png`) before producing a
distributable bundle — the app builds and runs in dev without them, but a
release bundle typically expects app icons to be present.
