use bevy::prelude::*;

mod app_state;
mod components;
mod constants;
mod data;
mod judge;
mod randomizer;
mod resources;
mod rotation_system;
mod snapshot;
mod start_game;
mod systems;

use crate::data::{GameEvent, JudgeEvent};
use crate::judge::{judge_system, Judge};
use crate::systems::active::active_phase_system;
use crate::systems::game_over::game_over_check;
use crate::systems::line_clear_delay::line_clear_delay_system;
use crate::systems::spawning::spawning_system;
use crate::systems::tick::tick_counter;
use app_state::AppState;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_message::<JudgeEvent>()
        .add_message::<GameEvent>()
        .init_resource::<crate::resources::Board>()
        .init_resource::<crate::resources::CurrentPhase>()
        .init_resource::<crate::resources::GameProgress>()
        .init_resource::<crate::resources::DasState>()
        .init_resource::<crate::resources::RotationBuffer>()
        .init_resource::<crate::resources::PendingCompaction>()
        .init_resource::<crate::resources::DropTracking>()
        .init_resource::<crate::resources::InputState>()
        .init_resource::<crate::randomizer::Randomizer>()
        .init_resource::<Judge>()
        // TODO: inserted by start_game (Task 17): NextPiece, RotationSystemRes, GameModeRes, RotationKind
        .add_systems(
            FixedUpdate,
            (
                tick_counter,
                active_phase_system,
                line_clear_delay_system,
                spawning_system,
                judge_system,
                game_over_check,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}
