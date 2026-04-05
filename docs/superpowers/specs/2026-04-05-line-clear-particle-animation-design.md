# Line Clear Particle Animation — Design Spec

**Date:** 2026-04-05

## Overview

Add a particle animation during the `LineClearDelay` phase. When lines are cleared, the cells in those rows remain visible on the board and are launched as particles — flying outward from the board center and falling off the bottom of the screen under gravity. Board compaction is deferred to the end of the delay.

## Game State Changes (`game.rs`)

### New field on `Game`

```rust
pub rows_pending_compaction: Vec<usize>,
```

This field holds the row indices of fully-cleared lines. It is populated at lock time and cleared after board compaction at the end of `LineClearDelay`. It is always empty outside of the `LineClearDelay` phase.

### Split `clear_lines` responsibilities

**At lock time** (`lock_piece` → `clear_lines`):
- Identify all full rows (unchanged logic).
- Update `lines` and `level` immediately (unchanged — keeps scoring snappy and simplifies future back-to-back detection).
- Populate `rows_pending_compaction` with the cleared row indices.
- **Do not compact the board.** The cleared rows remain in `self.board` with their cell data intact.

**At end of `LineClearDelay`** (when `ticks_left` hits 0 in the phase tick handler):
- Compact the board: remove rows listed in `rows_pending_compaction`, prepend empty rows at the top (existing compaction logic, just moved here).
- Clear `rows_pending_compaction`.
- Transition to `Spawning { ticks_left: SPAWN_DELAY_NORMAL }` as today.

### Edge cases

- **`game_won`**: Set inside `clear_lines` at lock time as today (unchanged). Compaction still defers normally.
- **`game_over`**: Detected in `spawn_piece`, which runs after compaction — unaffected.
- **No lines cleared**: `rows_pending_compaction` stays empty, no `LineClearDelay` phase entered — unchanged path.

## Renderer Changes (`renderer.rs`)

### Particle rendering during `LineClearDelay`

In `render_board`, detect when `game.piece_phase` is `LineClearDelay { ticks_left }`:

1. Draw all rows **not** in `rows_pending_compaction` normally (existing bordered-cell logic).
2. For each cell in the pending rows that has `Some(kind)`, compute its particle screen position and draw it with `draw_cell` (no border).

### Particle position formula

```
t = (LINE_CLEAR_DELAY - ticks_left) as f32   // frames elapsed since delay began

initial_x = BOARD_X + col as f32 * CELL
initial_y = BOARD_Y + row as f32 * CELL

dist = col as f32 - (BOARD_COLS as f32 - 1.0) / 2.0   // signed distance from center; range [-4.5, +4.5]

x = initial_x + dist * PARTICLE_VX_SCALE * t
y = initial_y + PARTICLE_VY_INITIAL * t + 0.5 * PARTICLE_GRAVITY * t²
```

- Cells left of center (negative `dist`) move left; cells right of center move right.
- `PARTICLE_VY_INITIAL` gives a small initial downward nudge.
- `PARTICLE_GRAVITY` accelerates cells downward each frame.
- Cells whose computed `(x, y)` falls outside the screen bounds are skipped (not drawn).

No alpha or color changes are applied to particles.

### New tuning constants (`constants.rs`)

| Constant | Approximate value | Meaning |
|---|---|---|
| `PARTICLE_VX_SCALE` | `0.3` px/frame per col-distance | Horizontal speed multiplier |
| `PARTICLE_VY_INITIAL` | `1.0` px/frame | Initial downward velocity |
| `PARTICLE_GRAVITY` | `0.4` px/frame² | Downward acceleration |

Exact values are tuning targets — adjust during implementation so all cleared cells exit the bottom of the screen within the 41-frame window. Note: with a board height of ~660px and 41 frames, the gravity value will likely need to be higher than 0.4 to get top-row cells off screen in time.

## Testing

The existing line-clear tests in `tests.rs` assert final board state after clearing. These must be updated to account for deferred compaction: tests that call into the line-clear path will need to advance through `LineClearDelay` (or directly trigger end-of-delay compaction) before asserting final board layout.

No new tests are required for the renderer — particle math is purely visual.