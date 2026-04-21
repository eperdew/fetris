use crate::hiscores::{self, HiScoreEntry};
use crate::rotation_system::Kind;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct GameConfig {
    pub game_mode: GameMode,
    pub rotation: Kind,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_mode: GameMode::Master,
            rotation: Kind::Ars,
        }
    }
}

impl GameConfig {
    pub fn load(storage: &crate::storage::Storage) -> Self {
        storage
            .get("game_config")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, storage: &mut crate::storage::Storage) {
        if let Ok(json) = serde_json::to_string(self) {
            storage.set("game_config", &json);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GameMode {
    Master,
    TwentyG,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuScreen {
    Main,
    HiScores,
    Controls,
}

pub struct Menu {
    screen: MenuScreen,
    cursor: usize,
    game_mode: GameMode,
    rotation: Kind,
    hi_scores_tab: usize,
    hi_scores_data: [Vec<HiScoreEntry>; 4],
}

#[derive(Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub back: bool,
}

pub enum MenuResult {
    Stay,
    StartGame { mode: GameMode, rotation: Kind },
}

impl Menu {
    pub fn new(config: GameConfig) -> Self {
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: config.game_mode,
            rotation: config.rotation,
            hi_scores_tab: 0,
            hi_scores_data: [vec![], vec![], vec![], vec![]],
        }
    }

    // Accessor methods needed by renderer and tests.
    pub fn screen(&self) -> MenuScreen {
        self.screen
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }
    pub fn rotation(&self) -> Kind {
        self.rotation
    }
    pub fn hi_scores_tab(&self) -> usize {
        self.hi_scores_tab
    }
    pub fn hi_scores_data(&self) -> &[Vec<HiScoreEntry>; 4] {
        &self.hi_scores_data
    }

    pub fn tick(&mut self, input: &MenuInput, storage: &crate::storage::Storage) -> MenuResult {
        match self.screen {
            MenuScreen::Main => self.tick_main(input, storage),
            MenuScreen::HiScores => {
                if input.back {
                    self.screen = MenuScreen::Main;
                } else if input.left {
                    self.hi_scores_tab = self.hi_scores_tab.saturating_sub(1);
                } else if input.right {
                    self.hi_scores_tab = (self.hi_scores_tab + 1).min(3);
                }
                MenuResult::Stay
            }
            MenuScreen::Controls => {
                if input.back {
                    self.screen = MenuScreen::Main;
                }
                MenuResult::Stay
            }
        }
    }

    fn tick_main(&mut self, input: &MenuInput, storage: &crate::storage::Storage) -> MenuResult {
        if input.up {
            self.cursor = self.cursor.saturating_sub(1);
        }
        if input.down {
            self.cursor = (self.cursor + 1).min(4);
        }
        match self.cursor {
            0 => {
                if input.left || input.right {
                    self.game_mode = match self.game_mode {
                        GameMode::Master => GameMode::TwentyG,
                        GameMode::TwentyG => GameMode::Master,
                    };
                }
            }
            1 => {
                if input.left || input.right {
                    self.rotation = match self.rotation {
                        Kind::Ars => Kind::Srs,
                        Kind::Srs => Kind::Ars,
                    };
                }
            }
            2 => {
                if input.confirm {
                    self.hi_scores_tab = match (self.game_mode, self.rotation) {
                        (GameMode::Master, Kind::Ars) => 0,
                        (GameMode::Master, Kind::Srs) => 1,
                        (GameMode::TwentyG, Kind::Ars) => 2,
                        (GameMode::TwentyG, Kind::Srs) => 3,
                    };
                    self.hi_scores_data = [
                        hiscores::load(storage, GameMode::Master, Kind::Ars),
                        hiscores::load(storage, GameMode::Master, Kind::Srs),
                        hiscores::load(storage, GameMode::TwentyG, Kind::Ars),
                        hiscores::load(storage, GameMode::TwentyG, Kind::Srs),
                    ];
                    self.screen = MenuScreen::HiScores;
                }
            }
            3 => {
                if input.confirm {
                    self.screen = MenuScreen::Controls;
                }
            }
            4 => {
                if input.confirm {
                    return MenuResult::StartGame {
                        mode: self.game_mode,
                        rotation: self.rotation,
                    };
                }
            }
            _ => unreachable!(),
        }
        MenuResult::Stay
    }
}
