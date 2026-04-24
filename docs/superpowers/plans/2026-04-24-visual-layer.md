# Visual Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decouple the renderer from game internals via a `GameSnapshot` + `GameEvent` event bus, make the renderer stateful, and add an improved particle burst + scanline-shader text overlay for multi-line clears.

**Architecture:** `Game` accumulates `Vec<GameEvent>` internally and exposes `drain_events()` + `snapshot()` each tick; `main.rs` calls both and passes results to `renderer.render(&snapshot, &events)`; the `Renderer` owns particle and overlay state, updating it from events and advancing physics each frame.

**Tech Stack:** Rust, macroquad 0.4, GLSL ES 1.0 (WebGL-compatible shaders), insta (snapshot tests)

**Spec:** `docs/superpowers/specs/2026-04-24-visual-layer-design.md`

---

## File Map

| File | Role |
|---|---|
| `src/types.rs` | Add `GameEvent`, `GameSnapshot` |
| `src/game.rs` | Add `events` field, `drain_events()`, `snapshot()`, private `compute_ghost_row()` |
| `src/renderer.rs` | Add `Particle`, `LineClearOverlay`, `OverlayKind`, GLSL shader consts, `RenderTarget`, `Material`; update `render`/`render_ready` signatures |
| `src/constants.rs` | Update particle speed/gravity constants for improved burst feel |
| `src/main.rs` | Call `snapshot()` + `drain_events()` per tick, pass both to renderer |
| `src/tests.rs` | Add tests for `drain_events`, `snapshot` |

---

## Task 1: Add `GameEvent` + `drain_events` to Game

**Files:**
- Modify: `src/types.rs`
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1.1: Add `GameEvent` to `src/types.rs`**

Append after the `JudgeEvent` section (around line 328):

```rust
// ---------------------------------------------------------------------------
// Renderer events
// ---------------------------------------------------------------------------

pub enum GameEvent {
    LineClear { count: u32 },
}
```

- [ ] **Step 1.2: Add `events` field and `drain_events` to `src/game.rs`**

Add `use crate::types::GameEvent;` to the import block at the top.

Add the field to the `Game` struct (after `randomizer`):
```rust
    events: Vec<GameEvent>,
```

Initialize it in `Game::new`:
```rust
            events: Vec::new(),
```

Add `drain_events` method in the `impl Game` block:
```rust
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
```

- [ ] **Step 1.3: Push `LineClear` events in `lock_piece`**

In `lock_piece`, after `self.lines += count;` inside `clear_lines` — actually, emit the event in `lock_piece` after the call to `clear_lines`. Find this block in `lock_piece` (around line 353):

```rust
        self.piece_phase = if lines_cleared > 0 {
            PiecePhase::LineClearDelay {
                ticks_left: LINE_CLEAR_DELAY,
            }
        } else {
```

Add one line before it:
```rust
        if lines_cleared > 0 {
            self.events.push(GameEvent::LineClear { count: lines_cleared });
        }
        self.piece_phase = if lines_cleared > 0 {
```

- [ ] **Step 1.4: Write failing tests in `src/tests.rs`**

Add at the end of `tests.rs`:

```rust
// ---------------------------------------------------------------------------
// drain_events
// ---------------------------------------------------------------------------

/// Helper: fill all cells in a row on the given board.
fn fill_row(board: &mut Board, row: usize) {
    for c in 0..BOARD_COLS {
        board[row][c] = Some(PieceKind::O);
    }
}

#[test]
fn drain_events_no_clear_is_empty() {
    let mut game = make_game(PieceKind::T);
    // Drop a T-piece into an empty board — no line clear.
    while game.piece_phase == PiecePhase::Falling {
        idle(&mut game, 1);
    }
    let events = game.drain_events();
    assert!(events.is_empty(), "no line clear should produce no events");
}

#[test]
fn drain_events_single_clear() {
    let mut game = make_game(PieceKind::I);
    // Pre-fill the bottom row with gaps only where the I-piece will land.
    fill_row(&mut game.board, BOARD_ROWS - 1);
    game.board[BOARD_ROWS - 1][3] = None;
    game.board[BOARD_ROWS - 1][4] = None;
    game.board[BOARD_ROWS - 1][5] = None;
    game.board[BOARD_ROWS - 1][6] = None;
    // Place active I-piece at the bottom row in horizontal orientation.
    game.active.row = BOARD_ROWS as i32 - 2;
    game.active.col = 3;
    // Lock immediately with soft drop.
    press(&mut game, GameKey::SoftDrop);
    let events = game.drain_events();
    let counts: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            crate::types::GameEvent::LineClear { count } => Some(*count),
        })
        .collect();
    assert_eq!(counts, vec![1]);
}

#[test]
fn drain_events_clears_after_drain() {
    let mut game = make_game(PieceKind::I);
    fill_row(&mut game.board, BOARD_ROWS - 1);
    game.board[BOARD_ROWS - 1][3] = None;
    game.board[BOARD_ROWS - 1][4] = None;
    game.board[BOARD_ROWS - 1][5] = None;
    game.board[BOARD_ROWS - 1][6] = None;
    game.active.row = BOARD_ROWS as i32 - 2;
    game.active.col = 3;
    press(&mut game, GameKey::SoftDrop);
    let _ = game.drain_events();
    // Second drain should be empty.
    let events2 = game.drain_events();
    assert!(events2.is_empty(), "drain_events should clear the buffer");
}
```

- [ ] **Step 1.5: Run tests and verify they pass**

```bash
cargo test drain_events
```

Expected: 3 tests pass. If any fail, investigate — the I-piece horizontal layout at row BOARD_ROWS-2, col 3 should cover columns 3–6 in ARS rotation 0.

- [ ] **Step 1.6: Commit**

```bash
git add src/types.rs src/game.rs src/tests.rs
git commit -m "feat: add GameEvent bus with drain_events"
```

---

## Task 2: Add `GameSnapshot` + `snapshot()` + move ghost row

**Files:**
- Modify: `src/types.rs`
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

- [ ] **Step 2.1: Add `GameSnapshot` to `src/types.rs`**

Add after `GameEvent` (still in the Renderer events section):

```rust
pub struct GameSnapshot {
    pub board: Board,
    /// Kind of the active piece (for color); None during Spawning / LineClearDelay.
    pub active_kind: Option<PieceKind>,
    /// Absolute board positions of the active piece cells; None when hidden.
    pub active_cells: Option<[(i32, i32); 4]>,
    /// Absolute board positions of the ghost piece cells; None when hidden or piece is on floor.
    pub ghost_cells: Option<[(i32, i32); 4]>,
    /// Relative (dc, dr) offsets for the active piece — always set, used for preview in Ready state.
    pub active_preview_offsets: [(i32, i32); 4],
    pub active_preview_y_offset: i32,
    /// Relative (dc, dr) offsets for the next piece, used for its preview.
    pub next_kind: PieceKind,
    pub next_preview_offsets: [(i32, i32); 4],
    pub next_preview_y_offset: i32,
    pub piece_phase: PiecePhase,
    pub rows_pending_compaction: Vec<usize>,
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub score: u32,
    pub grade: Grade,
    pub game_over: bool,
    pub game_won: bool,
}
```

- [ ] **Step 2.2: Add `compute_ghost_row` private method to `src/game.rs`**

Add this private method in `impl Game` (before `lock_piece` is fine):

```rust
    fn compute_ghost_row(&self) -> i32 {
        let mut ghost_row = self.active.row;
        loop {
            let next = ghost_row + 1;
            let blocked = self
                .rotation_system
                .cells(self.active.kind, self.active.rotation)
                .iter()
                .any(|&(dc, dr)| {
                    let c = self.active.col + dc;
                    let r = next + dr;
                    r >= BOARD_ROWS as i32
                        || (c >= 0
                            && c < BOARD_COLS as i32
                            && r >= 0
                            && self.board[r as usize][c as usize].is_some())
                });
            if blocked {
                break;
            }
            ghost_row = next;
        }
        ghost_row
    }
```

- [ ] **Step 2.3: Add `snapshot()` method to `src/game.rs`**

Add `use crate::types::GameSnapshot;` to the import block.

Add this public method in `impl Game`:

```rust
    pub fn snapshot(&self) -> GameSnapshot {
        let show_active = !matches!(
            self.piece_phase,
            PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
        );

        let active_offsets = self
            .rotation_system
            .cells(self.active.kind, self.active.rotation);

        let (active_kind, active_cells, ghost_cells) = if show_active {
            let cells = active_offsets
                .map(|(dc, dr)| (self.active.col + dc, self.active.row + dr));
            let ghost_row = self.compute_ghost_row();
            let ghost = if ghost_row != self.active.row {
                Some(
                    active_offsets
                        .map(|(dc, dr)| (self.active.col + dc, ghost_row + dr)),
                )
            } else {
                None
            };
            (Some(self.active.kind), Some(cells), ghost)
        } else {
            (None, None, None)
        };

        let next_offsets = self
            .rotation_system
            .cells(self.next.kind, self.next.rotation);

        GameSnapshot {
            board: self.board,
            active_kind,
            active_cells,
            ghost_cells,
            active_preview_offsets: active_offsets,
            active_preview_y_offset: self.rotation_system.preview_y_offset(self.active.kind),
            next_kind: self.next.kind,
            next_preview_offsets: next_offsets,
            next_preview_y_offset: self.rotation_system.preview_y_offset(self.next.kind),
            piece_phase: self.piece_phase,
            rows_pending_compaction: self.rows_pending_compaction.clone(),
            level: self.level,
            lines: self.lines,
            ticks_elapsed: self.ticks_elapsed,
            score: self.score(),
            grade: self.grade(),
            game_over: self.game_over,
            game_won: self.game_won,
        }
    }
```

- [ ] **Step 2.4: Write snapshot tests in `src/tests.rs`**

Add at the end of `tests.rs`:

```rust
// ---------------------------------------------------------------------------
// snapshot
// ---------------------------------------------------------------------------

#[test]
fn snapshot_active_hidden_during_spawning() {
    let mut game = make_game(PieceKind::T);
    // Force the Spawning phase.
    game.piece_phase = PiecePhase::Spawning { ticks_left: 5 };
    let snap = game.snapshot();
    assert!(snap.active_kind.is_none(), "active should be hidden during Spawning");
    assert!(snap.active_cells.is_none());
    assert!(snap.ghost_cells.is_none());
}

#[test]
fn snapshot_active_hidden_during_line_clear_delay() {
    let mut game = make_game(PieceKind::T);
    game.piece_phase = PiecePhase::LineClearDelay { ticks_left: 10 };
    let snap = game.snapshot();
    assert!(snap.active_kind.is_none());
}

#[test]
fn snapshot_active_visible_during_falling() {
    let mut game = make_game(PieceKind::T);
    // Default piece_phase is Falling.
    let snap = game.snapshot();
    assert_eq!(snap.active_kind, Some(PieceKind::T));
    assert!(snap.active_cells.is_some());
}

#[test]
fn snapshot_ghost_none_when_piece_on_floor() {
    let mut game = make_game(PieceKind::O);
    // Move O piece to the bottom row (rows 18-19 for ARS O in rotation 0).
    game.active.row = 18;
    let snap = game.snapshot();
    // Ghost row == active row → ghost_cells should be None.
    assert!(
        snap.ghost_cells.is_none(),
        "ghost should be None when piece is already on floor"
    );
}

#[test]
fn snapshot_ghost_present_above_floor() {
    let mut game = make_game(PieceKind::O);
    game.active.row = 0; // piece near top, lots of room to fall
    let snap = game.snapshot();
    assert!(
        snap.ghost_cells.is_some(),
        "ghost should be Some when piece can still fall"
    );
}
```

- [ ] **Step 2.5: Run tests and verify they pass**

```bash
cargo test snapshot
```

Expected: 5 tests pass.

- [ ] **Step 2.6: Commit**

```bash
git add src/types.rs src/game.rs src/tests.rs
git commit -m "feat: add GameSnapshot and snapshot() method, move ghost row calc to Game"
```

---

## Task 3: Switch renderer to `GameSnapshot`; update `main.rs`

This task has no new tests — correct compilation + existing test suite passing is the verification.

**Files:**
- Modify: `src/renderer.rs`
- Modify: `src/main.rs`

- [ ] **Step 3.1: Update imports in `src/renderer.rs`**

Replace the current `use crate::types::{...}` import block at the top with:

```rust
use crate::types::{
    BOARD_COLS, BOARD_ROWS, GameSnapshot, Grade, MenuScreen, PieceKind,
};
```

Remove `use crate::game::Game;` — it's no longer needed.

- [ ] **Step 3.2: Remove `compute_ghost_row` from `src/renderer.rs`**

Delete the entire `compute_ghost_row` free function (lines 631–654 in the original file).

- [ ] **Step 3.3: Update `render` signature and body**

Change:
```rust
    pub fn render(&self, game: &Game) {
        clear_background(grade_bg_color(game.grade().index()));
        self.render_board(game);
        self.render_grade_bar(game);
        self.render_sidebar(game);
        self.render_overlay(game);
    }
```

To:
```rust
    pub fn render(&self, snapshot: &GameSnapshot) {
        clear_background(grade_bg_color(snapshot.grade.index()));
        self.render_board(snapshot);
        self.render_grade_bar(snapshot);
        self.render_sidebar(snapshot);
        self.render_overlay(snapshot);
    }
```

- [ ] **Step 3.4: Update `render_ready` signature and body**

Change:
```rust
    pub fn render_ready(&self, game: &Game) {
        clear_background(grade_bg_color(game.grade().index()));
        draw_rectangle(BOARD_X, BOARD_Y, BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL, BOARD_BG);
        self.render_piece_preview(game, &game.active);
        draw_rectangle(BOARD_X, BOARD_Y, BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL, Color::new(0.0, 0.0, 0.0, 0.1));
        self.render_grade_bar(game);
        self.render_sidebar(game);
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        self.draw_centered_x("READY", cx, cy, 28.0, WHITE);
    }
```

To:
```rust
    pub fn render_ready(&self, snapshot: &GameSnapshot) {
        clear_background(grade_bg_color(snapshot.grade.index()));
        draw_rectangle(BOARD_X, BOARD_Y, BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL, BOARD_BG);
        self.render_piece_preview(
            snapshot.active_kind.unwrap_or(snapshot.next_kind),
            &snapshot.active_preview_offsets,
            snapshot.active_preview_y_offset,
        );
        draw_rectangle(BOARD_X, BOARD_Y, BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL, Color::new(0.0, 0.0, 0.0, 0.1));
        self.render_grade_bar(snapshot);
        self.render_sidebar(snapshot);
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        self.draw_centered_x("READY", cx, cy, 28.0, WHITE);
    }
```

- [ ] **Step 3.5: Update `render_piece_preview`**

Change:
```rust
    fn render_piece_preview(&self, game: &Game, piece: &Piece) {
        for (dc, dr) in game.rotation_system.cells(piece.kind, piece.rotation) {
            let y_offset = game.rotation_system.preview_y_offset(piece.kind);
            let c = 3 + dc;
            let r = -3 + dr + y_offset;
            draw_cell(BOARD_X, BOARD_Y - PAD, c, r, piece_color(piece.kind), &self.cell_texture);
        }
    }
```

To:
```rust
    fn render_piece_preview(
        &self,
        kind: PieceKind,
        offsets: &[(i32, i32); 4],
        preview_y_offset: i32,
    ) {
        for &(dc, dr) in offsets {
            let c = 3 + dc;
            let r = -3 + dr + preview_y_offset;
            draw_cell(BOARD_X, BOARD_Y - PAD, c, r, piece_color(kind), &self.cell_texture);
        }
    }
```

- [ ] **Step 3.6: Update `render_board`**

Replace the entire `render_board` method with:

```rust
    fn render_board(&self, snapshot: &GameSnapshot) {
        let texture = &self.cell_texture;

        draw_rectangle(
            BOARD_X, BOARD_Y,
            BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL,
            BOARD_BG,
        );

        // Ghost piece
        if let (Some(kind), Some(ghost_cells)) = (snapshot.active_kind, &snapshot.ghost_cells) {
            let base = piece_color(kind);
            let ghost_color = Color { a: 0.25, ..base };
            for &(c, r) in ghost_cells {
                if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                    draw_cell(BOARD_X, BOARD_Y, c, r, ghost_color, texture);
                }
            }
        }

        // Locked cells (skip rows pending compaction — drawn as particles)
        for (r, row) in snapshot.board.iter().enumerate() {
            if snapshot.rows_pending_compaction.contains(&r) {
                continue;
            }
            for (c, cell) in row.iter().enumerate() {
                if let Some(kind) = cell {
                    let left_border   = c == 0              || snapshot.board[r][c - 1].is_none();
                    let top_border    = r == 0              || snapshot.board[r - 1][c].is_none();
                    let right_border  = c == BOARD_COLS - 1 || snapshot.board[r][c + 1].is_none();
                    let bottom_border = r == BOARD_ROWS - 1 || snapshot.board[r + 1][c].is_none();
                    draw_cell_bordered(
                        BOARD_X, BOARD_Y, c as i32, r as i32,
                        piece_color(*kind), texture,
                        left_border, top_border, right_border, bottom_border,
                    );
                }
            }
        }

        // Particles — now handled by the stateful particle system (rendered separately)

        // Active piece
        if let (Some(kind), Some(active_cells)) = (snapshot.active_kind, &snapshot.active_cells) {
            for &(c, r) in active_cells {
                if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                    draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(kind), texture);
                }
            }
        }

        self.render_piece_preview(
            snapshot.next_kind,
            &snapshot.next_preview_offsets,
            snapshot.next_preview_y_offset,
        );

        draw_rectangle(
            BOARD_X, BOARD_Y,
            BOARD_COLS as f32 * CELL, BOARD_ROWS as f32 * CELL,
            Color::new(0.0, 0.0, 0.0, 0.1),
        );
    }
```

- [ ] **Step 3.7: Update `render_grade_bar`**

Change:
```rust
    fn render_grade_bar(&self, game: &Game) {
        let score = game.score();
        let grade = game.grade();
        let (prev, next_opt) = Grade::grade_progress(score);
```

To:
```rust
    fn render_grade_bar(&self, snapshot: &GameSnapshot) {
        let score = snapshot.score;
        let grade = snapshot.grade;
        let (prev, next_opt) = Grade::grade_progress(score);
```

The rest of the method body stays identical.

- [ ] **Step 3.8: Update `render_sidebar`**

Change the signature and all field accesses:

```rust
    fn render_sidebar(&self, snapshot: &GameSnapshot) {
        // ... (keep body identical but replace:)
        // game.level()          → snapshot.level
        // game.next_level_barrier() → next_level_barrier(snapshot.level)
        // game.lines            → snapshot.lines
        // game.ticks_elapsed    → snapshot.ticks_elapsed
        // game.score()          → snapshot.score
        // game.grade()          → snapshot.grade
        // Grade::grade_progress(game.score()) → Grade::grade_progress(snapshot.score)
```

Add a free function after the `impl Renderer` block for the level barrier calculation (it was a method on Game, now used locally):

```rust
fn next_level_barrier(level: u32) -> u32 {
    let round_up = (level + 1).next_multiple_of(100);
    if round_up == 1000 { 999 } else { round_up }
}
```

Full updated `render_sidebar`:

```rust
    fn render_sidebar(&self, snapshot: &GameSnapshot) {
        const FONT_LG: f32 = 26.0;
        const FONT_SM: f32 = 18.0;
        const LH: f32 = 30.0;
        const DIM: Color = Color::new(0.5, 0.5, 0.5, 1.0);

        let x = SIDEBAR_X;
        let mut y = BOARD_Y + 22.0;

        self.draw_text("LEVEL", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{:03}", snapshot.level), x, y, FONT_LG, WHITE);
        y += 6.0;
        draw_line(x, y, x + 48.0, y, 2.0, DIM);
        y += 24.0;
        self.draw_text(&format!("{}", next_level_barrier(snapshot.level)), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("LINES", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.lines), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("TIME", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format_time(snapshot.ticks_elapsed), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("SCORE", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.score), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("GRADE", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.grade), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("NEXT", x, y, FONT_SM, DIM);
        y += LH;
        let (_, next_opt) = Grade::grade_progress(snapshot.score);
        let next_str = match next_opt {
            Some(n) => format!("{}", n),
            None => "??????".to_string(),
        };
        self.draw_text(&next_str, x, y, FONT_LG, WHITE);
    }
```

- [ ] **Step 3.9: Update `render_overlay`**

Change:
```rust
    fn render_overlay(&self, game: &Game) {
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        if game.game_won {
            self.draw_text("LEVEL 999", cx - 60.0, cy - 16.0, 28.0, WHITE);
            self.draw_text(&format_time(game.ticks_elapsed), cx - 50.0, cy + 20.0, 22.0, LIGHTGRAY);
        } else if game.game_over {
            self.draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
        }
    }
```

To:
```rust
    fn render_overlay(&self, snapshot: &GameSnapshot) {
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        if snapshot.game_won {
            self.draw_text("LEVEL 999", cx - 60.0, cy - 16.0, 28.0, WHITE);
            self.draw_text(&format_time(snapshot.ticks_elapsed), cx - 50.0, cy + 20.0, 22.0, LIGHTGRAY);
        } else if snapshot.game_over {
            self.draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
        }
    }
```

- [ ] **Step 3.10: Update `main.rs` to call `snapshot()` and `drain_events()`**

In the `AppState::Playing` arm, replace:
```rust
                renderer.render(game);
```

With:
```rust
                let snapshot = game.snapshot();
                let _events = game.drain_events(); // will be used in Task 5
                renderer.render(&snapshot);
```

In the `AppState::Ready` arm, replace:
```rust
                renderer.render_ready(game);
```

With:
```rust
                renderer.render_ready(&game.snapshot());
```

- [ ] **Step 3.11: Remove `next_level_barrier` from `game.rs`**

In `src/game.rs`, delete the `next_level_barrier` method (it's now a free function in renderer.rs).

Note: If any tests call `game.next_level_barrier()`, update them to call the free function directly or inline the logic.

- [ ] **Step 3.12: Build and run all tests**

```bash
cargo test
```

Expected: all existing tests pass, no compilation errors.

- [ ] **Step 3.13: Commit**

```bash
git add src/renderer.rs src/main.rs src/game.rs
git commit -m "refactor: switch renderer to GameSnapshot, remove direct Game field access"
```

---

## Task 4: Make internal Game fields private

**Files:**
- Modify: `src/game.rs`

- [ ] **Step 4.1: Remove `pub` from truly internal fields**

In `src/game.rs`, change these four fields from `pub` to no visibility modifier (fully private):

```rust
    gravity_accumulator: u32,    // was pub
    rotation_buffer: Option<RotationDirection>,  // was pub
    soft_drop_frames: u32,       // was pub
    sonic_drop_rows: u32,        // was pub
```

- [ ] **Step 4.2: Verify compilation**

```bash
cargo test
```

Expected: all tests pass. If any test in `tests.rs` references these fields, it will fail to compile — investigate and either add a getter or accept that the test needs updating (these fields should not be needed externally).

- [ ] **Step 4.3: Commit**

```bash
git add src/game.rs
git commit -m "refactor: make internal Game fields private"
```

---

## Task 5: Stateful particle system in Renderer

**Files:**
- Modify: `src/constants.rs`
- Modify: `src/renderer.rs`
- Modify: `src/main.rs`

- [ ] **Step 5.1: Update particle constants in `src/constants.rs`**

Replace:
```rust
pub const PARTICLE_INITIAL_SPEED: f32 = 1.0;
pub const PARTICLE_GRAVITY: f32 = 0.4;
```

With:
```rust
/// Base speed for single-line-clear particles (pixels/frame). Scaled up by line count.
pub const PARTICLE_BASE_SPEED: f32 = 3.5;
/// Downward acceleration of particles (pixels/frame²).
pub const PARTICLE_GRAVITY: f32 = 0.35;
/// Base particle lifetime in frames. Each particle gets a small random jitter added.
pub const PARTICLE_BASE_LIFETIME: u32 = 55;
```

- [ ] **Step 5.2: Add `Particle` struct and helper to `src/renderer.rs`**

Add near the top of `renderer.rs`, after the constant declarations:

```rust
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: u32,
    lifetime: u32,
    color: Color,
}

fn rand_f32() -> f32 {
    macroquad::rand::rand() as f32 / u32::MAX as f32
}
```

- [ ] **Step 5.3: Add `particles` field to `Renderer` and initialize it**

Change the struct:
```rust
pub(crate) struct Renderer {
    cell_texture: Texture2D,
    font: Font,
    particles: Vec<Particle>,
}
```

In `Renderer::new()`, add `particles: Vec::new()` to the constructor.

- [ ] **Step 5.4: Add particle spawning function**

Add this free function in `renderer.rs`:

```rust
fn spawn_particles(
    particles: &mut Vec<Particle>,
    board: &crate::types::Board,
    rows: &[usize],
    count: u32,
) {
    use crate::constants::{PARTICLE_BASE_LIFETIME, PARTICLE_BASE_SPEED};

    let particles_per_cell: u32 = if count >= 4 { 3 } else { 1 };
    let speed_scale = match count {
        1 => 1.0,
        2 => 1.4,
        3 => 1.8,
        _ => 2.5,
    };

    for &r in rows {
        for (c, cell) in board[r].iter().enumerate() {
            if let Some(kind) = cell {
                for _ in 0..particles_per_cell {
                    // Base outward direction from horizontal center, slight upward bias.
                    let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                    let base_angle = dist.atan2(-1.5_f32); // negative y = upward in screen coords
                    let spread = (rand_f32() - 0.5) * std::f32::consts::FRAC_PI_3;
                    let angle = base_angle + spread;
                    let speed = PARTICLE_BASE_SPEED * speed_scale * (0.6 + 0.8 * rand_f32());

                    let lifetime = PARTICLE_BASE_LIFETIME + (rand_f32() * 25.0) as u32;
                    particles.push(Particle {
                        x: BOARD_X + c as f32 * CELL + CELL * 0.5,
                        y: BOARD_Y + r as f32 * CELL + CELL * 0.5,
                        vx: angle.sin() * speed,
                        vy: -angle.cos().abs() * speed,
                        age: 0,
                        lifetime,
                        color: piece_color(*kind),
                    });
                }
            }
        }
    }
}
```

- [ ] **Step 5.5: Add particle update and render methods to `Renderer`**

Add to `impl Renderer`:

```rust
    fn update_particles(&mut self) {
        use crate::constants::PARTICLE_GRAVITY;
        for p in &mut self.particles {
            p.x += p.vx;
            p.y += p.vy;
            p.vy += PARTICLE_GRAVITY;
            p.age += 1;
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    fn render_particles(&self) {
        for p in &self.particles {
            let alpha = 1.0 - p.age as f32 / p.lifetime as f32;
            let color = Color { a: alpha, ..p.color };
            draw_cell_at(p.x - CELL * 0.5, p.y - CELL * 0.5, color, &self.cell_texture);
        }
    }
```

- [ ] **Step 5.6: Wire particles into `render`**

`render` now takes events as well. Change the signature and body:

```rust
    pub fn render(&mut self, snapshot: &GameSnapshot, events: &[GameEvent]) {
        // Process events: spawn particles.
        for event in events {
            match event {
                GameEvent::LineClear { count } => {
                    spawn_particles(
                        &mut self.particles,
                        &snapshot.board,
                        &snapshot.rows_pending_compaction,
                        *count,
                    );
                }
            }
        }

        self.update_particles();

        clear_background(grade_bg_color(snapshot.grade.index()));
        self.render_board(snapshot);
        self.render_particles();
        self.render_grade_bar(snapshot);
        self.render_sidebar(snapshot);
        self.render_overlay(snapshot);
    }
```

Add `GameEvent` to the imports at the top of `renderer.rs`:
```rust
use crate::types::{
    BOARD_COLS, BOARD_ROWS, GameEvent, GameSnapshot, Grade, MenuScreen, PieceKind,
};
```

- [ ] **Step 5.7: Update `main.rs` to pass events to `render`**

Change the Playing arm in `main.rs` from:
```rust
                let snapshot = game.snapshot();
                let _events = game.drain_events();
                renderer.render(&snapshot);
```

To:
```rust
                let snapshot = game.snapshot();
                let events = game.drain_events();
                renderer.render(&snapshot, &events);
```

- [ ] **Step 5.8: Build and test**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 5.9: Visual smoke test**

```bash
cargo run --release
```

Play a game. Clearing lines should produce a particle burst that arcs outward and fades. Four-line clears should produce a noticeably larger burst than singles.

- [ ] **Step 5.10: Commit**

```bash
git add src/constants.rs src/renderer.rs src/main.rs
git commit -m "feat: stateful particle system with improved line-clear burst"
```

---

## Task 6: Text overlay with scanline shader

**Files:**
- Modify: `src/renderer.rs`

- [ ] **Step 6.1: Add shader source constants to `src/renderer.rs`**

Add near the top of the file, after the existing constants:

```rust
const OVERLAY_VERTEX_SHADER: &str = r#"
    #version 100
    attribute vec3 position;
    attribute vec2 texcoord;
    attribute vec4 color0;
    varying vec2 uv;
    varying vec4 color;
    uniform mat4 Model;
    uniform mat4 Projection;
    void main() {
        gl_Position = Projection * Model * vec4(position, 1.0);
        color = color0 / 255.0;
        uv = texcoord;
    }
"#;

const OVERLAY_FRAGMENT_SHADER: &str = r#"
    #version 100
    precision mediump float;
    varying vec2 uv;
    varying vec4 color;
    uniform sampler2D Texture;
    uniform float frame_parity;
    uniform float hue_shift;
    uniform float overlay_opacity;

    vec3 hue_rotate(vec3 col, float angle) {
        float c = cos(angle);
        float s = sin(angle);
        return vec3(
            dot(col, vec3(0.299 + 0.701*c + 0.168*s,
                          0.587 - 0.587*c + 0.330*s,
                          0.114 - 0.114*c - 0.497*s)),
            dot(col, vec3(0.299 - 0.299*c - 0.328*s,
                          0.587 + 0.413*c + 0.035*s,
                          0.114 - 0.114*c + 0.292*s)),
            dot(col, vec3(0.299 - 0.300*c + 1.250*s,
                          0.587 - 0.588*c - 1.050*s,
                          0.114 + 0.886*c - 0.203*s))
        );
    }

    void main() {
        if (mod(floor(gl_FragCoord.y), 2.0) != frame_parity) {
            discard;
        }
        vec4 tex = texture2D(Texture, uv) * color;
        if (hue_shift > 0.001) {
            tex.rgb = hue_rotate(tex.rgb, hue_shift * 6.28318);
        }
        tex.a *= overlay_opacity;
        gl_FragColor = tex;
    }
"#;

const OVERLAY_LIFETIME: u32 = 90;
```

- [ ] **Step 6.2: Add overlay types**

Add to `renderer.rs` near the `Particle` struct:

```rust
enum OverlayKind {
    Double,
    Triple,
    Fetris,
}

struct LineClearOverlay {
    kind: OverlayKind,
    frames_remaining: u32,
}

impl LineClearOverlay {
    fn label(&self) -> &'static str {
        match self.kind {
            OverlayKind::Double => "DOUBLE",
            OverlayKind::Triple => "TRIPLE",
            OverlayKind::Fetris => "FETRIS",
        }
    }

    fn base_opacity(&self) -> f32 {
        match self.kind {
            OverlayKind::Double => 0.45,
            OverlayKind::Triple => 0.75,
            OverlayKind::Fetris => 1.0,
        }
    }

    fn hue_shift(&self, ticks_elapsed: u64) -> f32 {
        match self.kind {
            OverlayKind::Fetris => (ticks_elapsed as f32 * 0.03) % 1.0,
            _ => 0.0,
        }
    }
}
```

- [ ] **Step 6.3: Add overlay fields to `Renderer` and update `new()`**

Change the struct:
```rust
pub(crate) struct Renderer {
    cell_texture: Texture2D,
    font: Font,
    particles: Vec<Particle>,
    overlay: Option<LineClearOverlay>,
    overlay_target: RenderTarget,
    overlay_material: Material,
}
```

Update `Renderer::new()` — it must become `async` because `load_material` may require GPU context in macroquad. Add the new fields:

```rust
    pub fn new() -> Self {
        let font =
            load_ttf_font_from_bytes(include_bytes!("../assets/font/Oxanium-Regular.ttf")).unwrap();
        let overlay_target = render_target(560, 780);
        overlay_target.texture.set_filter(FilterMode::Nearest);
        let overlay_material = load_material(
            ShaderSource::Glsl {
                vertex: OVERLAY_VERTEX_SHADER,
                fragment: OVERLAY_FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("frame_parity", UniformType::Float1),
                    UniformDesc::new("hue_shift", UniformType::Float1),
                    UniformDesc::new("overlay_opacity", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .expect("overlay shader failed to compile");
        Self {
            cell_texture: make_cell_texture(),
            font,
            particles: Vec::new(),
            overlay: None,
            overlay_target,
            overlay_material,
        }
    }
```

Add to the imports at the top of `renderer.rs`:
```rust
use macroquad::material::{load_material, MaterialParams, UniformDesc, UniformType};
use macroquad::prelude::*; // already present; ensure FilterMode, RenderTarget, ShaderSource are available
```

Note: `render_target`, `RenderTarget`, `FilterMode`, `ShaderSource`, `Material` are all in `macroquad::prelude` for macroquad 0.4. If any are missing, check `macroquad::texture` or `macroquad::material`.

- [ ] **Step 6.4: Add overlay rendering method**

Add to `impl Renderer`:

```rust
    fn render_line_clear_overlay(&mut self, ticks_elapsed: u64) {
        // Extract all data from overlay before any rendering calls to avoid borrow conflicts.
        let (label, opacity, hue_shift, frame_parity) = match &self.overlay {
            None => return,
            Some(o) => {
                let progress = o.frames_remaining as f32 / OVERLAY_LIFETIME as f32;
                (
                    o.label(),
                    o.base_opacity() * progress,
                    o.hue_shift(ticks_elapsed),
                    (o.frames_remaining % 2) as f32,
                )
            }
        };

        // Render text to off-screen target.
        set_camera(&Camera2D {
            zoom: vec2(2.0 / 560.0, -2.0 / 780.0),
            target: vec2(280.0, 390.0),
            render_target: Some(self.overlay_target.clone()),
            ..Default::default()
        });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        self.draw_centered_x(label, cx, cy, 40.0, WHITE);
        set_default_camera();

        // Draw to screen with scanline shader.
        self.overlay_material.set_uniform("frame_parity", frame_parity);
        self.overlay_material.set_uniform("hue_shift", hue_shift);
        self.overlay_material.set_uniform("overlay_opacity", opacity);
        gl_use_material(&self.overlay_material);
        draw_texture_ex(
            &self.overlay_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(560.0, 780.0)),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();

        // Tick the overlay — check first, then mutate or clear.
        let done = self.overlay.as_ref().map_or(true, |o| o.frames_remaining == 0);
        if done {
            self.overlay = None;
        } else if let Some(o) = &mut self.overlay {
            o.frames_remaining -= 1;
        }
    }
```

- [ ] **Step 6.5: Wire overlay into event processing and `render`**

In the `render` method, add overlay spawning inside the `LineClear` match arm:

```rust
                GameEvent::LineClear { count } => {
                    spawn_particles(
                        &mut self.particles,
                        &snapshot.board,
                        &snapshot.rows_pending_compaction,
                        *count,
                    );
                    self.overlay = match count {
                        2 => Some(LineClearOverlay { kind: OverlayKind::Double, frames_remaining: OVERLAY_LIFETIME }),
                        3 => Some(LineClearOverlay { kind: OverlayKind::Triple, frames_remaining: OVERLAY_LIFETIME }),
                        4 => Some(LineClearOverlay { kind: OverlayKind::Fetris, frames_remaining: OVERLAY_LIFETIME }),
                        _ => None,
                    };
                }
```

At the end of the `render` method body, after `render_overlay`, add:

```rust
        self.render_line_clear_overlay(snapshot.ticks_elapsed);
```

- [ ] **Step 6.6: Build and verify**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6.7: Visual smoke test**

```bash
cargo run --release
```

Verify:
- Clearing 1 line: particle burst, no text
- Clearing 2 lines: "DOUBLE" appears over the board with scanline flicker, dims out over ~1.5s
- Clearing 3 lines: "TRIPLE" appears brighter
- Clearing 4 lines: "FETRIS" appears with cycling rainbow hue + scanline flicker

If the overlay appears upside-down, try removing `flip_y: true`. If text doesn't appear at all, verify the camera zoom values produce correct coordinates (adjust if your window size differs from 560×780).

- [ ] **Step 6.8: Commit**

```bash
git add src/renderer.rs
git commit -m "feat: scanline shader overlay for DOUBLE/TRIPLE/FETRIS line clears"
```
