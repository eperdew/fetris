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

/// Renders the board as a Vec of lines. If `prev_cells` is given (absolute board positions),
/// those cells show `. `; current active piece shows `[]`; overlap shows `[]` (current wins).
fn board_lines(game: &Game, prev_cells: Option<&[(i32, i32)]>) -> Vec<String> {
    let active: [(i32, i32); 4] = game
        .active
        .cells()
        .map(|(dc, dr)| (game.active.col + dc, game.active.row + dr));

    let mut lines = vec!["  ┌────────────────────┐".to_string()];
    for r in 0..BOARD_ROWS {
        let mut row = if r % 5 == 0 { format!("{r:2}") } else { "  ".to_string() };
        row.push('│');
        for c in 0..BOARD_COLS {
            let pos = (c as i32, r as i32);
            row.push_str(if active.contains(&pos) {
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
        row.push('│');
        lines.push(row);
    }
    lines.push("20└────────────────────┘".to_string());
    lines
}

fn active_abs(game: &Game) -> Vec<(i32, i32)> {
    game.active
        .cells()
        .into_iter()
        .map(|(dc, dr)| (game.active.col + dc, game.active.row + dr))
        .collect()
}

fn side_by_side(boards: &[(String, Vec<String>)]) -> String {
    const SEP: &str = "   ";
    let board_width = boards[0].1[0].len();
    let height = boards[0].1.len();

    let mut out = String::new();
    for (i, (label, _)) in boards.iter().enumerate() {
        if i > 0 { out.push_str(SEP); }
        out.push_str(&format!("{label:<board_width$}"));
    }
    out.push('\n');
    for r in 0..height {
        for (i, (_, lines)) in boards.iter().enumerate() {
            if i > 0 { out.push_str(SEP); }
            out.push_str(&lines[r]);
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[test]
fn i_piece_rotations() {
    let mut game = make_i_game();
    let mut boards = Vec::new();

    for rot in 0..4 {
        let prev = active_abs(&game);
        game.handle_action(GameAction::RotateCw);
        boards.push((format!("{}→{}", rot, (rot + 1) % 4), board_lines(&game, Some(&prev))));
    }

    insta::assert_snapshot!(side_by_side(&boards), @"
    0→1                                                                    1→2                                                                    2→3                                                                    3→0                                                                 
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │          []        │     │          .         │     │        []          │     │        .           │
      │      . . [].       │     │          .         │     │        []          │     │      [][][][]      │
    10│- - - - - []- - - - │   10│- - - [][][][]- - - │   10│- - - . []. . - - - │   10│- - - - . - - - - - │
      │          []        │     │          .         │     │        []          │     │        .           │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘   20└────────────────────┘   20└────────────────────┘
    ");
}
