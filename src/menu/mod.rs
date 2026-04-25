use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;
pub mod controls;
pub mod hi_scores;
pub mod main_screen;
pub mod state;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // Chain so hi_scores/controls run first; if they change screen to Main,
        // main_menu_system sees the new state in the same frame (no flicker).
        app.init_resource::<state::MenuState>().add_systems(
            EguiPrimaryContextPass,
            (
                hi_scores::hi_scores_system,
                controls::controls_system,
                main_screen::main_menu_system,
            )
                .chain()
                .run_if(in_state(crate::app_state::AppState::Menu)),
        );
    }
}
