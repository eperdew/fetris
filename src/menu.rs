use crate::hiscores::{self, HiScoreEntry};
use crate::rotation_system::Kind;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn new() -> Self {
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: GameMode::Master,
            rotation: Kind::Ars,
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

    pub fn tick(&mut self, input: &MenuInput) -> MenuResult {
        match self.screen {
            MenuScreen::Main => self.tick_main(input),
            MenuScreen::HiScores => {
                if input.back {
                    self.screen = MenuScreen::Main;
                }
                if input.left {
                    self.hi_scores_tab = self.hi_scores_tab.saturating_sub(1);
                }
                if input.right {
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

    fn tick_main(&mut self, input: &MenuInput) -> MenuResult {
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
                    self.hi_scores_data = [
                        hiscores::load(GameMode::Master, Kind::Ars),
                        hiscores::load(GameMode::Master, Kind::Srs),
                        hiscores::load(GameMode::TwentyG, Kind::Ars),
                        hiscores::load(GameMode::TwentyG, Kind::Srs),
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
