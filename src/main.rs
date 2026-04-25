use bevy::prelude::*;

mod app_state;
mod components;
mod constants;
mod data;
mod judge;
mod randomizer;
mod resources;
mod rotation_system;
mod systems;

use app_state::AppState;
use crate::systems::tick::tick_counter;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_state::<AppState>()
        .add_systems(FixedUpdate, tick_counter.run_if(in_state(AppState::Playing)))
        .run();
}
