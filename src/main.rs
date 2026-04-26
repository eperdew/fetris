use bevy::camera::visibility::RenderLayers;
use bevy::camera::ScalingMode;
use bevy::camera::Viewport;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPlugin, WindowResolution};
use bevy_egui::{egui, EguiContexts};
use bevy_pkv::PkvStore;

mod app_state;
mod audio;
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
#[cfg(test)]
mod snapshot;
mod start_game;

pub(crate) mod systems;

#[cfg(test)]
mod tests;

use crate::data::{GameConfig, GameEvent, JudgeEvent};
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
        let pkv = world.resource::<PkvStore>();
        let cfg: GameConfig = pkv.get("game_config").unwrap_or_default();
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
    game_mode: Res<crate::resources::GameModeRes>,
    rotation: Res<crate::resources::RotationKind>,
) {
    let entry = judge.grade_entry();
    crate::hiscores::submit(&mut pkv, game_mode.0, rotation.0, entry);
}

fn init_menu_state(mut commands: Commands, pkv: Res<PkvStore>) {
    commands.insert_resource(crate::menu::state::MenuState::new(&pkv));
}

fn setup_egui_font(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let ctx = contexts.ctx_mut().expect("egui context");
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "oxanium".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/font/Oxanium-Regular.ttf"
        ))),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "oxanium".to_owned());
    ctx.set_fonts(fonts);
    *done = true;
}

#[derive(Component)]
struct MainCamera;

fn update_camera_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, With<MainCamera>>,
    mut pixel_scale: ResMut<crate::resources::PixelScale>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    let win_w = window.physical_width() as f32;
    let win_h = window.physical_height() as f32;
    if win_w == 0.0 || win_h == 0.0 {
        return;
    }
    const GAME_W: f32 = 560.0;
    const GAME_H: f32 = 780.0;
    let (vp_w, vp_h) = if win_w / win_h > GAME_W / GAME_H {
        let h = win_h;
        (h * GAME_W / GAME_H, h)
    } else {
        let w = win_w;
        (w, w * GAME_H / GAME_W)
    };
    pixel_scale.0 = vp_h / GAME_H;
    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(
            ((win_w - vp_w) / 2.0).round() as u32,
            ((win_h - vp_h) / 2.0).round() as u32,
        ),
        physical_size: UVec2::new(vp_w.round() as u32, vp_h.round() as u32),
        depth: 0.0..1.0,
    });
}

fn setup_camera(mut commands: Commands, mut egui_settings: ResMut<bevy_egui::EguiGlobalSettings>) {
    // Disable auto-context so the overlay render-to-texture camera (spawned in
    // RenderPlugin::build) doesn't steal the primary egui context.
    egui_settings.auto_create_primary_context = false;

    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: 560.0,
        height: 780.0,
    };
    projection.viewport_origin = Vec2::new(0.0, 1.0); // top-left origin
    commands.spawn((
        Camera2d,
        MainCamera,
        Projection::Orthographic(projection),
        Transform::from_scale(Vec3::new(1.0, -1.0, 1.0)),
        RenderLayers::layer(0),
        bevy_egui::PrimaryEguiContext,
    ));
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let window = Window {
        title: "fetris".into(),
        mode: bevy::window::WindowMode::BorderlessFullscreen(
            bevy::window::MonitorSelection::Current,
        ),
        ..default()
    };
    #[cfg(target_arch = "wasm32")]
    let window = Window {
        title: "fetris".into(),
        resolution: WindowResolution::new(560, 780),
        resizable: false,
        ..default()
    };
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..default()
    });

    // WebGL2 is the only viable backend for broad browser compatibility.
    // Explicitly force it so wgpu doesn't try WebGPU features WebGL2 lacks.
    #[cfg(target_arch = "wasm32")]
    let plugins = {
        use bevy::asset::{AssetMetaCheck, AssetPlugin};
        use bevy::render::{
            settings::{Backends, RenderCreation, WgpuSettings, WgpuSettingsPriority},
            RenderPlugin,
        };
        plugins
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::GL),
                    priority: WgpuSettingsPriority::WebGL2,
                    ..default()
                }),
                ..default()
            })
    };

    App::new()
        .add_plugins(plugins)
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
        .init_resource::<crate::resources::PixelScale>()
        .init_resource::<crate::randomizer::Randomizer>()
        .init_resource::<Judge>()
        // TODO: inserted by start_game (Task 17): NextPiece, RotationSystemRes, GameModeRes, RotationKind
        .add_systems(Startup, (setup_camera, init_menu_state, audio::setup_audio))
        .add_systems(
            OnEnter(AppState::Ready),
            (start_game_on_ready, audio::play_ready_sound),
        )
        .add_systems(OnEnter(AppState::Menu), reset_game_on_enter_menu)
        .add_systems(OnEnter(AppState::GameOver), submit_score_on_game_over)
        .add_systems(Update, setup_egui_font)
        .add_systems(Update, update_camera_viewport)
        .add_systems(Update, systems::global_input::handle_global_input)
        .add_systems(Update, systems::post_game::return_to_menu_on_space)
        .add_systems(
            Update,
            systems::input::sample_input.run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            audio::audio_event_system.run_if(in_state(AppState::Playing)),
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
