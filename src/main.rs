use bevy::prelude::*;

mod components;
mod constants;
mod data;
mod judge;
mod randomizer;
mod resources;
mod rotation_system;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, hello_world)
        .run();
}

fn hello_world() {
    println!("fetris bevy scaffold");
}
