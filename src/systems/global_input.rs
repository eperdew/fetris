use crate::app_state::AppState;
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;

pub fn handle_global_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut muted: ResMut<crate::stub_storage::MutedRes>,
    mut menu: ResMut<MenuState>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        muted.0 = !muted.0;
    }
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::Menu => {
                if menu.screen == MenuScreen::Main {
                    exit.write(AppExit::Success);
                } else {
                    menu.screen = MenuScreen::Main;
                }
            }
            _ => {
                exit.write(AppExit::Success);
            }
        }
    }
}
