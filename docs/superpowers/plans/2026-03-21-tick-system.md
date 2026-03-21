# Tick System & Input Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate fetris from immediate keypress-driven logic to a deterministic 60Hz tick system with DAS, lock delay, spawn delay (ARE), and rotation buffering.

**Architecture:** All game logic moves into `game.tick(&InputState)`. The main loop accumulates key-up/key-down events into `InputState` between ticks and passes it to the game each frame. Keyboard enhancement gives us real key-release events; stacked ticks are drained and processed before each render.

**Tech Stack:** Rust, crossterm 0.28 (`PushKeyboardEnhancementFlags`), ratatui 0.29, insta (snapshot tests).

---

## File Map

| File | Change |
|------|--------|
| `src/constants.rs` | **Create** — all numeric game constants |
| `src/input.rs` | **Modify** — add `GameKey`, `InputState`; keep `GameAction` temporarily |
| `src/game.rs` | **Modify** — new types (`PiecePhase`, `HorizDir`), new `Game` fields, implement `tick(&InputState)`, remove `handle_action` |
| `src/main.rs` | **Modify** — keyboard enhancement, 60Hz timer, `AppEvent` variants, input accumulation, stacked-tick drain |
| `src/tests.rs` | **Modify** — replace `handle_action` calls with tick-based helpers; update snapshots |

---

## Task 1: Add `constants.rs`

**Files:**
- Create: `src/constants.rs`
- Modify: `src/main.rs` (add `mod constants;`)

- [ ] **Create `src/constants.rs`**

```rust
pub const GRAVITY_DELAY: u32 = 30; // ticks per row
pub const LOCK_DELAY: u32 = 30;    // ticks on floor before locking
pub const SPAWN_DELAY: u32 = 30;   // ticks between lock and spawn (ARE)
pub const DAS_CHARGE: u32 = 16;    // ticks before auto-repeat
pub const DAS_REPEAT: u32 = 6;     // ticks between auto-repeat steps
```

- [ ] **Add `mod constants;` to `src/main.rs`** (alongside the other `mod` declarations at the top)

- [ ] **Verify it compiles**

```bash
cargo check
```
Expected: no errors.

- [ ] **Commit**

```bash
git add src/constants.rs src/main.rs
git commit -m "Add constants.rs"
```

---

## Task 2: Add `GameKey` and `InputState` to `input.rs`

`GameAction` stays for now (used by `handle_action` in game.rs). `GameKey` and `InputState` are added alongside it.

**Files:**
- Modify: `src/input.rs`

- [ ] **Add `GameKey` and `InputState` to `src/input.rs`**

```rust
use std::collections::HashSet;
use crossterm::event::KeyCode;

// Existing GameAction stays unchanged for now.

/// Renderer-agnostic held-trackable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKey {
    Left,
    Right,
    RotateCw,
    RotateCcw,
    SoftDrop,
    SonicDrop,
}

/// Snapshot of input state for one tick.
/// `held`: keys currently held down.
/// `just_pressed`: keys that transitioned to pressed this tick (subset of held).
/// Both are HashSets — ordering within a 16ms tick is not meaningful.
pub struct InputState {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}

impl InputState {
    pub fn empty() -> Self {
        Self {
            held: HashSet::new(),
            just_pressed: HashSet::new(),
        }
    }
}

/// Maps a KeyCode to a GameKey. Returns None for unrecognised keys.
pub fn map_game_key(code: KeyCode) -> Option<GameKey> {
    match code {
        KeyCode::Left | KeyCode::Char('h')  => Some(GameKey::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(GameKey::Right),
        KeyCode::Down | KeyCode::Char('j')  => Some(GameKey::SoftDrop),
        KeyCode::Char(' ')                  => Some(GameKey::SonicDrop),
        KeyCode::Char('x')                  => Some(GameKey::RotateCw),
        KeyCode::Char('z')                  => Some(GameKey::RotateCcw),
        _ => None,
    }
}
```

- [ ] **Verify it compiles**

```bash
cargo check
```
Expected: no errors (old `GameAction` / `map_key` still present).

- [ ] **Commit**

```bash
git add src/input.rs
git commit -m "Add GameKey and InputState to input.rs"
```

---

## Task 3: Add new game state types and fields

**Files:**
- Modify: `src/game.rs`

- [ ] **Add `PiecePhase` and `HorizDir` enums to `src/game.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PiecePhase {
    Falling,
    Locking { ticks_left: u32 },
    Spawning { ticks_left: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizDir {
    Left,
    Right,
}
```

- [ ] **Add new fields to the `Game` struct**

```rust
pub struct Game {
    pub board: Board,
    pub active: Piece,
    pub next: Piece,
    pub level: u32,
    pub lines: u32,
    pub game_over: bool,
    pub randomizer: Randomizer,
    // New fields:
    pub piece_phase: PiecePhase,
    pub gravity_counter: u32,
    pub das_direction: Option<HorizDir>,
    pub das_counter: u32,
    pub rotation_buffer: Option<RotationDirection>,
}
```

- [ ] **Initialise new fields in `Game::new()`**

```rust
piece_phase: PiecePhase::Falling,
gravity_counter: 0,
das_direction: None,
das_counter: 0,
rotation_buffer: None,
```

- [ ] **Verify it compiles**

```bash
cargo check
```

- [ ] **Run existing tests to confirm nothing is broken**

```bash
cargo test
```
Expected: all existing tests pass.

- [ ] **Commit**

```bash
git add src/game.rs
git commit -m "Add PiecePhase, HorizDir, and new Game fields"
```

---

## Task 4: Implement `game.tick(&InputState)` and remove `handle_action`

This is the core of the refactor. Implement each tick phase, then remove `handle_action` and the old `tick()`.

**Files:**
- Modify: `src/game.rs`
- Modify: `src/tests.rs` (add tick-based helpers; existing snapshot tests will need updating in Task 6)

- [ ] **Add imports to `src/game.rs`**

```rust
use crate::constants::{DAS_CHARGE, DAS_REPEAT, GRAVITY_DELAY, LOCK_DELAY, SPAWN_DELAY};
use crate::input::{GameKey, InputState};
```

- [ ] **Replace `pub fn tick(&mut self)` and `pub fn handle_action(...)` with `pub fn tick(&mut self, input: &InputState)`**

The full implementation:

```rust
pub fn tick(&mut self, input: &InputState) {
    if self.game_over {
        return;
    }

    // Phase 1: Spawn delay — buffer rotation inputs, count down, then spawn.
    if let PiecePhase::Spawning { ticks_left } = &mut self.piece_phase {
        if input.just_pressed.contains(&GameKey::RotateCw) {
            self.rotation_buffer = Some(RotationDirection::Clockwise);
        } else if input.just_pressed.contains(&GameKey::RotateCcw) {
            self.rotation_buffer = Some(RotationDirection::Counterclockwise);
        }
        if *ticks_left == 0 {
            self.spawn_piece();
        } else {
            *ticks_left -= 1;
        }
        return; // No other input processed during spawn delay.
    }

    // Phase 2: Rotation (instant, not held).
    if input.just_pressed.contains(&GameKey::RotateCw) {
        self.try_rotate(RotationDirection::Clockwise);
    } else if input.just_pressed.contains(&GameKey::RotateCcw) {
        self.try_rotate(RotationDirection::Counterclockwise);
    }

    // Phase 3: Horizontal DAS.
    let horiz = if input.held.contains(&GameKey::Left) {
        Some(HorizDir::Left)
    } else if input.held.contains(&GameKey::Right) {
        Some(HorizDir::Right)
    } else {
        None
    };

    match horiz {
        None => {
            self.das_direction = None;
            self.das_counter = 0;
        }
        Some(dir) => {
            if self.das_direction != Some(dir) {
                // Direction changed or newly pressed: move immediately, reset counter.
                self.das_direction = Some(dir);
                self.das_counter = 0;
                let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                self.try_move(dcol, 0);
            } else {
                self.das_counter += 1;
                if self.das_counter >= DAS_CHARGE
                    && (self.das_counter - DAS_CHARGE) % DAS_REPEAT == 0
                {
                    let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                    self.try_move(dcol, 0);
                }
            }
        }
    }

    // Phase 4: Sonic drop (Space) — drop to floor, enter lock delay.
    if input.just_pressed.contains(&GameKey::SonicDrop) {
        while self.try_move(0, 1) {}
        self.piece_phase = PiecePhase::Locking {
            ticks_left: LOCK_DELAY,
        };
        return;
    }

    // Phase 5: Soft drop (Down) — bypass lock delay or advance gravity.
    if input.held.contains(&GameKey::SoftDrop) {
        match self.piece_phase {
            PiecePhase::Locking { .. } => {
                self.lock_piece();
                return;
            }
            _ => {
                self.try_move(0, 1);
            }
        }
    }

    // Phase 6: Gravity.
    self.gravity_counter += 1;
    if self.gravity_counter >= GRAVITY_DELAY {
        self.gravity_counter = 0;
        self.try_move(0, 1);
    }

    // Phase 7: Lock state transitions.
    let on_floor = !self.fits(self.active.col, self.active.row + 1, self.active.rotation);
    match self.piece_phase {
        PiecePhase::Falling => {
            if on_floor {
                self.piece_phase = PiecePhase::Locking {
                    ticks_left: LOCK_DELAY,
                };
            }
        }
        PiecePhase::Locking { ref mut ticks_left } => {
            if !on_floor {
                // Piece moved off its resting surface.
                self.piece_phase = PiecePhase::Falling;
            } else if *ticks_left == 0 {
                self.lock_piece();
            } else {
                *ticks_left -= 1;
            }
        }
        PiecePhase::Spawning { .. } => unreachable!(),
    }
}
```

- [ ] **Update `lock_piece` to buffer rotation, transition to `Spawning`, and preserve DAS**

`lock_piece` must now take `input: &InputState` so it can capture any held rotation key into `rotation_buffer` at the moment of locking. Update all call sites in `tick` to pass `input`.

```rust
fn lock_piece(&mut self, input: &InputState) {
    for (dc, dr) in self.active.cells() {
        let c = (self.active.col + dc) as usize;
        let r = (self.active.row + dr) as usize;
        if r < BOARD_ROWS {
            self.board[r][c] = Some(self.active.kind);
        }
    }
    self.clear_lines();
    // Buffer any held rotation key so it applies when the next piece spawns.
    if input.held.contains(&GameKey::RotateCw) {
        self.rotation_buffer = Some(RotationDirection::Clockwise);
    } else if input.held.contains(&GameKey::RotateCcw) {
        self.rotation_buffer = Some(RotationDirection::Counterclockwise);
    }
    // DAS charge carries over to the next piece (DAS buffering).
    // das_direction and das_counter are intentionally NOT reset here.
    // Queue the next piece; it will be spawned after SPAWN_DELAY ticks.
    let next_kind = self.randomizer.next();
    self.next = Piece::new(next_kind);
    self.piece_phase = PiecePhase::Spawning {
        ticks_left: SPAWN_DELAY,
    };
}
```

Update both `lock_piece()` call sites inside `tick` to `lock_piece(input)` (one in the soft-drop branch, one in the lock-state-transitions branch).

- [ ] **Add `spawn_piece` method**

```rust
fn spawn_piece(&mut self) {
    let next_kind = self.randomizer.next();
    self.active = std::mem::replace(&mut self.next, Piece::new(next_kind));
    self.active.col = 3;
    self.active.row = 0;
    self.gravity_counter = 0;
    self.piece_phase = PiecePhase::Falling;
    // Apply buffered rotation if any.
    if let Some(dir) = self.rotation_buffer.take() {
        self.try_rotate(dir);
    }
    if !self.fits(self.active.col, self.active.row, self.active.rotation) {
        self.game_over = true;
    }
}
```

- [ ] **Remove `handle_action` and the old gravity-only `tick()`**

Delete both methods entirely.

- [ ] **Verify it compiles**

```bash
cargo check
```
Expected: errors in `main.rs` and `tests.rs` referencing `handle_action` / `GameAction` — expected, will be fixed in subsequent tasks.

- [ ] **Fix `main.rs` temporarily** — comment out the `AppEvent::Input` arm and `AppEvent::Tick => game.tick()` line so it compiles while tests are updated. (Full main.rs refactor is Task 5.)

- [ ] **Commit**

```bash
git add src/game.rs src/main.rs
git commit -m "Implement tick(&InputState), remove handle_action"
```

---

## Task 5: Refactor `main.rs` — 60Hz loop with keyboard enhancement

**Files:**
- Modify: `src/main.rs`

- [ ] **Update `AppEvent`**

```rust
#[derive(Debug)]
enum AppEvent {
    KeyDown(GameKey),
    KeyUp(GameKey),
    Tick,
    Quit,
}
```

- [ ] **Enable keyboard enhancement in `main()`** (before `run()` call)

```rust
use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags};

// After enable_raw_mode():
stdout().execute(PushKeyboardEnhancementFlags(
    KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
))?;
```

And pop it in teardown (before `disable_raw_mode()`):

```rust
stdout().execute(PopKeyboardEnhancementFlags)?;
```

- [ ] **Update the timer thread to 60Hz**

```rust
const TICK_RATE_MS: u64 = 16; // ~60Hz
```

- [ ] **Update the input thread to send `KeyDown`/`KeyUp`**

```rust
use crossterm::event::{KeyEventKind};
use crate::input::map_game_key;

thread::spawn(move || {
    loop {
        if event::poll(Duration::from_millis(5)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                let app_event = match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => AppEvent::Quit,
                    other => match map_game_key(other) {
                        Some(game_key) => match key.kind {
                            KeyEventKind::Press | KeyEventKind::Repeat => AppEvent::KeyDown(game_key),
                            KeyEventKind::Release => AppEvent::KeyUp(game_key),
                        },
                        None => continue,
                    },
                };
                if input_tx.send(app_event).is_err() {
                    break;
                }
            }
        }
    }
});
```

- [ ] **Update the main loop to accumulate `InputState` and handle stacked ticks**

```rust
use crate::input::{InputState, GameKey};
use std::collections::HashSet;

let mut held: HashSet<GameKey> = HashSet::new();
let mut just_pressed: HashSet<GameKey> = HashSet::new();

loop {
    match rx.recv()? {
        AppEvent::Quit => break,
        AppEvent::KeyDown(key) => {
            held.insert(key);
            just_pressed.insert(key);
        }
        AppEvent::KeyUp(key) => {
            held.remove(&key);
        }
        AppEvent::Tick => {
            // Drain remaining queued events (including stacked Ticks).
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    AppEvent::KeyDown(key) => {
                        held.insert(key);
                        just_pressed.insert(key);
                    }
                    AppEvent::KeyUp(key) => {
                        held.remove(&key);
                    }
                    AppEvent::Tick => {
                        let input = InputState {
                            held: held.clone(),
                            just_pressed: just_pressed.clone(),
                        };
                        game.tick(&input);
                        just_pressed.clear();
                    }
                    AppEvent::Quit => {
                        quit = true;
                    }
                }
            }
            if quit { break; }
            let input = InputState { held: held.clone(), just_pressed };
            game.tick(&input);
            just_pressed = HashSet::new();
            terminal.draw(|frame| renderer::render(frame, &game))?;
        }
    }
    Ok(())
}
```

Add `let mut quit = false;` immediately before the `loop {` line.

- [ ] **Remove unused imports** (`GameAction`, old `map_key`, `TICK_RATE_MS` constant if inlined)

- [ ] **Verify game runs**

```bash
cargo run
```
Expected: game starts, pieces fall at ~2 rows/sec, keyboard input works, DAS activates after ~0.27s hold.

- [ ] **Commit**

```bash
git add src/main.rs
git commit -m "Refactor main loop: 60Hz, keyboard enhancement, InputState accumulation"
```

---

## Task 6: Update tests

The existing snapshot tests call `handle_action(GameAction::...)` which no longer exists. Replace with tick-based helpers.

**Files:**
- Modify: `src/tests.rs`

- [ ] **Add tick helpers to `src/tests.rs`**

```rust
use crate::input::{GameKey, InputState};
use std::collections::HashSet;

/// Simulate a single keypress (held for one tick).
fn press(game: &mut Game, key: GameKey) {
    game.tick(&InputState {
        held: HashSet::from([key]),
        just_pressed: HashSet::from([key]),
    });
}

/// Simulate N ticks with a set of keys held (not newly pressed).
fn hold(game: &mut Game, keys: &[GameKey], ticks: u32) {
    let input = InputState {
        held: keys.iter().copied().collect(),
        just_pressed: HashSet::new(),
    };
    for _ in 0..ticks {
        game.tick(&input);
    }
}

/// Simulate N ticks with no input.
fn idle(game: &mut Game, ticks: u32) {
    let input = InputState::empty();
    for _ in 0..ticks {
        game.tick(&input);
    }
}
```

- [ ] **Update all `handle_action(GameAction::RotateCw)` calls to `press(game, GameKey::RotateCw)`** (and similarly for RotateCcw, MoveLeft, MoveRight, HardDrop→SonicDrop)

In `rotation_snap`: replace `game.handle_action(GameAction::RotateCw)` with `press(&mut game, GameKey::RotateCw)`.

In `movement_snap`: replace `game.handle_action(action)` — `action` was a `GameAction`, now pass a `GameKey`. Update the signature: `fn movement_snap(kind: PieceKind, key: GameKey)`.

In `wall_kick_snap`: replace `game.handle_action(action)` with `press(&mut game, action)` where `action` is now a `GameKey`.

In `center_col_snap`: replace `cw.handle_action(GameAction::RotateCw)` with `press(&mut cw, GameKey::RotateCw)` (and similarly for ccw).

In `i_piece_no_wall_kicks`: same replacement.

In `l_j_asymmetric_wall_kicks`: same.

In `cw_ccw_equivalence`: replace `game.handle_action(GameAction::RotateCw)` with `press(&mut game, GameKey::RotateCw)` etc.

- [ ] **Update `make_game` if needed** — `Game::new()` now initialises the new fields automatically, so `make_game` should still work. Verify `active.row = 8` puts the piece far from the floor so gravity/lock phases don't interfere with rotation/movement tests.

- [ ] **Run tests and accept updated snapshots**

```bash
cargo insta test
cargo insta accept
```

Expected: all snapshot content unchanged (the visual output of rotations and kicks is the same; only the internal call path changed).

- [ ] **Add new tests for tick-system behaviour**

Before writing these tests, expose `try_move` for test use. In `src/game.rs`, change:

```rust
fn try_move(&mut self, dcol: i32, drow: i32) -> bool {
```
to:
```rust
pub(crate) fn try_move(&mut self, dcol: i32, drow: i32) -> bool {
```

Then write the tests:

```rust
#[test]
fn lock_delay_prevents_immediate_lock() {
    let mut game = make_game(PieceKind::T);
    // Drop piece to floor (T rot0 occupies rows 1-2 of its box, so row=17 puts it at board rows 18-19)
    while game.try_move(0, 1) {}
    // First idle tick: transitions from Falling to Locking
    idle(&mut game, 1);
    assert_eq!(game.piece_phase, PiecePhase::Locking { ticks_left: LOCK_DELAY - 1 });
    // After remaining LOCK_DELAY-1 ticks, piece locks and transitions to Spawning
    idle(&mut game, LOCK_DELAY - 1);
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }));
}

#[test]
fn soft_drop_bypasses_lock_delay() {
    let mut game = make_game(PieceKind::T);
    // Position piece on floor
    game.active.row = BOARD_ROWS as i32 - 3; // near bottom
    idle(&mut game, 1); // trigger Locking state
    // Soft drop should lock immediately
    press(&mut game, GameKey::SoftDrop);
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }));
}

#[test]
fn sonic_drop_enters_lock_delay() {
    let mut game = make_game(PieceKind::T);
    press(&mut game, GameKey::SonicDrop);
    assert!(matches!(game.piece_phase, PiecePhase::Locking { .. }));
}

#[test]
fn das_activates_after_charge() {
    let mut game = make_game(PieceKind::T);
    let start_col = game.active.col;
    // First press moves immediately
    press(&mut game, GameKey::Left);
    assert_eq!(game.active.col, start_col - 1);
    // Hold for DAS_CHARGE - 1 more ticks: no additional movement
    hold(&mut game, &[GameKey::Left], DAS_CHARGE - 1);
    assert_eq!(game.active.col, start_col - 1);
    // One more tick triggers auto-repeat
    hold(&mut game, &[GameKey::Left], 1);
    assert_eq!(game.active.col, start_col - 2);
}

#[test]
fn das_preserved_across_spawn() {
    // The invariant: if Left is held continuously through a lock, the DAS counter
    // is not reset on piece spawn — the new piece inherits the charged DAS state.
    let mut game = make_game(PieceKind::T);
    // Drop piece to floor while holding Left throughout.
    while game.try_move(0, 1) {}
    // Charge DAS fully (first press moves immediately, then charge).
    press(&mut game, GameKey::Left);
    hold(&mut game, &[GameKey::Left], DAS_CHARGE);
    let das_counter_at_charge = game.das_counter;
    assert_eq!(game.das_direction, Some(HorizDir::Left));
    // Lock the piece while still holding Left (so DAS state is preserved).
    hold(&mut game, &[GameKey::Left], LOCK_DELAY + 1); // enter Locking then lock → Spawning
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }));
    // DAS direction and a non-zero counter must survive the lock.
    assert_eq!(game.das_direction, Some(HorizDir::Left));
    assert!(game.das_counter >= das_counter_at_charge,
        "DAS counter should not have been reset on lock");
    // Continue holding Left through spawn delay — piece spawns and moves left immediately.
    let col_before_spawn = 3i32; // new piece spawns at col 3
    hold(&mut game, &[GameKey::Left], SPAWN_DELAY + 1);
    assert!(game.active.col < col_before_spawn,
        "expected leftward movement from inherited DAS, got col={}", game.active.col);
}

#[test]
fn rotation_buffer_applied_on_spawn() {
    let mut game = make_game(PieceKind::T);
    // Lock current piece
    game.active.row = BOARD_ROWS as i32 - 3;
    idle(&mut game, 1);
    idle(&mut game, LOCK_DELAY); // now in Spawning
    // Press rotate during spawn delay
    press(&mut game, GameKey::RotateCw);
    assert_eq!(game.rotation_buffer, Some(RotationDirection::Clockwise));
    // After spawn delay, piece should be in rotation 1
    idle(&mut game, SPAWN_DELAY);
    assert_eq!(game.active.rotation, 1);
}
```

Note: `try_move` may need to be exposed as `pub(crate)` for some tests, or tests can position the piece by setting `game.active.row` directly (which `make_game` already allows).

- [ ] **Run all tests**

```bash
cargo test
```
Expected: all pass.

- [ ] **Commit**

```bash
git add src/tests.rs
git commit -m "Update tests for tick-based input system"
```

---

## Task 7: Clean up — remove `GameAction` and old `map_key`

**Files:**
- Modify: `src/input.rs`

- [ ] **Delete `GameAction` enum and `map_key` function from `src/input.rs`**

These are now dead code. Remove them entirely.

- [ ] **Verify no remaining references**

```bash
grep -r "GameAction\|map_key" src/
```
Expected: no output.

- [ ] **Compile and test**

```bash
cargo test
```
Expected: all pass.

- [ ] **Commit**

```bash
git add src/input.rs
git commit -m "Remove deprecated GameAction and map_key"
```
