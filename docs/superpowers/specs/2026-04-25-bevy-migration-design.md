# Bevy Migration Design

**Date:** 2026-04-25
**Status:** Approved for planning

## Motivation

Three drivers, in order:

1. **Trust in maintenance.** Macroquad has felt under-maintained — sharp corners (recent MSAA render-target panic, the wasm `console_error_panic_hook` incompatibility) and uncertainty about whether web deployment will keep working.
2. **Learn ECS.** Bevy's ECS is interesting in its own right and applicable to other projects.
3. **Ecosystem leverage.** Replace bespoke code (`storage.rs`) with maintained crates (`bevy_pkv`).

Multiplayer / doubles is a stated future goal but explicitly **not** designed for now (YAGNI). When it arrives, the board will move from a `Resource` to a `Component` on a player entity; that refactor is accepted as future work.

## Scope

**In scope:** Full rewrite of the engine layer (rendering, input, audio, state machine, storage, web shell, deploy pipeline) on bevy. Game logic ported with equivalent behavior.

**Out of scope:**
- Multiplayer, doubles, networking, lobbies
- New gameplay features
- Test infrastructure improvements beyond porting
- Visual / gameplay tuning changes

## Architecture

Single bevy `App`. Standard plugins (with `bevy_egui` added; `AudioPlugin` keeps default `bevy_audio`).

**Schedules:**
- `FixedUpdate` at 60 Hz — all game logic. Matches the current tick model exactly.
- `Update` — rendering, input sampling, particle lifetime, audio event handling.

**`AppState` machine** uses bevy `States`:
- `Menu` → `Ready` → `Playing` → `GameOver` → `Menu`
- Systems gated with `run_if(in_state(...))`

**State storage:**
- `Resource<Board>` — playfield grid
- `Resource<Judge>` — score, combo, best grade
- `Resource<Box<dyn RotationSystem>>` — rotation kind selected at game start (trait gains `Send + Sync`)
- `Resource<Randomizer>` — TGM history-based piece bag
- `Resource<InputState>` — per-tick input snapshot (DAS, soft drop, IRS hold)
- `Resource<PkvStore>` — bevy_pkv handle (replaces `storage.rs`)

**Active piece is an entity** with components: `Position`, `Rotation`, `Kind`, `PiecePhase`, `GravityAccumulator`. On lock, the entity is despawned and cells are written to the board resource. On spawn, a new entity appears.

**Particles, line-clear overlays, popup text** are entities with a `Lifetime` component, despawned on expiry.

**`GameEvent` and `JudgeEvent` become bevy `Event`s** — produced by game systems via `EventWriter`, consumed by the judge / audio / particle-spawner systems via `EventReader`. Mirrors the current event-stream design.

## System Decomposition

`Game::tick` is split into seven systems running in `FixedUpdate`, each in its own file under `src/systems/`:

1. `input` — read `ButtonInput<KeyCode>` → write `Resource<InputState>`
2. `gravity` — runs in `Falling` phase; G/256 accumulation; piece move-down
3. `lock` — runs in `Locking` phase; decrements `ticks_left`; writes piece to board on expiry; emits `JudgeEvent::Lock`
4. `line_clear` — detects full lines; transitions to `LineClearDelay`; emits `JudgeEvent::LinesCleared`
5. `spawn` — runs in `Spawning` phase; decrements `ticks_left`; applies IRS; spawns next piece entity
6. `judge` — `EventReader<JudgeEvent>` → mutates `Resource<Judge>`
7. `game_over_check` — transitions `AppState`

System ordering enforced via `.chain()` or explicit `.before()` / `.after()` constraints where needed. Visual state never feeds back into game logic — invariant preserved from current architecture.

## Crates

- **bevy** — pin to current stable at start of work; do not bump during migration.
- **bevy_pkv** — replaces `storage.rs` entirely; native (sled) and WASM (localStorage).
- **bevy_egui** — for menu / hi-scores / controls screens. Chosen over `bevy_ui` for terseness on text-heavy menus.
- **bevy_audio** (built-in) — replaces macroquad audio. The `AudioPlayer` trait is **deleted**; tests omit the audio plugin from their headless `App`.
- `serde` / `serde_json` retained for hi-score serialization (bevy_pkv handles persistence; serde handles structure).
- `insta` retained for tests.

## Module Mapping

| Current | New |
|---|---|
| `main.rs` | `main.rs` — bevy `App` setup, plugin registration, `States` declaration |
| `types.rs` | `types.rs` — pure data types (`Kind`, `GameMode`, `Grade`); `Position`/`Rotation`/`PiecePhase` become `Component`s; `Board` becomes a `Resource`; `GameSnapshot` becomes a function over `&World` |
| `game.rs` | `systems/{input,gravity,lock,line_clear,spawn,judge,game_over_check}.rs` — one system per file (IRS folded into `spawn`) |
| `rotation_system.rs` | unchanged shape; trait gains `Send + Sync`; stored as `Resource<Box<dyn RotationSystem>>` |
| `judge.rs` | `Judge` becomes a `Resource`; `JudgeEvent` becomes a bevy `Event` |
| `menu.rs` | `menu.rs` — bevy_egui systems gated by `in_state(AppState::Menu)` |
| `hiscores.rs` | thin wrapper over `Res<PkvStore>` |
| `storage.rs` | **deleted** |
| `audio_player.rs` | **deleted** — direct bevy_audio calls in event-handler systems |
| `renderer.rs` | `render/{board,piece,particles,overlays,hud}.rs` — one concern per file |
| `constants.rs` | unchanged |
| `tests.rs` | reorganized: `GameSnapshot::from_world(&World)` reducer; tests build a headless `App`, tick it, snapshot |
| `web/index.html` | rewritten as a trunk template |
| `web/mq_js_bundle.js`, `web/fetris-storage.js` | **deleted** |
| `.github/workflows/deploy.yml` | install trunk + wasm-bindgen-cli + binaryen; `trunk build --release`; deploy `dist/` |

## WASM and Deploy

**Toolchain:** trunk replaces the macroquad direct-load shell. `wasm-bindgen` becomes a transitive dep of bevy. `wasm-opt` (binaryen) runs after trunk for size.

**Cargo profile:** add `[profile.wasm-release]` with `opt-level = "z"`, `lto = true`, `codegen-units = 1`.

**Expected size:** ~3 MB (current) → ~8–15 MB after wasm-opt. Accepted.

**Workflow:** install Rust + `wasm32-unknown-unknown` target, install trunk + wasm-bindgen-cli + binaryen, `trunk build --release`, deploy `dist/` to GitHub Pages.

**`.cargo/config.toml`:** the existing "no getrandom / no wasm-bindgen" workaround is **removed** — bevy requires both, properly.

**Audio gate:** browser autoplay policy still requires a user gesture before audio. The existing menu's first input is the gate; verify on first wasm playthrough.

## Tests

Snapshots themselves are preserved verbatim. The harness changes:

```rust
fn snapshot(app: &App) -> GameSnapshot { /* reduce World */ }

#[test]
fn example() {
    let mut app = headless_app();
    app.world_mut().resource_mut::<InputState>().press(GameKey::Left);
    app.update();
    insta::assert_yaml_snapshot!(snapshot(&app), @"...");
}
```

Headless `App` registers `MinimalPlugins` plus the game-logic systems and resources, but **not** the audio, render, or windowing plugins. `GameSnapshot::from_world` returns the same struct shape as today, so existing inline snapshot strings remain valid.

Further test-infrastructure improvements (better helpers, organization) are deferred as a follow-on after the rewrite.

## Migration Approach

**Big bang on a worktree.** Create `.worktrees/bevy-migration`. Rewrite there. Merge to `master` when green.

**Execution order:**

1. Scaffold empty bevy app — verify native + wasm builds.
2. Port pure logic (types, rotation systems, judge, randomizer, constants) — compiles standalone.
3. Port game systems headless; build `GameSnapshot::from_world`; port `tests.rs`. **All tests green is the gate to step 4.**
4. Renderer (board, piece, particles, overlays, hud).
5. Menu (bevy_egui).
6. Storage (bevy_pkv); hi-scores end-to-end.
7. WASM + deploy pipeline; verify Pages deploy.
8. Cleanup: delete dead files, update `CLAUDE.md`, commit.

## Risks

1. **Bevy version churn during the migration.** Mitigation: pin versions in step 1; do not bump until done.
2. **Test porting effort.** `tests.rs` is 2461 lines. Mitigation: get one snapshot test green first to prove the harness; then port en masse.
3. **WASM audio differences.** Mitigation: keep click-to-start gate; deploy early enough in the order to discover problems.
4. **bevy_egui wasm quirks.** Mitigation: fall back to `bevy_ui` if blocking. Not on the critical path.

## CLAUDE.md Updates Required

To be applied as part of step 8:

- Replace the source-layout file map with the new module structure.
- Replace the WASM gotchas section ("Don't add `getrandom`/`wasm-bindgen`/`console_error_panic_hook`") with bevy-specific gotchas discovered during migration.
- Update build/test commands (`trunk build` for wasm).
- Update the architecture section: bevy `App`, `States`, `FixedUpdate`, ECS-based piece entities, deleted `storage.rs` and `audio_player.rs`.
