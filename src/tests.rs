use std::collections::HashSet;
use crate::game::{BOARD_COLS, BOARD_ROWS, Board, Game, PiecePhase, RotationDirection};
use crate::input::{GameKey, InputState};
use crate::piece::{Piece, PieceKind};
use crate::constants::{DAS_CHARGE, LOCK_DELAY, SPAWN_DELAY, gravity_g};

fn make_game(kind: PieceKind) -> Game {
    let mut game = Game::new();
    game.board = [[None; BOARD_COLS]; BOARD_ROWS];
    game.active = Piece::new(kind);
    game.active.col = 3;
    game.active.row = 8;
    game.next = Piece::new(kind);
    game
}

/// Simulate a single keypress (held + just_pressed for one tick).
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
        press(&mut game, GameKey::RotateCw);
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
                let key = if cw { GameKey::RotateCw } else { GameKey::RotateCcw };

                let mut game = make_game(kind);
                game.active.rotation = start_rot;
                game.active.col = flush_col;

                let col_before = game.active.col;
                let prev = active_abs(&game);
                press(&mut game, key);

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

/// Like wall_kick_snap but captures cases where the rotation was attempted and
/// *failed* — the piece stayed at the same rotation and column. Shows before→after
/// (which look identical) to make the blocked rotation visible.
fn wall_no_kick_snap(kind: PieceKind) -> String {
    let mut boards = Vec::new();

    for &left_wall in &[true, false] {
        for start_rot in 0usize..4 {
            let rot_cells = crate::piece::cells(kind, start_rot);
            let min_dc = rot_cells.iter().map(|&(dc, _)| dc).min().unwrap();
            let max_dc = rot_cells.iter().map(|&(dc, _)| dc).max().unwrap();

            let flush_col = if left_wall {
                -min_dc
            } else {
                BOARD_COLS as i32 - 1 - max_dc
            };

            for &cw in &[true, false] {
                let new_rot = if cw { (start_rot + 1) % 4 } else { (start_rot + 3) % 4 };
                let key = if cw { GameKey::RotateCw } else { GameKey::RotateCcw };

                // Check that the new rotation wouldn't fit in place at flush_col.
                // We detect this by trying the rotation at the same flush position.
                let mut probe = make_game(kind);
                probe.active.rotation = start_rot;
                probe.active.col = flush_col;
                press(&mut probe, key);
                let fits_in_place = probe.active.rotation == new_rot && probe.active.col == flush_col;
                if fits_in_place {
                    continue; // rotation fits without any kick — not an interesting blocked case
                }

                let mut game = make_game(kind);
                game.active.rotation = start_rot;
                game.active.col = flush_col;

                let prev = active_abs(&game);
                press(&mut game, key);

                // Only include when rotation was blocked (no kick occurred)
                if game.active.rotation == start_rot {
                    let wall = if left_wall { "L" } else { "R" };
                    let dir = if cw { "↻" } else { "↺" };
                    boards.push((
                        format!("{wall}{dir} {start_rot}✗"),
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
    press(&mut cw, GameKey::RotateCw);

    let mut ccw = make();
    press(&mut ccw, GameKey::RotateCcw);

    side_by_side(&[
        ("↻".to_string(), board_lines(&cw, &init_cells)),
        ("↺".to_string(), board_lines(&ccw, &init_cells)),
    ])
}

fn movement_snap(kind: PieceKind, key: GameKey) -> String {
    let mut game = make_game(kind);
    let mut boards = Vec::new();
    let mut step = 1;
    loop {
        let prev = active_abs(&game);
        // Reset DAS so each press triggers an immediate move (simulates fresh key press).
        game.das_direction = None;
        press(&mut game, key);
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
fn gravity_g_lookup() {
    assert_eq!(gravity_g(0),   4,    "level 0 → 4 G/256");
    assert_eq!(gravity_g(29),  4,    "level 29 → still 4 G/256");
    assert_eq!(gravity_g(30),  6,    "level 30 → 6 G/256");
    assert_eq!(gravity_g(199), 144,  "level 199 → 144 G/256");
    assert_eq!(gravity_g(200), 4,    "level 200 → resets to 4 G/256");
    assert_eq!(gravity_g(251), 256,  "level 251 → 256 G/256 (1G)");
    assert_eq!(gravity_g(500), 5120, "level 500 → 5120 G/256 (20G)");
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
                press(&mut game, GameKey::RotateCw);
            }
            for _ in 0..ccw {
                press(&mut game, GameKey::RotateCcw);
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
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameKey::Left), @"
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
    insta::assert_snapshot!(movement_snap(PieceKind::O, GameKey::Right), @"
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

#[test]
fn i_piece_no_wall_kicks() {
    // I piece never kicks. Place it vertical (rot 1) flush against each wall
    // and show that both CW and CCW rotation attempts leave it unchanged.
    let make = |col: i32| {
        let mut game = make_game(PieceKind::I);
        game.active.rotation = 1;
        game.active.col = col;
        game
    };
    let rot1_cells = crate::piece::cells(PieceKind::I, 1);
    let min_dc = rot1_cells.iter().map(|&(dc, _)| dc).min().unwrap();
    let max_dc = rot1_cells.iter().map(|&(dc, _)| dc).max().unwrap();
    let left_col = -min_dc;
    let right_col = BOARD_COLS as i32 - 1 - max_dc;

    let boards: Vec<(String, Vec<String>)> = [
        ("L↻", left_col, GameKey::RotateCw),
        ("L↺", left_col, GameKey::RotateCcw),
        ("R↻", right_col, GameKey::RotateCw),
        ("R↺", right_col, GameKey::RotateCcw),
    ]
    .iter()
    .map(|&(label, col, key)| {
        let prev = active_abs(&make(col));
        let mut game = make(col);
        press(&mut game, key);
        (label.to_string(), board_lines(&game, &prev))
    })
    .collect();

    insta::assert_snapshot!(side_by_side(&boards), @"
               L↻                         L↺                         R↻                         R↺           
      ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐     ┌────────────────────┐
     0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │    0│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
     5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │    5│- - - - - - - - - - │
      │                    │     │                    │     │                    │     │                    │
      │                    │     │                    │     │                    │     │                    │
      │[]                  │     │[]                  │     │                  []│     │                  []│
      │[]                  │     │[]                  │     │                  []│     │                  []│
    10│[]- - - - - - - - - │   10│[]- - - - - - - - - │   10│- - - - - - - - - []│   10│- - - - - - - - - []│
      │[]                  │     │[]                  │     │                  []│     │                  []│
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

#[test]
fn lock_delay_prevents_immediate_lock() {
    let mut game = make_game(PieceKind::T);
    // Drop piece to floor manually
    while game.try_move(0, 1) {}
    // Tick once — transitions from Falling to Locking { ticks_left: LOCK_DELAY }
    idle(&mut game, 1);
    assert!(matches!(game.piece_phase, PiecePhase::Locking { .. }),
        "expected Locking, got {:?}", game.piece_phase);
    // LOCK_DELAY ticks decrement ticks_left to 0; one more tick fires the lock.
    idle(&mut game, LOCK_DELAY + 1);
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }),
        "expected Spawning, got {:?}", game.piece_phase);
}

#[test]
fn sonic_drop_enters_lock_delay() {
    let mut game = make_game(PieceKind::T);
    press(&mut game, GameKey::SonicDrop);
    assert!(matches!(game.piece_phase, PiecePhase::Locking { .. }),
        "expected Locking after sonic drop, got {:?}", game.piece_phase);
}

#[test]
fn soft_drop_on_floor_locks_immediately() {
    let mut game = make_game(PieceKind::T);
    // Drop to floor and enter locking state
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking
    // Soft drop bypasses lock delay
    press(&mut game, GameKey::SoftDrop);
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }),
        "expected Spawning after soft drop on floor, got {:?}", game.piece_phase);
}

#[test]
fn das_activates_after_charge() {
    let mut game = make_game(PieceKind::T);
    let start_col = game.active.col;
    // First press moves immediately
    press(&mut game, GameKey::Left);
    assert_eq!(game.active.col, start_col - 1, "expected immediate move on press");
    // Hold for DAS_CHARGE - 1 ticks: no additional movement (counter not yet at charge)
    hold(&mut game, &[GameKey::Left], DAS_CHARGE - 1);
    assert_eq!(game.active.col, start_col - 1, "no movement before DAS charge");
    // One more tick triggers first auto-repeat
    hold(&mut game, &[GameKey::Left], 1);
    assert_eq!(game.active.col, start_col - 2, "first auto-repeat after DAS charge");
}

#[test]
fn das_repeats_every_tick_after_charge() {
    let mut game = make_game(PieceKind::T);
    game.active.col = 8; // Start further right so we can move 5 columns left
    let start_col = game.active.col;
    press(&mut game, GameKey::Left);                 // immediate: start_col - 1
    hold(&mut game, &[GameKey::Left], DAS_CHARGE);   // first auto-repeat at charge: start_col - 2
    hold(&mut game, &[GameKey::Left], 3);            // 3 more repeats (DAS_REPEAT=1): start_col - 5
    assert_eq!(game.active.col, start_col - 5, "DAS should repeat every tick after charge");
}

#[test]
fn rotation_buffer_applied_on_spawn() {
    let mut game = make_game(PieceKind::T);
    // Move piece to floor
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking { ticks_left: LOCK_DELAY }
    idle(&mut game, LOCK_DELAY + 1); // decrement to 0, then lock → Spawning
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }));
    // Press rotate during spawn delay
    press(&mut game, GameKey::RotateCw);
    assert!(matches!(game.rotation_buffer, Some(RotationDirection::Clockwise)),
        "rotation buffer should be set during spawn delay");
    // After the press decremented ticks_left by 1, SPAWN_DELAY idle ticks finish the countdown and spawn.
    idle(&mut game, SPAWN_DELAY);
    assert_eq!(game.active.rotation, 1,
        "spawned piece should be rotated CW");
}
