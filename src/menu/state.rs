use bevy::prelude::*;
use crate::data::{GameMode, Kind};

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
