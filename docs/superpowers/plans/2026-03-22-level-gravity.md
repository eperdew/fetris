# Level, Gravity & Timer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement TGM1-accurate level progression (0–999), G/256 gravity accumulator, win condition, playtime timer (MM:SS.sss), split ARE, and matching constant tuning.

**Architecture:** Six sequential tasks that build on each other. Constants are tuned first, then gravity, then level/win state, then ARE split, then timer, then renderer. Each task is independently testable and commits cleanly.

**Tech Stack:** Rust, ratatui (renderer), cargo-insta (snapshot tests for renderer)

---

## File Map

| File | Changes |
|------|---------|
| `src/constants.rs` | Tasks 1, 2, 4: tune constants, add gravity table, split ARE |
| `src/game.rs` | Tasks 2, 3, 4: gravity accumulator, level/win logic, split ARE |
| `src/tests.rs` | Tasks 1–5: new unit + integration tests; Task 4: fix SPAWN_DELAY import |
| `src/renderer.rs` | Tasks 5, 6: sidebar timer, victory screen |

---

### Task 1: Tune DAS_REPEAT and LOCK_DELAY

**Files:**
- Modify: `src/constants.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write a failing test for DAS_REPEAT=1**

Add to `src/tests.rs`:

```rust
#[test]
fn das_repeats_every_tick_after_charge() {
    let mut game = make_game(PieceKind::T);
    let start_col = game.active.col;
    press(&mut game, GameKey::Left);                 // immediate: start_col - 1
    hold(&mut game, &[GameKey::Left], DAS_CHARGE);   // first auto-repeat at charge: start_col - 2
    hold(&mut game, &[GameKey::Left], 3);            // 3 more repeats (DAS_REPEAT=1): start_col - 5
    assert_eq!(game.active.col, start_col - 5, "DAS should repeat every tick after charge");
}
```

- [ ] **Step 2: Run test to verify it fails**

```
cargo test das_repeats_every_tick_after_charge
```

Expected: FAIL (with DAS_REPEAT=6, only 0 additional repeats fire in 3 ticks after charge)

- [ ] **Step 3: Update constants**

Replace the body of `src/constants.rs` with:

```rust
pub const GRAVITY_DELAY: u32 = 30; // removed in Task 2
pub const LOCK_DELAY: u32 = 29;    // N+1 countdown → 30 actual frames (TGM1)
pub const SPAWN_DELAY: u32 = 30;   // split into NORMAL/LINE_CLEAR in Task 4
pub const DAS_CHARGE: u32 = 16;    // unchanged (matches TGM1)
pub const DAS_REPEAT: u32 = 1;     // TGM1: auto-shift fires every frame once charged
```

- [ ] **Step 4: Run all tests**

```
cargo test
```

Expected: ALL PASS. Note: `lock_delay_prevents_immediate_lock` uses `idle(&mut game, LOCK_DELAY + 1)` — with LOCK_DELAY=29 that's 30 idles total (29 decrements to 0, one more fires lock). Still correct.

- [ ] **Step 5: Commit**

```bash
git add src/constants.rs src/tests.rs
git commit -m "tune: DAS_REPEAT 6→1, LOCK_DELAY 30→29 (TGM1)"
```

---

### Task 2: Gravity table + G/256 accumulator

**Files:**
- Modify: `src/constants.rs`
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write failing unit tests for gravity_g**

Add `gravity_g` to the constants import at the top of `src/tests.rs`:

```rust
use crate::constants::{DAS_CHARGE, LOCK_DELAY, SPAWN_DELAY, gravity_g};
```

Add test:

```rust
#[test]
fn gravity_g_lookup() {
    assert_eq!(gravity_g(0),   4,    "level 0 → 4 G/256");
    assert_eq!(gravity_g(29),  4,    "level 29 → still 4 G/256");
    assert_eq!(gravity_g(30),  6,    "level 30 → 6 G/256");
    assert_eq!(gravity_g(199), 144,  "level 199 → 144 G/256");
    assert_eq!(gravity_g(200), 4,    "level 200 → resets to 4 G/256");
    assert_eq!(gravity_g(251), 256,  "level 251 → 256 G/256 (1G)");
    assert_eq!(gravity_g(500), 5120, "level 500 → 5120 G/256 (20G)");
}
```

- [ ] **Step 2: Run to verify fail**

```
cargo test gravity_g_lookup
```

Expected: FAIL (`gravity_g` does not exist yet)

- [ ] **Step 3: Replace constants.rs with gravity table**

Full new `src/constants.rs`:

```rust
pub const LOCK_DELAY: u32 = 29;    // N+1 countdown → 30 actual frames (TGM1)
pub const SPAWN_DELAY: u32 = 30;   // split into NORMAL/LINE_CLEAR in Task 4
pub const DAS_CHARGE: u32 = 16;    // unchanged (matches TGM1)
pub const DAS_REPEAT: u32 = 1;     // TGM1: auto-shift fires every frame once charged

/// (min_level, G_value) pairs in ascending order. G is in units of G/256 per tick.
/// Source: TGM1 wiki. Notable: gravity resets to 4 at level 200, then ramps
/// rapidly to 20G at level 500 with a brief ease-up at 420/450.
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
    GRAVITY_TABLE
        .iter()
        .rev()
        .find(|(threshold, _)| level >= *threshold)
        .map(|(_, g)| *g)
        .unwrap_or(4)
}
```

- [ ] **Step 4: Run gravity_g test**

```
cargo test gravity_g_lookup
```

Expected: PASS

- [ ] **Step 5: Update game.rs — imports**

In `src/game.rs` line 1, change:

```rust
use crate::constants::{DAS_CHARGE, DAS_REPEAT, GRAVITY_DELAY, LOCK_DELAY, SPAWN_DELAY};
```

to:

```rust
use crate::constants::{DAS_CHARGE, DAS_REPEAT, LOCK_DELAY, SPAWN_DELAY, gravity_g};
```

- [ ] **Step 6: Update game.rs — struct field**

In the `Game` struct, rename the field:

```rust
pub gravity_accumulator: u32,  // was: gravity_counter
```

- [ ] **Step 7: Update game.rs — Game::new()**

Change `gravity_counter: 0` to `gravity_accumulator: 0`.

- [ ] **Step 8: Update game.rs — soft drop reset**

In Phase 5 (soft drop), change:

```rust
self.gravity_counter = 0; // soft drop resets gravity timer
```

to:

```rust
self.gravity_accumulator = 0;
```

- [ ] **Step 9: Update game.rs — Phase 6 gravity**

Replace the entire Phase 6 block:

```rust
// Phase 6: Gravity.
self.gravity_counter += 1;
if self.gravity_counter >= GRAVITY_DELAY {
    self.gravity_counter = 0;
    self.try_move(0, 1);
}
```

with:

```rust
// Phase 6: Gravity (G/256 accumulator).
self.gravity_accumulator += gravity_g(self.level);
let drops = self.gravity_accumulator / 256;
self.gravity_accumulator %= 256;
for _ in 0..drops {
    if !self.try_move(0, 1) { break; }
}
```

- [ ] **Step 10: Update game.rs — spawn_piece**

In `spawn_piece`, change `self.gravity_counter = 0` to `self.gravity_accumulator = 0`.

- [ ] **Step 11: Run all tests**

```
cargo test
```

Expected: ALL PASS

- [ ] **Step 12: Commit**

```bash
git add src/constants.rs src/game.rs src/tests.rs
git commit -m "feat: G/256 gravity accumulator and TGM1 gravity table"
```

---

### Task 3: TGM1 level counter + win state + ticks_elapsed

**Files:**
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

The vertical I-piece (rotation=1) occupies cells at `(col+2, row)` through `(col+2, row+3)`.
When placed at `col=0, row=BOARD_ROWS-4`, it covers column 2, rows 16–19 and is on the floor
(row 20 is out of bounds). This is the pattern used by all line-clear tests below.

- [ ] **Step 1: Write failing tests**

Add `can_piece_increment` to game import in `src/tests.rs` line 2:

```rust
use crate::game::{BOARD_COLS, BOARD_ROWS, Board, Game, PiecePhase, RotationDirection, can_piece_increment};
```

Add a helper and tests at the bottom of `src/tests.rs`:

```rust
/// Positions a vertical I-piece on the floor with `n` bottom rows pre-filled
/// (except column 2). Ticking once fires lock and clears n lines.
fn setup_line_clear(game: &mut Game, n: usize) {
    for r in (BOARD_ROWS - n)..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            if c != 2 {
                game.board[r][c] = Some(PieceKind::O);
            }
        }
    }
    game.active = Piece::new(PieceKind::I);
    game.active.col = 0;
    game.active.row = (BOARD_ROWS - 4) as i32;
    game.active.rotation = 1; // vertical: cells at (col+2, row..row+3)
    game.piece_phase = PiecePhase::Locking { ticks_left: 0 };
}

#[test]
fn can_piece_increment_section_stops() {
    assert!(!can_piece_increment(99),  "99 is section stop");
    assert!(!can_piece_increment(199), "199 is section stop");
    assert!(!can_piece_increment(899), "899 is section stop");
    assert!(!can_piece_increment(998), "998 is final stop");
    assert!(can_piece_increment(0),   "0 is not a stop");
    assert!(can_piece_increment(100), "100 is not a stop");
    assert!(can_piece_increment(500), "500 is not a stop");
}

#[test]
fn level_starts_at_zero() {
    let game = Game::new();
    assert_eq!(game.level, 0);
}

#[test]
fn level_increments_on_piece_spawn() {
    let mut game = make_game(PieceKind::T);
    game.level = 50;
    while game.try_move(0, 1) {}  // drop to floor
    idle(&mut game, 1);                  // enter Locking{LOCK_DELAY}
    idle(&mut game, LOCK_DELAY + 1);     // fire lock → Spawning{SPAWN_DELAY}
    idle(&mut game, SPAWN_DELAY + 1);    // complete ARE → spawn_piece called
    assert_eq!(game.level, 51, "level should increment from 50 to 51 on spawn");
}

#[test]
fn section_stop_blocks_piece_increment() {
    let mut game = make_game(PieceKind::T);
    game.level = 99;
    while game.try_move(0, 1) {}
    idle(&mut game, 1);
    idle(&mut game, LOCK_DELAY + 1);
    idle(&mut game, SPAWN_DELAY + 1);
    assert_eq!(game.level, 99, "section stop: level should remain 99 after spawn");
}

#[test]
fn line_clear_increments_level() {
    let mut game = Game::new();
    game.level = 50;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → 1 line cleared
    assert_eq!(game.level, 51, "1 line clear should increment level 50→51");
}

#[test]
fn line_clear_passes_section_stop() {
    let mut game = Game::new();
    game.level = 99;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1);
    assert_eq!(game.level, 100, "line clear should pass section stop 99→100");
}

#[test]
fn level_clamped_to_999() {
    let mut game = Game::new();
    game.level = 998;
    setup_line_clear(&mut game, 4); // tetris: +4 would be 1002, clamped to 999
    idle(&mut game, 1);
    assert_eq!(game.level, 999, "level should clamp to 999");
}

#[test]
fn game_won_on_reaching_999() {
    let mut game = Game::new();
    game.level = 998;
    setup_line_clear(&mut game, 1); // +1 = 999
    idle(&mut game, 1);
    assert!(game.game_won, "game_won should be set when level reaches 999");
}

#[test]
fn ticks_elapsed_increments_each_tick() {
    let mut game = Game::new();
    idle(&mut game, 5);
    assert_eq!(game.ticks_elapsed, 5);
}

#[test]
fn ticks_elapsed_stops_after_win() {
    let mut game = Game::new();
    game.level = 998;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fires win
    let frozen = game.ticks_elapsed;
    idle(&mut game, 10);
    assert_eq!(game.ticks_elapsed, frozen, "ticks_elapsed should freeze after win");
}
```

- [ ] **Step 2: Run to verify failures**

```
cargo test can_piece_increment_section_stops level_starts_at_zero level_increments_on_piece_spawn
```

Expected: FAIL (functions/fields don't exist yet)

- [ ] **Step 3: Add game_won and ticks_elapsed to Game struct**

In `src/game.rs`, add to the `Game` struct after `game_over`:

```rust
pub game_won: bool,
pub ticks_elapsed: u64,
```

In `Game::new()`, add after `game_over: false`:

```rust
game_won: false,
ticks_elapsed: 0,
```

- [ ] **Step 4: Add early-return guard and ticks_elapsed increment to tick()**

At the top of `tick()`, replace:

```rust
if self.game_over {
    return;
}
```

with:

```rust
if self.game_over || self.game_won {
    return;
}
self.ticks_elapsed += 1;
```

- [ ] **Step 5: Add can_piece_increment**

Add as a free function (pub(crate)) after the `impl Game` block in `src/game.rs`:

```rust
pub(crate) fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
```

- [ ] **Step 6: Update Game::new() — level starts at 0**

Change `level: 1` to `level: 0`.

- [ ] **Step 7: Update spawn_piece() — increment level**

At the top of `spawn_piece`, before the randomizer/piece swap, add:

```rust
if can_piece_increment(self.level) {
    self.level += 1;
}
```

- [ ] **Step 8: Update clear_lines() — TGM1 level logic**

Replace the last two lines of `clear_lines`:

```rust
self.lines += count;
self.level = 1 + self.lines / 10;
```

with:

```rust
self.lines += count;
self.level = (self.level + count).min(999);
if self.level == 999 {
    self.game_won = true;
}
```

- [ ] **Step 9: Run all tests**

```
cargo test
```

Expected: ALL PASS

- [ ] **Step 10: Commit**

```bash
git add src/game.rs src/tests.rs
git commit -m "feat: TGM1 level counter, section stops, win state, ticks_elapsed"
```

---

### Task 4: Split ARE (SPAWN_DELAY_NORMAL + SPAWN_DELAY_LINE_CLEAR)

**Files:**
- Modify: `src/constants.rs`
- Modify: `src/game.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write failing tests**

Add to `src/tests.rs`:

```rust
#[test]
fn normal_are_uses_spawn_delay_normal() {
    use crate::constants::SPAWN_DELAY_NORMAL;
    let mut game = make_game(PieceKind::T);
    while game.try_move(0, 1) {}
    idle(&mut game, 1);              // enter Locking
    idle(&mut game, LOCK_DELAY + 1); // fire lock (no lines cleared)
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_NORMAL),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_NORMAL={} }}, got {:?}",
        SPAWN_DELAY_NORMAL, game.piece_phase
    );
}

#[test]
fn line_clear_are_uses_spawn_delay_line_clear() {
    use crate::constants::SPAWN_DELAY_LINE_CLEAR;
    let mut game = Game::new();
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock + 1 line clear
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_LINE_CLEAR),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_LINE_CLEAR={} }}, got {:?}",
        SPAWN_DELAY_LINE_CLEAR, game.piece_phase
    );
}
```

- [ ] **Step 2: Run to verify fail**

```
cargo test normal_are_uses_spawn_delay_normal line_clear_are_uses_spawn_delay_line_clear
```

Expected: FAIL (`SPAWN_DELAY_NORMAL`/`SPAWN_DELAY_LINE_CLEAR` don't exist)

- [ ] **Step 3: Update constants.rs — split SPAWN_DELAY**

Remove `SPAWN_DELAY` and add:

```rust
pub const SPAWN_DELAY_NORMAL: u32 = 29;      // N+1 → 30 frames: ARE without line clear (TGM1)
pub const SPAWN_DELAY_LINE_CLEAR: u32 = 40;  // N+1 → 41 frames: ARE after line clear (TGM1)
```

Full updated `src/constants.rs`:

```rust
pub const LOCK_DELAY: u32 = 29;
pub const SPAWN_DELAY_NORMAL: u32 = 29;
pub const SPAWN_DELAY_LINE_CLEAR: u32 = 40;
pub const DAS_CHARGE: u32 = 16;
pub const DAS_REPEAT: u32 = 1;

// ... GRAVITY_TABLE and gravity_g unchanged
```

- [ ] **Step 4: Update game.rs — imports**

Change:

```rust
use crate::constants::{DAS_CHARGE, DAS_REPEAT, LOCK_DELAY, SPAWN_DELAY, gravity_g};
```

to:

```rust
use crate::constants::{DAS_CHARGE, DAS_REPEAT, LOCK_DELAY, SPAWN_DELAY_NORMAL, SPAWN_DELAY_LINE_CLEAR, gravity_g};
```

- [ ] **Step 5: Update game.rs — clear_lines returns u32**

Change the signature from `fn clear_lines(&mut self)` to `fn clear_lines(&mut self) -> u32`.

Add `return 0;` after the early-exit when `count == 0`. Change the function to return `count` at the end:

```rust
fn clear_lines(&mut self) -> u32 {
    let cleared: Vec<usize> = (0..BOARD_ROWS)
        .filter(|&r| self.board[r].iter().all(|c| c.is_some()))
        .collect();
    let count = cleared.len() as u32;
    if count == 0 {
        return 0;
    }
    let mut new_board: Board = [[None; BOARD_COLS]; BOARD_ROWS];
    let kept: Vec<[Option<PieceKind>; BOARD_COLS]> = self
        .board
        .iter()
        .filter(|row| row.iter().any(|c| c.is_none()))
        .copied()
        .collect();
    let offset = BOARD_ROWS - kept.len();
    for (i, row) in kept.into_iter().enumerate() {
        new_board[offset + i] = row;
    }
    self.board = new_board;
    self.lines += count;
    self.level = (self.level + count).min(999);
    if self.level == 999 {
        self.game_won = true;
    }
    count
}
```

- [ ] **Step 6: Update game.rs — lock_piece uses clear_lines return**

In `lock_piece`, change:

```rust
self.clear_lines();
// ...
self.piece_phase = PiecePhase::Spawning {
    ticks_left: SPAWN_DELAY,
};
```

to:

```rust
let lines_cleared = self.clear_lines();
// ...
let are = if lines_cleared > 0 { SPAWN_DELAY_LINE_CLEAR } else { SPAWN_DELAY_NORMAL };
self.piece_phase = PiecePhase::Spawning { ticks_left: are };
```

- [ ] **Step 7: Update tests.rs — fix SPAWN_DELAY import**

On line 5, change:

```rust
use crate::constants::{DAS_CHARGE, LOCK_DELAY, SPAWN_DELAY, gravity_g};
```

to:

```rust
use crate::constants::{DAS_CHARGE, LOCK_DELAY, SPAWN_DELAY_NORMAL, gravity_g};
```

In `level_increments_on_piece_spawn` and `section_stop_blocks_piece_increment`, change `SPAWN_DELAY` to `SPAWN_DELAY_NORMAL`.

In `rotation_buffer_applied_on_spawn`, change the comment and the idle call:

```rust
// After the press decremented ticks_left by 1, SPAWN_DELAY_NORMAL idle ticks finish the countdown and spawn.
idle(&mut game, SPAWN_DELAY_NORMAL);
```

- [ ] **Step 8: Run all tests**

```
cargo test
```

Expected: ALL PASS

- [ ] **Step 9: Commit**

```bash
git add src/constants.rs src/game.rs src/tests.rs
git commit -m "feat: split ARE into SPAWN_DELAY_NORMAL=29 and SPAWN_DELAY_LINE_CLEAR=40 (TGM1)"
```

---

### Task 5: Timer display in sidebar

**Files:**
- Modify: `src/renderer.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write a failing test for format_time**

Add to `src/tests.rs`:

```rust
#[test]
fn format_time_display() {
    use crate::renderer::format_time;
    assert_eq!(format_time(0),     "00:00.000");
    assert_eq!(format_time(60),    "00:01.000");
    assert_eq!(format_time(3600),  "01:00.000");
    assert_eq!(format_time(90),    "00:01.500");
    assert_eq!(format_time(5430),  "01:30.500");
}
```

- [ ] **Step 2: Run to verify fail**

```
cargo test format_time_display
```

Expected: FAIL (`format_time` does not exist)

- [ ] **Step 3: Add format_time to renderer.rs**

Add this public function to `src/renderer.rs` (before `render`):

```rust
pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}
```

- [ ] **Step 4: Run format_time test**

```
cargo test format_time_display
```

Expected: PASS

- [ ] **Step 5: Add timer to render_sidebar**

In `render_sidebar`, update the stats `Paragraph` to include the timer after Lines. Change:

```rust
let stats = Paragraph::new(vec![
    Line::from(""),
    Line::from(format!("Level: {}", game.level)),
    Line::from(format!("Lines: {}", game.lines)),
    Line::from(""),
    // ...
])
```

to:

```rust
let stats = Paragraph::new(vec![
    Line::from(""),
    Line::from(format!("Level: {}", game.level)),
    Line::from(format!("Lines: {}", game.lines)),
    Line::from(format_time(game.ticks_elapsed)),
    Line::from(""),
    // ...
])
```

- [ ] **Step 6: Run all tests**

```
cargo test
```

Expected: ALL PASS

- [ ] **Step 7: Commit**

```bash
git add src/renderer.rs src/tests.rs
git commit -m "feat: playtime timer display (MM:SS.sss) in sidebar"
```

---

### Task 6: Victory screen

**Files:**
- Modify: `src/renderer.rs`
- Modify: `src/tests.rs`

- [ ] **Step 1: Write failing snapshot test**

Add to `src/tests.rs` (add ratatui imports at the top of the file or within the test):

```rust
#[test]
fn victory_screen_snapshot() {
    use ratatui::{Terminal, backend::TestBackend};

    let mut game = Game::new();
    game.game_won = true;
    game.ticks_elapsed = 5430; // 01:30.500
    game.level = 999;
    game.lines = 100;

    let backend = TestBackend::new(36, 22);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| crate::renderer::render(frame, &game)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content()
        .chunks(36)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol().to_string())
                .collect::<String>()
                + "\n"
        })
        .collect();
    insta::assert_snapshot!(content);
}
```

Add `insta` to the dev-dependencies in `Cargo.toml` if not already present:

```toml
[dev-dependencies]
insta = "1"
```

- [ ] **Step 2: Run to verify fail**

```
cargo test victory_screen_snapshot
```

Expected: FAIL (victory screen not yet implemented)

- [ ] **Step 3: Implement victory screen in render_board**

In `src/renderer.rs`, update `render_board` to show the victory screen when `game.game_won`:

```rust
fn render_board(frame: &mut Frame, game: &Game, area: ratatui::layout::Rect) {
    if game.game_won {
        let time_str = format_time(game.ticks_elapsed);
        let victory = Paragraph::new(vec![
            Line::from(""),
            Line::from("  LEVEL 999"),
            Line::from(""),
            Line::from("  Time:"),
            Line::from(format!("  {}", time_str)),
            Line::from(""),
        ])
        .block(Block::default().title("fetris").borders(Borders::ALL));
        frame.render_widget(victory, area);
        return;
    }

    // ... existing board rendering code unchanged below ...
```

- [ ] **Step 4: Accept snapshot**

```
cargo test victory_screen_snapshot
cargo insta accept
```

- [ ] **Step 5: Run all tests**

```
cargo test
```

Expected: ALL PASS

- [ ] **Step 6: Commit**

```bash
git add src/renderer.rs src/tests.rs Cargo.toml Cargo.lock
git add snapshots/  # insta snapshot files in src/snapshots/
git commit -m "feat: victory screen at level 999"
```

---

## Verification

After all tasks:

```
cargo test
```

All tests should pass. Manually verify by playing until level 999 (or using `cargo test` to exercise the win path).
