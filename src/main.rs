use bevy::camera::visibility::RenderLayers;
use bevy::camera::ScalingMode;
use bevy::prelude::*;
use bevy::window::{WindowPlugin, WindowResolution};
use bevy_pkv::PkvStore;

mod app_state;
mod components;
mod constants;
mod data;
mod hiscores;
mod judge;
mod menu;
mod randomizer;
mod render;
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

fn reset_game_on_enter_menu(
    mut commands: Commands,
    mut board: ResMut<crate::resources::Board>,
    mut judge: ResMut<crate::judge::Judge>,
    mut progress: ResMut<crate::resources::GameProgress>,
    mut das: ResMut<crate::resources::DasState>,
    mut rot_buf: ResMut<crate::resources::RotationBuffer>,
    mut pending: ResMut<crate::resources::PendingCompaction>,
    mut drop_tracking: ResMut<crate::resources::DropTracking>,
    mut tick_start: ResMut<crate::resources::TickStartPhase>,
    render_entities: Query<
        Entity,
        Or<(
            With<crate::components::ActivePiece>,
            With<crate::render::particles::Particle>,
            With<crate::render::board::BoardSprite>,
            With<crate::render::piece::PieceSprite>,
            With<crate::render::piece::NextPreviewSprite>,
            With<crate::render::hud::HudNode>,
            With<crate::render::overlays::StateText>,
            With<crate::render::overlays::LineClearOverlay>,
        )>,
    >,
) {
    *board = Default::default();
    *judge = Default::default();
    *progress = Default::default();
    *das = Default::default();
    *rot_buf = Default::default();
    *pending = Default::default();
    *drop_tracking = Default::default();
    *tick_start = Default::default();
    for e in &render_entities {
        commands.entity(e).despawn();
    }
}

fn start_game_on_ready(world: &mut World) {
    let config = {
        let cfg = world.resource::<crate::stub_storage::GameConfigRes>();
        (cfg.game_mode, cfg.rotation)
    };
    crate::start_game::start_game(
        world,
        crate::start_game::StartGameOptions {
            mode: config.0,
            rotation: config.1,
            seed: None,
        },
    );
}

fn submit_score_on_game_over(
    mut pkv: ResMut<PkvStore>,
    judge: Res<crate::judge::Judge>,
    config: Res<crate::stub_storage::GameConfigRes>,
) {
    let entry = judge.grade_entry();
    crate::hiscores::submit(&mut pkv, config.game_mode, config.rotation, entry);
}

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
        RenderLayers::layer(0),
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
        .add_plugins(render::RenderPlugin)
        .add_plugins(menu::MenuPlugin)
        .insert_resource(ClearColor(Color::srgba(0.04, 0.04, 0.07, 1.0)))
        .insert_resource(PkvStore::new("fetris", "fetris"))
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
        .add_systems(OnEnter(AppState::Ready), start_game_on_ready)
        .add_systems(OnEnter(AppState::Menu), reset_game_on_enter_menu)
        .add_systems(OnEnter(AppState::GameOver), submit_score_on_game_over)
        .add_systems(Update, systems::global_input::handle_global_input)
        .add_systems(Update, systems::post_game::return_to_menu_on_space)
        .add_systems(
            Update,
            systems::input::sample_input.run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            (
                tick_counter,
                active_phase_system,
                line_clear_delay_system,
                spawning_system,
                judge_system,
                game_over_check,
                systems::input::clear_just_pressed,
            )
                .chain()
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}
