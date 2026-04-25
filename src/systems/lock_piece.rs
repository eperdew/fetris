use bevy::prelude::*;
use crate::components::*;
use crate::constants::{LINE_CLEAR_DELAY, SPAWN_DELAY_NORMAL};
use crate::data::{
    BOARD_COLS, BOARD_ROWS, GameEvent, GameKey, InputSnapshot, JudgeEvent,
    PieceKind, PiecePhase, RotationDirection,
};
use crate::resources::*;
use crate::rotation_system::RotationSystem;

/// Writes the active piece into the board, detects line clears, queues compaction,
/// emits events, and transitions PiecePhase. Mirrors `Game::lock_piece`.
#[allow(clippy::too_many_arguments)]
pub fn lock_piece(
    board: &mut Board,
    progress: &mut GameProgress,
    phase: &mut CurrentPhase,
    pending: &mut PendingCompaction,
    rotation_buffer: &mut RotationBuffer,
    drop_tracking: &DropTracking,
    rot_sys: &dyn RotationSystem,
    piece_kind: PieceKind,
    piece_pos: PiecePosition,
    piece_rot: PieceRotation,
    input: &InputSnapshot,
    judge_events: &mut MessageWriter<JudgeEvent>,
    game_events: &mut MessageWriter<GameEvent>,
) {
    // 1. Write piece cells into the board.
    for (dc, dr) in rot_sys.cells(piece_kind, piece_rot.0) {
        let c = (piece_pos.col + dc) as usize;
        let r = (piece_pos.row + dr) as usize;
        if r < BOARD_ROWS {
            board.0[r][c] = Some(piece_kind);
        }
    }

    // 2. Detect cleared lines.
    let cleared: Vec<usize> = (0..BOARD_ROWS)
        .filter(|&r| board.0[r].iter().all(|c| c.is_some()))
        .collect();
    let count = cleared.len() as u32;

    if count > 0 {
        pending.0 = cleared;
        progress.lines += count;
        progress.level = (progress.level + count).min(999);
        if progress.level == 999 {
            progress.game_won = true;
            game_events.write(GameEvent::GameEnded);
        }
        game_events.write(GameEvent::LineClear { count });
    }

    // 3. Buffer held rotation for next piece.
    if input.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    }

    // 4. Phase transition: LineClearDelay or Spawning.
    phase.0 = if count > 0 {
        PiecePhase::LineClearDelay { ticks_left: LINE_CLEAR_DELAY }
    } else {
        PiecePhase::Spawning { ticks_left: SPAWN_DELAY_NORMAL }
    };

    // 5. Emit JudgeEvent.
    let judge_event = if count > 0 {
        // Match original behavior: cleared_playfield is true iff every row outside
        // the cleared rows is empty (cleared rows are still Some at this point).
        let cleared_playfield = board.0.iter().enumerate().all(|(r, row)| {
            pending.0.contains(&r) || row.iter().all(|c| c.is_none())
        });
        JudgeEvent::ClearedLines {
            level: progress.level,  // post-increment level — matches original Game::lock_piece
            cleared_playfield,
            num_lines: count,
            frames_soft_drop_held: drop_tracking.soft_drop_frames,
            sonic_drop_rows: drop_tracking.sonic_drop_rows,
            ticks_elapsed: progress.ticks_elapsed,
        }
    } else {
        JudgeEvent::LockedWithoutClear
    };
    judge_events.write(judge_event);
}
