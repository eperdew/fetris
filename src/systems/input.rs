use crate::data::GameKey;
use crate::resources::InputState;
use bevy::prelude::*;

fn keycode_to_game_key(key: &KeyCode) -> Option<GameKey> {
    match key {
        KeyCode::ArrowLeft | KeyCode::KeyH => Some(GameKey::Left),
        KeyCode::ArrowRight | KeyCode::KeyL => Some(GameKey::Right),
        KeyCode::ArrowDown | KeyCode::KeyJ => Some(GameKey::SoftDrop),
        KeyCode::Space => Some(GameKey::SonicDrop),
        KeyCode::KeyX => Some(GameKey::RotateCw),
        KeyCode::KeyZ => Some(GameKey::RotateCcw),
        _ => None,
    }
}

/// Runs in Update each frame. Updates held keys immediately; accumulates just_pressed
/// so keypresses aren't lost if FixedUpdate doesn't run every frame.
pub fn sample_input(keys: Res<ButtonInput<KeyCode>>, mut input_state: ResMut<InputState>) {
    input_state.0.held = keys.get_pressed().filter_map(keycode_to_game_key).collect();
    for key in keys.get_just_pressed().filter_map(keycode_to_game_key) {
        input_state.0.just_pressed.insert(key);
    }
}

/// Runs at the end of each FixedUpdate tick. Clears just_pressed so a keypress
/// doesn't trigger actions on multiple ticks.
pub fn clear_just_pressed(mut input_state: ResMut<InputState>) {
    input_state.0.just_pressed.clear();
}
