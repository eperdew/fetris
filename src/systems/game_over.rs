use bevy::prelude::*;
use crate::app_state::AppState;
use crate::resources::GameProgress;

pub fn game_over_check(
    progress: Res<GameProgress>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if (progress.game_over || progress.game_won) && *state.get() == AppState::Playing {
        next_state.set(AppState::GameOver);
    }
}
