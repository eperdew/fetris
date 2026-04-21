mod constants;
mod game;
mod hiscores;
mod input;
mod judge;
mod menu;
mod piece;
mod randomizer;
mod renderer;
mod rotation_system;
mod storage;
#[cfg(test)]
mod tests;

use game::Game;
use input::{GameKey, InputState};
use macroquad::prelude::*;
use menu::{Menu, MenuInput, MenuResult, MenuScreen};
use std::collections::HashSet;

enum AppState {
    Menu(Menu),
    Playing(Game),
}

fn window_conf() -> Conf {
    Conf {
        window_title: String::from("fetris"),
        window_width: 530,
        window_height: 780,
        window_resizable: false,
        ..Default::default()
    }
}

fn build_input_state() -> InputState {
    let mappings: &[(KeyCode, GameKey)] = &[
        (KeyCode::Left, GameKey::Left),
        (KeyCode::H, GameKey::Left),
        (KeyCode::Right, GameKey::Right),
        (KeyCode::L, GameKey::Right),
        (KeyCode::Down, GameKey::SoftDrop),
        (KeyCode::J, GameKey::SoftDrop),
        (KeyCode::Space, GameKey::SonicDrop),
        (KeyCode::X, GameKey::RotateCw),
        (KeyCode::Z, GameKey::RotateCcw),
    ];
    let mut held = HashSet::new();
    let mut just_pressed = HashSet::new();
    for &(kc, gk) in mappings {
        if is_key_down(kc) {
            held.insert(gk);
        }
        if is_key_pressed(kc) {
            just_pressed.insert(gk);
        }
    }
    InputState { held, just_pressed }
}

fn build_menu_input() -> MenuInput {
    MenuInput {
        up: is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::K),
        down: is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::J),
        left: is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::H),
        right: is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::L),
        confirm: is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter),
        back: is_key_pressed(KeyCode::Backspace),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    macroquad::rand::srand(miniquad::date::now().to_bits());
    let renderer = renderer::Renderer::new();
    let mut state = AppState::Menu(Menu::new());
    let mut accumulator = 0.0f64;
    let mut pending_just_pressed: HashSet<GameKey> = HashSet::new();
    const TICK: f64 = 1.0 / 60.0;

    loop {
        let escape = is_key_pressed(KeyCode::Escape);
        let mut new_state: Option<AppState> = None;

        match &mut state {
            AppState::Menu(menu) => {
                // Escape on the main screen quits; on a sub-screen it goes back.
                let mut input = build_menu_input();
                if escape {
                    if menu.screen() == MenuScreen::Main {
                        break;
                    } else {
                        input.back = true;
                    }
                }
                if let MenuResult::StartGame { mode, rotation } = menu.tick(&input) {
                    new_state = Some(AppState::Playing(Game::new(
                        mode,
                        rotation,
                        rotation.create(),
                    )));
                }
                renderer.render_menu(menu);
            }
            AppState::Playing(game) => {
                if escape {
                    break;
                }
                accumulator += get_frame_time() as f64;
                let frame_input = build_input_state();
                pending_just_pressed.extend(&frame_input.just_pressed);
                while accumulator >= TICK {
                    let input = InputState {
                        held: frame_input.held.clone(),
                        just_pressed: std::mem::take(&mut pending_just_pressed),
                    };
                    game.tick(&input);
                    accumulator -= TICK;
                }
                // Submit score exactly once on game end
                if (game.game_over || game.game_won) && !game.score_submitted {
                    hiscores::submit(game.game_mode, game.rotation_kind, game.judge.grade_entry());
                    game.score_submitted = true;
                }
                if (game.game_over || game.game_won) && is_key_pressed(KeyCode::Space) {
                    new_state = Some(AppState::Menu(Menu::new()));
                }
                renderer.render(game);
            }
        }

        if let Some(s) = new_state {
            if matches!(s, AppState::Playing(_)) {
                accumulator = 0.0;
                pending_just_pressed.clear();
            }
            state = s;
        }

        next_frame().await;
    }
}
