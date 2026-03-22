# Level, Gravity & Timer — Design Spec

## Overview

Migrate fetris from a fixed-gravity, lines-based level system to TGM1-accurate level progression, fractional G/256 gravity, a win condition at level 999, and a live playtime display.

**In scope:** TGM1 level counter, section stops, gravity table (G/256 accumulator), win state, playtime timer (MM:SS.sss), constant tuning (LOCK_DELAY, SPAWN_DELAY_NORMAL, SPAWN_DELAY_LINE_CLEAR, DAS_REPEAT).

**Out of scope:** Scoring, TGM2/TAP modes, configurable mode system (noted as future direction).

---

## Section 1 — Level System (`game.rs`)

Replace `level = 1 + lines / 10` with a TGM1-style counter. `level` starts at 0 in `Game::new()`.

### Counter rules

- Starts at 0, ends at 999.
- **Piece placement** increments level by 1 in `spawn_piece`, subject to section stops. The increment happens before gravity is applied to the new piece, so the post-increment level governs gravity from the first tick onward.
- **Line clears** increment level by the number of lines cleared in `clear_lines`, not subject to section stops. Level is clamped to 999 after adding cleared lines.

### Section stops

Piece placement cannot push the level past a hundreds boundary or past 998:

```rust
fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
```

Line clears always advance freely. If a line clear brings the level to 999 (after clamping), `game_won` is set.

### Existing fields

`lines: u32` is kept and displayed in the sidebar. The `level` field type stays `u32`.

---

## Section 2 — Win State & Timer (`game.rs`, `renderer.rs`)

### New `Game` fields

```rust
pub game_won: bool,      // initialised to false
pub ticks_elapsed: u64,  // initialised to 0
```

### Tick behaviour

`tick()` returns early when `game_won` (same pattern as `game_over`). `ticks_elapsed` increments every tick, but only when neither `game_over` nor `game_won` — it freezes at the moment of victory.

### Timer display

`ticks_elapsed` is converted to `MM:SS.sss` for display (at 60 Hz):

```
seconds = ticks_elapsed / 60
ms      = (ticks_elapsed % 60) * 1000 / 60
MM = seconds / 60
SS = seconds % 60
format: "{:02}:{:02}.{:03}", MM, SS, ms
```

The timer is added to the Stats section of the sidebar, displayed on every frame including game over. Example layout:

```
┌Stats────────┐
│             │
│ Level: 142  │
│ Lines: 87   │
│ 01:23.456   │
│             │
│ ←→  move    │
│ ...         │
└─────────────┘
```

### Victory screen

When `game_won` is true, `render_board` replaces the board widget content with a victory message:

```
┌fetris───────────────┐
│                     │
│   LEVEL 999         │
│                     │
│   Time:             │
│   MM:SS.sss         │
│                     │
└─────────────────────┘
```

The sidebar continues to render normally (showing the final level, lines, and time).

---

## Section 3 — Gravity (`constants.rs`, `game.rs`)

### Accumulator

Replace `gravity_counter: u32` with `gravity_accumulator: u32` (units: G/256 per tick). Reset to 0 on piece spawn. Also reset to 0 on soft drop (matching the existing `gravity_counter = 0` behaviour).

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
/// Source: TGM1 wiki. Notable: gravity resets to 4 at level 200 before ramping
/// rapidly to 20G at level 500.
pub const GRAVITY_TABLE: &[(u32, u32)] = &[
    (0,   4),
    (30,  6),
    (35,  8),
    (40,  10),
    (50,  12),
    (60,  16),
    (70,  32),
    (80,  48),
    (90,  64),
    (100, 80),
    (120, 96),
    (140, 112),
    (160, 128),
    (170, 144),
    (200, 4),    // resets at section 2
    (220, 32),
    (230, 64),
    (233, 96),
    (236, 128),
    (239, 160),
    (243, 192),
    (247, 224),
    (251, 256),  // 1G
    (300, 512),  // 2G
    (330, 768),  // 3G
    (360, 1024), // 4G
    (400, 1280), // 5G
    (420, 1024), // 4G — intentional ease before 20G
    (450, 768),  // 3G — intentional ease before 20G
    (500, 5120), // 20G
];

pub fn gravity_g(level: u32) -> u32 {
    GRAVITY_TABLE.iter()
        .rev()
        .find(|(threshold, _)| level >= *threshold)
        .map(|(_, g)| *g)
        .unwrap_or(4)
}
```

---

## Section 4 — Constant Tuning (`constants.rs`)

TGM1 uses different ARE depending on whether a line was cleared. `SPAWN_DELAY` splits into two constants:

| Constant | Old | New | Reason |
|---|---|---|---|
| `LOCK_DELAY` | 30 | 29 | N+1 countdown → 30 actual frames (TGM1) |
| `SPAWN_DELAY` | 30 | *(removed)* | Split into two (see below) |
| `SPAWN_DELAY_NORMAL` | — | 29 | N+1 → 30 frames: ARE with no line clear |
| `SPAWN_DELAY_LINE_CLEAR` | — | 40 | N+1 → 41 frames: ARE after a line clear |
| `DAS_REPEAT` | 6 | 1 | TGM1: auto-shift fires every frame once charged |
| `DAS_CHARGE` | 16 | 16 | Already matches TGM1 |
| `GRAVITY_DELAY` | 30 | *(removed)* | Replaced by gravity table |

`clear_lines` is changed to return `u32` (the number of lines cleared). `lock_piece` uses this count to select the ARE: if `count > 0`, use `SPAWN_DELAY_LINE_CLEAR`; otherwise use `SPAWN_DELAY_NORMAL`.

**Future:** these constants and the gravity table are candidates for a per-mode configuration struct (e.g., `TgmMode`, `Tap2Mode`) when additional game modes are added.

---

## Testing

- Unit tests for `can_piece_increment` covering 99, 199, 998, and non-stop levels.
- Unit test for `gravity_g` covering representative levels: 0, 29, 30, 199, 200, 251, 500.
- Tick-based test: level increments on piece spawn.
- Tick-based test: level increments on line clear.
- Tick-based test: section stop — piece placement at level 99 does not advance to 100; subsequent line clear does.
- Tick-based test: level clamped to 999 on multi-line clear past 999.
- Tick-based test: win — line clear at level 998 sets `game_won`.
- Tick-based test: `ticks_elapsed` does not increment after `game_won`.
- Tick-based test: normal ARE uses `SPAWN_DELAY_NORMAL`; post-line-clear ARE uses `SPAWN_DELAY_LINE_CLEAR`.
- Snapshot tests for the victory screen renderer.
- Existing lock/DAS/spawn tests updated to account for new constant values.
