use crate::app_state::AppState;
use crate::components::*;
use crate::data::{
    BoardGrid, GameKey, GameMode, InputSnapshot, Kind, PieceKind, PiecePhase, BOARD_COLS,
    BOARD_ROWS,
};
use crate::judge::Judge;
use crate::resources::*;
use crate::snapshot::GameSnapshot;
use crate::start_game::{start_game, StartGameOptions};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use std::collections::HashSet;

pub fn headless_app() -> App {
    use crate::data::{GameEvent, JudgeEvent};
    use crate::judge::judge_system;
    use crate::systems::tick::tick_counter;

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_message::<JudgeEvent>() // 0.18 API
        .add_message::<GameEvent>() // 0.18 API
        .add_systems(
            FixedUpdate,
            (
                tick_counter,
                crate::systems::active::active_phase_system,
                crate::systems::line_clear_delay::line_clear_delay_system,
                crate::systems::spawning::spawning_system,
                judge_system,
                crate::systems::game_over::game_over_check,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        );
    app
}

pub fn start_with(app: &mut App, mode: GameMode, rotation: Kind, kind: PieceKind) {
    start_game(
        app.world_mut(),
        StartGameOptions {
            mode,
            rotation,
            seed: Some(0),
        },
    );
    app.update();
    let mut q = app.world_mut().query_filtered::<
        (&mut PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>,
    >();
    let (mut k, mut pos, mut rot) = q.single_mut(app.world_mut()).unwrap();
    k.0 = kind;
    pos.col = 3;
    pos.row = 8;
    rot.0 = 0;
    app.world_mut().resource_mut::<Board>().0 = [[None; BOARD_COLS]; BOARD_ROWS];
    app.world_mut().resource_mut::<NextPiece>().0 = kind;
    app.world_mut().resource_mut::<CurrentPhase>().0 = PiecePhase::Falling;
}

pub fn make_app(kind: PieceKind) -> App {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Ars, kind);
    app
}

pub fn make_srs_app(kind: PieceKind) -> App {
    let mut app = headless_app();
    start_with(&mut app, GameMode::Master, Kind::Srs, kind);
    app
}

pub fn tick_with(app: &mut App, input: InputSnapshot) {
    app.world_mut().resource_mut::<InputState>().0 = input;
    let step = std::time::Duration::from_secs_f64(1.0 / 60.0);
    app.world_mut()
        .resource_mut::<Time<Fixed>>()
        .accumulate_overstep(step);
    app.update();
}

pub fn press(app: &mut App, key: GameKey) {
    let mut input = InputSnapshot::empty();
    input.held.insert(key);
    input.just_pressed.insert(key);
    tick_with(app, input);
}

pub fn hold(app: &mut App, keys: &[GameKey], ticks: u32) {
    let input = InputSnapshot {
        held: keys.iter().copied().collect(),
        just_pressed: HashSet::new(),
    };
    for _ in 0..ticks {
        tick_with(app, input.clone());
    }
}

pub fn idle(app: &mut App, ticks: u32) {
    for _ in 0..ticks {
        tick_with(app, InputSnapshot::empty());
    }
}

pub fn board_from_ascii(diagram: &str) -> BoardGrid {
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

pub fn snapshot(app: &mut App) -> GameSnapshot {
    GameSnapshot::from_world(app.world_mut())
}

pub fn active_kind(app: &mut App) -> PieceKind {
    let mut q = app
        .world_mut()
        .query_filtered::<&PieceKindComp, With<ActivePiece>>();
    q.single(app.world_mut()).unwrap().0
}

pub fn active_position(app: &mut App) -> PiecePosition {
    let mut q = app
        .world_mut()
        .query_filtered::<&PiecePosition, With<ActivePiece>>();
    *q.single(app.world_mut()).unwrap()
}

pub fn active_rotation(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<&PieceRotation, With<ActivePiece>>();
    q.single(app.world_mut()).unwrap().0
}

pub fn board(app: &mut App) -> BoardGrid {
    app.world().resource::<Board>().0
}

pub fn set_board(app: &mut App, b: BoardGrid) {
    app.world_mut().resource_mut::<Board>().0 = b;
}

pub fn piece_phase(app: &mut App) -> PiecePhase {
    app.world().resource::<CurrentPhase>().0
}

pub fn judge<'a>(app: &'a App) -> &'a Judge {
    app.world().resource::<Judge>()
}

pub fn level(app: &App) -> u32 {
    app.world().resource::<GameProgress>().level
}
pub fn lines(app: &App) -> u32 {
    app.world().resource::<GameProgress>().lines
}
pub fn ticks_elapsed(app: &App) -> u64 {
    app.world().resource::<GameProgress>().ticks_elapsed
}
pub fn game_over(app: &App) -> bool {
    app.world().resource::<GameProgress>().game_over
}
pub fn game_won(app: &App) -> bool {
    app.world().resource::<GameProgress>().game_won
}

pub fn active_abs(app: &mut App) -> Vec<(i32, i32)> {
    let kind = active_kind(app);
    let pos = active_position(app);
    let rot = active_rotation(app);
    let cells = app
        .world()
        .resource::<RotationSystemRes>()
        .0
        .cells(kind, rot);
    cells
        .into_iter()
        .map(|(dc, dr)| (pos.col + dc, pos.row + dr))
        .collect()
}

#[allow(dead_code)]
pub fn set_active_rot_col(app: &mut App, rot: usize, col: i32) {
    let mut q = app.world_mut().query_filtered::<
        (&mut PieceRotation, &mut PiecePosition), With<ActivePiece>,
    >();
    let (mut r, mut p) = q.single_mut(app.world_mut()).unwrap();
    r.0 = rot;
    p.col = col;
}

#[allow(dead_code)]
pub fn rotation_snap(kind: PieceKind, make: fn(PieceKind) -> App) -> String {
    let mut app = make(kind);
    let mut boards = Vec::new();
    for rot in 0..4 {
        let prev = active_abs(&mut app);
        press(&mut app, GameKey::RotateCw);
        boards.push((
            format!("{}→{}", rot, (rot + 1) % 4),
            board_lines(&mut app, &prev),
        ));
    }
    side_by_side(&boards)
}

/// Renders the board as a Vec of lines. If `prev_cells` is given (absolute board positions),
/// those cells show `'.`; current active piece shows `[]`; overlap shows `[]` (current wins).
#[allow(dead_code)]
pub fn board_lines(app: &mut App, prev_cells: &[(i32, i32)]) -> Vec<String> {
    let kind = active_kind(app);
    let pos = active_position(app);
    let rot = active_rotation(app);
    let active: [(i32, i32); 4] = app
        .world()
        .resource::<RotationSystemRes>()
        .0
        .cells(kind, rot)
        .map(|(dc, dr)| (pos.col + dc, pos.row + dr));

    let b = board(app);

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
            row.push_str(if b[r][c].is_some() {
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

#[allow(dead_code)]
pub fn side_by_side(boards: &[(String, Vec<String>)]) -> String {
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
#[allow(dead_code)]
pub fn wall_kick_snap(kind: PieceKind, make: fn(PieceKind) -> App) -> String {
    let mut boards = Vec::new();
    let app = make(kind);

    for &left_wall in &[true, false] {
        for start_rot in 0usize..4 {
            let rot_cells = app
                .world()
                .resource::<RotationSystemRes>()
                .0
                .cells(kind, start_rot);
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
                set_active_rot_col(&mut game, start_rot, flush_col);

                let col_before = active_position(&mut game).col;
                let prev = active_abs(&mut game);
                press(&mut game, key);

                // Only include when a kick actually happened
                if active_position(&mut game).col != col_before
                    && active_rotation(&mut game) == new_rot
                {
                    let wall = if left_wall { "L" } else { "R" };
                    let dir = if cw { "↻" } else { "↺" };
                    boards.push((
                        format!("{wall}{dir} {start_rot}→{new_rot}"),
                        board_lines(&mut game, &prev),
                    ));
                }
            }
        }
    }

    side_by_side(&boards)
}
