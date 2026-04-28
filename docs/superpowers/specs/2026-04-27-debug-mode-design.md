# Debug Mode Design

**Date:** 2026-04-27

## Overview

A "DEBUG" entry on the main menu opens a dedicated visual test bench for inspecting rendering and effects without playing the game. The bench reuses the live render pipeline driven by synthetic inputs (events + resource mutations), so anything visible here is exactly what shows up in-game. Effects are trigger-on-demand via a fixed keymap displayed on screen.

## Scope

In scope:

- A single static T-piece sitting in a fixed position on the board.
- Line-clear particle bursts: single, double, triple, tetris.
- Overlays: READY, GO, GRADE UP flash, GAME OVER, EXCELLENT, NEW RECORD.
- HUD: next-piece preview, score, level, grade, section/grade bar — cyclable through representative values.

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

## Menu Entry (`src/menu/main_screen.rs`, `src/data.rs`)

- Add `MenuScreen::Debug` to the `MenuScreen` enum in `src/data.rs`.
- Insert a "DEBUG" row on the main menu between "CONTROLS" and "START". Cursor max becomes 5 (was 4).
- Confirm on the DEBUG row sets `next_state` to `AppState::Debug`.

## Debug Screen (`src/menu/debug.rs`, new file)

A new module `src/menu/debug.rs` registered in `src/menu/mod.rs`. It contains:

- `DebugSceneState` resource: holds the synthetic state needed by the HUD demo cycle (current `Grade`, score, level, section), plus a small cursor for which preset is shown.
- `OnEnter(AppState::Debug)` system: spawn the active-piece entity with a `T` piece at a fixed board position; populate `Board` with an empty grid; insert `DebugSceneState::default()`; set `Judge` / `GameProgress` / `NextPiece` to the first preset values.
- `OnExit(AppState::Debug)` system: despawn the active-piece entity and any leftover particles/overlays; remove `DebugSceneState`.
- `debug_input_system` running in `Update` with `run_if(in_state(AppState::Debug))`. Reads `ButtonInput<KeyCode>` and dispatches:
  - `Digit1` / `Digit2` / `Digit3` / `Digit4`: emit `GameEvent::LineClear { count }` with `count` of 1/2/3/4 so the existing particle system spawns the corresponding burst. The debug screen does NOT populate synthetic rows on the board — the particle spawn system is event-driven and reads `count`, not board state, so re-triggering just works.
  - `Q`: trigger READY overlay.
  - `W`: trigger GO overlay.
  - `E`: emit `GameEvent::GradeAdvanced(grade)` (the grade-up flash is driven by this event).
  - `R`: trigger GAME OVER overlay.
  - `T`: trigger EXCELLENT overlay.
  - `Y`: trigger NEW RECORD overlay.
  - `ArrowUp` / `ArrowDown`: cycle the HUD preset (mutates `Judge` and `GameProgress`).
  - `Backspace`: `next_state.set(AppState::Menu)` and reset `MenuState::screen` to `Main`.
- An egui side panel listing the keymap so the user does not need to memorize it.

## Triggering overlays

The existing overlay code already has state it reads (e.g. game-over flag, ready countdown). Two paths depending on how each overlay is currently driven:

- If the overlay is event-driven, the debug system writes the same event the real game writes.
- If the overlay reads a resource flag (e.g. `IsGameOver(bool)`), the debug system flips that flag, then unflips it after a fixed display duration (tracked by a small per-overlay timer in `DebugSceneState`).

The plan step that wires each overlay will inspect the existing trigger path and choose the right one — no new overlay rendering code is added.

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
