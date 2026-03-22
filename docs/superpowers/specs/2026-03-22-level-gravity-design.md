# Level, Gravity & Timer — Design Spec

## Overview

Migrate fetris from a fixed-gravity, lines-based level system to TGM1-accurate level progression, fractional G/256 gravity, a win condition at level 999, and a live playtime display.

**In scope:** TGM1 level counter, section stops, gravity table (G/256 accumulator), win state, playtime timer (MM:SS.sss), constant tuning (LOCK_DELAY, SPAWN_DELAY, DAS_REPEAT).

**Out of scope:** Scoring, TGM2/TAP modes, configurable mode system (noted as future direction).

---

## Section 1 — Level System (`game.rs`)

Replace `level = 1 + lines / 10` with a TGM1-style counter.

### Counter rules

- Starts at 0, ends at 999.
- **Piece placement** increments level by 1 in `spawn_piece`, subject to section stops.
- **Line clears** increment level by the number of lines cleared in `clear_lines`, not subject to section stops.

### Section stops

Piece placement cannot push the level past a hundreds boundary or past 998:

```rust
fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
```

Line clears always advance freely. If a line clear brings the level to 999, `game_won` is set.

### Existing fields

`lines: u32` is kept and displayed in the sidebar. The `level` field type stays `u32`.

---

## Section 2 — Win State & Timer (`game.rs`, `renderer.rs`)

### New `Game` fields

```rust
pub game_won: bool,
pub ticks_elapsed: u64,
```

### Tick behaviour

`tick()` returns early when `game_won` (same pattern as `game_over`). `ticks_elapsed` increments every tick, but only when neither `game_over` nor `game_won` — it freezes at the moment of victory.

### Display

`ticks_elapsed` is converted to `MM:SS.sss` for display (at 60 Hz):

```
seconds = ticks_elapsed / 60
ms      = (ticks_elapsed % 60) * 1000 / 60
MM = seconds / 60
SS = seconds % 60
```

The timer is shown in the sidebar throughout gameplay, including on game over. On win, the renderer replaces the board area with a victory screen showing the final time.

---

## Section 3 — Gravity (`constants.rs`, `game.rs`)

### Accumulator

Replace `gravity_counter: u32` with `gravity_accumulator: u32` (units: G/256 per tick). Reset to 0 on piece spawn.

Each tick (only when `piece_phase != Spawning`), after DAS and soft/sonic drop, before lock-state transitions:

```rust
gravity_accumulator += gravity_g(self.level);
let drops = gravity_accumulator / 256;
gravity_accumulator %= 256;
for _ in 0..drops {
    if !self.try_move(0, 1) { break; }
}
```

`gravity_accumulator` always drains fully (divrem, not subtract-in-loop). The early `break` stops wasting move attempts once the piece is on the floor; Phase 7 (lock state transitions) then handles `Falling → Locking` as before.

### Gravity table (`constants.rs`)

`GRAVITY_DELAY` is removed. In its place, a `const` breakpoint table and a lookup function:

```rust
/// (min_level, G_value) pairs in ascending order. G is in units of G/256 per tick.
/// Values sourced from the TGM1 wiki.
const GRAVITY_TABLE: &[(u32, u32)] = &[
    (0,   4),
    // ... (exact values filled in during implementation from TGM wiki)
];

pub fn gravity_g(level: u32) -> u32 {
    GRAVITY_TABLE.iter()
        .rev()
        .find(|(threshold, _)| level >= *threshold)
        .map(|(_, g)| *g)
        .unwrap_or(4)
}
```

The table is a linear scan from the end; the first entry whose threshold is ≤ the current level wins.

---

## Section 4 — Constant Tuning (`constants.rs`)

| Constant | Old | New | Reason |
|---|---|---|---|
| `LOCK_DELAY` | 30 | 29 | N+1 countdown → 30 actual frames (TGM1) |
| `SPAWN_DELAY` | 30 | 29 | N+1 countdown → 30 actual frames (TGM1 ARE) |
| `DAS_REPEAT` | 6 | 1 | TGM1: auto-shift fires every frame once charged |
| `DAS_CHARGE` | 16 | 16 | Already matches TGM1 |
| `GRAVITY_DELAY` | 30 | *(removed)* | Replaced by gravity table |

**Future:** these constants and the gravity table are candidates for a per-mode configuration struct (e.g., `TgmMode`, `Tap2Mode`) when additional game modes are added.

---

## Testing

- Unit tests for `can_piece_increment` covering 99, 199, 998, and non-stop levels.
- Unit test for `gravity_g` covering a sample of level thresholds.
- Tick-based tests for level increment on piece spawn and line clear.
- Tick-based test for section stop: piece placement at level 99 does not advance to 100.
- Tick-based test for win: line clear at level 998 sets `game_won`.
- Tick-based test for timer freeze: `ticks_elapsed` does not increment after `game_won`.
- Snapshot tests for the victory screen renderer.
- Existing lock/DAS/spawn tests updated to account for new constant values.
