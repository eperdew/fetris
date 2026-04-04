mod constants;
mod game;
mod input;
mod piece;
mod randomizer;
mod renderer;
#[cfg(test)]
mod tests;

use std::collections::HashSet;
use macroquad::prelude::*;
use game::Game;
use input::{GameKey, InputState};


fn window_conf() -> Conf {
    Conf {
        window_title: String::from("fetris"),
        window_width: 530,
        window_height: 680,
        window_resizable: false,
        ..Default::default()
    }
}

fn build_input_state() -> InputState {
    let mappings: &[(KeyCode, GameKey)] = &[
        (KeyCode::Left,  GameKey::Left),
        (KeyCode::H,     GameKey::Left),
        (KeyCode::Right, GameKey::Right),
        (KeyCode::L,     GameKey::Right),
        (KeyCode::Down,  GameKey::SoftDrop),
        (KeyCode::J,     GameKey::SoftDrop),
        (KeyCode::Space, GameKey::SonicDrop),
        (KeyCode::X,     GameKey::RotateCw),
        (KeyCode::Z,     GameKey::RotateCcw),
    ];
    let mut held = HashSet::new();
    let mut just_pressed = HashSet::new();
    for &(kc, gk) in mappings {
        if is_key_down(kc)     { held.insert(gk); }
        if is_key_pressed(kc)  { just_pressed.insert(gk); }
    }
    InputState { held, just_pressed }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    loop {
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            break;
        }
        let input = build_input_state();
        game.tick(&input);
        renderer::render(&game);
        next_frame().await;
    }
}
