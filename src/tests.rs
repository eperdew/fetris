use crate::audio_player::null::Null;
use crate::constants::{DAS_CHARGE, LINE_CLEAR_DELAY, LOCK_DELAY, SPAWN_DELAY_NORMAL, gravity_g};
use crate::game::{Game, can_piece_increment};
use crate::rotation_system::Ars;
use crate::types::{
    BOARD_COLS, BOARD_ROWS, Board, GameKey, GameMode, InputState, Kind, Piece, PieceKind,
    PiecePhase,
};
use std::collections::HashSet;
use std::sync::Arc;

fn make_game_with(game_mode: GameMode, rotation_kind: Kind, kind: PieceKind) -> Game {
    let rotation_system = rotation_kind.create();
    let mut game = Game::new(game_mode, rotation_kind, rotation_system, Arc::new(Null));
    game.board = [[None; BOARD_COLS]; BOARD_ROWS];
    game.active = Piece::new(kind);
    game.active.col = 3;
    game.active.row = 8;
    game.next = Piece::new(kind);
    game
}

fn make_game(kind: PieceKind) -> Game {
    make_game_with(GameMode::Master, Kind::Ars, kind)
}

fn make_srs_game(kind: PieceKind) -> Game {
    make_game_with(GameMode::Master, Kind::Srs, kind)
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

fn rotation_snap(kind: PieceKind, make: fn(PieceKind) -> Game) -> String {
    let mut game = make(kind);
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
        .rotation_system
        .cells(game.active.kind, game.active.rotation)
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
            row.push_str(if game.board[r][c].is_some() {
                "##"
            } else if active.contains(&pos) {
                "[]"
            } else if prev_cells.contains(&pos) {
                "'."
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
    game.rotation_system
        .cells(game.active.kind, game.active.rotation)
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
fn wall_kick_snap(kind: PieceKind, make: fn(PieceKind) -> Game) -> String {
    let mut boards = Vec::new();
    let game = make(kind);

    for &left_wall in &[true, false] {
        for start_rot in 0usize..4 {
            let rot_cells = game.rotation_system.cells(kind, start_rot);
            let min_dc = rot_cells.iter().map(|&(dc, _)| dc).min().unwrap();
            let max_dc = rot_cells.iter().map(|&(dc, _)| dc).max().unwrap();

            let flush_col = if left_wall {
                -min_dc // leftmost cell at col 0
            } else {
                BOARD_COLS as i32 - 1 - max_dc // rightmost cell at col 9
            };

            for &cw in &[true, false] {
                let new_rot = if cw {
                    (start_rot + 1) % 4
                } else {
                    (start_rot + 3) % 4
                };
                let key = if cw {
                    GameKey::RotateCw
                } else {
                    GameKey::RotateCcw
                };

                let mut game = make(kind);
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
    let gravity_g = |level| gravity_g(GameMode::Master, level);
    assert_eq!(gravity_g(0), 4, "level 0 → 4 G/256");
    assert_eq!(gravity_g(29), 4, "level 29 → still 4 G/256");
    assert_eq!(gravity_g(30), 6, "level 30 → 6 G/256");
    assert_eq!(gravity_g(199), 144, "level 199 → 144 G/256");
    assert_eq!(gravity_g(200), 4, "level 200 → resets to 4 G/256");
    assert_eq!(gravity_g(251), 256, "level 251 → 256 G/256 (1G)");
    assert_eq!(gravity_g(500), 5120, "level 500 → 5120 G/256 (20G)");
}

#[test]
fn i_piece_rotations() {
    insta::assert_snapshot!(rotation_snap(PieceKind::I, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::O, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::T, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::S, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::Z, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::J, make_game), @"
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
    insta::assert_snapshot!(rotation_snap(PieceKind::L, make_game), @"
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
    insta::assert_snapshot!(wall_kick_snap(PieceKind::T, make_game), @"
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
    insta::assert_snapshot!(wall_kick_snap(PieceKind::J, make_game), @"
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
    insta::assert_snapshot!(wall_kick_snap(PieceKind::L, make_game), @"
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
    insta::assert_snapshot!(wall_kick_snap(PieceKind::S, make_game), @"
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
    insta::assert_snapshot!(wall_kick_snap(PieceKind::Z, make_game), @"
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
    game.board = board_from_ascii(
        "
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
        O.O.O.O.O.
        .O.O.O.O.O
    ",
    );
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
    let game = make_game(PieceKind::I);
    let make = |col: i32| {
        let mut g = make_game(PieceKind::I);
        g.active.rotation = 1;
        g.active.col = col;
        g
    };
    let rot1_cells = game.rotation_system.cells(PieceKind::I, 1);
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
    assert!(
        matches!(game.piece_phase, PiecePhase::Locking { .. }),
        "expected Locking, got {:?}",
        game.piece_phase
    );
    // LOCK_DELAY ticks decrement ticks_left to 0; one more tick fires the lock.
    idle(&mut game, LOCK_DELAY + 1);
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { .. }),
        "expected Spawning, got {:?}",
        game.piece_phase
    );
}

#[test]
fn sonic_drop_enters_lock_delay() {
    let mut game = make_game(PieceKind::T);
    press(&mut game, GameKey::SonicDrop);
    assert!(
        matches!(game.piece_phase, PiecePhase::Locking { .. }),
        "expected Locking after sonic drop, got {:?}",
        game.piece_phase
    );
}

#[test]
fn soft_drop_on_floor_locks_immediately() {
    let mut game = make_game(PieceKind::T);
    // Drop to floor and enter locking state
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking
    // Soft drop bypasses lock delay
    press(&mut game, GameKey::SoftDrop);
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { .. }),
        "expected Spawning after soft drop on floor, got {:?}",
        game.piece_phase
    );
}

#[test]
fn das_activates_after_charge() {
    let mut game = make_game(PieceKind::T);
    let start_col = game.active.col;
    // First press moves immediately
    press(&mut game, GameKey::Left);
    assert_eq!(
        game.active.col,
        start_col - 1,
        "expected immediate move on press"
    );
    // Hold for DAS_CHARGE - 1 ticks: no additional movement (counter not yet at charge)
    hold(&mut game, &[GameKey::Left], DAS_CHARGE - 1);
    assert_eq!(
        game.active.col,
        start_col - 1,
        "no movement before DAS charge"
    );
    // One more tick triggers first auto-repeat
    hold(&mut game, &[GameKey::Left], 1);
    assert_eq!(
        game.active.col,
        start_col - 2,
        "first auto-repeat after DAS charge"
    );
}

#[test]
fn das_repeats_every_tick_after_charge() {
    let mut game = make_game(PieceKind::T);
    game.active.col = 8; // Start further right so we can move 5 columns left
    let start_col = game.active.col;
    press(&mut game, GameKey::Left); // immediate: start_col - 1
    hold(&mut game, &[GameKey::Left], DAS_CHARGE); // first auto-repeat at charge: start_col - 2
    hold(&mut game, &[GameKey::Left], 3); // 3 more repeats (DAS_REPEAT=1): start_col - 5
    assert_eq!(
        game.active.col,
        start_col - 5,
        "DAS should repeat every tick after charge"
    );
}

#[test]
fn rotation_buffer_applied_on_spawn() {
    let mut game = make_game(PieceKind::T);
    // Move piece to floor
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking { ticks_left: LOCK_DELAY }
    idle(&mut game, LOCK_DELAY + 1); // lock → Spawning
    assert!(matches!(game.piece_phase, PiecePhase::Spawning { .. }));
    // Hold rotate through all of ARE — IRS only fires if held at spawn.
    hold(&mut game, &[GameKey::RotateCw], SPAWN_DELAY_NORMAL + 1);
    assert_eq!(
        game.active.rotation, 1,
        "spawned piece should be rotated CW"
    );
}

#[test]
fn rotation_released_during_are_does_not_rotate() {
    let mut game = make_game(PieceKind::T);
    while game.try_move(0, 1) {}
    idle(&mut game, 1);
    idle(&mut game, LOCK_DELAY + 1); // lock → Spawning
    // Tap rotate then release — should NOT trigger IRS.
    press(&mut game, GameKey::RotateCw);
    idle(&mut game, SPAWN_DELAY_NORMAL);
    assert_eq!(
        game.active.rotation, 0,
        "released key should not trigger IRS"
    );
}

/// Positions a vertical I-piece on the floor with `n` bottom rows pre-filled
/// (except column 2). Ticking once fires lock and clears n lines.
fn setup_line_clear(game: &mut Game, n: usize) {
    for r in (BOARD_ROWS - n)..BOARD_ROWS {
        for c in 0..BOARD_COLS {
            if c != 2 {
                game.board[r][c] = Some(PieceKind::O);
            }
        }
    }
    game.active = Piece::new(PieceKind::I);
    game.active.col = 0;
    game.active.row = (BOARD_ROWS - 4) as i32;
    game.active.rotation = 1; // vertical: cells at (col+2, row..row+3)
    game.piece_phase = PiecePhase::Locking { ticks_left: 0 };
}

#[test]
fn can_piece_increment_section_stops() {
    assert!(!can_piece_increment(99), "99 is section stop");
    assert!(!can_piece_increment(199), "199 is section stop");
    assert!(!can_piece_increment(899), "899 is section stop");
    assert!(!can_piece_increment(998), "998 is final stop");
    assert!(can_piece_increment(0), "0 is not a stop");
    assert!(can_piece_increment(100), "100 is not a stop");
    assert!(can_piece_increment(500), "500 is not a stop");
}

#[test]
fn level_starts_at_zero() {
    let game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    assert_eq!(game.level, 0);
}

#[test]
fn level_increments_on_piece_spawn() {
    let mut game = make_game(PieceKind::T);
    game.level = 50;
    while game.try_move(0, 1) {} // drop to floor
    idle(&mut game, 1); // enter Locking{LOCK_DELAY}
    idle(&mut game, LOCK_DELAY + 1); // fire lock → Spawning{SPAWN_DELAY_NORMAL}
    idle(&mut game, SPAWN_DELAY_NORMAL + 1); // complete ARE → spawn_piece called
    assert_eq!(
        game.level, 51,
        "level should increment from 50 to 51 on spawn"
    );
}

#[test]
fn section_stop_blocks_piece_increment() {
    let mut game = make_game(PieceKind::T);
    game.level = 99;
    while game.try_move(0, 1) {}
    idle(&mut game, 1);
    idle(&mut game, LOCK_DELAY + 1);
    idle(&mut game, SPAWN_DELAY_NORMAL + 1);
    assert_eq!(
        game.level, 99,
        "section stop: level should remain 99 after spawn"
    );
}

#[test]
fn line_clear_increments_level() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    game.level = 50;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → 1 line cleared
    assert_eq!(game.level, 51, "1 line clear should increment level 50→51");
}

#[test]
fn line_clear_passes_section_stop() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    game.level = 99;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1);
    assert_eq!(
        game.level, 100,
        "line clear should pass section stop 99→100"
    );
}

#[test]
fn level_clamped_to_999() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    game.level = 998;
    setup_line_clear(&mut game, 4); // tetris: +4 would be 1002, clamped to 999
    idle(&mut game, 1);
    assert_eq!(game.level, 999, "level should clamp to 999");
}

#[test]
fn game_won_on_reaching_999() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    game.level = 998;
    setup_line_clear(&mut game, 1); // +1 = 999
    idle(&mut game, 1);
    assert!(
        game.game_won,
        "game_won should be set when level reaches 999"
    );
}

#[test]
fn ticks_elapsed_increments_each_tick() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    idle(&mut game, 5);
    assert_eq!(game.ticks_elapsed, 5);
}

#[test]
fn ticks_elapsed_stops_after_win() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    game.level = 998;
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fires win
    let frozen = game.ticks_elapsed;
    idle(&mut game, 10);
    assert_eq!(
        game.ticks_elapsed, frozen,
        "ticks_elapsed should freeze after win"
    );
}

#[test]
fn normal_are_uses_spawn_delay_normal() {
    use crate::constants::SPAWN_DELAY_NORMAL;
    let mut game = make_game(PieceKind::T);
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking
    idle(&mut game, LOCK_DELAY + 1); // fire lock (no lines cleared)
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_NORMAL),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_NORMAL={} }}, got {:?}",
        SPAWN_DELAY_NORMAL,
        game.piece_phase
    );
}

#[test]
fn line_clear_enters_line_clear_delay() {
    use crate::constants::LINE_CLEAR_DELAY;
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock + 1 line clear
    assert!(
        matches!(game.piece_phase, PiecePhase::LineClearDelay { ticks_left } if ticks_left == LINE_CLEAR_DELAY),
        "expected LineClearDelay{{ ticks_left: LINE_CLEAR_DELAY={} }}, got {:?}",
        LINE_CLEAR_DELAY,
        game.piece_phase
    );
}

#[test]
fn line_clear_delay_transitions_to_are() {
    use crate::constants::{LINE_CLEAR_DELAY, SPAWN_DELAY_NORMAL};
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    idle(&mut game, LINE_CLEAR_DELAY + 1); // exhaust line clear delay → Spawning
    assert!(
        matches!(game.piece_phase, PiecePhase::Spawning { ticks_left } if ticks_left == SPAWN_DELAY_NORMAL),
        "expected Spawning{{ ticks_left: SPAWN_DELAY_NORMAL={} }}, got {:?}",
        SPAWN_DELAY_NORMAL,
        game.piece_phase
    );
}

#[test]
fn rows_pending_compaction_populated_during_delay() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    assert_eq!(
        game.rows_pending_compaction,
        vec![BOARD_ROWS - 1],
        "cleared row index should be in rows_pending_compaction during LineClearDelay"
    );
}

#[test]
fn board_not_compacted_during_delay() {
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    assert!(
        game.board[BOARD_ROWS - 1].iter().all(|c| c.is_some()),
        "cleared row should still be present in board during LineClearDelay"
    );
}

#[test]
fn board_compacted_and_pending_cleared_after_delay() {
    use crate::constants::LINE_CLEAR_DELAY;
    let mut game = Game::new(GameMode::Master, Kind::Ars, Box::new(Ars), Arc::new(Null));
    setup_line_clear(&mut game, 1);
    idle(&mut game, 1); // fire lock → LineClearDelay
    idle(&mut game, LINE_CLEAR_DELAY + 1); // exhaust delay → compaction → Spawning
    assert!(
        game.rows_pending_compaction.is_empty(),
        "rows_pending_compaction should be empty after compaction"
    );
    assert!(
        !game.board.iter().any(|row| row.iter().all(|c| c.is_some())),
        "no row should be fully filled after compaction"
    );
}

#[test]
fn lock_timer_resets_when_gravity_drops_piece() {
    // Set up a piece one row above the floor with a partially-spent lock timer.
    // A single 20G tick should drop it, re-land it, and reset the timer.
    let mut game = make_game(PieceKind::T);
    game.level = 500; // 20G
    while game.try_move(0, 1) {}
    game.try_move(0, -1); // lift 1 row so there's room to drop
    game.piece_phase = PiecePhase::Locking { ticks_left: 10 }; // partially spent

    insta::assert_snapshot!(
        format!("row={} phase={:?}", game.active.row, game.piece_phase),
        @"row=16 phase=Locking { ticks_left: 10 }"
    );
    idle(&mut game, 1);
    insta::assert_snapshot!(
        format!("row={} phase={:?}", game.active.row, game.piece_phase),
        @"row=17 phase=Locking { ticks_left: 29 }"
    );
}

#[test]
fn format_time_display() {
    use crate::renderer::format_time;
    assert_eq!(format_time(0), "00:00.000");
    assert_eq!(format_time(60), "00:01.000");
    assert_eq!(format_time(3600), "01:00.000");
    assert_eq!(format_time(90), "00:01.500");
    assert_eq!(format_time(5430), "01:30.500");
}

// ---------------------------------------------------------------------------
// I-piece right-well line clear tests
//
// Each test sets up an explicit board via board_from_ascii, places a vertical
// I piece (rotation 1, col 7 → board column 9) at the bottom, locks it, then
// snapshots the board with the active piece parked off-screen so only the
// remaining locked cells are visible.
// ---------------------------------------------------------------------------

/// Place a vertical I piece (rotation 1) in the right well (col 9) with its top at row 16.
fn place_vertical_i_right_well(game: &mut Game) {
    game.active = Piece::new(PieceKind::I);
    game.active.rotation = 1;
    game.active.col = 7;
    game.active.row = 16;
}

/// Lock the active piece (sonic drop to floor, then soft drop to lock immediately),
/// then tick through LineClearDelay into ARE so the next piece has spawned at the top.
/// Snapshots during ARE — the new piece is at row 0, far from the cleared rows.
fn lock_and_snap(mut game: Game) -> String {
    let soft = InputState {
        held: HashSet::from([GameKey::SoftDrop]),
        just_pressed: HashSet::new(),
    };
    press(&mut game, GameKey::SonicDrop); // drop to floor, enter Locking phase
    game.tick(&soft); // SoftDrop while Locking → lock + line clear → LineClearDelay
    idle(&mut game, LINE_CLEAR_DELAY + 1 + SPAWN_DELAY_NORMAL + 1); // tick through LineClearDelay and ARE
    board_lines(&game, &[]).join("\n")
}

#[test]
fn i_right_well_clears_4() {
    // All 4 rows filled left 9 cols; I piece fills col 9 on all 4 → tetris, board empty
    let mut game = make_game(PieceKind::I);
    game.board = board_from_ascii(
        "
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
    ",
    );
    place_vertical_i_right_well(&mut game);
    insta::assert_snapshot!(lock_and_snap(game), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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

#[test]
fn i_right_well_clears_top_3() {
    // Top 3 rows filled; bottom row empty → top 3 clear, stub at bottom
    let mut game = make_game(PieceKind::I);
    game.board = board_from_ascii(
        "
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOO.OOOOO.
    ",
    );
    place_vertical_i_right_well(&mut game);
    insta::assert_snapshot!(lock_and_snap(game), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │######  ############│
    20└────────────────────┘
    ");
}

#[test]
fn i_right_well_clears_bottom_3() {
    // Top row empty; bottom 3 rows filled → bottom 3 clear, stub at top
    let mut game = make_game(PieceKind::I);
    game.board = board_from_ascii(
        "
        .OOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
    ",
    );
    place_vertical_i_right_well(&mut game);
    insta::assert_snapshot!(lock_and_snap(game), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │                    │
      │  ##################│
    20└────────────────────┘
    ");
}

#[test]
fn i_right_well_clears_middle_2() {
    // Middle 2 rows filled; top and bottom empty → middle 2 clear, stubs at top and bottom
    let mut game = make_game(PieceKind::I);
    game.board = board_from_ascii(
        "
        .OOOOOOOO.
        OOOOOOOOO.
        OOOOOOOOO.
        OOO.OOOOO.
    ",
    );
    place_vertical_i_right_well(&mut game);
    insta::assert_snapshot!(lock_and_snap(game), @"
      ┌────────────────────┐
     0│- - - - - - - - - - │
      │      [][][][]      │
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
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │                    │
      │                    │
      │  ##################│
      │######  ############│
    20└────────────────────┘
    ");
}

#[cfg(test)]
mod menu_tests {
    use crate::menu::Menu;
    use crate::types::{GameConfig, GameMode, Kind, MenuInput, MenuResult, MenuScreen};

    fn storage() -> crate::storage::Storage {
        crate::storage::Storage::new()
    }

    fn menu() -> Menu {
        Menu::new(GameConfig::default())
    }

    fn input() -> MenuInput {
        MenuInput::default()
    }

    fn tick(m: &mut Menu, input: &MenuInput) -> MenuResult {
        m.tick(input, &storage())
    }

    #[test]
    fn cursor_starts_at_zero() {
        assert_eq!(menu().cursor(), 0);
    }

    #[test]
    fn cursor_moves_down() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        );
        assert_eq!(m.cursor(), 1);
    }

    #[test]
    fn cursor_clamps_at_top() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                up: true,
                ..input()
            },
        );
        assert_eq!(m.cursor(), 0);
    }

    #[test]
    fn cursor_clamps_at_bottom() {
        let mut m = menu();
        for _ in 0..10 {
            tick(
                &mut m,
                &MenuInput {
                    down: true,
                    ..input()
                },
            );
        }
        assert_eq!(m.cursor(), 4);
    }

    #[test]
    fn cursor_moves_up_after_down() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        );
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        );
        tick(
            &mut m,
            &MenuInput {
                up: true,
                ..input()
            },
        );
        assert_eq!(m.cursor(), 1);
    }

    #[test]
    fn game_mode_toggles_on_right() {
        let mut m = menu(); // cursor=0, mode=Master
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        );
        assert_eq!(m.game_mode(), GameMode::TwentyG);
    }

    #[test]
    fn game_mode_toggles_back_on_second_right() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        );
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        );
        assert_eq!(m.game_mode(), GameMode::Master);
    }

    #[test]
    fn game_mode_toggles_on_left() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                left: true,
                ..input()
            },
        );
        assert_eq!(m.game_mode(), GameMode::TwentyG);
    }

    #[test]
    fn rotation_toggles_when_cursor_on_rotation() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=1
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        );
        assert_eq!(m.rotation(), Kind::Srs);
    }

    #[test]
    fn toggle_noop_when_cursor_not_on_item() {
        let mut m = menu(); // cursor=0 (game mode)
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=1 (rotation)
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        ); // toggles rotation, not game mode
        assert_eq!(m.game_mode(), GameMode::Master); // unchanged
    }

    #[test]
    fn confirm_on_hiscores_item_opens_hiscores() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=1
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=2 (HI SCORES)
        tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert_eq!(m.screen(), MenuScreen::HiScores);
    }

    #[test]
    fn confirm_on_controls_item_opens_controls() {
        let mut m = menu();
        for _ in 0..3 {
            tick(
                &mut m,
                &MenuInput {
                    down: true,
                    ..input()
                },
            ); // cursor=3
        }
        tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert_eq!(m.screen(), MenuScreen::Controls);
    }

    #[test]
    fn back_from_hiscores_returns_to_main() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        );
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=2
        tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        ); // open HiScores
        tick(
            &mut m,
            &MenuInput {
                back: true,
                ..input()
            },
        );
        assert_eq!(m.screen(), MenuScreen::Main);
    }

    #[test]
    fn cursor_preserved_after_returning_from_subscreen() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        );
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=2
        tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        ); // open HiScores
        tick(
            &mut m,
            &MenuInput {
                back: true,
                ..input()
            },
        );
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn confirm_on_toggle_item_does_not_open_subscreen() {
        let mut m = menu(); // cursor=0
        tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert_eq!(m.screen(), MenuScreen::Main);
    }

    #[test]
    fn start_returns_start_game_with_defaults() {
        let mut m = menu();
        for _ in 0..4 {
            tick(
                &mut m,
                &MenuInput {
                    down: true,
                    ..input()
                },
            ); // cursor=4
        }
        let result = tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert!(matches!(
            result,
            MenuResult::StartGame {
                mode: GameMode::Master,
                rotation: Kind::Ars,
            }
        ));
    }

    #[test]
    fn start_returns_selected_settings() {
        let mut m = menu();
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        ); // game mode → TwentyG (cursor=0)
        tick(
            &mut m,
            &MenuInput {
                down: true,
                ..input()
            },
        ); // cursor=1
        tick(
            &mut m,
            &MenuInput {
                right: true,
                ..input()
            },
        ); // rotation → Srs
        for _ in 0..3 {
            tick(
                &mut m,
                &MenuInput {
                    down: true,
                    ..input()
                },
            ); // cursor=4
        }
        let result = tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert!(matches!(
            result,
            MenuResult::StartGame {
                mode: GameMode::TwentyG,
                rotation: Kind::Srs,
            }
        ));
    }

    #[test]
    fn confirm_on_non_start_item_returns_stay() {
        let mut m = menu(); // cursor=0
        let result = tick(
            &mut m,
            &MenuInput {
                confirm: true,
                ..input()
            },
        );
        assert!(matches!(result, MenuResult::Stay));
    }
}

#[cfg(test)]
mod srs_tests {
    use super::*;

    #[test]
    fn srs_rotation_snap_i() {
        insta::assert_snapshot!(rotation_snap(PieceKind::I, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_o() {
        insta::assert_snapshot!(rotation_snap(PieceKind::O, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_t() {
        insta::assert_snapshot!(rotation_snap(PieceKind::T, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_s() {
        insta::assert_snapshot!(rotation_snap(PieceKind::S, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_z() {
        insta::assert_snapshot!(rotation_snap(PieceKind::Z, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_j() {
        insta::assert_snapshot!(rotation_snap(PieceKind::J, make_srs_game));
    }

    #[test]
    fn srs_rotation_snap_l() {
        insta::assert_snapshot!(rotation_snap(PieceKind::L, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_i() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::I, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_t() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::T, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_j() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::J, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_l() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::L, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_s() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::S, make_srs_game));
    }

    #[test]
    fn srs_wall_kick_snap_z() {
        insta::assert_snapshot!(wall_kick_snap(PieceKind::Z, make_srs_game));
    }
}

#[test]
fn lock_and_move_regression_test() {
    // Make sure that locking a piece on the same frame you move it does something sensible.
    let mut game = make_game_with(GameMode::TwentyG, Kind::Ars, PieceKind::O);
    let mut input_state = InputState::empty();
    game.board = board_from_ascii(
        "
        ....O.....
        OOOOOOOOOO
    ",
    );
    game.tick(&input_state);
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
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │        [][]        │
      │        [][]        │
      │        ##          │
      │####################│
    20└────────────────────┘
    ");
    // TODO-someday: Add input state helpers.
    //
    // Press right and down on the same frame. We want this to either lock the piece or move it to the right (where it will fall a square).
    let mut inputs = HashSet::new();
    inputs.insert(GameKey::Right);
    inputs.insert(GameKey::SoftDrop);
    input_state.held = inputs.clone();
    input_state.just_pressed = inputs.clone();
    game.tick(&input_state);
    // Here, locking is applied first before horizontal movement, so locking wins.
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
      │                    │
      │                    │
      │                    │
    15│- - - - - - - - - - │
      │        ####        │
      │        ####        │
      │        ##          │
      │####################│
    20└────────────────────┘
    ");
}

// ---------------------------------------------------------------------------
// drain_events
// ---------------------------------------------------------------------------

/// Helper: fill all cells in a row on the given board.
fn fill_row(board: &mut Board, row: usize) {
    for c in 0..BOARD_COLS {
        board[row][c] = Some(PieceKind::O);
    }
}

#[test]
fn drain_events_no_clear_is_empty() {
    let mut game = make_game(PieceKind::T);
    // Drop a T-piece into an empty board — no line clear.
    while game.piece_phase == PiecePhase::Falling {
        idle(&mut game, 1);
    }
    let events = game.drain_events();
    assert!(events.is_empty(), "no line clear should produce no events");
}

#[test]
fn drain_events_single_clear() {
    let mut game = make_game(PieceKind::I);
    // Pre-fill the bottom row with gaps only where the I-piece will land.
    fill_row(&mut game.board, BOARD_ROWS - 1);
    game.board[BOARD_ROWS - 1][3] = None;
    game.board[BOARD_ROWS - 1][4] = None;
    game.board[BOARD_ROWS - 1][5] = None;
    game.board[BOARD_ROWS - 1][6] = None;
    // Place active I-piece at the bottom row in horizontal orientation.
    game.active.row = BOARD_ROWS as i32 - 2;
    game.active.col = 3;
    // Drop to floor and enter locking state
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking
    // Lock immediately with soft drop.
    press(&mut game, GameKey::SoftDrop);
    let events = game.drain_events();
    let counts: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            crate::types::GameEvent::LineClear { count } => Some(*count),
        })
        .collect();
    assert_eq!(counts, vec![1]);
}

#[test]
fn drain_events_clears_after_drain() {
    let mut game = make_game(PieceKind::I);
    fill_row(&mut game.board, BOARD_ROWS - 1);
    game.board[BOARD_ROWS - 1][3] = None;
    game.board[BOARD_ROWS - 1][4] = None;
    game.board[BOARD_ROWS - 1][5] = None;
    game.board[BOARD_ROWS - 1][6] = None;
    game.active.row = BOARD_ROWS as i32 - 2;
    game.active.col = 3;
    // Drop to floor and enter locking state
    while game.try_move(0, 1) {}
    idle(&mut game, 1); // enter Locking
    // Lock immediately with soft drop.
    press(&mut game, GameKey::SoftDrop);
    let _ = game.drain_events();
    // Second drain should be empty.
    let events2 = game.drain_events();
    assert!(events2.is_empty(), "drain_events should clear the buffer");
}
