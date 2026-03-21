# Tick System & Input Refactor — Design Spec

## Overview

Migrate fetris from immediate keypress-driven logic to a deterministic 60Hz tick system. This enables DAS, DAS buffering, rotation buffering, lock delay, spawn delay, and lays groundwork for rollback netcode.

**In scope:** tick loop, gravity (hardcoded), lock delay, spawn delay (ARE), DAS + DAS buffering, rotation buffering, soft drop, sonic drop.
**Out of scope:** gravity changes over level, score, netcode.

---

## Input Layer (`input.rs`)

Replace `GameAction` with `GameKey` — a renderer-agnostic enum of held-trackable keys:

```rust
pub enum GameKey { Left, Right, RotateCw, RotateCcw, SoftDrop, SonicDrop }
```

`InputState` is a plain data struct passed to `game.tick()` each frame:

```rust
pub struct InputState {
    pub held: HashSet<GameKey>,
    pub just_pressed: HashSet<GameKey>,
}
```

`just_pressed` is a `HashSet` (not `Vec`) because ordering within a 16ms tick is not meaningful and duplicates cannot occur with proper key-up/key-down tracking.

`GameAction` and `handle_action` are removed entirely. All game logic flows through `tick(&InputState)`.

---

## Main Loop (`main.rs`)

**Keyboard enhancement:** enable `PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES)` on startup so crossterm delivers `KeyEventKind::Release` events. Pop on teardown.

**AppEvent:**

```rust
enum AppEvent {
    KeyDown(GameKey),
    KeyUp(GameKey),
    Tick,
    Quit,
}
```

**Tick rate:** timer thread fires every 16ms (~60Hz).

**Main loop accumulates `InputState` in the main thread:**

```
loop {
    match rx.recv() {
        KeyDown(key) => { held.insert(key); just_pressed.insert(key); }
        KeyUp(key)   => { held.remove(key); }
        Tick => {
            // drain remaining events (including stacked Ticks) before processing
            while let Ok(ev) = rx.try_recv() { handle ev }
            game.tick(&input_state);
            terminal.draw(...);
            just_pressed.clear();
        }
        Quit => break,
    }
}
```

**Stacked tick handling:** stacked `Tick` events are drained alongside input events in the `try_recv()` loop, running `game.tick()` for each but rendering only once at the end. This prevents the game from falling behind — critical for eventual rollback netcode where clock divergence between clients causes one-sided rollback.

---

## Constants (`constants.rs`)

```rust
pub const GRAVITY_DELAY: u32 = 30;   // ticks per row (1/30 at 60Hz = 2 rows/sec)
pub const LOCK_DELAY: u32 = 30;       // ticks on floor before locking (~0.5s)
pub const SPAWN_DELAY: u32 = 30;      // ARE: ticks between lock and next spawn (~0.5s)
pub const DAS_CHARGE: u32 = 16;       // ticks before auto-repeat activates
pub const DAS_REPEAT: u32 = 6;        // ticks between auto-repeat steps
```

All values are tunable. TGM varies `SPAWN_DELAY` and `GRAVITY_DELAY` by level — those changes are a follow-on.

---

## Game State (`game.rs`)

New fields on `Game`:

```rust
gravity_counter: u32,
piece_phase: PiecePhase,
das_direction: Option<HorizDir>,
das_counter: u32,
rotation_buffer: Option<RotationDirection>,
```

New types:

```rust
enum PiecePhase {
    Falling,
    Locking { ticks_left: u32 },
    Spawning { ticks_left: u32 },   // "ARE" in TGM
}

enum HorizDir { Left, Right }
```

---

## Tick Flow (`game.tick(&InputState)`)

Each tick runs these phases in order:

### 1. Rotation buffer
If `piece_phase == Spawning` and `just_pressed` contains a rotation key, store it in `rotation_buffer`.

### 2. Horizontal input (DAS)
- If a horizontal key was just pressed (direction changed or new press): move immediately, reset `das_counter = 0`, update `das_direction`.
- If horizontal key held: increment `das_counter`. Move when `das_counter == DAS_CHARGE`, then every `DAS_REPEAT` ticks thereafter.
- If no horizontal key held: clear `das_direction`, reset `das_counter = 0`.

### 3. Sonic drop
If `SonicDrop` in `just_pressed`: drop piece to floor via repeated `try_move(0, 1)`. Transition `piece_phase` to `Locking { ticks_left: LOCK_DELAY }`.

### 4. Soft drop
If `SoftDrop` in `just_pressed` or `held`:
- If `piece_phase == Locking`: lock immediately (bypass remaining delay).
- Otherwise: call `try_move(0, 1)` directly (independent of `gravity_counter`).

### 5. Gravity
Increment `gravity_counter`. When it reaches `GRAVITY_DELAY`, call `try_move(0, 1)` and reset to 0.

### 6. Lock state transitions
After movement and gravity:
- If piece cannot move down and `piece_phase == Falling`: transition to `Locking { ticks_left: LOCK_DELAY }`.
- If `piece_phase == Locking`: decrement `ticks_left`. If zero, lock the piece.
- If piece can move down and `piece_phase == Locking`: transition back to `Falling` (piece shifted off its resting surface).

### 7. Spawning phase
When `piece_phase == Spawning`: decrement `ticks_left`. If zero, spawn the queued next piece and apply `rotation_buffer` if set.

---

## Locking a Piece

When a piece locks (either from lock delay expiring or soft drop bypass):
1. Write piece cells to board, clear lines (existing logic).
2. **Preserve** `das_direction` and `das_counter` (DAS buffering — charge carries over to next piece).
3. Check `held` for a rotation key; if present, set `rotation_buffer`.
4. Transition `piece_phase` to `Spawning { ticks_left: SPAWN_DELAY }`.
5. Advance `next` to become the queued active piece (not yet spawned); generate new `next` from randomizer.

---

## Drop Behavior Summary

| Action | Effect |
|--------|--------|
| Hold Left/Right | DAS: immediate move, then auto-repeat after charge |
| SonicDrop (Space) | Drop to floor, start lock delay |
| SoftDrop (Down) on floor | Bypass lock delay, lock immediately |
| SoftDrop (Down) in air | Advance one row (independent of gravity counter) |
| SonicDrop then SoftDrop | Drop to floor, immediately hard lock |

---

## Input Adapter Boundary

`InputState` is defined in `input.rs` with no crossterm dependency. The crossterm-specific code that populates it (keyboard enhancement, key mapping) stays in `main.rs`. `game.rs` never imports crossterm. This boundary makes swapping to a non-terminal renderer (native GUI, web) a matter of writing a new input adapter, not touching game logic.
