use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};

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
mod stub_storage;
pub(crate) mod systems;

#[cfg(test)]
mod tests;

use crate::data::{GameEvent, JudgeEvent};
use crate::judge::{judge_system, Judge};
use crate::systems::active::active_phase_system;
use crate::systems::game_over::game_over_check;
use crate::systems::line_clear_delay::line_clear_delay_system;
use crate::systems::spawning::spawning_system;
use crate::systems::tick::tick_counter;
use app_state::AppState;

fn setup_camera(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: 560.0,
        height: 780.0,
    };
    projection.viewport_origin = Vec2::new(0.0, 1.0); // top-left origin
    commands.spawn((
        Camera2d,
        Projection::Orthographic(projection),
        Transform::from_scale(Vec3::new(1.0, -1.0, 1.0)),
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "fetris".into(),
                resolution: WindowResolution::new(560, 780),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_egui::EguiPlugin::default())
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
        .init_resource::<crate::resources::TickStartPhase>()
        .init_resource::<crate::randomizer::Randomizer>()
        .init_resource::<Judge>()
        .init_resource::<stub_storage::GameConfigRes>()
        .init_resource::<stub_storage::HiScoresRes>()
        .init_resource::<stub_storage::MutedRes>()
        // TODO: inserted by start_game (Task 17): NextPiece, RotationSystemRes, GameModeRes, RotationKind
        .add_systems(Startup, setup_camera)
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
