use bevy::prelude::*;
use crate::app_state::AppState;

pub fn return_to_menu_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::GameOver && keys.just_pressed(KeyCode::Space) {
        next_state.set(AppState::Menu);
    }
}
