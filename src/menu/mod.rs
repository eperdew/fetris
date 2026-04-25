use bevy::prelude::*;
pub mod controls;
pub mod hi_scores;
pub mod main_screen;
pub mod state;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<state::MenuState>().add_systems(
            Update,
            (
                main_screen::main_menu_system,
                hi_scores::hi_scores_system,
                controls::controls_system,
            )
                .run_if(in_state(crate::app_state::AppState::Menu)),
        );
    }
}
