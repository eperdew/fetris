use crate::app_state::AppState;
use crate::components::{ActivePiece, PieceKindComp};
use crate::data::{GameEvent, PieceKind};
use crate::menu::debug::DebugSceneState;
use crate::resources::{Board, NextPiece};
use bevy::ecs::message::Messages;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

fn debug_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_message::<GameEvent>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<crate::resources::Board>()
        .init_resource::<crate::resources::CurrentPhase>()
        .init_resource::<crate::resources::GameProgress>()
        .init_resource::<crate::resources::PendingCompaction>()
        .init_resource::<crate::judge::Judge>()
        .init_resource::<crate::menu::state::MenuState>()
        .add_systems(OnEnter(AppState::Debug), crate::menu::debug::on_enter_debug)
        .add_systems(
            Update,
            crate::menu::debug::debug_input_system.run_if(in_state(AppState::Debug)),
        )
        .add_systems(
            FixedUpdate,
            crate::menu::debug::debug_tick_system.run_if(in_state(AppState::Debug)),
        );
    app
}

fn enter_debug(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Debug);
    app.update();
}

#[test]
fn entering_debug_spawns_t_piece() {
    let mut app = debug_app();
    enter_debug(&mut app);
    let world = app.world_mut();
    let mut q = world.query_filtered::<&PieceKindComp, With<ActivePiece>>();
    let kind = q.single(world).expect("ActivePiece").0;
    assert_eq!(kind, PieceKind::T);
    assert_eq!(world.resource::<NextPiece>().0, PieceKind::T);
    assert!(world.contains_resource::<DebugSceneState>());
}

#[test]
fn digit4_emits_fetris_line_clear() {
    let mut app = debug_app();
    enter_debug(&mut app);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Digit4);
    }
    app.update();
    let world = app.world_mut();
    let messages = world.resource::<Messages<GameEvent>>();
    let mut cursor = messages.get_cursor();
    let evs: Vec<_> = cursor.read(messages).copied().collect();
    assert_eq!(evs, vec![GameEvent::LineClear { count: 4 }]);
    let board = &world.resource::<Board>().0;
    let bottom = crate::data::BOARD_ROWS - 1;
    assert!(board[bottom][0].is_some());
}

#[test]
fn backspace_returns_to_menu() {
    let mut app = debug_app();
    enter_debug(&mut app);
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::Backspace);
    }
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::Menu
    );
}
