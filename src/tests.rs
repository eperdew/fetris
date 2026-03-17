use crate::game::{BOARD_COLS, BOARD_ROWS, Board, Game};
use crate::input::GameAction;
use crate::piece::{Piece, PieceKind};

fn make_game(kind: PieceKind) -> Game {
    let mut game = Game::new();
    game.board = [[None; BOARD_COLS]; BOARD_ROWS];
    game.active = Piece::new(kind);
    game.active.col = 3;
    game.active.row = 8;
    game.next = Piece::new(kind);
    game
}

/// Parses an ASCII diagram of `.` (empty) and `O` (occupied) into a board,
/// aligned to the bottom. The diagram must be exactly BOARD_COLS wide per row.
fn board_from_ascii(diagram: &str) -> Board {
    let mut board = [[None; BOARD_COLS]; BOARD_ROWS];
    let lines: Vec<&str> = diagram
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let offset = BOARD_ROWS.saturating_sub(lines.len());
    for (i, line) in lines.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            if c < BOARD_COLS {
                board[offset + i][c] = if ch == 'O' { Some(PieceKind::O) } else { None };
            }
        }
    }
    board
}

fn rotation_snap(kind: PieceKind) -> String {
    let mut game = make_game(kind);
    let mut boards = Vec::new();
    for rot in 0..4 {
        let prev = active_abs(&game);
        game.handle_action(GameAction::RotateCw);
        boards.push((
            format!("{}→{}", rot, (rot + 1) % 4),
            board_lines(&game, &prev),
        ));
    }
    side_by_side(&boards)
}

/// Renders the board as a Vec of lines. If `prev_cells` is given (absolute board positions),
/// those cells show `. `; current active piece shows `[]`; overlap shows `[]` (current wins).
fn board_lines(game: &Game, prev_cells: &[(i32, i32)]) -> Vec<String> {
    let active: [(i32, i32); 4] = game
        .active
        .cells()
        .map(|(dc, dr)| (game.active.col + dc, game.active.row + dr));

    let mut lines = vec!["  ┌────────────────────┐".to_string()];
    for r in 0..BOARD_ROWS {
        let mut row = if r % 5 == 0 {
            format!("{r:2}")
        } else {
            "  ".to_string()
        };
        row.push('│');
        for c in 0..BOARD_COLS {
            let pos = (c as i32, r as i32);
            row.push_str(if active.contains(&pos) {
                "[]"
            } else if prev_cells.contains(&pos) {
                "'."
            } else if game.board[r][c].is_some() {
                "##"
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
    const VIS_WIDTH: usize = BOARD_COLS * 2 + 4; // 2 row-label + │ + cells + │

    let center = |s: &str| {
        let pad = VIS_WIDTH.saturating_sub(s.chars().count());
        format!("{}{}{}", " ".repeat(pad / 2), s, " ".repeat(pad - pad / 2))
    };

    let header = boards
        .iter()
        .map(|(label, _)| center(label))
        .collect::<Vec<_>>()
        .join(SEP);

    let rows = (0..boards[0].1.len()).map(|r| {
        boards
            .iter()
            .map(|(_, lines)| lines[r].as_str())
            .collect::<Vec<_>>()
            .join(SEP)
    });

    std::iter::once(header)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

/// For each rotation of `kind`, positions the piece flush against the left then
/// right wall and attempts CW and CCW rotations. Collects every case where the
/// piece actually kicked (col changed) and shows before→after in a side-by-side
/// grid labelled by wall side, direction, and rotation transition.
fn wall_kick_snap(kind: PieceKind) -> String {
    let mut boards = Vec::new();

    for &left_wall in &[true, false] {
        for start_rot in 0usize..4 {
            let rot_cells = crate::piece::cells(kind, start_rot);
            let min_dc = rot_cells.iter().map(|&(dc, _)| dc).min().unwrap();
            let max_dc = rot_cells.iter().map(|&(dc, _)| dc).max().unwrap();

            let flush_col = if left_wall {
                -min_dc // leftmost cell at col 0
            } else {
                BOARD_COLS as i32 - 1 - max_dc // rightmost cell at col 9
            };

            for &cw in &[true, false] {
                let new_rot = if cw { (start_rot + 1) % 4 } else { (start_rot + 3) % 4 };
                let action = if cw { GameAction::RotateCw } else { GameAction::RotateCcw };

                let mut game = make_game(kind);
                game.active.rotation = start_rot;
                game.active.col = flush_col;

                let col_before = game.active.col;
                let prev = active_abs(&game);
                game.handle_action(action);

                // Only include when a kick actually happened
                if game.active.col != col_before && game.active.rotation == new_rot {
                    let wall = if left_wall { "L" } else { "R" };
                    let dir = if cw { "↻" } else { "↺" };
                    boards.push((
                        format!("{wall}{dir} {start_rot}→{new_rot}"),
                        board_lines(&game, &prev),
                    ));
                }
            }
        }
    }

    side_by_side(&boards)
}

/// Places `kind` at col 3, row 8 in rotation `start_rot`, puts a single board
/// obstacle at (col+obs_dc, row+obs_dr), then attempts CW and CCW rotations.
/// Shows the two resulting board states side by side.
fn center_col_snap(kind: PieceKind, start_rot: usize, obstacles: &[(i32, i32)]) -> String {
    let col = 3i32;
    let row = 8i32;

    let make = || {
        let mut game = make_game(kind);
        game.active.rotation = start_rot;
        game.active.col = col;
        game.active.row = row;
        for &(obs_dc, obs_dr) in obstacles {
            game.board[(row + obs_dr) as usize][(col + obs_dc) as usize] = Some(PieceKind::O);
        }
        game
    };

    let init_cells = active_abs(&make());

    let mut cw = make();
    cw.handle_action(GameAction::RotateCw);

    let mut ccw = make();
    ccw.handle_action(GameAction::RotateCcw);

    side_by_side(&[
        ("↻".to_string(), board_lines(&cw, &init_cells)),
        ("↺".to_string(), board_lines(&ccw, &init_cells)),
    ])
}

fn movement_snap(kind: PieceKind, action: GameAction) -> String {
    let mut game = make_game(kind);
    let mut boards = Vec::new();
    let mut step = 1;
    loop {
        let prev = active_abs(&game);
        game.handle_action(action);
        let curr = active_abs(&game);
        if curr == prev {
            break;
        }
        boards.push((format!("{step}"), board_lines(&game, &prev)));
        step += 1;
    }
    side_by_side(&boards)
}

#[test]
fn i_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::I), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │          []        │     │          '.        │     │          []        │     │          '.        │
      │      '.'.[]'.      │     │      [][][][]      │     │      '.'.[]'.      │     │      [][][][]      │
    10│- - - - - []- - - - │   10│- - - - - '.- - - - │   10│- - - - - []- - - - │   10│- - - - - '.- - - - │
      │          []        │     │          '.        │     │          []        │     │          '.        │
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

#[test]
fn o_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::O), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │        [][]        │     │        [][]        │     │        [][]        │     │        [][]        │
    10│- - - - [][]- - - - │   10│- - - - [][]- - - - │   10│- - - - [][]- - - - │   10│- - - - [][]- - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn t_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::T), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │        []          │     │        '.          │     │        []          │     │        '.          │
      │      [][]'.        │     │      '.[]          │     │        [][]        │     │      [][][]        │
    10│- - - - []- - - - - │   10│- - - [][][]- - - - │   10│- - - '.[]'.- - - - │   10│- - - - []- - - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn s_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::S), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │      []            │     │      '.            │     │      []            │     │      '.            │
      │      [][]'.        │     │      '.[][]        │     │      [][]'.        │     │      '.[][]        │
    10│- - - '.[]- - - - - │   10│- - - [][]- - - - - │   10│- - - '.[]- - - - - │   10│- - - [][]- - - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn z_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::Z), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │          []        │     │          '.        │     │          []        │     │          '.        │
      │      '.[][]        │     │      [][]'.        │     │      '.[][]        │     │      [][]'.        │
    10│- - - - []'.- - - - │   10│- - - - [][]- - - - │   10│- - - - []'.- - - - │   10│- - - - [][]- - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn j_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::J), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │        []          │     │        '.          │     │        [][]        │     │        '.'.        │
      │      '.[]'.        │     │      []'.          │     │      '.[]          │     │      [][][]        │
    10│- - - [][]'.- - - - │   10│- - - [][][]- - - - │   10│- - - '.[]'.- - - - │   10│- - - - '.[]- - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn l_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::L), @"
              0→1                        1→2                        2→3                        3→0           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │      [][]          │     │      '.'.          │     │        []          │     │        '.          │
      │      '.[]'.        │     │        '.[]        │     │        []'.        │     │      [][][]        │
    10│- - - '.[]- - - - - │   10│- - - [][][]- - - - │   10│- - - '.[][]- - - - │   10│- - - []'.'.- - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn t_piece_wall_kicks() {
    insta::assert_snapshot!(wall_kick_snap(PieceKind::T), @"
             L↻ 3→0                     L↺ 3→2                     R↻ 1→2                     R↺ 1→0         
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │'.                  │     │'.                  │     │                  '.│     │                  '.│
      │[][][]              │     │'.[]                │     │                []'.│     │              [][][]│
    10│'.[]- - - - - - - - │   10│[][][]- - - - - - - │   10│- - - - - - - [][][]│   10│- - - - - - - - []'.│
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn j_piece_wall_kicks() {
    insta::assert_snapshot!(wall_kick_snap(PieceKind::J), @"
             L↻ 3→0                     L↺ 3→2                     R↻ 1→2                     R↺ 1→0         
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │'.'.                │     │'.'.                │     │                  '.│     │                  '.│
      │[][][]              │     │[]                  │     │              []  '.│     │              [][][]│
    10│'.- []- - - - - - - │   10│[][][]- - - - - - - │   10│- - - - - - - [][][]│   10│- - - - - - - - '.[]│
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn l_piece_wall_kicks() {
    insta::assert_snapshot!(wall_kick_snap(PieceKind::L), @"
             L↻ 3→0                     L↺ 3→2                     R↻ 1→2                     R↺ 1→0         
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │'.                  │     │'.                  │     │                '.'.│     │                '.'.│
      │[][][]              │     │'.  []              │     │                  []│     │              [][][]│
    10│[]'.- - - - - - - - │   10│[][][]- - - - - - - │   10│- - - - - - - [][][]│   10│- - - - - - - []- '.│
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn s_piece_wall_kicks() {
    insta::assert_snapshot!(wall_kick_snap(PieceKind::S), @"
             R↻ 1→2                     R↺ 1→0                     R↻ 3→0                     R↺ 3→2         
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                '.  │     │                '.  │     │                '.  │     │                '.  │
      │                [][]│     │                [][]│     │                [][]│     │                [][]│
    10│- - - - - - - [][]'.│   10│- - - - - - - [][]'.│   10│- - - - - - - [][]'.│   10│- - - - - - - [][]'.│
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn z_piece_wall_kicks() {
    insta::assert_snapshot!(wall_kick_snap(PieceKind::Z), @"
             L↻ 1→2                     L↺ 1→0                     L↻ 3→0                     L↺ 3→2         
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │  '.                │     │  '.                │     │  '.                │     │  '.                │
      │[][]                │     │[][]                │     │[][]                │     │[][]                │
    10│'.[][]- - - - - - - │   10│'.[][]- - - - - - - │   10│'.[][]- - - - - - - │   10│'.[][]- - - - - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn l_piece_center_col_blocks() {
    let s = |rot, dc, dr| center_col_snap(PieceKind::L, rot, &[(dc, dr)]);
    insta::assert_snapshot!([
        ("rot0/pos2", s(0, 1, 0)),
        ("rot0/pos8", s(0, 1, 2)),
        ("rot2/pos2", s(2, 1, 0)),
        ("rot2/pos5", s(2, 1, 1)),
    ]
    .iter()
    .map(|(label, snap)| format!("=== {label} ===\n{snap}"))
    .collect::<Vec<_>>()
    .join("\n\n"), @"
    === rot0/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │      [][][]        │     │      [][][]        │
    10│- - - []- - - - - - │   10│- - - []- - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot0/pos8 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │      [][][]        │     │      [][][]        │
    10│- - - []##- - - - - │   10│- - - []##- - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot2/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │          []        │     │          []        │
    10│- - - [][][]- - - - │   10│- - - [][][]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot2/pos5 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │        ##[]        │     │        ##[]        │
    10│- - - [][][]- - - - │   10│- - - [][][]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘
    ");
}

#[test]
fn j_piece_center_col_blocks() {
    let s = |rot, dc, dr| center_col_snap(PieceKind::J, rot, &[(dc, dr)]);
    insta::assert_snapshot!([
        ("rot0/pos2", s(0, 1, 0)),
        ("rot0/pos8", s(0, 1, 2)),
        ("rot2/pos2", s(2, 1, 0)),
        ("rot2/pos5", s(2, 1, 1)),
    ]
    .iter()
    .map(|(label, snap)| format!("=== {label} ===\n{snap}"))
    .collect::<Vec<_>>()
    .join("\n\n"), @"
    === rot0/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │      [][][]        │     │      [][][]        │
    10│- - - - - []- - - - │   10│- - - - - []- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot0/pos8 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │      [][][]        │     │      [][][]        │
    10│- - - - ##[]- - - - │   10│- - - - ##[]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot2/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │      []            │     │      []            │
    10│- - - [][][]- - - - │   10│- - - [][][]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot2/pos5 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │      []##          │     │      []##          │
    10│- - - [][][]- - - - │   10│- - - [][][]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘
    ");
}

#[test]
fn t_piece_center_col_blocks() {
    let s = |rot, dc, dr| center_col_snap(PieceKind::T, rot, &[(dc, dr)]);
    insta::assert_snapshot!([
        ("rot0/pos2", s(0, 1, 0)),
        ("rot2/pos2", s(2, 1, 0)),
    ]
    .iter()
    .map(|(label, snap)| format!("=== {label} ===\n{snap}"))
    .collect::<Vec<_>>()
    .join("\n\n"), @"
    === rot0/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │      [][][]        │     │      [][][]        │
    10│- - - - []- - - - - │   10│- - - - []- - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘

    === rot2/pos2 ===
               ↻                          ↺            
      ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │        ##          │     │        ##          │
      │        []          │     │        []          │
    10│- - - [][][]- - - - │   10│- - - [][][]- - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    15│- - - - - - - - - - │   15│- - - - - - - - - - │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
      │                    │     │                    │
    20└────────────────────┘   20└────────────────────┘
    ");
}

#[test]
fn board_ascii_checkerboard() {
    let mut game = make_game(PieceKind::O);
    game.active.row = BOARD_ROWS as i32; // park piece below visible area
    game.board = board_from_ascii("
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
    ");
    insta::assert_snapshot!(board_lines(&game, &[]).join("\n"), @"
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
      │                    │
    10│- - - - - - - - - - │
      │                    │
      │##  ##  ##  ##  ##  │
      │  ##  ##  ##  ##  ##│
      │##  ##  ##  ##  ##  │
    15│- ##- ##- ##- ##- ##│
      │##  ##  ##  ##  ##  │
      │  ##  ##  ##  ##  ##│
      │##  ##  ##  ##  ##  │
      │  ##  ##  ##  ##  ##│
    20└────────────────────┘
    ");
}

#[test]
fn cw_ccw_equivalence() {
    for kind in PieceKind::all() {
        let rotated = |cw: usize, ccw: usize| {
            let mut game = make_game(kind);
            for _ in 0..cw {
                game.handle_action(GameAction::RotateCw);
            }
            for _ in 0..ccw {
                game.handle_action(GameAction::RotateCcw);
            }
            active_abs(&game)
        };

        assert_eq!(rotated(1, 0), rotated(0, 3), "{kind:?}: 1 CW != 3 CCW");
        assert_eq!(rotated(2, 0), rotated(0, 2), "{kind:?}: 2 CW != 2 CCW");
        assert_eq!(rotated(3, 0), rotated(0, 1), "{kind:?}: 3 CW != 1 CCW");
        assert_eq!(rotated(4, 0), rotated(0, 0), "{kind:?}: 4 CW != rotation 0");
        assert_eq!(
            rotated(0, 4),
            rotated(0, 0),
            "{kind:?}: 4 CCW != rotation 0"
        );
    }
}

#[test]
fn o_piece_move_left() {
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameAction::MoveLeft), @"
               1                          2                          3                          4            
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │      [][]'.        │     │    [][]'.          │     │  [][]'.            │     │[][]'.              │
    10│- - - [][]'.- - - - │   10│- - [][]'.- - - - - │   10│- [][]'.- - - - - - │   10│[][]'.- - - - - - - │
      │                    │     │                    │     │                    │     │                    │
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

#[test]
fn o_piece_move_right() {
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameAction::MoveRight), @"
               1                          2                          3                          4            
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │        '.[][]      │     │          '.[][]    │     │            '.[][]  │     │              '.[][]│
    10│- - - - '.[][]- - - │   10│- - - - - '.[][]- - │   10│- - - - - - '.[][]- │   10│- - - - - - - '.[][]│
      │                    │     │                    │     │                    │     │                    │
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

// Asymmetric wall-kick tests: one direction kicks, the other is suppressed
// because the center-column check is direction-aware (uses destination rotation cells).

#[test]
fn l_j_asymmetric_wall_kicks() {
    // CW-only 1: L rot0, obstacles x../ooo/ox. → (dc=0,dr=0) and (dc=1,dr=2)
    // CW first collision in rot1 is at dc=0 → kick allowed; CCW first collision in rot3 at dc=1 → suppressed
    let cw_only_1 = center_col_snap(PieceKind::L, 0, &[(0, 0), (1, 2)]);

    // CW-only 2: J rot2, obstacles ..x/ox./ooo → (dc=2,dr=0) and (dc=1,dr=1)
    // CW first collision in rot3 is at dc=2 → kick allowed; CCW first collision in rot1 at dc=1 → suppressed
    let cw_only_2 = center_col_snap(PieceKind::J, 2, &[(2, 0), (1, 1)]);

    // CCW-only 1: J rot0, obstacles ..x/ooo/.xo → (dc=2,dr=0) and (dc=1,dr=2)
    // CW first collision in rot1 at dc=1 → suppressed; CCW first collision in rot3 at dc=2 → kick allowed
    let ccw_only_1 = center_col_snap(PieceKind::J, 0, &[(2, 0), (1, 2)]);

    // CCW-only 2: L rot2, obstacles x../.xo/ooo → (dc=0,dr=0) and (dc=1,dr=1)
    // CW first collision in rot3 at dc=1 → suppressed; CCW first collision in rot1 at dc=0 → kick allowed
    let ccw_only_2 = center_col_snap(PieceKind::L, 2, &[(0, 0), (1, 1)]);

    insta::assert_snapshot!([
        ("CW-only/L-rot0", cw_only_1),
        ("CW-only/J-rot2", cw_only_2),
        ("CCW-only/J-rot0", ccw_only_1),
        ("CCW-only/L-rot2", ccw_only_2),
    ]
    .iter()
    .map(|(label, snap)| format!("=== {label} ===\n{snap}"))
    .collect::<Vec<_>>()
    .join("\n\n"), @"
=== CW-only/L-rot0 ===
           ↻                          ↺            
  ┌────────────────────┐     ┌────────────────────┐
 0│- - - - - - - - - - │    0│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
 5│- - - - - - - - - - │    5│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │      ##[][]        │     │      ##            │
  │      '.'.[]        │     │      [][][]        │
10│- - - '.##[]- - - - │   10│- - - []##- - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
15│- - - - - - - - - - │   15│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
20└────────────────────┘   20└────────────────────┘

=== CW-only/J-rot2 ===
           ↻                          ↺            
  ┌────────────────────┐     ┌────────────────────┐
 0│- - - - - - - - - - │    0│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
 5│- - - - - - - - - - │    5│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │      [][]##        │     │          ##        │
  │      []##          │     │      []##          │
10│- - - []'.'.- - - - │   10│- - - [][][]- - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
15│- - - - - - - - - - │   15│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
20└────────────────────┘   20└────────────────────┘

=== CCW-only/J-rot0 ===
           ↻                          ↺            
  ┌────────────────────┐     ┌────────────────────┐
 0│- - - - - - - - - - │    0│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
 5│- - - - - - - - - - │    5│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │          ##        │     │      [][]##        │
  │      [][][]        │     │      []'.'.        │
10│- - - - ##[]- - - - │   10│- - - []##'.- - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
15│- - - - - - - - - - │   15│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
20└────────────────────┘   20└────────────────────┘

=== CCW-only/L-rot2 ===
           ↻                          ↺            
  ┌────────────────────┐     ┌────────────────────┐
 0│- - - - - - - - - - │    0│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
 5│- - - - - - - - - - │    5│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │      ##            │     │      ##[][]        │
  │        ##[]        │     │        ##[]        │
10│- - - [][][]- - - - │   10│- - - '.'.[]- - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
15│- - - - - - - - - - │   15│- - - - - - - - - - │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
  │                    │     │                    │
20└────────────────────┘   20└────────────────────┘
    ");
}
