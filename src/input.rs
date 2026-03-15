use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy)]
pub enum GameAction {
    MoveLeft,
    MoveRight,
    MoveDown,
    RotateCw,
    RotateCcw,
    HardDrop,
}

pub fn map_key(code: KeyCode) -> Option<GameAction> {
    match code {
        KeyCode::Left | KeyCode::Char('h') => Some(GameAction::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(GameAction::MoveRight),
        KeyCode::Down | KeyCode::Char('j') => Some(GameAction::MoveDown),
        KeyCode::Char(' ') => Some(GameAction::HardDrop),
        KeyCode::Char('x') => Some(GameAction::RotateCw),
        KeyCode::Char('z') => Some(GameAction::RotateCcw),
        _ => None,
    }
}
