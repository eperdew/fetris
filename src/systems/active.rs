use crate::components::*;
use crate::constants::{gravity_g, DAS_CHARGE, DAS_REPEAT, LOCK_DELAY};
use crate::data::{GameEvent, GameKey, HorizDir, JudgeEvent, PiecePhase, RotationDirection};
use crate::resources::*;
use crate::rotation_system::{PieceState, RotationSystem};
use crate::systems::lock_piece::lock_piece;
use bevy::prelude::*;

#[allow(clippy::too_many_arguments)]
pub fn active_phase_system(
    mut piece: Query<(&PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>>,
    mut board: ResMut<Board>,
    mut phase: ResMut<CurrentPhase>,
    mut progress: ResMut<GameProgress>,
    mut das: ResMut<DasState>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    mut pending: ResMut<PendingCompaction>,
    mut drop_tracking: ResMut<DropTracking>,
    rot_sys: Res<RotationSystemRes>,
    mode: Res<GameModeRes>,
    input: Res<InputState>,
    start: Res<TickStartPhase>,
    mut judge_events: MessageWriter<JudgeEvent>,
    mut game_events: MessageWriter<GameEvent>,
) {
    if progress.game_over || progress.game_won {
        return;
    }
    // Gate on the start-of-tick phase to prevent this system from running after
    // a phase transition made by an earlier system in the same tick.
    let Some(start_phase) = start.0 else {
        return;
    };
    if !matches!(start_phase, PiecePhase::Falling | PiecePhase::Locking { .. }) {
        return;
    }
    if !matches!(phase.0, PiecePhase::Falling | PiecePhase::Locking { .. }) {
        return;
    }

    let Ok((kind, mut pos, mut rot)) = piece.single_mut() else {
        return;
    };
    let kind = kind.0;
    let input_snapshot = input.0.clone();

    // Extract an immutable reference to the rotation system trait object once,
    // to avoid re-borrowing the Res inside closures.
    let rot_sys_ref: &dyn RotationSystem = &*rot_sys.0;

    let try_move = |pos: &mut PiecePosition,
                    rot: &PieceRotation,
                    dcol: i32,
                    drow: i32,
                    board: &Board|
     -> bool {
        let new_col = pos.col + dcol;
        let new_row = pos.row + drow;
        if rot_sys_ref.fits(&board.0, kind, new_col, new_row, rot.0) {
            pos.col = new_col;
            pos.row = new_row;
            true
        } else {
            false
        }
    };

    let try_rotate = |pos: &mut PiecePosition,
                      rot: &mut PieceRotation,
                      dir: RotationDirection,
                      board: &Board| {
        let state = PieceState {
            kind,
            rotation: rot.0,
            col: pos.col,
            row: pos.row,
        };
        if let Some(new) = rot_sys_ref.try_rotate(&state, dir, &board.0) {
            pos.col = new.col;
            pos.row = new.row;
            rot.0 = new.rotation;
        }
    };

    // Phase 2: rotation
    if input_snapshot.just_pressed.contains(&GameKey::RotateCw) {
        try_rotate(&mut pos, &mut rot, RotationDirection::Clockwise, &board);
    } else if input_snapshot.just_pressed.contains(&GameKey::RotateCcw) {
        try_rotate(
            &mut pos,
            &mut rot,
            RotationDirection::Counterclockwise,
            &board,
        );
    }

    // Phase 3: sonic drop
    if input_snapshot.just_pressed.contains(&GameKey::SonicDrop) {
        let row_before = pos.row;
        while try_move(&mut pos, &rot, 0, 1, &board) {}
        drop_tracking.sonic_drop_rows += (pos.row - row_before) as u32;
        if matches!(phase.0, PiecePhase::Falling) {
            phase.0 = PiecePhase::Locking {
                ticks_left: LOCK_DELAY,
            };
            game_events.write(GameEvent::PieceBeganLocking);
        }
        return;
    }

    // Phase 4: soft drop
    if input_snapshot.held.contains(&GameKey::SoftDrop) {
        drop_tracking.soft_drop_frames += 1;
        match phase.0 {
            PiecePhase::Locking { .. } => {
                lock_piece(
                    &mut board,
                    &mut progress,
                    &mut phase,
                    &mut pending,
                    &mut rotation_buffer,
                    &drop_tracking,
                    rot_sys_ref,
                    kind,
                    *pos,
                    *rot,
                    &input_snapshot,
                    &mut judge_events,
                    &mut game_events,
                );
                return;
            }
            _ => {
                try_move(&mut pos, &rot, 0, 1, &board);
                drop_tracking.gravity_accumulator = 0;
            }
        }
    }

    // Phase 5: horizontal DAS
    let horiz = if input_snapshot.held.contains(&GameKey::Left) {
        Some(HorizDir::Left)
    } else if input_snapshot.held.contains(&GameKey::Right) {
        Some(HorizDir::Right)
    } else {
        None
    };

    match horiz {
        None => {
            das.direction = None;
            das.counter = 0;
        }
        Some(dir) => {
            if das.direction != Some(dir) {
                das.direction = Some(dir);
                das.counter = 0;
                let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                try_move(&mut pos, &rot, dcol, 0, &board);
            } else {
                das.counter += 1;
                if das.counter >= DAS_CHARGE && (das.counter - DAS_CHARGE) % DAS_REPEAT == 0 {
                    let dcol = if dir == HorizDir::Left { -1 } else { 1 };
                    try_move(&mut pos, &rot, dcol, 0, &board);
                }
            }
        }
    }

    // Phase 6: gravity (G/256 accumulator)
    let row_before = pos.row;
    drop_tracking.gravity_accumulator += gravity_g(mode.0, progress.level);
    let drops = drop_tracking.gravity_accumulator / 256;
    drop_tracking.gravity_accumulator %= 256;
    for _ in 0..drops {
        if !try_move(&mut pos, &rot, 0, 1, &board) {
            break;
        }
    }
    let moved_down = pos.row > row_before;

    // Phase 7: lock state transitions
    let on_floor = !rot_sys_ref.fits(&board.0, kind, pos.col, pos.row + 1, rot.0);
    match phase.0 {
        PiecePhase::Falling => {
            if on_floor {
                phase.0 = PiecePhase::Locking {
                    ticks_left: LOCK_DELAY,
                };
                game_events.write(GameEvent::PieceBeganLocking);
            }
        }
        PiecePhase::Locking { ref mut ticks_left } => {
            if !on_floor {
                phase.0 = PiecePhase::Falling;
            } else if moved_down {
                *ticks_left = LOCK_DELAY;
            } else if *ticks_left == 0 {
                lock_piece(
                    &mut board,
                    &mut progress,
                    &mut phase,
                    &mut pending,
                    &mut rotation_buffer,
                    &drop_tracking,
                    rot_sys_ref,
                    kind,
                    *pos,
                    *rot,
                    &input_snapshot,
                    &mut judge_events,
                    &mut game_events,
                );
            } else {
                *ticks_left -= 1;
            }
        }
        _ => unreachable!(),
    }
}
