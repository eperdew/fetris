# Agent Guide — fetris

Reimplementation of [TGM1](https://tetris.wiki/Tetris_The_Grand_Master) in Rust using [macroquad](https://macroquad.rs/).

## Project Structure

| File | Purpose |
|---|---|
| `src/main.rs` | Entry point, game loop, input mapping |
| `src/game.rs` | All game logic: piece phases, gravity, locking, line clearing |
| `src/renderer.rs` | Rendering only — pure function over `Game` state, no mutable state |
| `src/piece.rs` | Piece shapes and rotation tables |
| `src/constants.rs` | Tuning constants: gravity, delays, particle animation |
| `src/input.rs` | Input abstraction (`InputState`, `GameKey`) |
| `src/randomizer.rs` | Piece randomizer |
| `src/tests.rs` | All tests |

## Key Concepts

**Piece phases** (`PiecePhase` in `game.rs`): the active piece is always in one of four phases — `Falling`, `Locking`, `LineClearDelay`, or `Spawning`. Phase transitions drive all timing logic.

**Board representation**: `Board = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS]`. Row 0 is the top, row 19 is the bottom.

**Tick rate**: game logic runs at 60 ticks/second, decoupled from render rate. All delays (gravity, lock, line clear, ARE) are measured in ticks.

**Gravity**: fractional G system — gravity accumulates in units of G/256 per tick, matching TGM1 values from `GRAVITY_TABLE` in `constants.rs`.

**Line clear animation**: when lines are cleared, rows are stored in `rows_pending_compaction` and left intact in the board. The renderer draws them as physics particles during `LineClearDelay`. Board compaction happens at the end of the delay.

**Renderer is stateless**: `renderer::render` is a pure function of `&Game`. Particle positions are computed from `ticks_left` each frame — no mutable renderer state.

## Build & Test

```sh
cargo build
cargo test
cargo run --release
```

## Conventions

- Install the pre-commit hook before committing: `cp hooks/pre-commit .git/hooks/pre-commit`
- Tests use `insta` for snapshot assertions — run `cargo insta accept` to accept new snapshots, never inline them manually
- New feature work goes on a branch; use a git worktree under `.worktrees/` (already git-ignored)
- Specs live in `docs/superpowers/specs/`, implementation plans in `docs/superpowers/plans/`