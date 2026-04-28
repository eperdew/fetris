# Debug Mode Design

**Date:** 2026-04-27

## Overview

A "DEBUG" entry on the main menu opens a dedicated visual test bench for inspecting rendering and effects without playing the game. The bench reuses the live render pipeline driven by synthetic inputs (events + resource mutations), so anything visible here is exactly what shows up in-game. Effects are trigger-on-demand via a fixed keymap displayed on screen.

## Scope

In scope (limited to overlays/effects that actually exist in the codebase today):

- A single static T-piece sitting in a fixed position on the board.
- Line-clear particle bursts: single, double, triple, tetris (4 lines). Particle spawn reads the cells of the cleared rows from `Board`, so the debug screen populates synthetic cells in those rows + sets `PendingCompaction.0 = vec![row_indices]` + emits `GameEvent::LineClear { count }`.
- Line-clear text overlays — DOUBLE / TRIPLE / FETRIS — driven by the same `GameEvent::LineClear { count }` for `count` of 2/3/4. (Single-line clears emit no overlay; this is the existing behavior.)
- State-text overlays: READY (used during the pre-game countdown) and GAME OVER. Plus the LEVEL 999 win-screen variant (which shows when `progress.game_won`).
- HUD: next-piece preview, score, level, grade, grade bar — cyclable through representative presets.

Not in scope (these don't exist in the current renderer; out for now):

- "GO", "GRADE UP" flash, "EXCELLENT", "NEW RECORD" — the spec previously listed these but the code has no such overlays. `GameEvent::GradeAdvanced` is currently audio-only.

Out of scope:

- Piece movement, gravity, IRS, lock, line-clear logic.
- Multiple piece kinds or rotations.
- ARS-vs-SRS comparison.
- Audio toggling (audio plays in response to the events we emit, which is a useful side benefit but not the focus).

## State Machine Change (`main.rs`, `app_state.rs`)

Add `AppState::Debug` as a peer to `Playing` / `Ready` / `GameOver`.

- Render systems currently gated by `run_if(in_state(AppState::Playing))` (and the equivalent for `Ready` / `GameOver`) become gated by `run_if(in_state(...).or_else(in_state(AppState::Debug)))` for those that should run on the debug screen: `board`, `piece`, `particles`, `overlays`, `hud`, asset loading.
- Game-logic systems (input, gravity, lock, line_clear, spawn, judge, game_over_check) do NOT run in `AppState::Debug`. Only render systems and the debug menu system do.
- Entry: from `MenuScreen::Debug` confirm, transition to `AppState::Debug`.
- Exit: Backspace returns to `AppState::Menu` and `MenuScreen::Main`.

## Menu Entry (`src/menu/main_screen.rs`, `src/menu/state.rs`, `src/data.rs`)

- Add `MenuScreen::Debug` to the `MenuScreen` enum in `src/data.rs`.
- Add `debug_unlocked: bool` field to `MenuState` in `src/menu/state.rs`, default `false`. Not persisted via `bevy_pkv` — resets on each app launch.
- In `main_menu_system`, when on `MenuScreen::Main` and `KeyCode::KeyD` is just-pressed, set `menu.debug_unlocked = true`.
- The "DEBUG" row only renders when `debug_unlocked` is true; insert it between "CONTROLS" and "START". Cursor max is 4 when locked, 5 when unlocked.
- Confirm on the DEBUG row sets `next_state` to `AppState::Debug`.
- No toggle-off: once revealed for the session, it stays. Closing and re-opening the app re-locks it.

## Debug Screen (`src/menu/debug.rs`, new file)

A new module `src/menu/debug.rs` registered in `src/menu/mod.rs`. It contains:

- `DebugSceneState` resource: holds the synthetic state needed by the HUD demo cycle (current `Grade`, score, level, section), plus a small cursor for which preset is shown.
- `OnEnter(AppState::Debug)` system: insert the resources that gameplay normally inserts via `start_game` but that the renderer needs (`RotationSystemRes`, `NextPiece`); spawn the active-piece entity at a fixed mid-board position with `PieceKind::T`; set `CurrentPhase(PiecePhase::Falling)` so `render_active_piece` doesn't early-return; insert `DebugSceneState::default()`; apply the first HUD preset to `Judge` / `GameProgress`.
- `OnExit(AppState::Debug)` system: despawn the active-piece entity and any leftover particles / overlays / state-text / HUD / board / piece / next-preview entities (same set as `reset_game_on_enter_menu`); remove `DebugSceneState`. The existing `reset_game_on_enter_menu` runs on entry to `Menu` and already does this — we just need to make sure leaving Debug routes through `Menu`.
- `debug_input_system` running in `Update` with `run_if(in_state(AppState::Debug))`. Reads `ButtonInput<KeyCode>` and dispatches:
  - `Digit1` / `Digit2` / `Digit3` / `Digit4`: populate `Board.0` with synthetic T-colored cells in the bottom 1/2/3/4 rows, set `PendingCompaction.0` to those row indices, then emit `GameEvent::LineClear { count }`. Particles spawn (reading the synthetic cells), overlay text spawns for counts 2/3/4. After `OVERLAY_LIFETIME` ticks (~45) the debug screen wipes the synthetic rows + clears `PendingCompaction.0` so re-triggering works on a clean board.
  - `Q`: show "READY" state-text overlay (via the override flag described below) for ~90 ticks.
  - `W`: show "GAME OVER" state-text overlay for ~90 ticks.
  - `R`: show "LEVEL 999" win-screen state-text overlay for ~90 ticks.
  - `ArrowUp` / `ArrowDown`: cycle the HUD preset (mutates `Judge` and `GameProgress`).
  - `Backspace`: `next_state.set(AppState::Menu)` and reset `MenuState::screen` to `Main`.
- An egui side panel listing the keymap so the user does not need to memorize it.

## Triggering overlays

Two distinct paths in this codebase:

- **Line-clear overlays** (DOUBLE / TRIPLE / FETRIS) are event-driven by `GameEvent::LineClear { count }`, same path as the particles. Emit the event; the existing `spawn_line_clear_overlay` system handles it.
- **State-text overlays** (READY / GAME OVER / LEVEL 999) are AppState-driven. `render_state_text` in `src/render/overlays.rs` matches on `Res<State<AppState>>` and `progress.game_won`. To trigger them from the debug screen without leaving `AppState::Debug`, add a debug-only override:
  - New enum `DebugStateOverlay { None, Ready, GameOver, Won }` carried on `DebugSceneState`.
  - `render_state_text` is broadened to also run in `AppState::Debug` and, when in Debug, branches on `DebugStateOverlay` instead of the regular AppState match.
  - The debug input system sets the override on Q/W/R and clears it after a per-overlay tick countdown.

No new overlay rendering code is introduced — only a new branch in `render_state_text`.

## HUD presets

`DebugSceneState::hud_preset: usize` cycles through a small fixed list (e.g. 6 entries) covering:

- Empty start (Grade 9, level 0, section 0, score 0).
- Mid-grade (Grade 1, level 250, section 1, mid section bar).
- Promotional grades (S1 / S5 / S9).
- Master-class.

Each preset writes the corresponding values into `Judge` and `GameProgress` so the existing HUD render system draws them. ArrowUp/ArrowDown step the cursor.

## Files touched

- `src/data.rs` — add `MenuScreen::Debug`.
- `src/app_state.rs` — add `AppState::Debug`.
- `src/main.rs` — register the new state and adjust `run_if` gates on render systems.
- `src/menu/main_screen.rs` — DEBUG row + confirm handler.
- `src/menu/mod.rs` — register the new submodule.
- `src/menu/debug.rs` (new) — everything above.
- `src/render/*.rs` — broaden render-system `run_if` to also include `AppState::Debug`.

No changes to `src/systems/*` (game-logic systems stay strictly gameplay-only).

## Testing

A headless test under `src/tests/` that:

- Boots the app with `MinimalPlugins`, transitions through `Menu` → `Debug`.
- Asserts the active-piece entity is a T-piece at the expected position.
- Sends a `KeyCode::Digit4` press, ticks, and asserts particle entities exist.
- Sends `KeyCode::Backspace`, ticks, and asserts state is `Menu` and the active-piece entity is despawned.

`insta` inline snapshot of `GameSnapshot::from_world` after entry to confirm board / piece / HUD-preset state.
