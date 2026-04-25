use bevy::prelude::*;

mod constants;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, hello_world)
        .run();
}

fn hello_world() {
    println!("fetris bevy scaffold");
}
