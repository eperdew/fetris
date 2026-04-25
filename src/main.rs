use bevy::prelude::*;

mod app_state;
mod components;
mod constants;
mod data;
mod judge;
mod randomizer;
mod resources;
mod rotation_system;

use app_state::AppState;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .init_state::<AppState>()
        .run();
}
