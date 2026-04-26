# CLAUDE.md — fetris

Reimplementation of [TGM1](https://tetris.wiki/Tetris_The_Grand_Master) in Rust using [Bevy](https://bevyengine.org/). Builds for native and WASM (deployed to GitHub Pages on push to `master`).

## Source layout

| File | Purpose |
|---|---|
| `src/main.rs` | App setup, plugin registration, `States` declaration, top-level systems |
| `src/data.rs` | Pure data types: `PieceKind`, `BoardGrid`, `PiecePhase`, `GameKey`, `Kind`, `GameMode`, `Grade`, `GameEvent`, `JudgeEvent`, `HiScoreEntry`, `GameConfig`, `MenuScreen` |
| `src/components.rs` | ECS components on the active-piece entity: `ActivePiece`, `PieceKindComp`, `PiecePosition`, `PieceRotation` |
| `src/resources.rs` | Resources holding game state: `Board`, `CurrentPhase`, `NextPiece`, `GameProgress`, `DasState`, `RotationBuffer`, `PendingCompaction`, `DropTracking`, `InputState`, `RotationSystemRes`, `GameModeRes`, `RotationKind` |
| `src/constants.rs` | Tuning constants: gravity table, delays, particle/animation timings |
| `src/rotation_system.rs` | `RotationSystem` trait + `Ars` and `Srs` impls; stored as `RotationSystemRes(Box<dyn RotationSystem>)` |
| `src/randomizer.rs` | TGM history-based piece bag (Resource) |
| `src/judge.rs` | TGM scoring; `Judge` is a Resource; consumes `JudgeEvent`s, emits `GameEvent::GradeAdvanced` |
| `src/hiscores.rs` | Per-(mode, rotation) hi-score persistence backed by `bevy_pkv` |
| `src/audio.rs` | `bevy_audio` event-driven sound system; `AudioHandles` resource; mute via `PkvStore` |
| `src/systems/` | Game-logic systems running in `FixedUpdate` at 60 Hz: `input`, `gravity`, `lock`, `line_clear`, `spawn`, `judge`, `game_over_check`, `global_input`, `post_game` |
| `src/render/` | Rendering systems running in `Update`: `board`, `piece`, `particles`, `overlays`, `hud`, `assets` |
| `src/menu/` | bevy_egui menu screens: `main_screen`, `hi_scores`, `controls`, `state` |
| `src/tests/` | Headless tests using `MinimalPlugins` + `GameSnapshot::from_world`; `insta` inline snapshots only |

## Architecture

**Bevy `App`** with `DefaultPlugins`, `bevy_egui::EguiPlugin`, and game-logic plugins. Game state lives in resources (`Board`, `Judge`, `Randomizer`, `InputState`, `GameProgress`, `Box<dyn RotationSystem>`) plus an active-piece *entity* with `PieceKindComp` / `PiecePosition` / `PieceRotation` / `PiecePhase` components.

**`AppState` machine** uses bevy `States`: `Menu` → `Ready` → `Playing` → `GameOver` → `Menu`. Systems are gated with `run_if(in_state(...))`.

**Schedules:**
- `FixedUpdate` at 60 Hz — all game logic.
- `Update` — rendering, input sampling, particle motion, menu UI, audio.

**Tick model:** `Time::<Fixed>::from_hz(60.0)` keeps game logic decoupled from frame rate; bevy runs `FixedUpdate` zero or more times per frame to catch up.

**Piece phases** (`PiecePhase` component): `Falling`, `Locking { ticks_left }`, `LineClearDelay { ticks_left }`, `Spawning { ticks_left }`. Phase transitions drive timing logic.

**IRS (Initial Rotation System)**: holding rotation keys during the previous piece's spawn delay (or pre-game Ready countdown) causes the next piece to spawn pre-rotated. Folded into the `spawn` system.

**Gravity**: fractional G/256 system — gravity accumulates per tick from `MASTER_GRAVITY_TABLE` in `constants.rs` (TGM1 values).

**Game / Renderer separation**: render systems read snapshot data from the `Board` resource and active-piece entity. They never write back. Particles are entities with `Particle` + `Sprite` + `Transform`; spawned by an `EventReader<GameEvent>` system handling `GameEvent::LineClear`, ticked in `FixedUpdate`.

**Rotation systems**: `RotationSystem` trait (`Send + Sync`) with `Ars` and `Srs` impls, stored as `Resource<Box<dyn RotationSystem>>`. Hi-scores tracked separately per rotation system.

**Scoring**: `Judge` resource consumes `JudgeEvent`s emitted by game-logic systems and tracks score, combo, and best `Grade` reached.

**Hi-scores & config**: stored via `bevy_pkv::PkvStore` (sled native, localStorage on WASM). Per-(GameMode, Kind) slot, top 5 by grade. Storage keys preserved from the macroquad version for backward compatibility with existing user data.

**Audio**: `bevy_audio` (built-in) plays sounds in response to `GameEvent`s (`PieceBeganLocking`, `LineClear`, `GradeAdvanced`, `GameEnded`) and on `OnEnter(AppState::Ready)`. Mute state persists via `PkvStore`. Direct bevy_audio calls in `src/audio.rs`; no `AudioPlayer` trait.

## WASM target

Build via trunk:

```sh
trunk build --release
```

Output is in `dist/` — `index.html`, `fetris-<hash>.js`, `fetris-<hash>.wasm`, `assets/`. The `wasm-release` cargo profile (in `Cargo.toml`) compiles with `opt-level = "z"`, `lto = true`, `codegen-units = 1`. Trunk runs `wasm-opt -Oz` post-build via the `data-wasm-opt` attribute in `index.html`.

For local iteration: `trunk serve --release` builds and serves at `http://127.0.0.1:8080`.

`.github/workflows/deploy.yml` installs trunk + wasm-bindgen-cli + binaryen, runs `trunk build --release`, deploys `dist/` to GitHub Pages on every push to `master`.

**Gotchas**:
- `wasm-bindgen-cli` must match the `wasm-bindgen` crate version in `Cargo.lock` exactly. Mismatches cause runtime errors. The CI workflow auto-detects and installs the right version.
- `bevy_pkv` on WASM uses `localStorage`; data is per-origin and survives page reloads but not domain changes.
- Bevy + wasm-opt-z produces a ~10MB binary. Acceptable trade-off for small fetris.

## Build & test

```sh
cargo build
cargo test
cargo run --release
trunk build --release       # WASM build
trunk serve --release       # WASM dev server at localhost:8080
```

## Conventions

- Install the pre-commit hook: `cp hooks/pre-commit .git/hooks/pre-commit` (runs `cargo fmt`)
- Tests use `insta` for snapshot assertions — always inline (`@"..."`), never external `.snap` files. To accept new snapshots: `cargo insta accept` (never edit them by hand).
- New feature work goes on a branch under `.worktrees/` (git-ignored)
- Specs live in `docs/superpowers/specs/`, implementation plans in `docs/superpowers/plans/`
- Bevy and bevy_egui versions are pinned in `Cargo.toml`. Don't bump them as part of unrelated changes — bevy ecosystem versioning churns and a casual upgrade can break a lot of code.

## Maintaining this file

This file holds *non-obvious-from-code* facts and *cross-file invariants*. Update it in the same change that invalidates a fact here.

**Update when changing**:
- The source-layout file map (adding/removing/renaming files)
- The `AppState` machine, piece phases, or tick model
- A trait shape (`RotationSystem`)
- Bevy plugin set, new resources/components added at the app-wide level
- Storage backend (e.g., switching away from bevy_pkv)
- Build, test, or deploy commands, or adding a new target
- Project-wide conventions

**Do NOT put here** (and so do not maintain here):
- Specific numbers that live in code: gravity table values, grade thresholds, frame counts, scoring formulas
- Struct field lists, function or test-helper signatures
- Anything an agent would answer correctly by reading one named file

**Self-check**: before claiming a task done, ask "does any heading in CLAUDE.md now lie?" If yes, fix it in the same change.
