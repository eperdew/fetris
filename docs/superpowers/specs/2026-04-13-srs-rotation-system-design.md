# SRS Rotation System Design

**Date:** 2026-04-13
**Status:** Approved

## Summary

Implement the Super Rotation System (SRS) as a proper second rotation system, alongside the existing ARS implementation. This requires refactoring how piece shapes are owned (moving them from `piece.rs` into the rotation system), introducing a `RotationSystem` trait, and filling in the SRS kick tables per the [Tetris wiki spec](https://tetris.wiki/Super_Rotation_System).

---

## Architecture

### Core change: `RotationSystem` becomes a trait

`rotation_system.rs` defines:

```rust
pub trait RotationSystem {
    fn cells(&self, kind: PieceKind, rotation: usize) -> [(i32, i32); 4];
    fn fits(&self, board: &Board, kind: PieceKind, col: i32, row: i32, rotation: usize) -> bool;
    fn try_rotate(&self, piece: &Piece, direction: RotationDirection, board: &Board) -> Option<Piece>;
}
```

`fits` has a default implementation using `self.cells()`. `try_rotate` returns `Some(new_piece)` with updated `col` and `rotation` on success, `None` if no kick position fits. This avoids the borrow conflict that would arise from taking `&mut Game` while `Game` owns the rotation system, and makes rotation logic independently testable.

Two concrete types implement the trait: `pub struct Ars;` and `pub struct Srs;`.

### `rotation_system::Kind` enum

A `pub enum Kind { Ars, Srs }` in `rotation_system.rs` replaces the old `RotationSystem` enum for use in the menu. It provides:

```rust
impl Kind {
    pub fn create(self) -> Box<dyn RotationSystem> { ... }
}
```

### `Game` holds `Box<dyn RotationSystem>`

`game.rs` stores `rotation_system: Box<dyn RotationSystem>`. Dynamic dispatch is negligible here — rotation fires at most a few times per second.

---

## `piece.rs` changes

Remove:
- Free function `cells(kind, rotation)`
- Free function `parse_rotations(diagram)`
- `Piece::cells()` method

`Piece` becomes a pure data struct: `{ kind, rotation, col, row }`.

---

## `rotation_system.rs` changes

### Compile-time shape parsing

```rust
const fn parse_rotations(diagram: &str) -> [[(i32, i32); 4]; 4]
```

Implemented with byte-level `while` loops (no iterator methods — fully const-stable in Rust 1.85 / edition 2024). Scans bytes, finds non-empty lines, splits on `|` by scanning for the `|` byte, trims leading/trailing spaces within each segment, records `O` positions as `(col, row)` pairs. Panics (compile-time) if any rotation doesn't have exactly 4 cells.

Shape tables are module-level `const` items:

```rust
const ARS_I: [[(i32, i32); 4]; 4] = parse_rotations("...diagram...");
const SRS_I: [[(i32, i32); 4]; 4] = parse_rotations("...diagram...");
// etc. for all 7 piece kinds × 2 systems
```

### SRS piece shapes

Per the Tetris wiki. Key differences from current ARS shapes:

- **I-piece**: In SRS, rotation 0 occupies row 1 (not row 1) of a 4×4 box, rotation 2 occupies row 2. Rotations 1 and 3 use column 2 and column 1 respectively (asymmetric).
- **O-piece**: Identical in both systems (no kicks needed).
- **JLSTZ**: Subtle offset differences to match canonical SRS spawn/rotation positions.

### SRS kick tables

Per the wiki, wall kicks are defined as five (col, row) offset tests per rotation transition, tried in order. The first position that fits is used. Test 1 is always `(0, 0)` (basic rotation), so it subsumes the current "try basic first" logic.

```rust
// 8 transitions: 0→1, 1→0, 1→2, 2→1, 2→3, 3→2, 3→0, 0→3
// (CW and CCW for each of the 4 base rotations)
const JLSTZ_KICKS: [[(i32, i32); 5]; 8] = [...];
const I_KICKS:     [[(i32, i32); 5]; 8] = [...];
```

O-piece: no kicks (return `None` after basic rotation fails). All other pieces use `JLSTZ_KICKS`.

Transition index mapping: `(from_rotation * 2 + cw_offset)` where CW = 0, CCW = 1, using a small lookup to map `(from, direction)` → index.

### `Ars` implementation

Moves existing `try_rotate_ars` logic into `Ars::try_rotate`. Uses `ARS_*` shape constants via `Ars::cells`. The `center_column_blocked_first` helper moves inside the `Ars` impl block.

### `Srs` implementation

`Srs::cells` dispatches to `SRS_*` shape constants. `Srs::try_rotate` iterates the appropriate kick table and returns the first fitting position.

---

## `game.rs` changes

- `Game::new` takes `Box<dyn RotationSystem>` instead of `RotationSystem` (old enum).
- `game.fits(col, row, rotation)` delegates to `self.rotation_system.fits(&self.board, self.active.kind, col, row, rotation)`.
- `game.try_rotate(direction)` applies `Option<Piece>` returned by `self.rotation_system.try_rotate(...)`.
- `lock_piece` replaces `self.active.cells()` with `self.rotation_system.cells(self.active.kind, self.active.rotation)`.

---

## `menu.rs` changes

- Stores `rotation: rotation_system::Kind` instead of old `RotationSystem` enum.
- `MenuResult::StartGame` carries `rotation: rotation_system::Kind`.
- Caller converts with `.create()` when constructing `Game`.

---

## `tests.rs` changes

- `make_game` constructs with `Box::new(Ars)`.
- `active_abs` calls `game.rotation_system.cells(game.active.kind, game.active.rotation)`.
- Existing ARS snapshot tests should pass unchanged.
- New SRS snapshot tests cover: basic rotation for each piece, wall kicks (left and right wall for each piece kind), I-piece kicks (which differ from JLSTZ).

---

## What is NOT changing

- Game loop, gravity, locking, DAS, scoring, rendering.
- ARS behavior — all existing ARS logic is preserved exactly, just relocated.
- The `RotationDirection` enum stays in `game.rs` (it's also used by game input logic).
