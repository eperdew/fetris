# Line Clear Particle Animation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Animate cleared line cells as physics particles during `LineClearDelay`, deferring board compaction to the end of the delay.

**Architecture:** Add `rows_pending_compaction: Vec<usize>` to `Game`. Split `clear_lines` so scoring updates immediately but compaction is deferred to the end of `LineClearDelay`. The renderer reads this field to skip those rows in the normal draw pass and instead computes their screen positions using a stateless parabolic formula.

**Tech Stack:** Rust, macroquad

---

## File Map

| File | Changes |
|---|---|
| `src/game.rs` | Add `rows_pending_compaction` field; split `clear_lines` (no compact at lock); add `compact_pending_rows`; call it at end of `LineClearDelay` |
| `src/constants.rs` | Add `PARTICLE_VX_SCALE`, `PARTICLE_VY_INITIAL`, `PARTICLE_GRAVITY` |
| `src/renderer.rs` | Add `draw_cell_at` helper; skip pending rows in board loop; draw particles during `LineClearDelay` |
| `src/tests.rs` | Add tests for deferred compaction behavior |

---

### Task 1: Add `rows_pending_compaction` to `Game` and write failing tests

**Files:**
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write three failing tests in `src/tests.rs`**

Add these three tests anywhere after the existing `line_clear_delay_transitions_to_are` test (around line 1514):

```rust
#[test]
fn rows_pending_compaction_populated_during_delay() {
    let mut game = Game::new();
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    assert_eq!(
        game.rows_pending_compaction,
        vec![BOARD_ROWS - 1],
        "cleared row index should be in rows_pending_compaction during LineClearDelay"
    );
}

#[test]
fn board_not_compacted_during_delay() {
    let mut game = Game::new();
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    assert!(
        game.board[BOARD_ROWS - 1].iter().all(|c| c.is_some()),
        "cleared row should still be present in board during LineClearDelay"
    );
}

#[test]
fn board_compacted_and_pending_cleared_after_delay() {
    use crate::constants::LINE_CLEAR_DELAY;
    let mut game = Game::new();
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    idle(&mut game, LINE_CLEAR_DELAY + 1); // exhaust delay → compaction → Spawning
    assert!(
        game.rows_pending_compaction.is_empty(),
        "rows_pending_compaction should be empty after compaction"
    );
    assert!(
        game.board[BOARD_ROWS - 1].iter().all(|c| c.is_none()),
        "bottom row should be empty after compaction"
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/eperdew/Software/fetris && cargo test rows_pending_compaction 2>&1 | tail -20
cargo test board_not_compacted 2>&1 | tail -10
cargo test board_compacted_and_pending 2>&1 | tail -10
```

Expected: compile error — field `rows_pending_compaction` does not exist on `Game`.

- [ ] **Step 3: Add `rows_pending_compaction` field to `Game` struct in `src/game.rs`**

In the `Game` struct (around line 38), add after `rotation_buffer`:

```rust
pub rows_pending_compaction: Vec<usize>,
```

In `Game::new()` (around line 61), add after `rotation_buffer: None`:

```rust
rows_pending_compaction: Vec::new(),
```

- [ ] **Step 4: Run tests to confirm they still fail (logic not implemented yet)**

```bash
cd /Users/eperdew/Software/fetris && cargo test rows_pending_compaction 2>&1 | tail -20
```

Expected: FAIL with assertion errors (field exists but is never populated).

- [ ] **Step 5: Commit**

```bash
cd /Users/eperdew/Software/fetris
git add src/game.rs src/tests.rs
git commit -m "test: add failing tests for deferred line clear compaction"
```

---

### Task 2: Defer compaction — update `clear_lines` and add `compact_pending_rows`

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 1: Replace the `clear_lines` method body**

Find `clear_lines` (around line 385) and replace its entire body:

```rust
fn clear_lines(&mut self) -> u32 {
    let cleared: Vec<usize> = (0..BOARD_ROWS)
        .filter(|&r| self.board[r].iter().all(|c| c.is_some()))
        .collect();
    let count = cleared.len() as u32;
    if count == 0 {
        return 0;
    }
    self.rows_pending_compaction = cleared;
    self.lines += count;
    self.level = (self.level + count).min(999);
    if self.level == 999 {
        self.game_won = true;
    }
    count
}
```

- [ ] **Step 2: Add `compact_pending_rows` method after `clear_lines`**

```rust
fn compact_pending_rows(&mut self) {
    if self.rows_pending_compaction.is_empty() {
        return;
    }
    let mut new_board: Board = [[None; BOARD_COLS]; BOARD_ROWS];
    let kept: Vec<[Option<PieceKind>; BOARD_COLS]> = self
        .board
        .iter()
        .enumerate()
        .filter(|(r, _)| !self.rows_pending_compaction.contains(r))
        .map(|(_, row)| *row)
        .collect();
    let offset = BOARD_ROWS - kept.len();
    for (i, row) in kept.into_iter().enumerate() {
        new_board[offset + i] = row;
    }
    self.board = new_board;
    self.rows_pending_compaction.clear();
}
```

- [ ] **Step 3: Call `compact_pending_rows` at the end of `LineClearDelay`**

In the `LineClearDelay` tick handler (around line 90), change:

```rust
if *ticks_left == 0 {
    self.piece_phase = PiecePhase::Spawning {
        ticks_left: SPAWN_DELAY_NORMAL,
    };
}
```

to:

```rust
if *ticks_left == 0 {
    self.compact_pending_rows();
    self.piece_phase = PiecePhase::Spawning {
        ticks_left: SPAWN_DELAY_NORMAL,
    };
}
```

- [ ] **Step 4: Run all tests**

```bash
cd /Users/eperdew/Software/fetris && cargo test 2>&1 | tail -30
```

Expected: all tests pass, including the three new ones.

- [ ] **Step 5: Commit**

```bash
cd /Users/eperdew/Software/fetris
git add src/game.rs
git commit -m "feat: defer board compaction to end of LineClearDelay"
```

---

### Task 3: Add particle tuning constants

**Files:**
- Modify: `src/constants.rs`

- [ ] **Step 1: Append particle constants to `src/constants.rs`**

```rust
/// Horizontal velocity scale for line-clear particles, in pixels per frame per
/// column-distance from board center. Negative dist → moves left, positive → right.
pub const PARTICLE_VX_SCALE: f32 = 0.5;

/// Initial downward velocity of line-clear particles, in pixels per frame.
pub const PARTICLE_VY_INITIAL: f32 = 2.0;

/// Downward acceleration of line-clear particles, in pixels per frame².
pub const PARTICLE_GRAVITY: f32 = 0.8;
```

- [ ] **Step 2: Verify the crate still compiles**

```bash
cd /Users/eperdew/Software/fetris && cargo build 2>&1 | tail -10
```

Expected: compiles with no errors.

- [ ] **Step 3: Commit**

```bash
cd /Users/eperdew/Software/fetris
git add src/constants.rs
git commit -m "feat: add particle animation tuning constants"
```

---

### Task 4: Add particle rendering to the renderer

**Files:**
- Modify: `src/renderer.rs`

- [ ] **Step 1: Update the `use` import at the top of `src/renderer.rs`**

Change:

```rust
use crate::game::{BOARD_COLS, BOARD_ROWS, Game, PiecePhase};
```

to:

```rust
use crate::constants::{LINE_CLEAR_DELAY, PARTICLE_GRAVITY, PARTICLE_VX_SCALE, PARTICLE_VY_INITIAL};
use crate::game::{BOARD_COLS, BOARD_ROWS, Game, PiecePhase};
```

- [ ] **Step 2: Add a `draw_cell_at` helper and update `draw_cell` to use it**

Replace the existing `draw_cell` function (around line 70):

```rust
/// Draw a single CELL×CELL block at pixel position (x, y).
fn draw_cell_at(x: f32, y: f32, color: Color, texture: &Texture2D) {
    draw_texture_ex(
        texture,
        x + INSET,
        y + INSET,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            ..Default::default()
        },
    );
}

/// Draw a single CELL×CELL block at grid position (col, row) relative to (origin_x, origin_y).
fn draw_cell(
    origin_x: f32,
    origin_y: f32,
    col: usize,
    row: usize,
    color: Color,
    texture: &Texture2D,
) {
    draw_cell_at(
        origin_x + col as f32 * CELL,
        origin_y + row as f32 * CELL,
        color,
        texture,
    );
}
```

- [ ] **Step 3: Update `render_board` to skip pending rows and draw particles**

In `render_board`, find the `// Locked cells` section (around line 169). Replace the entire locked-cells loop with this version that skips pending rows, and add the particle block immediately after:

```rust
// Locked cells (skip rows pending compaction — they are drawn as particles below)
for (r, row) in game.board.iter().enumerate() {
    if game.rows_pending_compaction.contains(&r) {
        continue;
    }
    for (c, cell) in row.iter().enumerate() {
        if let Some(kind) = cell {
            let left_border = c == 0 || game.board[r][c - 1].is_none();
            let top_border = r == 0 || game.board[r - 1][c].is_none();
            let right_border = c == BOARD_COLS - 1 || game.board[r][c + 1].is_none();
            let bottom_border = r == BOARD_ROWS - 1 || game.board[r + 1][c].is_none();
            draw_cell_bordered(
                BOARD_X,
                BOARD_Y,
                c,
                r,
                piece_color(*kind),
                texture,
                left_border,
                top_border,
                right_border,
                bottom_border,
            );
        }
    }
}

// Particles: cells from cleared rows fly off screen during LineClearDelay
if let PiecePhase::LineClearDelay { ticks_left } = game.piece_phase {
    let t = (LINE_CLEAR_DELAY - ticks_left) as f32;
    for &r in &game.rows_pending_compaction {
        for (c, cell) in game.board[r].iter().enumerate() {
            if let Some(kind) = cell {
                let initial_x = BOARD_X + c as f32 * CELL;
                let initial_y = BOARD_Y + r as f32 * CELL;
                let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                let px = initial_x + dist * PARTICLE_VX_SCALE * t;
                let py = initial_y + PARTICLE_VY_INITIAL * t + 0.5 * PARTICLE_GRAVITY * t * t;
                if px > -CELL && px < screen_width() && py > -CELL && py < screen_height() {
                    draw_cell_at(px, py, piece_color(*kind), texture);
                }
            }
        }
    }
}
```

- [ ] **Step 4: Build and run tests**

```bash
cd /Users/eperdew/Software/fetris && cargo test 2>&1 | tail -20
```

Expected: all tests pass, no compile errors.

- [ ] **Step 5: Commit**

```bash
cd /Users/eperdew/Software/fetris
git add src/renderer.rs
git commit -m "feat: render line clear cells as particles during LineClearDelay"
```

---

### Task 5: Tune particle constants

**Files:**
- Modify: `src/constants.rs`

This task is done by running the game and visually adjusting the three constants in `src/constants.rs` until the animation looks right. The goal: all cleared cells should exit the bottom of the screen (or far enough off the sides) well within the 41-frame window.

- [ ] **Step 1: Run the game**

```bash
cd /Users/eperdew/Software/fetris && cargo run
```

Clear some lines (fill rows and drop pieces). Observe the particle animation.

- [ ] **Step 2: Adjust constants in `src/constants.rs` if needed**

Key dials:
- `PARTICLE_GRAVITY`: increase if top-row cells don't fall far enough in 41 frames
- `PARTICLE_VX_SCALE`: increase if cells near the edges don't spread enough
- `PARTICLE_VY_INITIAL`: increase for a snappier initial drop; decrease for a more floaty start

Reference: the board top is at `y ≈ 20px`, bottom at `y ≈ 660px`. A cell starting at row 0 needs to travel ~640px in 41 frames. With `VY_INITIAL=2` and `GRAVITY=0.8`: displacement = `2*41 + 0.5*0.8*41² = 82 + 672 = 754px` ✓

- [ ] **Step 3: Commit final tuned values**

```bash
cd /Users/eperdew/Software/fetris
git add src/constants.rs
git commit -m "tune: adjust particle animation constants for visual feel"
```