# Ready Screen Design

**Date:** 2026-04-21

## Overview

Add a 1.5-second (90-tick) delay between pressing START on the title screen and the game logic beginning. During this delay the game board is rendered normally but no ticks run, and "READY" is displayed centered over the board.

## State Machine Change (`main.rs`)

Add a new `AppState::Ready { game: Box<Game>, ticks_left: u32 }` variant.

- On `MenuResult::StartGame`, transition to `Ready` with `ticks_left: 90` instead of directly to `Playing`.
- Reset `accumulator` and `pending_just_pressed` on entry, same as the existing `Playing` transition.
- Each frame in `Ready`: advance the accumulator, decrement `ticks_left` by the number of elapsed ticks (capped at remaining), render via `render_ready`, and transition to `Playing` when `ticks_left` reaches zero.
- Escape exits the app. All other input is ignored during `Ready`.

## Renderer Change (`renderer.rs`)

Add `render_ready(&self, game: &Game)`:

- Calls the existing `render(game)` to draw the full game board and sidebar.
- Overlays "READY" centered on the board using `draw_centered_x`, matching the font size and style of the existing game-over overlay (28px, WHITE).
