use bevy::prelude::*;
use crate::components::*;
use crate::constants::{ARE_DAS_FROZEN_FRAMES, SPAWN_DELAY_NORMAL, gravity_g};
use crate::data::{GameEvent, GameKey, HorizDir, PiecePhase, RotationDirection};
use crate::randomizer::Randomizer;
use crate::resources::*;
use crate::rotation_system::{PieceState, RotationSystem};

#[allow(clippy::too_many_arguments)]
pub fn spawning_system(
    mut piece: Query<(&mut PieceKindComp, &mut PiecePosition, &mut PieceRotation), With<ActivePiece>>,
    mut phase: ResMut<CurrentPhase>,
    mut next: ResMut<NextPiece>,
    mut progress: ResMut<GameProgress>,
    mut das: ResMut<DasState>,
    mut rotation_buffer: ResMut<RotationBuffer>,
    mut drop_tracking: ResMut<DropTracking>,
    mut randomizer: ResMut<Randomizer>,
    rot_sys: Res<RotationSystemRes>,
    mode: Res<GameModeRes>,
    board: Res<Board>,
    input: Res<InputState>,
    mut game_events: MessageWriter<GameEvent>,
) {
    if progress.game_over || progress.game_won { return; }
    let PiecePhase::Spawning { ticks_left } = &mut phase.0 else { return };

    if input.0.held.contains(&GameKey::RotateCw) {
        rotation_buffer.0 = Some(RotationDirection::Clockwise);
    } else if input.0.held.contains(&GameKey::RotateCcw) {
        rotation_buffer.0 = Some(RotationDirection::Counterclockwise);
    } else {
        rotation_buffer.0 = None;
    }

    let tl = *ticks_left;
    if tl == 0 {
        let Ok((mut k, mut pos, mut rot)) = piece.single_mut() else { return };
        if can_piece_increment(progress.level) {
            progress.level += 1;
        }
        let next_kind = randomizer.next();
        k.0 = next.0;
        next.0 = next_kind;
        pos.col = 3;
        pos.row = 0;
        rot.0 = 0;
        drop_tracking.gravity_accumulator = 0;
        drop_tracking.soft_drop_frames = 0;
        drop_tracking.sonic_drop_rows = 0;
        phase.0 = PiecePhase::Falling;

        if let Some(dir) = rotation_buffer.0.take() {
            let state = PieceState { kind: k.0, rotation: rot.0, col: pos.col, row: pos.row };
            if let Some(new) = rot_sys.0.try_rotate(&state, dir, &board.0) {
                pos.col = new.col;
                pos.row = new.row;
                rot.0 = new.rotation;
            }
        }

        if !rot_sys.0.fits(&board.0, k.0, pos.col, pos.row, rot.0) {
            progress.game_over = true;
            game_events.write(GameEvent::GameEnded);
        }

        drop_tracking.gravity_accumulator += gravity_g(mode.0, progress.level);
        let drops = drop_tracking.gravity_accumulator / 256;
        drop_tracking.gravity_accumulator %= 256;
        for _ in 0..drops {
            let new_row = pos.row + 1;
            if rot_sys.0.fits(&board.0, k.0, pos.col, new_row, rot.0) {
                pos.row = new_row;
            } else { break; }
        }
    } else {
        *ticks_left -= 1;
        if tl <= SPAWN_DELAY_NORMAL - ARE_DAS_FROZEN_FRAMES {
            let horiz = if input.0.held.contains(&GameKey::Left) { Some(HorizDir::Left) }
                else if input.0.held.contains(&GameKey::Right) { Some(HorizDir::Right) }
                else { None };
            match horiz {
                None => { das.direction = None; das.counter = 0; }
                Some(dir) => {
                    if das.direction != Some(dir) {
                        das.direction = Some(dir);
                        das.counter = 0;
                    } else {
                        das.counter += 1;
                    }
                }
            }
        }
    }
}

fn can_piece_increment(level: u32) -> bool {
    level % 100 != 99 && level != 998
}
