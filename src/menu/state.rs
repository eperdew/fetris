use crate::data::{GameMode, Kind};
use bevy::prelude::*;
use bevy_pkv::PkvStore;

#[derive(Resource)]
pub struct MenuState {
    pub screen: MenuScreen,
    pub cursor: usize,
    pub game_mode: GameMode,
    pub rotation: Kind,
    pub hi_scores_tab: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuScreen {
    Main,
    HiScores,
    Controls,
}

impl MenuState {
    pub fn new(pkv: &PkvStore) -> Self {
        let config: crate::data::GameConfig = pkv.get("game_config").unwrap_or_default();
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: config.game_mode,
            rotation: config.rotation,
            hi_scores_tab: 0,
        }
    }
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: GameMode::Master,
            rotation: Kind::Ars,
            hi_scores_tab: 0,
        }
    }
}
