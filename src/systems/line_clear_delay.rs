use crate::constants::SPAWN_DELAY_NORMAL;
use crate::data::{GameKey, PiecePhase, RotationDirection, BOARD_COLS, BOARD_ROWS};
use crate::resources::*;
use bevy::prelude::*;

pub fn line_clear_delay_system(
    mut phase: ResMut<CurrentPhase>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    progress: Res<GameProgress>,
    input: Res<InputState>,
    start: Res<TickStartPhase>,
) {
    if progress.game_over || progress.game_won {
        return;
    }
    // Gate on the start-of-tick phase to prevent running after a phase
    // transition made by an earlier system in the same tick.
    let Some(start_phase) = start.0 else {
        return;
    };
    if !matches!(start_phase, PiecePhase::LineClearDelay { .. }) {
        return;
    }
    let PiecePhase::LineClearDelay { ticks_left } = &mut phase.0 else {
        return;
    };

    if input.0.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.0.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    }

    if *ticks_left == 0 {
        compact_pending(&mut board.0, &mut pending.0);
        phase.0 = PiecePhase::Spawning {
            ticks_left: SPAWN_DELAY_NORMAL,
        };
    } else {
        *ticks_left -= 1;
    }
}

fn compact_pending(board: &mut crate::data::BoardGrid, pending: &mut Vec<usize>) {
    if pending.is_empty() {
        return;
    }
    let mut new_board: crate::data::BoardGrid = [[None; BOARD_COLS]; BOARD_ROWS];
    let kept: Vec<[Option<crate::data::PieceKind>; BOARD_COLS]> = board
        .iter()
        .enumerate()
        .filter(|(r, _)| !pending.contains(r))
        .map(|(_, row)| *row)
        .collect();
    let offset = BOARD_ROWS - kept.len();
    for (i, row) in kept.into_iter().enumerate() {
        new_board[offset + i] = row;
    }
    *board = new_board;
    pending.clear();
}
