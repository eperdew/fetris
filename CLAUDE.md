# CLAUDE.md — fetris

Reimplementation of [TGM1](https://tetris.wiki/Tetris_The_Grand_Master) in Rust using [macroquad](https://macroquad.rs/). Builds for native and `wasm32-unknown-unknown` (deployed to GitHub Pages on push to `master`).

## Source layout

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point, `AppState` machine, input mapping, frame loop |
| `src/types.rs` | Shared data types: `Board`, `Piece`, `PiecePhase`, `GameKey`, `InputState`, `Kind`, `GameMode`, `Grade`, `GameSnapshot`, `GameEvent`, `JudgeEvent`, `Menu*` types |
| `src/game.rs` | Core game logic: piece phases, gravity, locking, line clearing, randomizer, IRS |
| `src/rotation_system.rs` | `RotationSystem` trait + `Ars` and `Srs` impls (selectable from menu) |
| `src/judge.rs` | TGM scoring + best-grade tracking; consumes `JudgeEvent`s from `Game` |
| `src/menu.rs` | Menu state machine: main / hi-scores / controls screens |
| `src/hiscores.rs` | Per-(mode, rotation) hi-score persistence via `Storage` |
| `src/storage.rs` | Key-value storage abstraction; native (file-backed) and WASM (`localStorage` via JS bindings) |
| `src/audio_player.rs` | `AudioPlayer` trait + `macroquad::Macroquad` and (test-only) `null::Null` impls |
| `src/renderer.rs` | All rendering: menu, board, particles, line-clear overlays, shaders |
| `src/constants.rs` | Tuning constants: gravity table, delays, particle/animation timings |
| `src/tests.rs` | All tests (insta inline snapshots only) |

## Architecture

**AppState machine** (in `main.rs`): `Menu(Menu)` → `Ready { game, ticks_left }` → `Playing(Game)`. Escape exits from any state. From the post-game-over screen, Space returns to the menu.

**Tick model**: game logic runs at a fixed 60 ticks/second, decoupled from render rate. The main loop accumulates frame time and calls `game.tick(...)` zero or more times per frame. All game-state delays (gravity, lock, line clear, ARE) are measured in ticks. `Renderer` animation timings are also tick-driven.

**Piece phases** (`PiecePhase` in `types.rs`): the active piece is always in one of `Falling`, `Locking { ticks_left }`, `LineClearDelay { ticks_left }`, or `Spawning { ticks_left }`. Phase transitions drive all timing logic.

**IRS (Initial Rotation System)**: holding a rotation key during the previous piece's spawn delay (or the pre-game Ready countdown) causes the next piece to spawn pre-rotated. Applied in `Game::apply_irs()`.

**Gravity**: fractional G/256 system — gravity accumulates per tick from `GRAVITY_TABLE` in `constants.rs` (TGM1 values).

**Game / Renderer separation**: `Game` produces a `GameSnapshot` (immutable view) plus a drained event stream each tick. `Renderer` owns visual-only animation state (particles, line-clear overlays) driven by those events. Visual state never feeds back into game logic.

**Rotation systems**: `RotationSystem` trait with `Ars` and `Srs` impls. `Kind::create() -> Box<dyn RotationSystem>` is called when a game starts. Hi-scores are tracked separately per rotation system.

**Scoring**: `Judge` consumes `JudgeEvent`s from `Game` and tracks score, combo, and best `Grade` reached. The TGM scoring formula lives in `judge.rs::on_event`.

**Hi-scores**: stored per `(GameMode, Kind)` combination via `Storage`, top 5 by grade (ties broken by lower tick count).

**Storage**: key-value abstraction with two cfg-split implementations — native uses a JSON file (`local.data`), WASM calls extern `storage_get` / `storage_set` backed by `localStorage` (see `web/fetris-storage.js`).

**Audio**: `AudioPlayer` trait wraps macroquad's audio. `Game` holds `Arc<dyn AudioPlayer>` and triggers sounds on state transitions. Tests use `audio_player::null::Null`.

## WASM target

Build:
```sh
cargo build --target wasm32-unknown-unknown --release
```

Output is `target/wasm32-unknown-unknown/release/fetris.wasm`. The web shell lives in `web/`: `index.html` loads `mq_js_bundle.js` (macroquad's runtime), then `fetris-storage.js` (registers the `storage_get` / `storage_set` extern functions backed by `localStorage`), then loads `fetris.wasm`.

`.github/workflows/deploy.yml` builds and deploys to GitHub Pages on every push to `master`.

**Gotchas**:
- Don't add `getrandom` or `wasm-bindgen` — macroquad's built-in `rand` works on all targets. See `.cargo/config.toml`.
- Don't add `console_error_panic_hook` — incompatible with macroquad.

## Build & test

```sh
cargo build
cargo test
cargo run --release
cargo build --target wasm32-unknown-unknown --release
```

## Conventions

- Install the pre-commit hook: `cp hooks/pre-commit .git/hooks/pre-commit` (runs `cargo fmt`)
- Tests use `insta` for snapshot assertions — always inline (`@"..."`), never external `.snap` files. To accept new snapshots: `cargo insta accept` (never edit them by hand).
- New feature work goes on a branch under `.worktrees/` (git-ignored)
- Specs live in `docs/superpowers/specs/`, implementation plans in `docs/superpowers/plans/`

## Maintaining this file

This file holds *non-obvious-from-code* facts and *cross-file invariants*. Update it in the same change that invalidates a fact here.

**Update when changing**:
- The source-layout file map (adding/removing/renaming files)
- The `AppState` machine, piece phases, or tick model
- A trait shape (`RotationSystem`, `AudioPlayer`, `Storage`)
- Build, test, or deploy commands, or adding a new target
- Project-wide conventions

**Do NOT put here** (and so do not maintain here):
- Specific numbers that live in code: gravity table values, grade thresholds, frame counts, scoring formulas
- Struct field lists, function or test-helper signatures
- Anything an agent would answer correctly by reading one named file

**Self-check**: before claiming a task done, ask "does any heading in CLAUDE.md now lie?" If yes, fix it in the same change.
