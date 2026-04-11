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

    pub fn tick(&mut self, _input: &MenuInput) -> MenuResult {
        MenuResult::Stay // stub — logic added in Tasks 2–5
    }
}
