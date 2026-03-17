# fetris — agent guide

Rust Tetris clone using [Ratatui](https://ratatui.rs/) for terminal rendering.

## Source layout

| File | Purpose |
|------|---------|
| `src/piece.rs` | Piece shapes and rotation tables |
| `src/game.rs` | Game state, physics, ARS rotation logic |
| `src/randomizer.rs` | TGM-style piece randomizer |
| `src/input.rs` | `GameAction` enum (keyboard → action) |
| `src/renderer.rs` | Ratatui rendering |
| `src/tests.rs` | All tests (inline snapshots only) |

## Key types

- `Board = [[Option<PieceKind>; BOARD_COLS]; BOARD_ROWS]` — `None` = empty, `Some(kind)` = locked cell.
- `Piece` — kind, rotation (0–3), and `(col, row)` of the bounding box top-left corner.
- `piece::cells(kind, rotation) -> [(i32, i32); 4]` — `(dc, dr)` offsets from the piece's `(col, row)`. The shapes are defined as ASCII diagrams in `piece.rs` and parsed at compile time.

## Randomizer (TGM)

Implemented in `src/randomizer.rs`. `Game` owns a `Randomizer` instance and calls `randomizer.next()` each time a new piece is needed.

- History of 4 pieces, initialized to `[Z; 4]`.
- Each draw: try up to 4 times to pick a piece not in the history; settle with the last attempt if all fail.
- First piece is restricted to `{I, T, J, L}` (never S, Z, or O) to avoid a forced overhang.

## Rotation system (ARS)

Implemented in `game.rs::try_rotate`:

1. Try basic rotation in place.
2. I-piece: never kicks, stop.
3. L/J/T from a 3-wide orientation (rot 0 or 2): check `center_column_blocked_first(new_rot)` — scans destination rotation cells left-to-right, top-to-bottom; if the first one that collides with the board is at `dc == 1` (center column), suppress kicks for this direction.
4. Try kick right (+1 col), then left (−1 col).

The center-column check is direction-aware: CW and CCW each check their own destination rotation, so the same board position can kick in one direction but not the other.

## Testing

All tests live in `src/tests.rs` and use [insta](https://insta.rs/) snapshot testing.

**Always use inline snapshots** (`@"..."`). Never use external `.snap` files.

To accept new snapshots: `cargo insta accept` (not manual editing).

### Test helpers

- `make_game(kind)` — empty board, piece at col=3, row=8.
- `board_from_ascii(diagram)` — parses `.`/`O` ASCII, bottom-aligned to the board.
- `board_lines(game, prev_cells)` — renders board as strings: `[]` active piece, `'.` ghost (prev position), `##` locked cells, `- ` row-5 guide marks.
- `side_by_side(boards)` — joins multiple board renderings horizontally.
- `wall_kick_snap(kind)` — shows all wall-flush positions where a kick actually occurred.
- `center_col_snap(kind, rot, obstacles)` — shows CW and CCW attempts with board obstacles placed at given offsets from the piece.
