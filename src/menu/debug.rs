//! Debug visual test bench.
//!
//! Reuses the live render pipeline by populating the same resources gameplay
//! uses, then driving effects via `GameEvent::LineClear` events and a
//! debug-only `state_overlay` override on `render_state_text`.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::components::ActivePieceBundle;
use crate::data::{GameMode, Kind, PieceKind, PiecePhase};
use crate::judge::Judge;
use crate::resources::{
    Board, CurrentPhase, GameModeRes, GameProgress, NextPiece, PendingCompaction, RotationKind,
    RotationSystemRes,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum DebugStateOverlay {
    #[default]
    None,
    Ready,
    GameOver,
    Won,
}

#[derive(Resource, Default)]
pub struct DebugSceneState {
    pub hud_preset: usize,
    pub state_overlay: DebugStateOverlay,
    pub state_overlay_ticks_left: u32,
    pub line_clear_cleanup_ticks_left: u32,
}

/// Six presets covering Grade 9 → S9 with matching score/level.
/// Score values land at the lower bound of each grade band.
pub(crate) const HUD_PRESETS: &[(u32, u32)] = &[
    (0, 0),        // Grade 9
    (1700, 250),   // Grade 6
    (8500, 500),   // Grade 2
    (16000, 700),  // Grade S1
    (52000, 850),  // Grade S5
    (120000, 999), // Grade S9
];

pub(crate) fn apply_hud_preset(judge: &mut Judge, progress: &mut GameProgress, idx: usize) {
    let (score, level) = HUD_PRESETS[idx % HUD_PRESETS.len()];
    *judge = Judge::new();
    judge.set_score_for_debug(score);
    progress.level = level;
    progress.lines = level / 10;
    progress.ticks_elapsed = (level as u64) * 60;
    progress.initial_delay_ticks = 0;
    progress.game_over = false;
    progress.game_won = false;
}

pub fn on_enter_debug(world: &mut World) {
    world.insert_resource(RotationSystemRes(Kind::Ars.create()));
    world.insert_resource(GameModeRes(GameMode::Master));
    world.insert_resource(RotationKind(Kind::Ars));
    world.insert_resource(NextPiece(PieceKind::T));
    world.insert_resource(CurrentPhase(PiecePhase::Falling));
    world.insert_resource(DebugSceneState::default());

    let prior: Vec<Entity> = world
        .query::<(Entity, &crate::components::ActivePiece)>()
        .iter(world)
        .map(|(e, _)| e)
        .collect();
    for e in prior {
        world.despawn(e);
    }
    let mut bundle = ActivePieceBundle::new(PieceKind::T);
    bundle.position.row = 8;
    bundle.position.col = 4;
    world.spawn(bundle);

    world.resource_scope::<Judge, _>(|world, mut judge| {
        let mut progress = world.resource_mut::<GameProgress>();
        apply_hud_preset(&mut judge, &mut progress, 0);
    });
}

pub fn debug_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
    mut events: bevy::ecs::message::MessageWriter<crate::data::GameEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    mut menu: ResMut<crate::menu::state::MenuState>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        menu.screen = crate::menu::state::MenuScreen::Main;
        next_state.set(AppState::Menu);
        return;
    }

    let count = if keys.just_pressed(KeyCode::Digit1) {
        Some(1u32)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(4)
    } else {
        None
    };

    if let Some(count) = count {
        if scene.line_clear_cleanup_ticks_left > 0 {
            return;
        }
        let n = count as usize;
        let rows: Vec<usize> = (crate::data::BOARD_ROWS - n..crate::data::BOARD_ROWS).collect();
        for &r in &rows {
            for c in 0..crate::data::BOARD_COLS {
                board.0[r][c] = Some(PieceKind::T);
            }
        }
        pending.0 = rows;
        events.write(crate::data::GameEvent::LineClear { count });
        scene.line_clear_cleanup_ticks_left = 3;
    }
}

pub fn debug_tick_system(
    mut scene: ResMut<DebugSceneState>,
    mut board: ResMut<Board>,
    mut pending: ResMut<PendingCompaction>,
) {
    if scene.line_clear_cleanup_ticks_left > 0 {
        scene.line_clear_cleanup_ticks_left -= 1;
        if scene.line_clear_cleanup_ticks_left == 0 {
            for r in &pending.0 {
                for c in 0..crate::data::BOARD_COLS {
                    board.0[*r][c] = None;
                }
            }
            pending.0.clear();
        }
    }
}
