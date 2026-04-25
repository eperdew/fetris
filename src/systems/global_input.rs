use crate::app_state::AppState;
use crate::menu::state::{MenuScreen, MenuState};
use bevy::prelude::*;
use bevy_pkv::PkvStore;

pub fn handle_global_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut pkv: ResMut<PkvStore>,
    mut menu: ResMut<MenuState>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        let muted: bool = pkv.get("muted").unwrap_or(false);
        let _ = pkv.set("muted", &!muted);
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
