# Types Module Refactor

**Date:** 2026-04-20

## Goals

1. Move all plain, stateless types into a single `src/types.rs` module.
2. Make all stateful types private to their module (or `pub(crate)` where cross-module access is required).

## What Moves to `src/types.rs`

The following types — along with their `impl` blocks — move to `types.rs`:

| Type(s) | Currently in |
|---------|-------------|
| `PieceKind`, `Piece` | `piece.rs` |
| `Board`, `BOARD_COLS`, `BOARD_ROWS`, `PiecePhase`, `HorizDir`, `RotationDirection` | `game.rs` |
| `GameKey`, `InputState` | `input.rs` |
| `GameMode`, `MenuScreen`, `MenuInput`, `MenuResult` | `menu.rs` |
| `GameConfig` (struct + `Default` impl only) | `menu.rs` |
| `Grade`, `JudgeEvent` | `judge.rs` |
| `HiScoreEntry` | `hiscores.rs` |
| `Kind` | `rotation_system.rs` |

### Special case: `GameConfig::load` / `GameConfig::save`

These methods depend on `Storage` (a stateful type). To avoid `types.rs` importing a stateful type, the `impl GameConfig { load, save }` block stays in `menu.rs`. Rust permits split impl blocks within the same crate.

### Deleted modules

`piece.rs` and `input.rs` become empty after the migration and are deleted. `mod piece;` and `mod input;` are removed from `main.rs`.

## Visibility Changes for Stateful Types

| Type | Module | New visibility |
|------|--------|---------------|
| `Game` | `game.rs` | `pub(crate)` |
| `Menu` | `menu.rs` | `pub(crate)` |
| `Judge` | `judge.rs` | `pub(crate)` |
| `Storage` | `storage.rs` | `pub(crate)` (both `imp::Storage` variants) |
| `Renderer` | `renderer.rs` | `pub(crate)` |
| `Randomizer` | `randomizer.rs` | private (no visibility modifier) |

`Randomizer` can be fully private — it is never used outside `game.rs`.

`Game`'s fields remain `pub` for now. Adding accessor methods to replace direct field access (primarily from `renderer.rs`) is a follow-on refactor.

## Import Convention

All modules that currently import stateless types from `piece`, `input`, `menu`, `judge`, `hiscores`, or `rotation_system` switch to `use crate::types::...`.

Example — `game.rs` before:
```rust
use crate::piece::{Piece, PieceKind};
use crate::input::{GameKey, InputState};
use crate::menu::GameMode;
```

After:
```rust
use crate::types::{GameKey, GameMode, HorizDir, InputState, Piece, PieceKind, PiecePhase, RotationDirection};
```

`main.rs` adds `mod types;` and removes `mod piece;` and `mod input;`.

## Out of Scope

- Adding accessor methods to `Game` to replace direct field access from `renderer.rs` (follow-on refactor).
- The `RotationSystem` trait and `Ars`/`Srs` unit structs remain in `rotation_system.rs` — they are behavior objects, not plain data types.
