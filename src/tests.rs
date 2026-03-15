use crate::game::{BOARD_COLS, BOARD_ROWS, Game};
use crate::input::GameAction;
use crate::piece::{Piece, PieceKind};

fn make_i_game() -> Game {
    let mut game = Game::new();
    game.board = [[None; BOARD_COLS]; BOARD_ROWS];
    game.active = Piece::new(PieceKind::I);
    game.active.col = 3;
    game.active.row = 8;
    game.next = Piece::new(PieceKind::I);
    game
}

/// Renders just the board grid. If `prev_cells` is given (absolute board positions of the
/// previous active piece), those cells are shown as `. `; the current active piece is `[]`.
/// Overlapping cells show `[]` (current wins).
fn board_str(game: &Game, prev_cells: Option<&[(i32, i32)]>) -> String {
    let active: [(i32, i32); 4] = game
        .active
        .cells()
        .map(|(dc, dr)| (game.active.col + dc, game.active.row + dr));

    let mut out = String::from("  ┌────────────────────┐\n");
    for r in 0..BOARD_ROWS {
        if r % 5 == 0 {
            out.push_str(&format!("{r:2}"));
        } else {
            out.push_str("  ");
        }
        out.push('│');
        for c in 0..BOARD_COLS {
            let pos = (c as i32, r as i32);
            out.push_str(if active.contains(&pos) {
                "[]"
            } else if prev_cells.is_some_and(|p| p.contains(&pos)) {
                ". "
            } else if game.board[r][c].is_some() {
                "[]"
            } else if r % 5 == 0 {
                "- "
            } else {
                "  "
            });
        }
        out.push_str("│\n");
    }
    out.push_str("20└────────────────────┘");
    out
}

fn active_abs(game: &Game) -> Vec<(i32, i32)> {
    game.active
        .cells()
        .into_iter()
        .map(|(dc, dr)| (game.active.col + dc, game.active.row + dr))
        .collect()
}

#[test]
fn i_piece_rotations() {
    let mut game = make_i_game();
    let mut snap = String::new();

    snap.push_str("── rotation 0 ──\n");
    snap.push_str(&board_str(&game, None));

    for rot in 1..=4 {
        let prev = active_abs(&game);
        game.handle_action(GameAction::RotateCw);
        snap.push_str(&format!(
            "\n\n── rotation {rot} (diff from {prev_rot}) ──\n",
            prev_rot = rot - 1,
        ));
        snap.push_str(&board_str(&game, Some(&prev)));
    }

    insta::assert_snapshot!(snap, @"
    ── rotation 0 ──
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │      [][][][]      │
    10│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘

    ── rotation 1 (diff from 0) ──
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │          []        │
      │      . . [].       │
    10│- - - - - []- - - - │
      │          []        │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘

    ── rotation 2 (diff from 1) ──
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │          .         │
      │          .         │
    10│- - - [][][][]- - - │
      │          .         │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘

    ── rotation 3 (diff from 2) ──
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │        []          │
      │        []          │
    10│- - - . []. . - - - │
      │        []          │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘

    ── rotation 4 (diff from 3) ──
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
     5│- - - - - - - - - - │
      │                    │
      │                    │
      │        .           │
      │      [][][][]      │
    10│- - - - . - - - - - │
      │        .           │
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │                    │
    20└────────────────────┘
    ");
}
