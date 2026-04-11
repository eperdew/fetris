#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameMode {
    Master,
    TwentyG,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationSystem {
    Ars,
    Srs,
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
    rotation: RotationSystem,
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
    StartGame {
        mode: GameMode,
        rotation: RotationSystem,
    },
}

impl Menu {
    pub fn new() -> Self {
        Self {
            screen: MenuScreen::Main,
            cursor: 0,
            game_mode: GameMode::Master,
            rotation: RotationSystem::Ars,
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
    pub fn rotation(&self) -> RotationSystem {
        self.rotation
    }

    pub fn tick(&mut self, input: &MenuInput) -> MenuResult {
        match self.screen {
            MenuScreen::Main => self.tick_main(input),
            MenuScreen::HiScores | MenuScreen::Controls => {
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
                        RotationSystem::Ars => RotationSystem::Srs,
                        RotationSystem::Srs => RotationSystem::Ars,
                    };
                }
            }
            2 => {
                if input.confirm {
                    self.screen = MenuScreen::HiScores;
                }
            }
            3 => {
                if input.confirm {
                    self.screen = MenuScreen::Controls;
                }
            }
            _ => {}
        }
        MenuResult::Stay
    }
}
