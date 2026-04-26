use crate::components::*;
use crate::data::{BoardGrid, Grade, PieceKind, PiecePhase};
use crate::judge::Judge;
use crate::resources::*;
use crate::rotation_system::RotationSystem;
use bevy::prelude::*;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GameSnapshot {
    pub board: BoardGrid,
    pub active_kind: Option<PieceKind>,
    pub active_cells: Option<[(i32, i32); 4]>,
    pub ghost_cells: Option<[(i32, i32); 4]>,
    pub active_preview_offsets: [(i32, i32); 4],
    pub active_preview_y_offset: i32,
    pub next_kind: PieceKind,
    pub next_preview_offsets: [(i32, i32); 4],
    pub next_preview_y_offset: i32,
    pub rows_pending_compaction: Vec<usize>,
    pub level: u32,
    pub lines: u32,
    pub ticks_elapsed: u64,
    pub score: u32,
    pub grade: Grade,
    pub game_over: bool,
    pub game_won: bool,
}

impl GameSnapshot {
    pub fn from_world(world: &mut World) -> Self {
        let phase = world.resource::<CurrentPhase>().0;
        let board = world.resource::<Board>().0;
        let progress_level;
        let progress_lines;
        let progress_ticks;
        let progress_game_over;
        let progress_game_won;
        {
            let progress = world.resource::<GameProgress>();
            progress_level = progress.level;
            progress_lines = progress.lines;
            progress_ticks = progress.ticks_elapsed;
            progress_game_over = progress.game_over;
            progress_game_won = progress.game_won;
        }
        let pending = world.resource::<PendingCompaction>().0.clone();
        let next = world.resource::<NextPiece>().0;
        let (judge_score, judge_grade) = {
            let judge = world.resource::<Judge>();
            (judge.score(), judge.grade())
        };

        let (active_kind_val, active_pos, active_rot) = {
            let mut q = world.query_filtered::<
                (&PieceKindComp, &PiecePosition, &PieceRotation),
                With<ActivePiece>,
            >();
            let (k, p, r) = q.single(world).expect("ActivePiece entity");
            (k.0, *p, *r)
        };

        let rot_sys = world.resource::<RotationSystemRes>();

        let show_active = !matches!(
            phase,
            PiecePhase::Spawning { .. } | PiecePhase::LineClearDelay { .. }
        );
        let active_offsets = rot_sys.0.cells(active_kind_val, active_rot.0);

        let (active_kind, active_cells, ghost_cells) = if show_active {
            let cells = active_offsets.map(|(dc, dr)| (active_pos.col + dc, active_pos.row + dr));
            let ghost_row = compute_ghost_row(
                &board,
                &*rot_sys.0,
                active_kind_val,
                active_rot.0,
                active_pos,
            );
            let ghost = if ghost_row != active_pos.row {
                Some(active_offsets.map(|(dc, dr)| (active_pos.col + dc, ghost_row + dr)))
            } else {
                None
            };
            (Some(active_kind_val), Some(cells), ghost)
        } else {
            (None, None, None)
        };

        let next_offsets = rot_sys.0.cells(next, 0);

        GameSnapshot {
            board,
            active_kind,
            active_cells,
            ghost_cells,
            active_preview_offsets: active_offsets,
            active_preview_y_offset: rot_sys.0.preview_y_offset(active_kind_val),
            next_kind: next,
            next_preview_offsets: next_offsets,
            next_preview_y_offset: rot_sys.0.preview_y_offset(next),
            rows_pending_compaction: pending,
            level: progress_level,
            lines: progress_lines,
            ticks_elapsed: progress_ticks,
            score: judge_score,
            grade: judge_grade,
            game_over: progress_game_over,
            game_won: progress_game_won,
        }
    }
}

fn compute_ghost_row(
    board: &BoardGrid,
    rot_sys: &dyn RotationSystem,
    kind: PieceKind,
    rotation: usize,
    pos: PiecePosition,
) -> i32 {
    use crate::data::{BOARD_COLS, BOARD_ROWS};
    let mut ghost_row = pos.row;
    loop {
        let next = ghost_row + 1;
        let blocked = rot_sys.cells(kind, rotation).iter().any(|&(dc, dr)| {
            let c = pos.col + dc;
            let r = next + dr;
            r >= BOARD_ROWS as i32
                || (c >= 0
                    && c < BOARD_COLS as i32
                    && r >= 0
                    && board[r as usize][c as usize].is_some())
        });
        if blocked {
            break;
        }
        ghost_row = next;
    }
    ghost_row
}
